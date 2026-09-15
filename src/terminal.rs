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
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};

use sdl3::EventSubsystem;
use sdl3::event::EventSender;

use crate::piece_table::PieceTable;

mod listing;
mod completion;
mod selection;
mod locations;
mod frame;
mod colors;
mod git;
pub(crate) use frame::FrameSchedule;
pub(crate) use selection::OutputCommand;


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
    ListingReady,
    StreamClosed,
    ReaderError(String),
}

#[cfg(test)]
pub(crate) fn register_test_events(events: &EventSubsystem) {
    // SDL's Rust custom-event registry survives separate SDL test contexts.
    if let Err(error) = events.register_custom_event::<TerminalEvent>() {
        assert_eq!(error.to_string(), "The same event type can not be registered twice!");
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TerminalAction {
    None,
    Commit(git::Commit),
    Edit(String),
    View(String),
    ListedFile(PathBuf),
    Location(PathBuf, SourceLocation),
    EnterDirectory(PathBuf),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKind {
    Text,
    Binary,
    Directory,
    Unreadable,
    Commit,
}

#[derive(Clone, Debug)]
pub(crate) struct OutputEntry {
    pub commit: Option<git::Commit>,
    pub range: std::ops::Range<usize>,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub location: Option<SourceLocation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceLocation {
    pub line: usize,
    pub column: Option<usize>,
    pub byte_column: bool,
}

impl OutputEntry {
    pub fn action(&self) -> Option<TerminalAction> {
        if let Some(commit) = &self.commit { return Some(TerminalAction::Commit(commit.clone())); }
        if let Some(location) = self.location {
            return Some(TerminalAction::Location(self.path.clone(), location));
        }
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
    output_layout_reset: u64,
    output_trimmed: usize,
    selection: selection::Selection,
    entries: Vec<OutputEntry>,
    location_scanner: locations::Scanner,
    colors: colors::Colors,
    git_view: Option<Box<git::View>>,
    git_back_pending: bool,
    input: String,
    cursor: usize,
    cwd: PathBuf,
    history: VecDeque<String>,
    history_position: Option<usize>,
    history_draft: String,
    running: Option<RunningProcess>,
    listing: Option<listing::Job>,
    listing_width: usize,
    completions: completion::Cache,
    last_command: Option<String>,
    status: Option<String>,
    scroll_back: usize,
    ansi_state: AnsiState,
    pending_utf8: Vec<u8>,
    pending_carriage_return: bool,
    output_wake: Arc<AtomicBool>,
    output_pending: bool,
    listing_pending: bool,
    events: Option<EventSubsystem>,
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
            output_layout_reset: 0,
            output_trimmed: 0,
            selection: selection::Selection::default(),
            entries: Vec::new(),
            location_scanner: locations::Scanner::default(),
            colors: colors::Colors::default(),
            git_view: None,
            git_back_pending: false,
            input: String::new(),
            cursor: 0,
            cwd,
            history: VecDeque::new(),
            history_position: None,
            history_draft: String::new(),
            running: None,
            listing: None,
            listing_width: 80,
            completions: completion::Cache::default(),
            last_command: None,
            status: None,
            scroll_back: 0,
            ansi_state: AnsiState::Ground,
            pending_utf8: Vec::new(),
            pending_carriage_return: false,
            output_wake: Arc::new(AtomicBool::new(false)),
            output_pending: false,
            listing_pending: false,
            events: None,
            output_sender,
            output_receiver,
        };

        terminal.append_text(
            "Pötyi command terminal\n\
Built-ins: cd, pwd, ls, touch, edit, view, clear, help, exit\n\
Ctrl+` switches between terminal and editor\n\
Tab / Shift+Tab completes filenames from the latest ls\n\
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
        self.focus_prompt();
        self.status = None;
        self.scroll_back = 0;
    }

    pub fn close_to_editor(&mut self) {
        self.active = false;
        self.focus_prompt();
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
        self.running.is_some() || self.listing.is_some()
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

    pub fn output_layout_origin(&self) -> (u64, usize) {
        (self.output_layout_reset, self.output_trimmed)
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
        self.completions.reset_cycle();
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
        self.completions.reset_cycle();
        if self.cursor > 0 {
            self.cursor =
                previous_char_boundary(
                    &self.input,
                    self.cursor,
                );
        }
    }

    pub fn move_right(&mut self) {
        self.completions.reset_cycle();
        if self.cursor < self.input.len() {
            self.cursor =
                next_char_boundary(
                    &self.input,
                    self.cursor,
                );
        }
    }

    pub fn move_home(&mut self) {
        self.completions.reset_cycle();
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.completions.reset_cycle();
        self.cursor = self.input.len();
    }

    pub fn history_previous(&mut self) {
        self.completions.reset_cycle();
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
        self.completions.reset_cycle();
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
        self.focus_prompt();
        self.completions.reset_cycle();
        self.output = PieceTable::empty()?;
        self.output_generation = self.output_generation.wrapping_add(1);
        self.output_layout_reset = self.output_layout_reset.wrapping_add(1);
        self.output_trimmed = 0;
        self.entries.clear();
        self.colors = colors::Colors::default();
        self.location_scanner.clear_pending();
        self.scroll_back = 0;
        self.status = None;
        Ok(())
    }

    /// Use the same execution, history and action handling as the terminal prompt.
    pub(crate) fn run_command(&mut self, command: &str) -> io::Result<TerminalAction> {
        if self.is_running() {
            return Err(io::Error::other("A command is already running"));
        }
        let events = self.events.clone().ok_or_else(|| io::Error::other("Terminal events are unavailable"))?;
        self.input = command.to_string();
        self.cursor = self.input.len();
        self.submit(&events)
    }

    pub fn submit(
        &mut self,
        events: &EventSubsystem,
    ) -> io::Result<TerminalAction> {
        if self.is_running() {
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

        // Built-ins handle standalone commands only. Let the shell interpret
        // pipelines, redirects and command lists, including `ls | grep ...`.
        if has_shell_operators(&command) {
            self.spawn_command(&command, events)?;
            return Ok(TerminalAction::None);
        }

        // A drive change in a child cmd.exe cannot update this terminal's cwd.
        if cfg!(windows) && let Some(root) = windows_drive_root(&command) {
            self.enter_directory(&root)?;
            return Ok(TerminalAction::None);
        }

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
                        display_path(&self.cwd),
                    )
                )?;
                Ok(TerminalAction::None)
            }

            "ls" => {
                self.start_listing(arguments, None)?;
                Ok(TerminalAction::None)
            }

            "touch" => {
                touch_files(arguments, &self.cwd)?;
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
touch [-c] [--] PATH...  create files or update their timestamps\n\
Tab / Shift+Tab  cycle matching filenames from the latest ls or cd\n\
Ctrl/Cmd+V     paste into command input\n\
Shift+Up      select output from command input; Esc returns to input\n\
Ctrl/Cmd+C     copy selection; Ctrl+Shift+C copies all; Ctrl+C stops if unselected\n\
Vim output    h/j/k/l, w/b, 0/$, gg/G, v/V selection, y copy\n\
Underlined names are clickable: green text files, blue folders; amber = binary\n\
edit PATH[:LINE[:COLUMN]]  edit a file\n\
PATH           open a text file (e.g. Cargo.lock or ./Cargo.lock)\n\
view PATH[:LINE[:COLUMN]]  open read-only\n\
clear          clear terminal output\n\
exit           return to the editor\n\
grep [OPTIONS] PATTERN [FILE...]  run installed grep with its supported flags\n\
grep -nH PATTERN FILE  clickable results jump to the reported line\n\
Pipelines and redirects run through the system shell (e.g. ls | grep .rs).\n"
                )?;
                #[cfg(windows)]
                self.append_text("C: or cd C:    switch to the drive root (C:\\); cd C:\\PATH opens a folder\n")?;
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
                if let Some(action) = self.direct_file_action(&command)? {
                    self.status = None;
                    return Ok(action);
                }
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
        if self.is_running() {
            return Ok(TerminalAction::None);
        }

        if let Some(view) = &self.git_view {
            self.open_commit(view.commit.clone())?;
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
        if self.listing.take().is_some() {
            self.ensure_output_newline()?;
            self.append_text("[listing stopped]\n")?;
            self.status = None;
            return Ok(());
        }
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

    pub fn set_events(&mut self, events: EventSubsystem) {
        self.events = Some(events);
    }

    pub fn has_pending_work(&self) -> bool {
        self.output_pending || self.listing_pending
    }

    pub fn handle_event(&mut self, event: TerminalEvent) -> io::Result<bool> {
        match event {
            TerminalEvent::OutputReady => self.output_pending = true,
            TerminalEvent::ListingReady => self.listing_pending = true,
            TerminalEvent::ReaderError(error) => {
                self.append_text(&format!("[output error: {error}]\n"))?;
                return Ok(true);
            }
            TerminalEvent::StreamClosed => {
                if let Some(process) = self.running.as_mut() {
                    process.open_streams = process.open_streams.saturating_sub(1);
                }
                // Finish only after poll_background has drained queued bytes.
            }
        }
        Ok(false)
    }

    fn poll_output(&mut self) -> io::Result<bool> {
        // Reset before reading: a producer arriving after this point wakes the
        // event loop again. Remaining queued data also keeps the next turn live.
        self.output_wake.store(false, Ordering::Release);
        self.output_pending = false;
        let start = Instant::now();
        let mut batch = String::new();
        for _ in 0..16 {
            match self.output_receiver.try_recv() {
                Ok(bytes) => batch.push_str(&self.clean_output(&bytes)),
                Err(_) => { self.output_pending = false; break; }
            }
            self.output_pending = true;
            if start.elapsed() >= Duration::from_millis(2) { break; }
        }
        if batch.is_empty() { return Ok(false); }
        self.append_text(&batch)?;
        Ok(true)
    }

    pub fn resolve_path(
        &self,
        value: &str,
    ) -> io::Result<PathBuf> {
        // file_action has already decoded quoting. Preserve literal quotes
        // and leading/trailing spaces in the filename instead of decoding twice.
        if value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "A file path is required",
            ));
        }

        let path = PathBuf::from(
            expand_home(value)
        );

        if path.is_absolute() {
            Ok(path)
        } else {
            Ok(self.cwd.join(path))
        }
    }

    fn direct_file_action(&self, command: &str) -> io::Result<Option<TerminalAction>> {
        let Ok(words) = touch_arguments(command) else { return Ok(None); };
        let [value] = words.as_slice() else { return Ok(None); };
        if value.is_empty() { return Ok(None); }
        let path = self.cwd.join(expand_home(value));
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        if !metadata.is_file() || executable_file(&path, &metadata) { return Ok(None); }

        // A local text file must not shadow an installed command of the same
        // name. An explicit ./path still identifies the file unambiguously.
        if !value.contains(['/', '\\']) && command_on_path(value, &self.cwd) {
            return Ok(None);
        }
        Ok((classify_entry(&path, &metadata) == EntryKind::Text)
            .then_some(TerminalAction::ListedFile(path)))
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
        self.completions.extend(&listing.entries);
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
        self.start_listing("", Some(path.to_path_buf()))
    }

    fn change_directory(
        &mut self,
        arguments: &str,
    ) -> io::Result<()> {
        let value =
            unquote_argument(arguments.trim())?;

        if cfg!(windows) && let Some(root) = windows_drive_root(&value) {
            return self.enter_directory(&root);
        }

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

        self.enter_directory(&target)
    }

    pub(crate) fn set_listing_width(&mut self, columns: usize) {
        self.listing_width = columns.clamp(1, 1000);
    }

    fn start_listing(&mut self, arguments: &str, enter: Option<PathBuf>) -> io::Result<()> {
        self.listing = Some(listing::start(arguments.to_string(), self.cwd.clone(), enter, self.listing_width, self.events.as_ref().map(EventSubsystem::event_sender))?);
        self.status = Some("Listing… Ctrl+C stops".into());
        self.scroll_back = 0;
        Ok(())
    }

    // Called once per event-loop turn. Filesystem work stays in the worker;
    // only one bounded batch is appended before handling input and redrawing.
    pub fn poll_background(&mut self) -> io::Result<bool> {
        let mut changed = self.poll_output()?;
        if !self.output_pending { changed |= self.poll_process()?; }
        self.listing_pending = false;
        let message = self.listing.as_ref().map(|job| job.receiver.try_recv());
        match message {
            Some(Ok(listing::Message::Started(directory))) => {
                self.listing_pending = true;
                self.completions.clear();
                if let Some(directory) = directory { self.cwd = directory; }
                changed = true;
            }
            Some(Ok(listing::Message::Chunk(chunk))) => {
                self.listing_pending = true;
                self.append_listing(chunk)?;
                changed = true;
            }
            Some(Ok(listing::Message::Finished(result))) => {
                self.listing = None;
                self.status = result.err();
                changed = true;
            }
            Some(Err(TryRecvError::Disconnected)) => {
                self.listing = None;
                self.status = Some("Directory listing ended unexpectedly".into());
                changed = true;
            }
            _ => {}
        }
        Ok(changed)
    }

    fn spawn_command(
        &mut self,
        command: &str,
        events: &EventSubsystem,
    ) -> io::Result<()> {
        let child = spawn_shell(command, &self.cwd)?;
        self.location_scanner = locations::Scanner::new(command, &self.cwd);
        self.colors.begin(command);
        self.start_child(child, events)
    }

    fn start_child(&mut self, mut child: Child, events: &EventSubsystem) -> io::Result<()> {
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
            self.output_wake.clone(),
            events.event_sender(),
        );

        spawn_output_reader(
            stderr,
            self.output_sender.clone(),
            self.output_wake.clone(),
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

    fn poll_process(&mut self) -> io::Result<bool> {
        let Some(process) =
            self.running.as_mut()
        else {
            return Ok(false);
        };
        if process.open_streams != 0 { return Ok(false); }
        let Some(status) = process.child.try_wait()? else { return Ok(false); };
        self.running = None;
        self.flush_pending_output()?;
        self.ensure_output_newline()?;
        self.location_scanner = locations::Scanner::default();
        self.colors.end();
        self.append_text(
            &format_exit_status(status)
        )?;

        self.status = None;
        if self.git_back_pending { self.restore_git_view(); }
        Ok(true)
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

    pub(crate) fn output_color(&self, byte: usize) -> Option<(u8, u8, u8)> { self.colors.at(byte) }

    fn append_text(
        &mut self,
        text: &str,
    ) -> io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        let start = self.output.len();
        self.output.insert(
            start,
            text,
        )?;
        self.location_scanner.append(text, start, &mut self.entries);
        self.colors.append(text, start);

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

        // Look near the cutoff instead of reading the entire discarded prefix.
        let sample = self.output.read_range(desired, (self.output.len() - desired).min(4096))?;
        let remove = sample.iter().position(|byte| *byte == b'\n')
            .map(|index| desired + index + 1)
            .unwrap_or_else(|| {
                let mut boundary = 0;
                while boundary < sample.len() && sample[boundary] & 0xc0 == 0x80 { boundary += 1; }
                desired + boundary
            });
        if self.output.byte_at(remove.saturating_sub(1))? != Some(b'\n') {
            // A cut inside a logical line changes wrapping of the retained text.
            self.output_layout_reset = self.output_layout_reset.wrapping_add(1);
        }
        self.output_trimmed = self.output_trimmed.saturating_add(remove);

        self.output.delete(0, remove)?;
        self.trim_selection(remove);
        self.location_scanner.trim(remove);
        self.colors.trim(remove);
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
        self.completions.reset_cycle();
        self.history_position = None;
        self.history_draft.clear();
    }

    pub fn complete_path(&mut self, backwards: bool) {
        if self.is_running() { return; }
        self.history_position = None;
        self.history_draft.clear();
        self.status = self.completions.complete(&mut self.input, &mut self.cursor, &self.cwd, backwards);
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
    wake_pending: Arc<AtomicBool>,
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

                    if !wake_pending.swap(true, Ordering::AcqRel) && sender.push_custom_event(
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
    non_login_shell(&shell, command, cwd).spawn()
}

#[cfg(not(windows))]
fn non_login_shell(shell: &std::ffi::OsStr, command: &str, cwd: &Path) -> Command {
    let mut process = Command::new(shell);
    // Reuse the application's environment. A login shell reloads profiles
    // (package managers, version managers, etc.) for every submitted command.
    configure_process(
        process.arg("-c").arg(command),
        cwd,
    );
    process
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

fn parse_ls_arguments(arguments: &str) -> io::Result<(LsOptions, Vec<String>)> {
    let mut options = LsOptions::default();
    let mut paths = Vec::new();
    let mut parse_options = true;

    for argument in touch_arguments(arguments)? {
        if parse_options && argument == "--" {
            parse_options = false;
            continue;
        }

        if parse_options && argument.starts_with("--") {
            match argument.as_str() {
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

    Ok((options, paths))
}

#[cfg(test)]
fn list_directory(arguments: &str, cwd: &Path) -> io::Result<DirectoryListing> {
    let (options, paths) = parse_ls_arguments(arguments)?;

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

#[cfg(test)]
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
        output.text.push_str(&display_path(Path::new(display_name)));
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
                    &display_path(&entry.path()),
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
        location: None,
        commit: None,
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

fn has_shell_operators(command: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;

    for character in command.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        // cmd.exe uses caret escapes and only double quotes; Unix shells
        // also support single quotes and backslash escapes.
        if (cfg!(windows) && character == '^' && quote.is_none())
            || (!cfg!(windows) && character == '\\' && quote != Some('\''))
        {
            escaped = true;
            continue;
        }

        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            }
            continue;
        }

        match character {
            '"' => quote = Some(character),
            '\'' if !cfg!(windows) => quote = Some(character),
            '|' | '&' | '<' | '>' | '\n' | '(' | ')' => return true,
            ';' if !cfg!(windows) => return true,
            _ => {}
        }
    }

    false
}

// Keep quoted filenames together while preserving native Windows separators.
fn touch_arguments(arguments: &str) -> io::Result<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    let mut chars = arguments.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\\' && !cfg!(windows) && quote != Some('\'') {
            if let Some(next) = chars.peek().copied() {
                if quote.is_none() || matches!(next, '"' | '\\' | '$' | '`') {
                    word.push(chars.next().unwrap());
                    started = true;
                    continue;
                }
            } else {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Incomplete path escape"));
            }
        }
        if let Some(delimiter) = quote {
            if character == delimiter { quote = None; } else { word.push(character); }
        } else if matches!(character, '"' | '\'') {
            quote = Some(character);
            started = true;
        } else if character.is_whitespace() {
            if started { words.push(std::mem::take(&mut word)); started = false; }
        } else {
            word.push(character);
            started = true;
        }
    }
    if quote.is_some() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Close the quoted path"));
    }
    if started { words.push(word); }
    Ok(words)
}

fn touch_files(arguments: &str, cwd: &Path) -> io::Result<()> {
    let mut no_create = false;
    let mut options = true;
    let mut paths = Vec::new();
    for argument in touch_arguments(arguments)? {
        match argument.as_str() {
            "--" if options => options = false,
            "-c" | "--no-create" if options => no_create = true,
            option if options && option.starts_with('-') && option != "-" => {
                return Err(io::Error::new(io::ErrorKind::InvalidInput,
                    format!("Unsupported touch option: {option}. Usage: touch [-c] [--] PATH...")));
            }
            "" => return Err(io::Error::new(io::ErrorKind::InvalidInput, "A file path is required")),
            _ => paths.push(cwd.join(expand_home(&argument))),
        }
    }
    if paths.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Usage: touch [-c] [--] PATH..."));
    }

    let now = std::time::SystemTime::now();
    let times = fs::FileTimes::new().set_accessed(now).set_modified(now);
    let mut errors = Vec::new();
    for path in paths {
        let result = fs::OpenOptions::new().write(true).create(!no_create).truncate(false)
            .open(&path).and_then(|file| file.set_times(times));
        if let Err(error) = result {
            if no_create && error.kind() == io::ErrorKind::NotFound { continue; }
            errors.push(format!("{}: {error}", display_path(&path)));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(io::Error::other(errors.join("\n"))) }
}

// Deliberately recognize only a bare ASCII drive letter. Drive-relative paths
// (C:folder), quoted filenames, and shell command lists are not drive switches.
fn windows_drive_root(value: &str) -> Option<PathBuf> {
    let bytes = value.as_bytes();
    if bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        Some(PathBuf::from(format!("{}:\\", bytes[0].to_ascii_uppercase() as char)))
    } else {
        None
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
    // Completion uses shell-style quote concatenation for a filename that
    // contains both quote characters. Decode that form for editor built-ins.
    if !cfg!(windows) && value.starts_with('\'') && value.contains("'\\''") {
        let mut words = touch_arguments(value)?;
        if words.len() == 1 { return Ok(words.remove(0)); }
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "A single file path is required"));
    }
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

fn executable_file(path: &Path, metadata: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = path;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        let _ = metadata;
        let extension = path.extension().and_then(|value| value.to_str()).unwrap_or("");
        env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';').any(|suffix| suffix.trim_start_matches('.').eq_ignore_ascii_case(extension))
    }
    #[cfg(not(any(unix, windows)))]
    { let _ = (path, metadata); false }
}

fn command_on_path(name: &str, cwd: &Path) -> bool {
    let Some(search_path) = env::var_os("PATH") else { return false; };
    let mut names = vec![name.to_string()];
    if cfg!(windows) {
        names.extend(env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';').filter(|suffix| !suffix.is_empty()).map(|suffix| format!("{name}{suffix}")));
    }
    env::split_paths(&search_path).any(|directory| {
        names.iter().any(|name| {
            let path = cwd.join(&directory).join(name);
            fs::metadata(&path).is_ok_and(|metadata| metadata.is_file() && executable_file(&path, &metadata))
        })
    })
}

/// Keep canonical paths for filesystem access; use ordinary Windows spelling in output.
pub(crate) fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    #[cfg(windows)]
    { windows_display_path(&text) }
    #[cfg(not(windows))]
    { text.into_owned() }
}

#[cfg(any(windows, test))]
fn windows_display_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\") {
        if let Some(share) = rest.strip_prefix(r"UNC\") {
            return format!(r"\\{share}");
        }
        let bytes = rest.as_bytes();
        if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/') {
            return rest.to_owned();
        }
    }
    path.to_owned()
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
    fn git_colors_preserve_clean_output_bytes_and_reset_on_clear() {
        let mut terminal = Terminal::new(env::current_dir().unwrap()).unwrap();
        terminal.clear().unwrap(); terminal.colors.begin("git diff");
        let first = terminal.clean_output(b"\x1b[31m-old\x1b[");
        terminal.append_text(&first).unwrap();
        let second = terminal.clean_output("0m\n\x1b[32m+🦀new\x1b[0m\n".as_bytes());
        terminal.append_text(&second).unwrap();
        assert_eq!(terminal.output.read_range(0, terminal.output.len()).unwrap(), "-old\n+🦀new\n".as_bytes());
        assert_eq!(terminal.output_color(0), Some((245,145,145)));
        assert_eq!(terminal.output_color(5), Some((150,220,165)));
        terminal.clear().unwrap(); terminal.append_text("+plain\n").unwrap();
        assert_eq!(terminal.output_color(0), None);
    }

    #[test]
    fn windows_display_paths_remove_only_verbatim_disk_and_unc_prefixes() {
        for (raw, expected) in [
            (r"\\?\C:\Windows", r"C:\Windows"),
            (r"\\?\c:/Windows", "c:/Windows"),
            (r"\\?\D:\", r"D:\"),
            (r"\\?\C:\Users\Name With Spaces\東京", r"C:\Users\Name With Spaces\東京"),
            (r"\\?\UNC\server\share\folder", r"\\server\share\folder"),
            (r"\\server\share", r"\\server\share"),
            (r"C:\Windows", r"C:\Windows"),
            ("/tmp/project", "/tmp/project"),
            (r"\\?\Volume{example}\", r"\\?\Volume{example}\"),
            (r"\\.\pipe\example", r"\\.\pipe\example"),
        ] {
            assert_eq!(windows_display_path(raw), expected);
        }
    }

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

    fn finish_listing(terminal: &mut Terminal) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while terminal.is_running() {
            assert!(std::time::Instant::now() < deadline, "Listing did not finish");
            if !terminal.poll_background().unwrap() {
                thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

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
    fn touch_creates_multiple_files_and_preserves_existing_contents() {
        let root = Fixture::new();
        let existing = root.0.join("existing.txt");
        fs::write(&existing, "keep this text").unwrap();
        let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        fs::OpenOptions::new().write(true).open(&existing).unwrap()
            .set_times(fs::FileTimes::new().set_accessed(old).set_modified(old)).unwrap();
        touch_files("existing.txt \"é new file.txt\" second.txt", &root.0).unwrap();
        let metadata = fs::metadata(&existing).unwrap();
        assert!(metadata.modified().unwrap() > old);
        assert!(metadata.accessed().unwrap() > old);
        assert_eq!(fs::read_to_string(&existing).unwrap(), "keep this text");
        assert_eq!(fs::read(root.0.join("é new file.txt")).unwrap(), b"");
        assert!(root.0.join("second.txt").is_file());
        touch_files("-c absent.txt existing.txt", &root.0).unwrap();
        assert!(!root.0.join("absent.txt").exists());
        touch_files("--no-create another-absent.txt", &root.0).unwrap();
        assert!(!root.0.join("another-absent.txt").exists());
        touch_files("-- -notes.txt", &root.0).unwrap();
        assert!(root.0.join("-notes.txt").is_file());
    }

    #[test]
    fn touch_reports_invalid_paths_and_keeps_processing_valid_files() {
        let root = Fixture::new();
        for input in ["", "--", "\"\"", "\"unfinished", "first.txt --bad"] {
            assert!(touch_files(input, &root.0).is_err(), "{input}");
        }
        assert!(!root.0.join("first.txt").exists());
        let error = touch_files("missing/child.txt valid.txt", &root.0).unwrap_err();
        assert!(error.to_string().contains("child.txt"));
        assert!(root.0.join("valid.txt").is_file());
        assert!(!root.0.join("missing").exists());
        let absolute = root.0.join("absolute file.txt");
        touch_files(&format!("\"{}\"", absolute.display()), &root.0.join("unused")).unwrap();
        assert!(absolute.is_file());
    }

    #[test]
    fn touch_quote_parser_preserves_paths() {
        assert_eq!(touch_arguments("'first file' \"second file\" plain").unwrap(),
            ["first file", "second file", "plain"]);
        #[cfg(windows)]
        assert_eq!(touch_arguments(r#""C:\folder name\file.txt""#).unwrap(), [r"C:\folder name\file.txt"]);
        #[cfg(not(windows))]
        assert_eq!(touch_arguments(r#"a\ file "a\"b" 'a\b'"#).unwrap(), ["a file", "a\"b", r"a\b"]);
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
        finish_listing(&mut terminal);
        assert_eq!(terminal.cwd, root.0.join("a folder"));
        assert!(terminal.output_text().unwrap().contains("child"));
        assert!(terminal.entries.iter().any(|entry| entry.action() == Some(TerminalAction::ListedFile(root.0.join("original")))));
        terminal.enter_directory(&root.0.join("original")).unwrap();
        finish_listing(&mut terminal);
        assert_eq!(terminal.status(), Some("Not a directory"));
        assert_eq!(terminal.cwd, root.0.join("a folder"));
    }

    #[test]
    fn completion_refreshes_after_ls_cd_and_user_edits() {
        let root = Fixture::new();
        fs::write(root.0.join("notes-a.txt"), "").unwrap();
        fs::write(root.0.join("notes-b.txt"), "").unwrap();
        fs::create_dir(root.0.join("child")).unwrap();
        fs::write(root.0.join("child/new file.txt"), "").unwrap();
        let mut terminal = Terminal::new(root.0.clone()).unwrap();
        terminal.start_listing("", None).unwrap();
        finish_listing(&mut terminal);
        terminal.insert_text("edit no");
        terminal.complete_path(false);
        assert_eq!(terminal.input(), "edit notes-a.txt");
        terminal.complete_path(false);
        assert_eq!(terminal.input(), "edit notes-b.txt");
        terminal.complete_path(true);
        assert_eq!(terminal.input(), "edit notes-a.txt");
        terminal.move_home();
        terminal.complete_path(false);
        assert_eq!(terminal.input(), "edit notes-a.txt");
        terminal.input.clear();
        terminal.cursor = 0;
        terminal.change_directory("child").unwrap();
        finish_listing(&mut terminal);
        terminal.insert_text("edit ne");
        terminal.complete_path(false);
        assert_eq!(terminal.input(), "edit \"new file.txt\"");
        fs::remove_file(root.0.join("child/new file.txt")).unwrap();
        terminal.start_listing("", None).unwrap();
        finish_listing(&mut terminal);
        terminal.input = "edit ne".into();
        terminal.cursor = terminal.input.len();
        terminal.complete_path(false);
        assert_eq!(terminal.input(), "edit ne");
        assert!(terminal.status().unwrap().contains("No matching"));
    }

    #[test]
    fn standalone_text_paths_open_relative_absolute_and_quoted_files() {
        let root = Fixture::new();
        fs::write(root.0.join("Cargo.lock"), "version = 4\n").unwrap();
        fs::write(root.0.join("é notes.txt"), "notes").unwrap();
        let terminal = Terminal::new(root.0.clone()).unwrap();
        for input in ["Cargo.lock", "./Cargo.lock"] {
            assert_eq!(terminal.direct_file_action(input).unwrap(),
                Some(TerminalAction::ListedFile(root.0.join("Cargo.lock"))));
        }
        assert_eq!(terminal.direct_file_action("\"é notes.txt\"").unwrap(),
            Some(TerminalAction::ListedFile(root.0.join("é notes.txt"))));
        let absolute = format!("\"{}\"", root.0.join("Cargo.lock").display());
        assert_eq!(terminal.direct_file_action(&absolute).unwrap(),
            Some(TerminalAction::ListedFile(root.0.join("Cargo.lock"))));
        assert_eq!(terminal.direct_file_action("Cargo.lock argument").unwrap(), None);
        assert_eq!(terminal.direct_file_action("missing.txt").unwrap(), None);
        fs::write(root.0.join("binary.dat"), b"\0binary").unwrap();
        assert_eq!(terminal.direct_file_action("binary.dat").unwrap(), None);
    }

    #[test]
    #[cfg(unix)]
    fn executable_scripts_and_installed_commands_keep_shell_behavior() {
        use std::os::unix::fs::PermissionsExt;
        let root = Fixture::new();
        let script = root.0.join("run.sh");
        fs::write(&script, "#!/bin/sh\necho done\n").unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let terminal = Terminal::new(root.0.clone()).unwrap();
        assert_eq!(terminal.direct_file_action("./run.sh").unwrap(), None);
        fs::write(root.0.join("sh"), "a local text file named like a command").unwrap();
        assert!(command_on_path("sh", &root.0));
        assert_eq!(terminal.direct_file_action("sh").unwrap(), None);
        assert_eq!(terminal.direct_file_action("./sh").unwrap(),
            Some(TerminalAction::ListedFile(root.0.join("sh"))));
    }

    #[test]
    fn completed_editor_paths_preserve_literal_quotes_and_edge_spaces() {
        let root = Fixture::new();
        for name in [" edge spaces ", "a\"b'c"] {
            if cfg!(windows) && name.contains('"') { continue; }
            fs::write(root.0.join(name), "text").unwrap();
            let mut terminal = Terminal::new(root.0.clone()).unwrap();
            terminal.start_listing("", None).unwrap();
            finish_listing(&mut terminal);
            terminal.insert_text(if name.starts_with(' ') { "edit \" " } else { "edit a" });
            terminal.complete_path(false);
            let arguments = terminal.input()[5..].to_string();
            let TerminalAction::Edit(value) = terminal.file_action(&arguments, false).unwrap() else { panic!() };
            assert_eq!(terminal.resolve_path(&value).unwrap(), root.0.join(name));
        }
    }

    #[test]
    fn default_listing_columns_fit_width_and_keep_click_targets_across_batches() {
        let root = Fixture::new();
        for i in 0..150 {
            fs::write(root.0.join(format!("file-{i:03}{}", "x".repeat(i % 7))), "text").unwrap();
        }
        fs::create_dir(root.0.join("a folder")).unwrap();
        let mut terminal = Terminal::new(root.0.clone()).unwrap();
        terminal.clear().unwrap();
        terminal.set_listing_width(60);
        terminal.start_listing("", None).unwrap();
        finish_listing(&mut terminal);
        let text = terminal.output_text().unwrap();
        assert_eq!(terminal.entries.len(), 152);
        let mut starts = std::collections::BTreeSet::new();
        for entry in terminal.entries.clone() {
            let line_start = text[..entry.range.start].rfind('\n').map_or(0, |p| p + 1);
            starts.insert(text[line_start..entry.range.start].chars().count());
            let shown = &text[entry.range.clone()];
            assert!(!shown.contains('\n'));
            let expected = if entry.path == root.0.parent().unwrap() {
                "../".to_string()
            } else {
                format!("{}{}", entry.path.file_name().unwrap().to_string_lossy(),
                    if entry.kind == EntryKind::Directory { "/" } else { "" })
            };
            assert_eq!(shown, expected);
            assert_eq!(terminal.action_at_output_offset(entry.range.start).unwrap(), entry.action());
        }
        assert_eq!(starts.into_iter().collect::<Vec<_>>(), vec![0, 17, 34]);
        assert!(text.lines().all(|line| line.chars().count() <= 60));
        for (width, options) in [(10, ""), (60, "-1"), (60, "-lh")] {
            terminal.clear().unwrap();
            terminal.set_listing_width(width);
            terminal.start_listing(options, None).unwrap();
            finish_listing(&mut terminal);
            let text = terminal.output_text().unwrap();
            let lines: std::collections::BTreeSet<_> = terminal.entries.iter().map(|entry|
                text[..entry.range.start].bytes().filter(|&b| b == b'\n').count()).collect();
            assert_eq!(lines.len(), terminal.entries.len(), "{options} at width {width}");
        }
    }

    #[test]
    fn large_listing_streams_in_batches_and_can_be_cancelled_without_stale_output() {
        let root = Fixture::new();
        for i in 0..800 { fs::write(root.0.join(format!("file-{i:04}.txt")), "text").unwrap(); }
        let mut terminal = Terminal::new(root.0.clone()).unwrap();
        terminal.start_listing("-1", None).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while terminal.entries.is_empty() {
            assert!(std::time::Instant::now() < deadline);
            terminal.poll_background().unwrap();
            thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(terminal.is_running());
        assert!(terminal.entries.len() <= 64);
        terminal.insert_text("still responsive");
        assert_eq!(terminal.input(), "still responsive");
        terminal.stop().unwrap();
        assert!(!terminal.is_running());
        terminal.clear().unwrap();
        terminal.start_listing("file-0799.txt", None).unwrap();
        finish_listing(&mut terminal);
        assert_eq!(terminal.entries.len(), 1);
        assert!(terminal.output_text().unwrap().contains("file-0799.txt"));
        assert!(!terminal.output_text().unwrap().contains("file-0000.txt"));
    }

    #[test]
    fn output_events_wake_without_work_and_batches_are_bounded() {
        let mut terminal = Terminal::new(env::temp_dir()).unwrap();
        terminal.clear().unwrap();
        for n in 0..32 {
            terminal.output_sender.send(format!("{n}: 東京\n").into_bytes()).unwrap();
        }
        assert!(!terminal.handle_event(TerminalEvent::OutputReady).unwrap());
        assert!(terminal.output_text().unwrap().is_empty());
        terminal.poll_background().unwrap();
        let first = terminal.output_text().unwrap();
        assert!(first.lines().count() <= 16);
        assert!(terminal.has_pending_work());
        while terminal.has_pending_work() { terminal.poll_background().unwrap(); }
        let expected = (0..32).map(|n| format!("{n}: 東京\n")).collect::<String>();
        assert_eq!(terminal.output_text().unwrap(), expected);
        assert!(!terminal.poll_background().unwrap());
        assert!(!terminal.has_pending_work());
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
        finish_listing(&mut terminal);
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
    fn shell_operators_bypass_standalone_builtins() {
        for command in [
            "ls | grep -i src",
            "ls|grep src",
            "cd src && grep -n main main.rs",
            "ls missing || pwd",
            "ls > files.txt",
            "ls 2>> errors.txt",
            "ls < input.txt",
            "pwd\ngrep pattern file.txt",
        ] {
            assert!(has_shell_operators(command), "{command}");
        }

        for command in ["ls -la", "cd src", "edit \"a|b & c.txt\"", "ls \"a>b\""] {
            assert!(!has_shell_operators(command), "{command}");
        }
    }

    #[test]
    fn drive_switches_require_one_ascii_letter_and_a_colon() {
        for letter in b'A'..=b'Z' {
            let expected = PathBuf::from(format!("{}:\\", letter as char));
            for spelling in [letter, letter.to_ascii_lowercase()] {
                assert_eq!(windows_drive_root(&format!("{}:", spelling as char)), Some(expected.clone()));
            }
        }
        for value in ["", "C", ":", "1:", "é:", "Ｃ:", "C::", "C:folder", "C:\\", "C:/", "C: extra", "\"C:\"", "C: && pwd", "C:|more", "C:\npwd"] {
            assert_eq!(windows_drive_root(value), None, "{value:?}");
        }
    }

    #[test]
    fn failed_directory_changes_preserve_the_terminal_directory() {
        let root = Fixture::new();
        fs::write(root.0.join("file.txt"), "text").unwrap();
        let mut terminal = Terminal::new(root.0.clone()).unwrap();
        let initial = terminal.cwd.clone();
        for value in ["missing-folder", "file.txt"] {
            terminal.change_directory(value).unwrap();
            finish_listing(&mut terminal);
            assert_eq!(terminal.cwd, initial);
            assert!(terminal.status().is_some());
            assert!(terminal.running.is_none());
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn unix_directory_names_with_colons_are_not_drive_switches() {
        let root = Fixture::new();
        fs::create_dir(root.0.join("c:")).unwrap();
        let mut terminal = Terminal::new(root.0.clone()).unwrap();
        terminal.change_directory("c:").unwrap();
        finish_listing(&mut terminal);
        assert_eq!(terminal.cwd, root.0.join("c:").canonicalize().unwrap());
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "Uses SDL events and a Windows drive; run with --ignored --test-threads=1"]
    fn windows_drive_commands_persist_in_the_terminal() {
        use std::path::{Component, Prefix};
        let sdl = sdl3::init().unwrap();
        let events = sdl.event().unwrap();
        register_test_events(&events);
        let root = Fixture::new();
        let initial = root.0.canonicalize().unwrap();
        let drive = match initial.components().next().unwrap() {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
                _ => panic!("Drive test needs a fixture on a local Windows drive"),
            },
            _ => panic!("Expected an absolute Windows fixture path"),
        };
        let expected = windows_drive_root(&format!("{}:", drive as char)).unwrap().canonicalize().unwrap();
        let mut terminal = Terminal::new(initial.clone()).unwrap();
        terminal.set_events(events);
        for command in [format!("{}:", drive as char), format!("{}:", drive.to_ascii_lowercase() as char), format!("cd {}:", drive as char), format!("cd \"{}:\"", drive as char)] {
            terminal.enter_directory(&initial).unwrap();
            finish_listing(&mut terminal);
            terminal.run_command(&command).unwrap();
            assert!(terminal.running.is_none(), "Drive changes must not spawn a shell");
            finish_listing(&mut terminal);
            assert_eq!(terminal.cwd, expected, "{command}");
            assert_eq!(terminal.history.back(), Some(&command));
            terminal.run_command("pwd").unwrap();
            assert!(terminal.output_text().unwrap().ends_with(&format!("{}\n", display_path(&expected))));
            terminal.run_command(&format!("cd \"{}\"", display_path(&initial))).unwrap();
            finish_listing(&mut terminal);
            assert_eq!(terminal.cwd, initial);
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn unix_shell_operators_respect_quotes_and_escapes() {
        for command in [r"edit 'a|b.txt'", r"edit a\|b.txt", r#"edit "a\"|b.txt""#] {
            assert!(!has_shell_operators(command), "{command}");
        }
        for command in ["pwd; grep pattern file.txt", r#"ls "a\"b" | grep b"#, r"ls 'a\' | grep a"] {
            assert!(has_shell_operators(command), "{command}");
        }
    }

    #[test]
    #[cfg(windows)]
    fn windows_shell_operators_respect_cmd_escaping() {
        assert!(!has_shell_operators("edit a^&b.txt"));
        assert!(!has_shell_operators("cd C:\\work\\"));
        assert!(!has_shell_operators("edit a;b.txt"));
        assert!(has_shell_operators("ls 'a|b'"));
        assert!(has_shell_operators("cd \"C:\\work\\\" && grep pattern file.txt"));
    }

    #[test]
    #[cfg(not(windows))]
    fn shell_preserves_grep_options_patterns_and_exit_codes() {
        let root = Fixture::new();
        fs::write(root.0.join("sample file.txt"), "Alpha one\nbeta.two\nALPHA three\ngamma\n").unwrap();
        fs::write(root.0.join("patterns.txt"), "gamma\n").unwrap();
        fs::create_dir(root.0.join("nested")).unwrap();
        fs::write(root.0.join("nested/child.txt"), "Alpha child\n").unwrap();

        for (command, expected, code) in [
            ("grep -in -e 'alpha one' -e gamma 'sample file.txt'", "1:Alpha one\n4:gamma\n", 0),
            ("grep -En '^(beta|gamma)' 'sample file.txt'", "2:beta.two\n4:gamma\n", 0),
            ("grep -F -e 'beta.two' -- 'sample file.txt'", "beta.two\n", 0),
            ("grep -ivc alpha 'sample file.txt'", "2\n", 0),
            ("grep -n -A 1 -B 1 beta 'sample file.txt'", "1-Alpha one\n2:beta.two\n3-ALPHA three\n", 0),
            ("grep -n -f patterns.txt 'sample file.txt'", "4:gamma\n", 0),
            ("grep -rn Alpha nested", "nested/child.txt:1:Alpha child\n", 0),
            ("grep -q missing 'sample file.txt'", "", 1),
            ("grep -q Alpha 'sample file.txt'", "", 0),
            ("ls | grep -F 'sample file'", "sample file.txt\n", 0),
            ("cd nested && grep -n Alpha child.txt", "1:Alpha child\n", 0),
            ("grep -in alpha < 'sample file.txt' | grep -v three > matches.txt; cat matches.txt", "1:Alpha one\n", 0),
        ] {
            let output = spawn_shell(command, &root.0).unwrap().wait_with_output().unwrap();
            assert_eq!(output.status.code(), Some(code), "{command}: {}", String::from_utf8_lossy(&output.stderr));
            assert_eq!(String::from_utf8(output.stdout).unwrap(), expected, "{command}");
        }
    }

    #[test]
    #[ignore = "Uses SDL events; run with --ignored --test-threads=1"]
    fn term_command_shortcut_shares_execution_history_and_busy_guard() {
        let sdl = sdl3::init().unwrap();
        let events = sdl.event().unwrap();
        let mut pump = sdl.event_pump().unwrap();
        register_test_events(&events);
        let root = Fixture::new();
        fs::create_dir(root.0.join("nested folder")).unwrap();
        fs::write(root.0.join("nested folder/sample.txt"), "hello").unwrap();
        let mut terminal = Terminal::new(root.0.clone()).unwrap();
        terminal.set_events(events);
        terminal.open(None);
        terminal.run_command("cd \"nested folder\"").unwrap();
        finish_listing(&mut terminal);
        assert_eq!(terminal.cwd, root.0.join("nested folder").canonicalize().unwrap());
        terminal.run_command("ls").unwrap();
        terminal.insert_text("unfinished draft");
        assert!(terminal.run_command("pwd").unwrap_err().to_string().contains("already running"));
        assert_eq!(terminal.input(), "unfinished draft");
        finish_listing(&mut terminal);
        assert!(terminal.output_text().unwrap().contains("sample.txt"));
        terminal.run_command("pwd").unwrap();
        assert_eq!(terminal.history.back().unwrap(), "pwd");
        assert!(terminal.input().is_empty());
        assert_eq!(terminal.run_command("edit sample.txt").unwrap(), TerminalAction::Edit("sample.txt".into()));
        assert_eq!(terminal.run_command("view sample.txt").unwrap(), TerminalAction::View("sample.txt".into()));
        let command = if cfg!(windows) { "echo shortcut-output > result.txt" }
            else { "printf '%s\\n' 'shortcut-output' | cat > result.txt" };
        terminal.run_command(command).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while terminal.is_running() {
            for event in pump.poll_iter() {
                if let Some(event) = event.as_user_event_type::<TerminalEvent>() {
                    terminal.handle_event(event).unwrap();
                }
            }
            terminal.poll_background().unwrap();
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(fs::read_to_string(terminal.cwd.join("result.txt")).unwrap().trim(), "shortcut-output");
        assert_eq!(terminal.history.back().unwrap(), command);
        terminal.run_command("exit").unwrap();
        assert!(!terminal.is_active());
    }

    #[test]
    #[cfg(not(windows))]
    #[ignore = "Uses SDL events; run with --ignored --test-threads=1"]
    fn submit_runs_grep_pipeline_and_preserves_navigation_builtins() {
        let sdl = sdl3::init().unwrap();
        let events = sdl.event().unwrap();
        register_test_events(&events);
        let mut pump = sdl.event_pump().unwrap();
        let root = Fixture::new();
        fs::write(root.0.join("sample file.txt"), "hello\n").unwrap();
        let mut terminal = Terminal::new(root.0.clone()).unwrap();
        terminal.insert_text("ls | grep -F 'sample file'");
        assert_eq!(terminal.submit(&events).unwrap(), TerminalAction::None);
        assert!(terminal.is_running());

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while terminal.is_running() {
            terminal.poll_background().unwrap();
            assert!(std::time::Instant::now() < deadline, "Pipeline did not finish");
            if let Some(event) = pump.wait_event_timeout(std::time::Duration::from_millis(100))
                && let Some(event) = event.as_user_event_type::<TerminalEvent>()
            {
                terminal.handle_event(event).unwrap();
            }
        }
        let output = terminal.output_text().unwrap();
        assert!(output.contains("\nsample file.txt\n"), "{output}");
        assert!(output.contains("[finished successfully]"), "{output}");

        terminal.insert_text("grep -n hello 'sample file.txt'");
        terminal.submit(&events).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while terminal.is_running() {
            terminal.poll_background().unwrap();
            assert!(std::time::Instant::now() < deadline, "Grep did not finish");
            if let Some(event) = pump.wait_event_timeout(std::time::Duration::from_millis(100))
                && let Some(event) = event.as_user_event_type::<TerminalEvent>()
            {
                terminal.handle_event(event).unwrap();
            }
        }
        let output = terminal.output_text().unwrap();
        let clicked = output.find("1:hello").unwrap() + 3;
        assert_eq!(terminal.action_at_output_offset(clicked).unwrap(), Some(TerminalAction::Location(
            root.0.join("sample file.txt"), SourceLocation { line: 1, column: None, byte_column: false },
        )));

        for _ in pump.poll_iter() {} // clear notifications from the completed command
        terminal.set_events(events.clone());
        terminal.insert_text("ls");
        terminal.submit(&events).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            assert!(std::time::Instant::now() < deadline, "Listing must wake the sleeping UI");
            if let Some(event) = pump.wait_event_timeout(std::time::Duration::from_secs(1))
                && matches!(event.as_user_event_type::<TerminalEvent>(), Some(TerminalEvent::ListingReady))
            { break; }
        }

        finish_listing(&mut terminal);
        assert!(!terminal.is_running());
        assert!(terminal.entries.iter().any(|entry| entry.path.ends_with("sample file.txt")));
        terminal.insert_text("touch \"created by terminal.txt\"");
        assert_eq!(terminal.submit(&events).unwrap(), TerminalAction::None);
        assert!(!terminal.is_running());
        assert!(root.0.join("created by terminal.txt").is_file());
        fs::write(root.0.join("Cargo.lock"), "version = 4\n").unwrap();
        for input in ["Cargo.lock", "./Cargo.lock", "\"created by terminal.txt\""] {
            terminal.insert_text(input);
            assert!(matches!(terminal.submit(&events).unwrap(), TerminalAction::ListedFile(_)));
            assert!(!terminal.is_running());
        }
        terminal.insert_text("edit 'a|b.txt'");
        assert_eq!(terminal.submit(&events).unwrap(), TerminalAction::Edit("a|b.txt".into()));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn commands_skip_login_profiles_but_keep_environment_and_shell_syntax() {
        use std::os::unix::fs::PermissionsExt;
        let root = Fixture::new();
        // A login shell would fail before running any user command.
        fs::write(root.0.join(".zprofile"), "printf login-profile-loaded >&2\nexit 93\n").unwrap();
        fs::write(root.0.join(".zshenv"), "export POTYI_SHELL_TEST_READY=present\n").unwrap();
        let tool = root.0.join("potyi-startup-test");
        fs::write(&tool, "#!/bin/sh\nprintf '%s|%s' \"$POTYI_INHERITED_TEST\" \"$POTYI_SHELL_TEST_READY\"\n").unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
        let output = non_login_shell(std::ffi::OsStr::new("/bin/zsh"),
            "potyi-startup-test | /usr/bin/tr 'a-z' 'A-Z'", &root.0)
            .env("ZDOTDIR", &root.0)
            .env("PATH", &root.0)
            .env("POTYI_INHERITED_TEST", "retained")
            .spawn().unwrap().wait_with_output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, b"RETAINED|PRESENT");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("login-profile-loaded"));
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
