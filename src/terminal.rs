// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


use std::collections::VecDeque;
use std::env;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{
    Child,
    Command,
    ExitStatus,
    Stdio,
};
use std::sync::mpsc::{
    Receiver,
    SyncSender,
    TryRecvError,
    sync_channel,
};
use std::thread;

use sdl3::EventSubsystem;
use sdl3::event::EventSender;

use crate::piece_table::PieceTable;


const MAX_OUTPUT_BYTES: usize =
    8 * 1024 * 1024;
const OUTPUT_TRIM_TARGET: usize =
    512 * 1024;
const MAX_COMMAND_HISTORY: usize = 100;
const READ_BUFFER_SIZE: usize = 16 * 1024;
const OUTPUT_QUEUE_CHUNKS: usize = 32;


#[derive(Debug)]
pub(crate) enum TerminalEvent {
    OutputReady,
    StreamClosed,
    ReaderError(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TerminalAction {
    None,
    Edit(String),
    View(String),
}

struct RunningProcess {
    child: Child,
    open_streams: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnsiState {
    Ground,
    Escape,
    Csi,
    Osc,
    OscEscape,
}

pub(crate) struct Terminal {
    active: bool,
    output: PieceTable,
    input: String,
    cursor: usize,
    cwd: PathBuf,
    history: VecDeque<String>,
    history_position: Option<usize>,
    history_draft: String,
    running: Option<RunningProcess>,
    last_command: Option<String>,
    status: Option<String>,
    scroll_back: usize,
    ansi_state: AnsiState,
    pending_utf8: Vec<u8>,
    pending_carriage_return: bool,
    output_sender: SyncSender<Vec<u8>>,
    output_receiver: Receiver<Vec<u8>>,
}

impl Terminal {
    pub fn new(
        initial_directory: PathBuf,
    ) -> io::Result<Self> {
        let cwd =
            normalize_initial_directory(
                initial_directory
            );
        let (output_sender, output_receiver) =
            sync_channel(OUTPUT_QUEUE_CHUNKS);

        let mut terminal = Self {
            active: false,
            output: PieceTable::empty()?,
            input: String::new(),
            cursor: 0,
            cwd,
            history: VecDeque::new(),
            history_position: None,
            history_draft: String::new(),
            running: None,
            last_command: None,
            status: None,
            scroll_back: 0,
            ansi_state: AnsiState::Ground,
            pending_utf8: Vec::new(),
            pending_carriage_return: false,
            output_sender,
            output_receiver,
        };

        terminal.append_text(
            "Pötyi command terminal\n\
Built-ins: cd, pwd, edit, view, clear, help, exit\n\
Ctrl+` switches between terminal and editor\n"
        )?;

        Ok(terminal)
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn open(
        &mut self,
        _document_path: Option<&Path>,
    ) {
        // The working directory belongs to the terminal session. Returning
        // from the editor must not undo a directory selected with `cd`.
        self.active = true;
        self.status = None;
        self.scroll_back = 0;
    }

    pub fn close_to_editor(&mut self) {
        self.active = false;
        self.status = None;
    }

    pub fn toggle(
        &mut self,
        document_path: Option<&Path>,
    ) {
        if self.active {
            self.close_to_editor();
        } else {
            self.open(document_path);
        }
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn prompt(&self) -> String {
        format!(
            "{} >",
            compact_path(&self.cwd),
        )
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn set_status(
        &mut self,
        status: impl Into<String>,
    ) {
        self.status = Some(status.into());
    }

    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    pub fn output_mut(&mut self) -> &mut PieceTable {
        &mut self.output
    }

    pub fn output_text(&self) -> io::Result<String> {
        let bytes = self.output.read_range(
            0,
            self.output.len(),
        )?;

        String::from_utf8(bytes).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Terminal output is not valid UTF-8: {error}"
                ),
            )
        })
    }

    pub fn scroll_back(&self) -> usize {
        self.scroll_back
    }

    pub fn set_cursor(
        &mut self,
        position: usize,
    ) {
        let mut position =
            position.min(self.input.len());

        while position > 0
            && !self.input
                .is_char_boundary(position)
        {
            position -= 1;
        }

        self.cursor = position;
    }

    pub fn insert_text(
        &mut self,
        text: &str,
    ) {
        if text.is_empty() {
            return;
        }

        let single_line =
            text.replace(['\r', '\n'], " ");

        self.input.insert_str(
            self.cursor,
            &single_line,
        );

        self.cursor += single_line.len();
        self.reset_history_navigation();
        self.status = None;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let previous =
            previous_char_boundary(
                &self.input,
                self.cursor,
            );

        self.input.drain(
            previous..self.cursor
        );
        self.cursor = previous;
        self.reset_history_navigation();
        self.status = None;
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }

        let next = next_char_boundary(
            &self.input,
            self.cursor,
        );

        self.input.drain(
            self.cursor..next
        );
        self.reset_history_navigation();
        self.status = None;
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor =
                previous_char_boundary(
                    &self.input,
                    self.cursor,
                );
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.input.len() {
            self.cursor =
                next_char_boundary(
                    &self.input,
                    self.cursor,
                );
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.input.len();
    }

    pub fn history_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }

        let position =
            match self.history_position {
                Some(position) =>
                    position.saturating_sub(1),

                None => {
                    self.history_draft =
                        self.input.clone();
                    self.history.len() - 1
                }
            };

        self.history_position = Some(position);
        self.input = self.history[position]
            .clone();
        self.cursor = self.input.len();
    }

    pub fn history_next(&mut self) {
        let Some(position) =
            self.history_position
        else {
            return;
        };

        if position + 1 < self.history.len() {
            let next = position + 1;
            self.history_position = Some(next);
            self.input = self.history[next]
                .clone();
        } else {
            self.history_position = None;
            self.input =
                std::mem::take(
                    &mut self.history_draft
                );
        }

        self.cursor = self.input.len();
    }

    pub fn scroll(
        &mut self,
        lines: isize,
    ) {
        if lines > 0 {
            self.scroll_back =
                self.scroll_back
                    .saturating_add(
                        lines as usize
                    );
        } else {
            self.scroll_back =
                self.scroll_back
                    .saturating_sub(
                        lines.unsigned_abs()
                    );
        }
    }

    pub fn clear(&mut self) -> io::Result<()> {
        self.output = PieceTable::empty()?;
        self.scroll_back = 0;
        self.status = None;
        Ok(())
    }

    pub fn submit(
        &mut self,
        events: &EventSubsystem,
    ) -> io::Result<TerminalAction> {
        if self.running.is_some() {
            self.status = Some(
                "A command is already running"
                    .to_string()
            );
            return Ok(TerminalAction::None);
        }

        let command =
            self.input.trim().to_string();

        self.input.clear();
        self.cursor = 0;
        self.reset_history_navigation();

        if command.is_empty() {
            return Ok(TerminalAction::None);
        }

        self.remember_command(&command);
        self.last_command = Some(command.clone());
        self.append_text(
            &format!("$ {command}\n")
        )?;
        self.scroll_back = 0;

        let (name, arguments) =
            split_command(&command);

        match name {
            "cd" => {
                self.change_directory(arguments)?;
                Ok(TerminalAction::None)
            }

            "pwd" if arguments.trim().is_empty() => {
                self.append_text(
                    &format!(
                        "{}\n",
                        self.cwd.display(),
                    )
                )?;
                Ok(TerminalAction::None)
            }

            "clear" if arguments.trim().is_empty() => {
                self.clear()?;
                Ok(TerminalAction::None)
            }

            "help" if arguments.trim().is_empty() => {
                self.append_text(
                    "cd PATH        change directory\n\
pwd            show current directory\n\
edit PATH[:LINE[:COLUMN]]  edit a file\n\
view PATH[:LINE[:COLUMN]]  open read-only\n\
clear          clear terminal output\n\
exit           return to the editor\n"
                )?;
                Ok(TerminalAction::None)
            }

            "exit" | "editor"
                if arguments.trim().is_empty() =>
            {
                self.close_to_editor();
                Ok(TerminalAction::None)
            }

            "edit" => Ok(
                self.file_action(
                    arguments,
                    false,
                )?
            ),

            "view" => Ok(
                self.file_action(
                    arguments,
                    true,
                )?
            ),

            _ => {
                self.spawn_command(
                    &command,
                    events,
                )?;
                Ok(TerminalAction::None)
            }
        }
    }

    pub fn run_again(
        &mut self,
        events: &EventSubsystem,
    ) -> io::Result<TerminalAction> {
        if self.running.is_some() {
            return Ok(TerminalAction::None);
        }

        let Some(command) =
            self.last_command.clone()
        else {
            self.status = Some(
                "No command to run again"
                    .to_string()
            );
            return Ok(TerminalAction::None);
        };

        self.input = command;
        self.cursor = self.input.len();
        self.submit(events)
    }

    pub fn stop(&mut self) -> io::Result<()> {
        let Some(process) =
            self.running.as_mut()
        else {
            self.status = Some(
                "No command is running"
                    .to_string()
            );
            return Ok(());
        };

        process.child.kill()?;
        self.status = Some(
            "Stopping command…".to_string()
        );
        Ok(())
    }

    pub fn handle_event(
        &mut self,
        event: TerminalEvent,
    ) -> io::Result<()> {
        match event {
            TerminalEvent::OutputReady => {
                loop {
                    let bytes = match self
                        .output_receiver
                        .try_recv()
                    {
                        Ok(bytes) => bytes,
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) =>
                            break,
                    };

                    let line_count = bytes.iter()
                        .filter(|byte| {
                            **byte == b'\n'
                        })
                        .count();

                    if self.scroll_back > 0 {
                        self.scroll_back =
                            self.scroll_back
                                .saturating_add(
                                    line_count
                                );
                    }

                    let text =
                        self.clean_output(&bytes);

                    self.append_text(&text)?;
                }
            }

            TerminalEvent::ReaderError(error) => {
                self.append_text(
                    &format!(
                        "[output error: {error}]\n"
                    )
                )?;
            }

            TerminalEvent::StreamClosed => {
                let should_finish =
                    match self.running.as_mut() {
                        Some(process) => {
                            process.open_streams =
                                process.open_streams
                                    .saturating_sub(1);

                            process.open_streams == 0
                        }

                        None => false,
                    };

                if should_finish {
                    self.finish_process()?;
                }
            }
        }

        Ok(())
    }

    pub fn resolve_path(
        &self,
        value: &str,
    ) -> io::Result<PathBuf> {
        let value =
            unquote_argument(value.trim())?;

        if value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "A file path is required",
            ));
        }

        let path = PathBuf::from(
            expand_home(&value)
        );

        if path.is_absolute() {
            Ok(path)
        } else {
            Ok(self.cwd.join(path))
        }
    }

    fn file_action(
        &mut self,
        arguments: &str,
        read_only: bool,
    ) -> io::Result<TerminalAction> {
        let value =
            unquote_argument(arguments.trim())?;

        if value.is_empty() {
            self.status = Some(
                if read_only {
                    "Usage: view path"
                } else {
                    "Usage: edit path"
                }
                .to_string()
            );
            return Ok(TerminalAction::None);
        }

        Ok(if read_only {
            TerminalAction::View(value)
        } else {
            TerminalAction::Edit(value)
        })
    }

    fn change_directory(
        &mut self,
        arguments: &str,
    ) -> io::Result<()> {
        let value =
            unquote_argument(arguments.trim())?;

        let target =
            if value.is_empty() {
                home_directory()
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::NotFound,
                            "Home directory is unavailable",
                        )
                    })?
            } else {
                let path = PathBuf::from(
                    expand_home(&value)
                );

                if path.is_absolute() {
                    path
                } else {
                    self.cwd.join(path)
                }
            };

        let canonical = target.canonicalize()?;

        if !canonical.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Not a directory: {}",
                    canonical.display(),
                ),
            ));
        }

        self.cwd = canonical;
        self.status = None;

        Ok(())
    }

    fn spawn_command(
        &mut self,
        command: &str,
        events: &EventSubsystem,
    ) -> io::Result<()> {
        let mut child =
            spawn_shell(command, &self.cwd)?;

        let stdout = child.stdout.take()
            .ok_or_else(|| {
                io::Error::other(
                    "Command stdout was not captured"
                )
            })?;

        let stderr = child.stderr.take()
            .ok_or_else(|| {
                io::Error::other(
                    "Command stderr was not captured"
                )
            })?;

        spawn_output_reader(
            stdout,
            self.output_sender.clone(),
            events.event_sender(),
        );

        spawn_output_reader(
            stderr,
            self.output_sender.clone(),
            events.event_sender(),
        );

        self.running = Some(
            RunningProcess {
                child,
                open_streams: 2,
            }
        );

        self.status = Some(
            "Running…".to_string()
        );

        Ok(())
    }

    fn finish_process(&mut self) -> io::Result<()> {
        let Some(mut process) =
            self.running.take()
        else {
            return Ok(());
        };

        let status = process.child.wait()?;
        self.flush_pending_output()?;
        self.ensure_output_newline()?;
        self.append_text(
            &format_exit_status(status)
        )?;

        self.status = None;
        Ok(())
    }

    fn clean_output(
        &mut self,
        bytes: &[u8],
    ) -> String {
        let mut clean = Vec::with_capacity(
            bytes.len()
        );

        for byte in bytes.iter().copied() {
            match self.ansi_state {
                AnsiState::Ground => {
                    if byte == 0x1b {
                        self.ansi_state =
                            AnsiState::Escape;
                    } else if byte == b'\r' {
                        self.pending_carriage_return =
                            true;
                    } else {
                        if self.pending_carriage_return {
                            if byte != b'\n' {
                                clean.push(b'\n');
                            }

                            self.pending_carriage_return =
                                false;
                        }

                        if byte == b'\n'
                            || byte == b'\t'
                            || byte >= 0x20
                        {
                            clean.push(byte);
                        }
                    }
                }

                AnsiState::Escape => {
                    self.ansi_state =
                        match byte {
                            b'[' => AnsiState::Csi,
                            b']' => AnsiState::Osc,
                            _ => AnsiState::Ground,
                        };
                }

                AnsiState::Csi => {
                    if (0x40..=0x7e)
                        .contains(&byte)
                    {
                        self.ansi_state =
                            AnsiState::Ground;
                    }
                }

                AnsiState::Osc => {
                    if byte == 0x07 {
                        self.ansi_state =
                            AnsiState::Ground;
                    } else if byte == 0x1b {
                        self.ansi_state =
                            AnsiState::OscEscape;
                    }
                }

                AnsiState::OscEscape => {
                    self.ansi_state =
                        if byte == b'\\' {
                            AnsiState::Ground
                        } else {
                            AnsiState::Osc
                        };
                }
            }
        }

        self.pending_utf8.extend(clean);

        let valid_length =
            match std::str::from_utf8(
                &self.pending_utf8
            ) {
                Ok(_) => self.pending_utf8.len(),

                Err(error) => {
                    if error.error_len().is_none() {
                        error.valid_up_to()
                    } else {
                        self.pending_utf8.len()
                    }
                }
            };

        let remaining =
            self.pending_utf8
                .split_off(valid_length);

        let text =
            String::from_utf8_lossy(
                &self.pending_utf8
            )
            .into_owned();

        self.pending_utf8 = remaining;
        text
    }

    fn flush_pending_output(
        &mut self,
    ) -> io::Result<()> {
        if self.pending_carriage_return {
            self.append_text("\n")?;
            self.pending_carriage_return = false;
        }

        if !self.pending_utf8.is_empty() {
            let text =
                String::from_utf8_lossy(
                    &self.pending_utf8
                )
                .into_owned();

            self.pending_utf8.clear();
            self.append_text(&text)?;
        }

        Ok(())
    }

    fn append_text(
        &mut self,
        text: &str,
    ) -> io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        self.output.insert(
            self.output.len(),
            text,
        )?;

        self.trim_output()
    }

    fn trim_output(&mut self) -> io::Result<()> {
        if self.output.len()
            <= MAX_OUTPUT_BYTES
        {
            return Ok(());
        }

        let desired =
            self.output.len()
                .saturating_sub(
                    MAX_OUTPUT_BYTES
                        - OUTPUT_TRIM_TARGET
                );

        let sample_length =
            desired.saturating_add(4096)
                .min(self.output.len());

        let sample =
            self.output.read_range(
                0,
                sample_length,
            )?;

        let remove = sample
            .iter()
            .enumerate()
            .skip(desired)
            .find(|(_, byte)| **byte == b'\n')
            .map(|(index, _)| index + 1)
            .unwrap_or_else(|| {
                let mut boundary =
                    desired.min(sample.len());

                while boundary < sample.len()
                    && sample[boundary] & 0b1100_0000
                        == 0b1000_0000
                {
                    boundary += 1;
                }

                boundary
            });

        self.output.delete(0, remove)?;
        Ok(())
    }

    fn ensure_output_newline(
        &mut self,
    ) -> io::Result<()> {
        if self.output.len() == 0 {
            return Ok(());
        }

        if self.output.byte_at(
            self.output.len() - 1
        )? != Some(b'\n')
        {
            self.append_text("\n")?;
        }

        Ok(())
    }

    fn remember_command(
        &mut self,
        command: &str,
    ) {
        if self.history.back()
            .map(String::as_str)
            == Some(command)
        {
            return;
        }

        if self.history.len()
            == MAX_COMMAND_HISTORY
        {
            self.history.pop_front();
        }

        self.history.push_back(
            command.to_string()
        );
    }

    fn reset_history_navigation(&mut self) {
        self.history_position = None;
        self.history_draft.clear();
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Some(process) =
            self.running.as_mut()
        {
            let _ = process.child.kill();
            let _ = process.child.wait();
        }
    }
}

fn spawn_output_reader<R>(
    mut reader: R,
    output: SyncSender<Vec<u8>>,
    sender: EventSender,
) where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer =
            vec![0u8; READ_BUFFER_SIZE];

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,

                Ok(length) => {
                    if output.send(
                        buffer[..length].to_vec()
                    ).is_err()
                    {
                        break;
                    }

                    if sender.push_custom_event(
                        TerminalEvent::OutputReady
                    ).is_err()
                    {
                        break;
                    }
                }

                Err(error)
                    if error.kind()
                        == io::ErrorKind::Interrupted =>
                {
                    continue;
                }

                Err(error) => {
                    let _ = sender
                        .push_custom_event(
                            TerminalEvent::ReaderError(
                                error.to_string()
                            )
                        );
                    break;
                }
            }
        }

        let _ = sender.push_custom_event(
            TerminalEvent::StreamClosed
        );
    });
}

#[cfg(not(windows))]
fn spawn_shell(
    command: &str,
    cwd: &Path,
) -> io::Result<Child> {
    let shell = env::var_os("SHELL")
        .unwrap_or_else(|| "/bin/sh".into());

    let mut process = Command::new(shell);
    configure_process(
        process.arg("-lc").arg(command),
        cwd,
    );
    process.spawn()
}

#[cfg(windows)]
fn spawn_shell(
    command: &str,
    cwd: &Path,
) -> io::Result<Child> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let shell = env::var_os("COMSPEC")
        .unwrap_or_else(|| "cmd.exe".into());

    let mut process = Command::new(shell);
    process.creation_flags(CREATE_NO_WINDOW);
    configure_process(
        process
            .arg("/D")
            .arg("/S")
            .arg("/C")
            .arg(command),
        cwd,
    );
    process.spawn()
}

fn configure_process(
    process: &mut Command,
    cwd: &Path,
) {
    process
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("TERM", "dumb")
        .env("NO_COLOR", "1")
        .env("PAGER", "cat")
        .env("GIT_PAGER", "cat");
}

fn format_exit_status(
    status: ExitStatus,
) -> String {
    match status.code() {
        Some(0) =>
            "[finished successfully]\n"
                .to_string(),

        Some(code) => format!(
            "[finished with exit code {code}]\n"
        ),

        None =>
            "[command terminated]\n"
                .to_string(),
    }
}

fn split_command(
    command: &str,
) -> (&str, &str) {
    let end = command
        .find(char::is_whitespace)
        .unwrap_or(command.len());

    (
        &command[..end],
        command[end..].trim_start(),
    )
}

fn unquote_argument(
    value: &str,
) -> io::Result<String> {
    if value.len() >= 2 {
        let first = value.chars().next();
        let last = value.chars().next_back();

        if first == last
            && matches!(first, Some('"' | '\''))
        {
            return Ok(
                value[1..value.len() - 1]
                    .to_string()
            );
        }
    }

    if value.starts_with('"')
        || value.starts_with('\'')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Close the quoted path",
        ));
    }

    Ok(value.to_string())
}

fn normalize_initial_directory(
    path: PathBuf,
) -> PathBuf {
    let directory =
        if path.is_dir() {
            path
        } else {
            path.parent()
                .map(Path::to_path_buf)
                .unwrap_or(path)
        };

    directory.canonicalize()
        .unwrap_or(directory)
}

fn home_directory() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| {
            env::var_os("USERPROFILE")
        })
        .map(PathBuf::from)
}

fn expand_home(value: &str) -> PathBuf {
    if value == "~" {
        return home_directory()
            .unwrap_or_else(|| {
                PathBuf::from(value)
            });
    }

    if let Some(rest) =
        value.strip_prefix("~/")
            .or_else(|| {
                value.strip_prefix("~\\")
            })
        && let Some(home) = home_directory()
    {
        return home.join(rest);
    }

    PathBuf::from(value)
}

fn compact_path(path: &Path) -> String {
    if let Some(home) = home_directory()
        && let Ok(relative) =
            path.strip_prefix(home)
    {
        if relative.as_os_str().is_empty() {
            return "~".to_string();
        }

        return format!(
            "~/{}",
            relative.display(),
        );
    }

    path.display().to_string()
}

fn previous_char_boundary(
    text: &str,
    position: usize,
) -> usize {
    text[..position]
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_char_boundary(
    text: &str,
    position: usize,
) -> usize {
    text[position..]
        .char_indices()
        .nth(1)
        .map(|(offset, _)| {
            position + offset
        })
        .unwrap_or(text.len())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_editing_is_utf8_safe() {
        let cwd = env::current_dir().unwrap();
        let mut terminal =
            Terminal::new(cwd).unwrap();

        terminal.insert_text("echo é🙂");
        terminal.move_left();
        terminal.backspace();

        assert_eq!(
            terminal.input(),
            "echo 🙂",
        );
    }

    #[test]
    fn history_is_bounded_and_skips_duplicates() {
        let cwd = env::current_dir().unwrap();
        let mut terminal =
            Terminal::new(cwd).unwrap();

        for index in 0..150 {
            terminal.remember_command(
                &format!("echo {index}")
            );
        }

        terminal.remember_command("echo 149");

        assert_eq!(terminal.history.len(), 100);
        assert_eq!(
            terminal.history.front()
                .map(String::as_str),
            Some("echo 50"),
        );
    }

    #[test]
    fn ansi_and_split_utf8_are_cleaned_incrementally() {
        let cwd = env::current_dir().unwrap();
        let mut terminal =
            Terminal::new(cwd).unwrap();

        let first = terminal.clean_output(
            b"\x1b[31mred\x1b[0m \xc3"
        );
        let second = terminal.clean_output(
            b"\xa9\rnext"
        );

        assert_eq!(first, "red ");
        assert_eq!(second, "é\nnext");
    }

    #[test]
    fn relative_paths_resolve_from_terminal_directory() {
        let cwd = env::current_dir().unwrap();
        let terminal =
            Terminal::new(cwd.clone()).unwrap();

        assert_eq!(
            terminal.resolve_path("src/main.rs")
                .unwrap(),
            cwd.join("src/main.rs"),
        );
    }

    #[test]
    fn reopening_preserves_terminal_directory() {
        let cwd = env::current_dir().unwrap();
        let mut terminal =
            Terminal::new(cwd.clone()).unwrap();

        terminal.cwd = cwd.join("terminal-session");
        terminal.open(Some(
            &cwd.join("another/file.rs")
        ));

        assert_eq!(
            terminal.cwd,
            cwd.join("terminal-session")
        );
    }

    #[test]
    fn shell_runs_noninteractive_commands() {
        let cwd = env::current_dir().unwrap();

        #[cfg(windows)]
        let command = "echo terminal-ok";

        #[cfg(not(windows))]
        let command = "printf terminal-ok";

        let mut child = spawn_shell(
            command,
            &cwd,
        ).unwrap();

        let mut output = String::new();
        child.stdout.as_mut().unwrap()
            .read_to_string(&mut output)
            .unwrap();

        assert!(child.wait().unwrap().success());
        assert!(output.contains("terminal-ok"));
    }

    #[test]
    fn output_is_capped() {
        let cwd = env::current_dir().unwrap();
        let mut terminal =
            Terminal::new(cwd).unwrap();
        let chunk = format!(
            "{}\n",
            "x".repeat(1024 * 1024)
        );

        for _ in 0..9 {
            terminal.append_text(&chunk).unwrap();
        }

        assert!(
            terminal.output.len()
                <= MAX_OUTPUT_BYTES
        );
    }
}
