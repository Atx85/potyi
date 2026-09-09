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
use std::fs;
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
    ListedFile(PathBuf),
    EnterDirectory(PathBuf),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKind {
    Text,
    Binary,
    Directory,
    Unreadable,
}

#[derive(Clone, Debug)]
pub(crate) struct OutputEntry {
    pub range: std::ops::Range<usize>,
    pub path: PathBuf,
    pub kind: EntryKind,
}

impl OutputEntry {
    pub fn action(&self) -> Option<TerminalAction> {
        match self.kind {
            EntryKind::Text => Some(TerminalAction::ListedFile(self.path.clone())),
            EntryKind::Directory => Some(TerminalAction::EnterDirectory(self.path.clone())),
            _ => None,
        }
    }
}

#[derive(Default)]
struct DirectoryListing {
    text: String,
    entries: Vec<OutputEntry>,
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
    output_generation: u64,
    entries: Vec<OutputEntry>,
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
            output_generation: 0,
            entries: Vec::new(),
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
Built-ins: cd, pwd, ls, edit, view, clear, help, exit\n\
Ctrl+` switches between terminal and editor\n\
ls links: green = edit file, blue = enter folder; amber = binary\n"
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

    pub fn set_scroll_back(&mut self, rows: usize) {
        self.scroll_back = rows;
    }

    pub fn output_generation(&self) -> u64 {
        self.output_generation
    }

    /// Resolve the original output offset, even when the visible row contains
    /// only part of a filename or compiler location.
    pub fn action_at_output_offset(&mut self, position: usize) -> io::Result<Option<TerminalAction>> {
        if let Some(entry) = self.entries_in(position..position.saturating_add(1)).first() {
            return Ok(entry.action());
        }
        if position >= self.output.len() {
            return Ok(None);
        }
        let (line, _) = self.output.line_column_at(position)?;
        let line_start = self.output.line_start(line)?;
        let text = self.output.line_text(line)?;
        let local = position - line_start;
        if local >= text.len() || !text.is_char_boundary(local)
            || text[local..].chars().next().is_none_or(char::is_whitespace)
        {
            return Ok(None);
        }
        let start = text[..local].char_indices().rev()
            .find(|(_, character)| character.is_whitespace())
            .map_or(0, |(index, character)| index + character.len_utf8());
        let end = text[local..].char_indices()
            .find(|(_, character)| character.is_whitespace())
            .map_or(text.len(), |(index, _)| local + index);
        let candidate = text[start..end].trim_matches(|character| {
            matches!(character, '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | ',' | ';' | '"' | '\'')
        });
        let looks_like_path = candidate.contains('/') || candidate.contains('\\')
            || candidate.contains('.')
            || candidate.rsplit_once(':').is_some_and(|(_, suffix)| suffix.parse::<usize>().is_ok());
        Ok(looks_like_path.then(|| TerminalAction::Edit(candidate.to_string())))
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
        self.output_generation = self.output_generation.wrapping_add(1);
        self.entries.clear();
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

            "ls" => {
                let listing = list_directory(
                    arguments,
                    &self.cwd,
                )?;
                self.append_listing(listing)?;
                self.status = None;
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
ls [OPTIONS] [PATH]  list directory contents\n\
Underlined names are clickable: green text files, blue folders; amber = binary\n\
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

    pub fn entries_in(&self, range: std::ops::Range<usize>) -> &[OutputEntry] {
        let start = self.entries.partition_point(|entry| entry.range.end <= range.start);
        let end = self.entries.partition_point(|entry| entry.range.start < range.end);
        &self.entries[start..end]
    }

    fn append_listing(&mut self, listing: DirectoryListing) -> io::Result<()> {
        let offset = self.output.len();
        self.output.insert(offset, &listing.text)?;
        self.entries.extend(listing.entries.into_iter().map(|mut entry| {
            entry.range = entry.range.start + offset..entry.range.end + offset;
            entry
        }));
        self.trim_output()
    }

    pub fn enter_directory(&mut self, path: &Path) -> io::Result<()> {
        if self.is_running() {
            self.set_status("Wait for the running command before entering a folder");
            return Ok(());
        }
        let target = path.canonicalize()?;
        if !target.is_dir() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Not a directory"));
        }
        let listing = list_directory("", &target)?;
        self.ensure_output_newline()?;
        self.append_text(&format!("{}:\n", target.display()))?;
        self.append_listing(listing)?;
        self.cwd = target;
        self.scroll_back = 0;
        self.status = None;
        Ok(())
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
        self.output_generation = self.output_generation.wrapping_add(1);
        self.entries.retain_mut(|entry| {
            if entry.range.start < remove {
                return false;
            }
            entry.range = entry.range.start - remove..entry.range.end - remove;
            true
        });
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

#[derive(Default)]
struct LsOptions {
    all: bool,
    long: bool,
    human_readable: bool,
    recursive: bool,
    one_per_line: bool,
}

fn list_directory(
    arguments: &str,
    cwd: &Path,
) -> io::Result<DirectoryListing> {
    let mut options = LsOptions::default();
    let mut paths = Vec::new();
    let mut parse_options = true;

    for argument in arguments.split_whitespace() {
        if parse_options && argument == "--" {
            parse_options = false;
            continue;
        }

        if parse_options && argument.starts_with("--") {
            match argument {
                "--all" => options.all = true,
                "--long" => options.long = true,
                "--human-readable" => {
                    options.human_readable = true;
                }
                "--recursive" => options.recursive = true,
                "--one-per-line" => {
                    options.one_per_line = true;
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "Unsupported ls option: {argument}"
                        ),
                    ));
                }
            }
        } else if parse_options && argument.starts_with('-')
            && argument != "-"
        {
            for option in argument[1..].chars() {
                match option {
                    'a' => options.all = true,
                    'l' => options.long = true,
                    'h' => options.human_readable = true,
                    'R' => options.recursive = true,
                    '1' => options.one_per_line = true,
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!(
                                "Unsupported ls option: -{option}"
                            ),
                        ));
                    }
                }
            }
        } else {
            paths.push(argument.to_string());
        }
    }

    if paths.is_empty() {
        paths.push(".".to_string());
    }

    let mut output = DirectoryListing::default();
    for (index, value) in paths.iter().enumerate() {
        if index > 0 {
            output.text.push('\n');
        }

        let path = resolve_ls_path(value, cwd);
        append_ls_path(
            &mut output,
            &path,
            value,
            &options,
            paths.len() > 1 || options.recursive,
        )?;
    }

    if !output.text.ends_with('\n') {
        output.text.push('\n');
    }

    Ok(output)
}

fn resolve_ls_path(
    value: &str,
    cwd: &Path,
) -> PathBuf {
    let path = PathBuf::from(expand_home(value));
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

fn append_ls_path(
    output: &mut DirectoryListing,
    path: &Path,
    display_name: &str,
    options: &LsOptions,
    show_header: bool,
) -> io::Result<()> {
    let metadata = fs::metadata(path)?;

    if !metadata.is_dir() {
        append_ls_entry(output, path, display_name, options)?;
        return Ok(());
    }

    if show_header {
        output.text.push_str(display_name);
        output.text.push_str(":\n");
    }

    let mut entries = fs::read_dir(path)?
        .collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    let directory = path.canonicalize()?;
    if let Some(parent) = directory.parent() {
        append_ls_entry(output, parent, "..", options)?;
    }

    for entry in &entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !options.all && name.starts_with('.') {
            continue;
        }

        append_ls_entry(
            output,
            &entry.path(),
            &name,
            options,
        )?;
    }

    if options.recursive {
        for entry in entries {
            let name = entry.file_name();
            if !options.all
                && name.to_string_lossy().starts_with('.')
            {
                continue;
            }

            if entry.file_type()?.is_dir() {
                output.text.push('\n');
                append_ls_path(
                    output,
                    &entry.path(),
                    &entry.path().display().to_string(),
                    options,
                    true,
                )?;
            }
        }
    }

    Ok(())
}

fn append_ls_entry(
    output: &mut DirectoryListing,
    path: &Path,
    display_name: &str,
    options: &LsOptions,
) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    let kind = classify_entry(path, &metadata);
    if options.long {
        let marker = if metadata.is_dir() { 'd' } else { '-' };
        let size = if options.human_readable {
            format_ls_size(metadata.len())
        } else {
            metadata.len().to_string()
        };
        output.text.push_str(&format!("{marker} {size:>12} "));
    }
    let start = output.text.len();
    // Keep a filename on one visual line so it has one unambiguous click target.
    for character in display_name.chars() {
        if character.is_control() {
            output.text.extend(character.escape_default());
        } else {
            output.text.push(character);
        }
    }
    if kind == EntryKind::Directory && !display_name.ends_with('/') {
        output.text.push('/');
    }
    output.entries.push(OutputEntry {
        range: start..output.text.len(),
        path: path.to_path_buf(),
        kind,
    });
    output.text.push_str(if options.long || options.one_per_line { "\n" } else { "    " });
    Ok(())
}

fn classify_entry(path: &Path, metadata: &fs::Metadata) -> EntryKind {
    if metadata.is_dir() {
        return EntryKind::Directory;
    }
    // Never open devices or pipes to sniff their contents.
    if !metadata.is_file() {
        return EntryKind::Unreadable;
    }
    let mut sample = Vec::new();
    let result = fs::File::open(path)
        .and_then(|file| file.take(8192).read_to_end(&mut sample));
    if result.is_err() {
        return EntryKind::Unreadable;
    }
    let utf8 = match std::str::from_utf8(&sample) {
        Ok(_) => true,
        Err(error) => error.error_len().is_none()
            && sample.len() == 8192 && metadata.len() > 8192,
    };
    if utf8 && !sample.iter().any(|byte| {
        *byte < 32 && !matches!(*byte, b'\t' | b'\n' | b'\r' | 12)
    }) {
        EntryKind::Text
    } else {
        EntryKind::Binary
    }
}

fn format_ls_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut value = size as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{}B", size)
    } else {
        format!("{value:.1}{}", UNITS[unit])
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
    fn ls_supports_common_options_and_relative_paths() {
        let cwd = env::current_dir().unwrap();

        let listing = list_directory(
            "--long --human-readable Cargo.toml",
            &cwd,
        ).unwrap();

        assert!(listing.text.contains("Cargo.toml"));
        assert!(listing.text.ends_with('\n'));
    }

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path = env::temp_dir().join(format!(
                "potyi-listing-{}-{}", std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                    .unwrap().as_nanos(),
            ));
            fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn listing_types_and_links_use_content_and_exact_paths() {
        let root = Fixture::new();
        let name = if cfg!(windows) { "notes é file" } else { "notes é:12" };
        fs::write(root.0.join(name), "hello\n").unwrap();
        fs::write(root.0.join("looks-like-text.txt"), b"hello\0world").unwrap();
        fs::write(root.0.join("empty"), "").unwrap();
        fs::create_dir(root.0.join("a folder")).unwrap();
        fs::write(root.0.join("a folder/child"), "text").unwrap();
        for options in ["", "-1", "-lh", "-R"] {
            let listing = list_directory(options, &root.0).unwrap();
            let note = listing.entries.iter().find(|entry| entry.path.ends_with(name)).unwrap();
            assert_eq!(note.kind, EntryKind::Text);
            assert_eq!(&listing.text[note.range.clone()], name);
            assert_eq!(note.action(), Some(TerminalAction::ListedFile(root.0.join(name))));
            let binary = listing.entries.iter().find(|entry| entry.path.ends_with("looks-like-text.txt")).unwrap();
            assert_eq!(binary.kind, EntryKind::Binary);
            assert_eq!(binary.action(), None);
            let folder = listing.entries.iter().find(|entry| entry.path.ends_with("a folder")).unwrap();
            assert_eq!(&listing.text[folder.range.clone()], "a folder/");
            assert_eq!(folder.action(), Some(TerminalAction::EnterDirectory(root.0.join("a folder"))));
        }
    }

    #[test]
    fn folder_click_lists_contents_and_preserves_old_targets() {
        let root = Fixture::new();
        fs::create_dir(root.0.join("a folder")).unwrap();
        fs::write(root.0.join("a folder/child"), "text").unwrap();
        fs::write(root.0.join("original"), "text").unwrap();
        let mut terminal = Terminal::new(root.0.clone()).unwrap();
        terminal.append_listing(list_directory("", &root.0).unwrap()).unwrap();
        terminal.enter_directory(&root.0.join("a folder")).unwrap();
        assert_eq!(terminal.cwd, root.0.join("a folder"));
        assert!(terminal.output_text().unwrap().contains("child"));
        assert!(terminal.entries.iter().any(|entry| entry.action() == Some(TerminalAction::ListedFile(root.0.join("original")))));
        assert!(terminal.enter_directory(&root.0.join("original")).is_err());
        assert_eq!(terminal.cwd, root.0.join("a folder"));
    }

    #[test]
    fn listing_offsets_survive_trimming_and_clear() {
        let root = Fixture::new();
        fs::write(root.0.join("note"), "text").unwrap();
        let mut terminal = Terminal::new(root.0.clone()).unwrap();
        terminal.clear().unwrap();
        terminal.append_listing(list_directory("", &root.0).unwrap()).unwrap();
        terminal.append_text(&"x\n".repeat(MAX_OUTPUT_BYTES / 2 - 32)).unwrap();
        terminal.append_listing(list_directory("", &root.0).unwrap()).unwrap();
        terminal.append_text(&"z\n".repeat(64)).unwrap();
        assert_eq!(terminal.entries.len(), 2);
        let entry = terminal.entries.iter().find(|entry| entry.path.ends_with("note")).unwrap();
        assert_eq!(&terminal.output_text().unwrap()[entry.range.clone()], "note");
        assert_eq!(terminal.entries_in(entry.range.clone()).len(), 1);
        terminal.clear().unwrap();
        assert!(terminal.entries.is_empty());
    }

    #[test]
    fn parent_link_in_empty_folder_navigates_up_and_refreshes_listing() {
        let root = Fixture::new();
        let child = root.0.join("empty folder");
        fs::create_dir(&child).unwrap();
        fs::write(root.0.join("sibling.txt"), "text").unwrap();
        for options in ["", "-1", "-alh", "-R"] {
            let listing = list_directory(options, &child.join(".")).unwrap();
            assert_eq!(listing.entries.len(), 1);
            let parent = &listing.entries[0];
            assert_eq!(&listing.text[parent.range.clone()], "../");
            assert_eq!(parent.action(), Some(TerminalAction::EnterDirectory(root.0.clone())));
        }
        let mut terminal = Terminal::new(child).unwrap();
        let listing = list_directory("", &terminal.cwd).unwrap();
        let Some(TerminalAction::EnterDirectory(parent)) = listing.entries[0].action() else {
            panic!("parent entry must be a folder link");
        };
        terminal.append_listing(listing).unwrap();
        terminal.enter_directory(&parent).unwrap();
        assert_eq!(terminal.cwd, root.0);
        assert!(terminal.output_text().unwrap().contains("sibling.txt"));
    }

    #[test]
    fn wrapped_listing_fragments_keep_exact_click_targets() {
        use crate::terminal_layout::{TerminalLayout, WrapMetrics};
        let root = Fixture::new();
        let name = "long filename with spaces 東京.txt";
        fs::write(root.0.join(name), "text").unwrap();
        fs::write(root.0.join("binary.file"), b"\0").unwrap();
        let mut terminal = Terminal::new(root.0.clone()).unwrap();
        terminal.clear().unwrap();
        terminal.append_listing(list_directory("", &root.0).unwrap()).unwrap();
        let original = terminal.output_text().unwrap();
        let mut layout = TerminalLayout::default();
        layout.update(&terminal.output, terminal.output_generation(), WrapMetrics {
            width: 6, cell_width: 1, tab_width: 4, font_size: 18,
        }, |_| 1).unwrap();
        let mut text_fragments = 0;
        for row in 0..layout.len() {
            let range = layout.row_range(row, &terminal.output).unwrap().unwrap();
            for entry in terminal.entries_in(range.clone()).to_vec() {
                let clicked = range.start.max(entry.range.start);
                assert_eq!(terminal.action_at_output_offset(clicked).unwrap(), entry.action());
                if entry.path.ends_with(name) { text_fragments += 1; }
            }
        }
        assert!(text_fragments > 2);
        assert_eq!(terminal.output_text().unwrap(), original);
    }

    #[test]
    fn wrapped_compiler_location_resolves_the_entire_original_token() {
        let mut terminal = Terminal::new(env::current_dir().unwrap()).unwrap();
        terminal.clear().unwrap();
        let path = "src/a_very_long_東京_filename.rs:42:7";
        terminal.append_text(&format!("error ({path}), failed\n")).unwrap();
        for (offset, _) in path.char_indices() {
            assert_eq!(terminal.action_at_output_offset("error (".len() + offset).unwrap(),
                Some(TerminalAction::Edit(path.to_string())));
        }
        assert_eq!(terminal.action_at_output_offset(5).unwrap(), None);
    }

    #[test]
    fn text_sample_allows_utf8_split_at_sample_boundary() {
        let root = Fixture::new();
        let path = root.0.join("text");
        fs::write(&path, format!("{}é", "a".repeat(8191))).unwrap();
        assert_eq!(classify_entry(&path, &fs::metadata(&path).unwrap()), EntryKind::Text);
        fs::write(&path, [vec![b'a'; 8191], vec![0xc3]].concat()).unwrap();
        assert_eq!(classify_entry(&path, &fs::metadata(&path).unwrap()), EntryKind::Binary);
        fs::write(&path, b"\xff").unwrap();
        assert_eq!(classify_entry(&path, &fs::metadata(&path).unwrap()), EntryKind::Binary);
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
