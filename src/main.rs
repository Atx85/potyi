// Pötyi - Lightweight text editor
// Copyright (C) 2026  Attila Banko
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


#![cfg_attr(
    all(windows, not(debug_assertions)),
    windows_subsystem = "windows"
)]

static FONT_DATA: &[u8] =
    include_bytes!("../fonts/DejaVuSansMono.ttf");
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use sdl3::event::{
    DisplayEvent,
    Event,
    WindowEvent,
};
use sdl3::keyboard::{Keycode, Mod};

use sdl3::mouse::MouseButton;
use renderer::{
    Renderer,
    TerminalHit,
    WindowControl,
};

mod dpi_text;
mod clipboard;
mod command_bar;
mod embedded_config;
mod formatting;
mod lsp;
mod lsp_ui;
mod lsp_setup;
mod workspace_edit;
#[cfg(test)]
mod benchmarks;
mod keybindings;
mod line_numbers;
mod piece_table;
mod renderer;
mod search;
mod search_ui;
mod startup;
mod window;
mod syntax;
mod syntax_core;
mod config;
mod multi_cursor;
mod terminal;
mod terminal_layout;
mod terminal_text_cache;
mod vim;

use keybindings::{Command, KeyBindings};
use piece_table::{
    Piece,
    PieceTable,
    PieceTableSnapshot,
};
use search::{
    SearchMode,
    SearchResult,
    Searcher,
};
use search_ui::SearchUi;
use config::{
    EditorConfig,
    KeybindingMode,
    LineNumberMode,
};
use clipboard::{
    copy_selection,
    cut_selection,
    paste,
    read_text,
};
use command_bar::{
    quote_argument,
    CommandBar,
    CommandBarHit,
    GotoMode,
    ParsedCommand,
};
use terminal::{
    Terminal,
    TerminalAction,
    TerminalEvent,
    OutputCommand,
};
use vim::{VimController, VimMode, VimUiAction};

// ==========================================================================
// Cursor / history
// ==========================================================================

#[derive(Clone)]
struct CursorState {
    secondary_cursors: Vec<piece_table::Cursor>,
    position: usize,
    line: usize,
    column: usize,
    desired_column: Option<usize>,
    anchor: usize,
    anchor_line: usize,
    anchor_column: usize,
}

struct TextChange {
    position: usize,
    deleted: Vec<Piece>,
    deleted_length: usize,
    inserted: Option<Piece>,
}

enum HistoryKind {
    Changes(Vec<TextChange>),
    Snapshot(PieceTableSnapshot),
    Workspace(workspace_edit::Marker),
    Sequence(Vec<HistoryKind>),
}

struct HistoryEntry {
    kind: HistoryKind,
    before: CursorState,
    after: CursorState,
}

#[derive(Clone, Copy)]
struct ReplacementRun {
    start: usize,
    end: usize,
    repetitions: usize,
}

// ==========================================================================
// Editor
// ==========================================================================

struct Editor {
    document: PieceTable,
    path: Option<PathBuf>,
    config: EditorConfig,

    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,

    multi_edit_group: Option<usize>,
    applying_history: bool,
    dirty: bool,
    read_only: bool,
}

impl Editor {

    
    fn new(config: EditorConfig) -> io::Result<Self> {
        let mut document = PieceTable::empty()?;
        piece_table::recovery::arm(&mut document, None);
        Ok(Self {
            document,
            path: None,
            config,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            multi_edit_group: None,
            applying_history: false,
            dirty: false,
            read_only: false,
        })
    }
    
    
    fn insert_tab(&mut self) -> io::Result<()> {
        if self.config.insert_spaces {
            let spaces = " ".repeat(self.config.tab_width);
            self.insert_text(&spaces)
        } else {
            self.insert_text("\t")
        }
    }
    fn cursor_state(&self) -> CursorState {
        CursorState {
            secondary_cursors: self.document.secondary_cursors.clone(),
            position: self.document.cursor.position,
            line: self.document.cursor.line,
            column: self.document.cursor.column,
            desired_column: self.document.cursor.desired_column,
            anchor: self.document.cursor.anchor,
            anchor_line: self.document.cursor.anchor_line,
            anchor_column: self.document.cursor.anchor_column,
        }
    }

    fn begin_history_group(&self) -> usize {
        self.undo_stack.len()
    }

    fn end_history_group(
        &mut self,
        start: usize,
    ) {
        // A workspace transaction must remain a top-level, coordinated undo step.
        let start = self.undo_stack.iter().enumerate().skip(start)
            .rfind(|(_, entry)| matches!(entry.kind, HistoryKind::Workspace(_)))
            .map_or(start, |(index, _)| index + 1);
        if start >= self.undo_stack.len() {
            return;
        }

        let entries: Vec<HistoryEntry> =
            self.undo_stack.drain(start..).collect();

        if entries.len() == 1 {
            self.undo_stack.push(
                entries.into_iter().next().unwrap()
            );
            return;
        }

        let before = entries.first().unwrap().before.clone();
        let after = entries.last().unwrap().after.clone();
        let kinds = entries.into_iter()
            .map(|entry| entry.kind)
            .collect();

        self.undo_stack.push(HistoryEntry {
            kind: HistoryKind::Sequence(kinds),
            before,
            after,
        });
    }

    fn apply_history_kind(
        &mut self,
        kind: &mut HistoryKind,
        undo: bool,
    ) -> io::Result<()> {
        match kind {
            HistoryKind::Changes(changes) => {
                if undo {
                    for change in changes {
                        if let Some(inserted) = change.inserted {
                            self.document.delete(
                                change.position,
                                inserted.length,
                            )?;
                        }

                        if !change.deleted.is_empty() {
                            self.document.insert_pieces(
                                change.position,
                                &change.deleted,
                            )?;
                        }
                    }
                } else {
                    for change in changes.iter_mut().rev() {
                        if !change.deleted.is_empty() {
                            self.document.delete(
                                change.position,
                                change.deleted_length,
                            )?;
                        }

                        if let Some(inserted) = change.inserted {
                            self.document.insert_pieces(
                                change.position,
                                std::slice::from_ref(&inserted),
                            )?;
                        }
                    }
                }
            }

            HistoryKind::Workspace(_) => return Err(io::Error::other("Workspace undo requires both panes")),

            HistoryKind::Snapshot(snapshot) => {
                self.document.swap_snapshot(snapshot);
            }

            HistoryKind::Sequence(kinds) => {
                if undo {
                    for kind in kinds.iter_mut().rev() {
                        self.apply_history_kind(kind, true)?;
                    }
                } else {
                    for kind in kinds {
                        self.apply_history_kind(kind, false)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn restore_cursor(
        &mut self,
        state: CursorState,
    ) {
        let collapse_occurrence_selection = self.config.keybinding_mode == KeybindingMode::Vim
            && !state.secondary_cursors.is_empty();
        self.document.secondary_cursors = if self.config.keybinding_mode == KeybindingMode::Conventional {
            state.secondary_cursors
        } else { Vec::new() };
        self.document.cursor.position =
            state.position;

        self.document.cursor.line =
            state.line;

        self.document.cursor.column =
            state.column;

        self.document.cursor.desired_column =
            state.desired_column;

        self.document.cursor.anchor =
            state.anchor;

        self.document.cursor.anchor_line =
            state.anchor_line;

        self.document.cursor.anchor_column =
            state.anchor_column;
        if collapse_occurrence_selection {
            self.document.cursor.anchor = state.position;
            self.document.cursor.anchor_line = state.line;
            self.document.cursor.anchor_column = state.column;
        }
    }

    fn new_document(&mut self, path: Option<&str>) -> io::Result<()> {
        if self.dirty {
            return Err(io::Error::other("Save the current document before creating a new file"));
        }
        if path == Some("") {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Enter a file path"));
        }

        let mut document = PieceTable::empty()?;
        if let Some(path) = path {
            document.write_to_new(std::path::Path::new(path)).map_err(|error| {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    io::Error::new(error.kind(), "File already exists. Use :open to edit it.")
                } else {
                    error
                }
            })?;
        }
        piece_table::recovery::arm(&mut document, path.map(std::path::Path::new));
        self.document = document;
        self.path = path.map(PathBuf::from);
        self.multi_edit_group = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.dirty = false;
        self.read_only = false;
        Ok(())
    }

    fn open(
        &mut self,
        path: &str,
    ) -> io::Result<()> {
        let start = Instant::now();

        let mut document =
            PieceTable::open(path)?;
        piece_table::recovery::arm(&mut document, Some(std::path::Path::new(path)));

        println!(
            "PieceTable::open({}): {:?}",
            path,
            start.elapsed()
        );

        self.document = document;
        self.path = Some(PathBuf::from(path));

        self.multi_edit_group = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.dirty = false;
        self.read_only = false;

        Ok(())
    }

fn save(&mut self) -> io::Result<()> {
    if self.read_only {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "This file is open for viewing",
        ));
    }

    if let Some(path) = self.path.as_deref() {
        self.document.write_to(path)?;
        self.document.recovery_saved(path);
        self.dirty = false;
        return Ok(());
    }

    let mut number = 1;

    let path = loop {
        let filename =
            format!("untitled_{}.txt", number);

        let path =
            std::env::current_dir()?.join(filename);

        if !path.exists() {
            break path;
        }

        number += 1;
    };

    self.document.write_to(&path)?;
    self.document.recovery_saved(&path);

    self.path = Some(path);
    self.dirty = false;

    Ok(())
}

    fn save_as(&mut self, path: &str, overwrite: bool) -> io::Result<()> {
        if self.read_only {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "This file is open for viewing",
            ));
        }
        if path.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Enter a destination path",
            ));
        }

        let destination = PathBuf::from(path);
        if overwrite || file_is_open_in(path, self) {
            self.document.write_to(&destination)?;
        } else {
            self.document.write_to_new(&destination).map_err(|error| {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    io::Error::new(
                        error.kind(),
                        "File already exists. Use :save-as! or :saveas! to overwrite it.",
                    )
                } else {
                    error
                }
            })?;
        }
        self.document.recovery_saved(&destination);
        self.path = Some(destination);
        self.dirty = false;
        Ok(())
    }

    fn recover_from(&mut self, root: &std::path::Path, number: usize) -> io::Result<bool> {
        if self.dirty { return Err(io::Error::other("Save the current document before opening recovered work, or switch to an empty pane")); }
        let (path, incomplete, session) = piece_table::recovery::restore_number_at(root, number)?;
        self.open(path.to_str().ok_or_else(|| io::Error::other("Recovery path is not UTF-8"))?)?;
        self.document.recovered_from(session);
        self.dirty = true;
        Ok(incomplete)
    }

    fn apply_formatted(
        &mut self,
        output: &mut (impl io::Read + io::Seek),
    ) -> io::Result<bool> {
        if self.read_only {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "This file is open for viewing"));
        }
        let before = self.cursor_state();
        output.rewind()?;
        let [cursor, anchor] = formatting::map_positions(output, [
            (before.line, before.column), (before.anchor_line, before.anchor_column),
        ])?;
        if output.stream_position()? == 0 && self.document.len() != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Formatter returned empty output; document preserved"));
        }
        output.rewind()?;
        let Some(snapshot) = self.document.replace_from_reader(output, formatting::MAX_OUTPUT_BYTES)? else {
            return Ok(false);
        };
        self.clear_secondary_cursors();
        let after = CursorState {
            secondary_cursors: Vec::new(),
            position: cursor.byte, line: cursor.line, column: cursor.column,
            anchor: anchor.byte, anchor_line: anchor.line, anchor_column: anchor.column,
            desired_column: None,
        };
        self.restore_cursor(after.clone());
        self.undo_stack.push(HistoryEntry { kind: HistoryKind::Snapshot(snapshot), before, after });
        self.redo_stack.clear();
        self.dirty = true;
        Ok(true)
    }

    fn format_document(&mut self, name: Option<&str>) -> io::Result<(String, bool)> {
        if self.read_only {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "This file is open for viewing"));
        }
        if self.document.len() > formatting::MAX_INPUT_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Formatting is limited to documents up to 2 MiB"));
        }
        let config = formatting::Formatters::load(std::path::Path::new("config/formatters.toml"))?;
        let (provider, executable) = config.select(self.path.as_deref(), name)?;
        let scratch = formatting::Scratch::create()?;
        let mut output = formatting::run(provider, &executable, self.path.as_deref(), &self.document, &scratch)?;
        let changed = self.apply_formatted(&mut output)?;
        Ok((provider.name.clone(), changed))
    }

    fn execute(
        &mut self,
        command: Command,
        page_lines: usize,
    ) -> io::Result<()> {
        if self.config.keybinding_mode == KeybindingMode::Conventional
            && !self.document.secondary_cursors.is_empty()
            && multi_cursor::is_cursor_motion(command)
        {
            return self.move_occurrence_cursors(command, page_lines);
        }
        if !matches!(command, Command::SelectNextOccurrence | Command::Delete | Command::Backspace
            | Command::Newline | Command::InsertTab | Command::Copy | Command::Cut | Command::Paste
            | Command::Undo | Command::Redo) {
            self.clear_secondary_cursors();
        }
        self.execute_single(command, page_lines)
    }

    fn execute_single(&mut self, command: Command, page_lines: usize) -> io::Result<()> {
        match command {
            Command::SelectNextOccurrence => self.select_next_occurrence(),
            Command::MoveLeft =>
                self.move_left(),
        
            Command::InsertTab =>
                self.insert_tab(),

            Command::MoveRight =>
                self.move_right(),

            Command::MoveUp =>
                self.move_up(),

            Command::MoveDown =>
                self.move_down(),

            Command::MoveWordLeft | Command::SelectWordLeft =>
                self.document.move_word(false, command == Command::SelectWordLeft),

            Command::MoveWordRight | Command::SelectWordRight =>
                self.document.move_word(true, command == Command::SelectWordRight),

            Command::PageUp | Command::SelectPageUp =>
                self.document.move_page(false, page_lines, command == Command::SelectPageUp),

            Command::PageDown | Command::SelectPageDown =>
                self.document.move_page(true, page_lines, command == Command::SelectPageDown),

            Command::SelectAll => self.document.select_all(),
            Command::SelectHome => self.document.select_home(),
            Command::SelectEnd => self.document.select_end(),

            Command::SelectLeft =>
                self.select_left(),

            Command::SelectRight =>
                self.select_right(),

            Command::SelectUp =>
                self.select_up(),

            Command::SelectDown =>
                self.select_down(),

            Command::Home =>
                self.home(),

            Command::End =>
                self.end(),

            Command::Delete =>
                self.delete(),

            Command::Backspace =>
                self.backspace(),

            Command::Newline =>
                self.newline(),

            Command::Save =>
                self.save(),

            Command::Undo =>
                self.undo(),

            Command::Redo =>
                self.redo(),

            Command::Copy
            | Command::Cut
            | Command::Paste =>
                Ok(()),

            Command::Quit =>
                Ok(()),

            Command::NewFile | Command::SaveAs | Command::FormatDocument => Ok(()), // Handled in the event loop.
        }
    }

    // ----------------------------------------------------------------------
    // Cursor movement
    // ----------------------------------------------------------------------

    fn move_left(&mut self) -> io::Result<()> {
        if self.document.has_selection() {
            return self.document.move_cursor(
                self.document.selection_start()
            );
        }

        self.document.cursor_left()
    }

    fn move_right(&mut self) -> io::Result<()> {
        if self.document.has_selection() {
            return self.document.move_cursor(
                self.document.selection_end()
            );
        }

        self.document.cursor_right()
    }

    fn move_up(&mut self) -> io::Result<()> {
        self.document.cursor_up()
    }

    fn move_down(&mut self) -> io::Result<()> {
        self.document.cursor_down()
    }

    fn select_left(&mut self) -> io::Result<()> {
        self.document.select_left()
    }

    fn select_right(&mut self) -> io::Result<()> {
        self.document.select_right()
    }

    fn select_up(&mut self) -> io::Result<()> {
        self.document.select_up()
    }

    fn select_down(&mut self) -> io::Result<()> {
        self.document.select_down()
    }

    fn home(&mut self) -> io::Result<()> {
        if self.document.has_selection() {
            let position =
                self.document.selection_start();

            self.document.move_cursor(position)?;
        }

        self.document.cursor_home()
    }

    fn end(&mut self) -> io::Result<()> {
        if self.document.has_selection() {
            let position =
                self.document.selection_end();

            self.document.move_cursor(position)?;
        }

        self.document.cursor_end()
    }

    // ----------------------------------------------------------------------
    // Editing
    // ----------------------------------------------------------------------

    fn newline(&mut self) -> io::Result<()> {
        self.insert_text("\n")
    }

    fn insert(
        &mut self,
        text: &str,
    ) -> io::Result<()> {
        self.insert_text(text)
    }

    fn insert_text(
        &mut self,
        text: &str,
    ) -> io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        if self.read_only {
            return Ok(());
        }

        if !self.document.secondary_cursors.is_empty() {
            return self.edit_occurrences(text, None);
        }

        let range =
            if self.document.has_selection() {
                SearchResult {
                    start:
                        self.document.selection_start(),
                    end:
                        self.document.selection_end(),
                }
            } else {
                let position =
                    self.document.cursor.position;

                SearchResult {
                    start: position,
                    end: position,
                }
            };

        self.replace_range(
            range,
            text,
        )?;

        Ok(())
    }

    /// Replace one UTF-8 byte range and record it as a single undo step.
    ///
    /// Returns `true` when the document contents changed. Replacing text
    /// with identical bytes still moves the caret past the range, but does
    /// not allocate a history entry or discard redo history.
    fn replace_range(
        &mut self,
        range: SearchResult,
        replacement: &str,
    ) -> io::Result<bool> {
        if self.read_only {
            return Ok(false);
        }

        if range.end < range.start
            || range.end > self.document.len()
        {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "replacement range is outside document",
                )
            );
        }

        let deleted_length =
            range.end - range.start;

        self.document
            .len()
            .checked_sub(deleted_length)
            .and_then(|length| {
                length.checked_add(replacement.len())
            })
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "replacement would overflow document length",
                )
            })?;

        let before =
            self.cursor_state();

        self.clear_secondary_cursors();

        let changed =
            !self.document.range_equals(
                range.start,
                deleted_length,
                replacement.as_bytes(),
            )?;

        let mut deleted = Vec::new();
        let mut inserted = None;

        if changed {
            deleted =
                self.document.capture_range(
                    range.start,
                    deleted_length,
                )?;

            inserted =
                self.document.store_text(
                    replacement
                )?;

            if deleted_length > 0 {
                self.document.delete(
                    range.start,
                    deleted_length,
                )?;
            }

            if let Some(piece) = inserted {
                self.document.insert_pieces(
                    range.start,
                    std::slice::from_ref(
                        &piece
                    ),
                )?;
            }
        }

        let new_position =
            range.start
                .checked_add(replacement.len())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "replacement cursor overflow",
                    )
                })?;

        self.set_cursor_and_anchor(
            new_position,
            new_position,
        )?;

        if changed && !self.applying_history {
            let after =
                self.cursor_state();

            self.undo_stack.push(
                HistoryEntry {
                    kind: HistoryKind::Changes(
                        vec![
                            TextChange {
                                position: range.start,
                                deleted,
                                deleted_length,
                                inserted,
                            }
                        ]
                    ),
                    before,
                    after,
                }
            );

            self.redo_stack.clear();
        }

        if changed {
            self.dirty = true;
        }

        Ok(changed)
    }

    /// Replace all leftmost, non-overlapping matches in one transaction.
    ///
    /// The unchanged document is searched once. Literal case-sensitive
    /// search therefore keeps its fixed working buffer; case-insensitive
    /// and regex search use the searcher's one document snapshot. Changes
    /// are then rebuilt in one forward piece-table pass, and one undo
    /// restores the entire operation.
    fn replace_all(
        &mut self,
        searcher: &Searcher,
        replacement: &str,
    ) -> io::Result<usize> {
        if self.read_only {
            return Ok(0);
        }

        let before =
            self.cursor_state();

        self.clear_secondary_cursors();

        let mut runs:
            Vec<ReplacementRun> = Vec::new();

        let mut mapped_position = None;
        let mut mapped_anchor = None;
        let mut added_before = 0usize;
        let mut removed_before = 0usize;

        let count = {
            let document =
                &self.document;

            searcher
                .visit_non_overlapping_replacements(
                    document,
                    replacement,
                    |found| {
                        if found.end < found.start
                            || found.end > document.len()
                        {
                            return Err(
                                io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "search returned an invalid replacement range",
                                )
                            );
                        }

                        Self::observe_position_change(
                            before.position,
                            &mut mapped_position,
                            found,
                            added_before,
                            removed_before,
                            replacement.len(),
                        )?;

                        Self::observe_position_change(
                            before.anchor,
                            &mut mapped_anchor,
                            found,
                            added_before,
                            removed_before,
                            replacement.len(),
                        )?;

                        let deleted_length =
                            found.end - found.start;

                        if replacement.len()
                            >= deleted_length
                        {
                            added_before =
                                added_before
                                    .checked_add(
                                        replacement.len()
                                            - deleted_length
                                    )
                                    .ok_or_else(|| {
                                        io::Error::new(
                                            io::ErrorKind::InvalidData,
                                            "replacement position overflow",
                                        )
                                    })?;
                        } else {
                            removed_before =
                                removed_before
                                    .checked_add(
                                        deleted_length
                                            - replacement.len()
                                    )
                                    .ok_or_else(|| {
                                        io::Error::new(
                                            io::ErrorKind::InvalidData,
                                            "replacement position overflow",
                                        )
                                    })?;
                        }

                        if let Some(last) =
                            runs.last_mut()
                        {
                            if last.end
                                == found.start
                            {
                                last.end =
                                    found.end;

                                last.repetitions =
                                    last.repetitions
                                        .checked_add(1)
                                        .ok_or_else(|| {
                                            io::Error::new(
                                                io::ErrorKind::InvalidData,
                                                "replacement count overflow",
                                            )
                                        })?;

                                return Ok(());
                            }
                        }

                        runs.push(
                            ReplacementRun {
                                start: found.start,
                                end: found.end,
                                repetitions: 1,
                            }
                        );

                        Ok(())
                    },
                )?
        };

        if runs.is_empty() {
            return Ok(0);
        }

        let position =
            Self::finish_position_mapping(
                before.position,
                mapped_position,
                added_before,
                removed_before,
            )?;

        let anchor =
            Self::finish_position_mapping(
                before.anchor,
                mapped_anchor,
                added_before,
                removed_before,
            )?;

        let snapshot =
            self.document
                .replace_runs(
                    runs.iter().map(|run| {
                        (
                            run.start,
                            run.end,
                            run.repetitions,
                        )
                    }),
                    replacement,
                )?
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "non-empty replacement plan produced no snapshot",
                    )
                })?;

        self.set_cursor_and_anchor(
            position,
            anchor,
        )?;

        let after =
            self.cursor_state();

        if !self.applying_history {
            self.undo_stack.push(
                HistoryEntry {
                    kind:
                        HistoryKind::Snapshot(
                            snapshot
                        ),
                    before,
                    after,
                }
            );

            self.redo_stack.clear();
        }

        self.dirty = true;

        Ok(count)
    }

    fn observe_position_change(
        position: usize,
        mapped: &mut Option<usize>,
        range: SearchResult,
        added_before: usize,
        removed_before: usize,
        replacement_length: usize,
    ) -> io::Result<()> {
        if mapped.is_some() {
            return Ok(());
        }

        let base =
            if position < range.start {
                position
            } else if position <= range.end {
                range.start
                    .checked_add(
                        replacement_length
                    )
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "mapped replacement position overflow",
                        )
                    })?
            } else {
                return Ok(());
            };

        *mapped =
            Some(
                base
                    .checked_add(added_before)
                    .and_then(|value| {
                        value.checked_sub(
                            removed_before
                        )
                    })
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "mapped replacement position overflow",
                        )
                    })?
            );

        Ok(())
    }

    fn finish_position_mapping(
        position: usize,
        mapped: Option<usize>,
        added_before: usize,
        removed_before: usize,
    ) -> io::Result<usize> {
        if let Some(mapped) = mapped {
            return Ok(mapped);
        }

        position
            .checked_add(added_before)
            .and_then(|value| {
                value.checked_sub(removed_before)
            })
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "mapped replacement position overflow",
                )
            })
    }

    fn set_cursor_and_anchor(
        &mut self,
        position: usize,
        anchor: usize,
    ) -> io::Result<()> {
        let (line, column) =
            self.document.line_column_at(
                position
            )?;

        let (
            anchor_line,
            anchor_column,
        ) =
            self.document.line_column_at(
                anchor
            )?;

        self.document.cursor.position =
            position;

        self.document.cursor.line =
            line;

        self.document.cursor.column =
            column;

        self.document.cursor.anchor =
            anchor;

        self.document.cursor.anchor_line =
            anchor_line;

        self.document.cursor.anchor_column =
            anchor_column;

        self.document.cursor.desired_column =
            None;

        Ok(())
    }

    fn delete_range(
        &mut self,
        position: usize,
        length: usize,
    ) -> io::Result<()> {
        if length == 0 {
            return Ok(());
        }

        let end =
            position
                .checked_add(length)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "delete range overflow",
                    )
                })?;

        self.replace_range(
            SearchResult {
                start: position,
                end,
            },
            "",
        )?;

        Ok(())
    }

    fn delete(&mut self) -> io::Result<()> {
        if !self.document.secondary_cursors.is_empty() {
            return self.edit_occurrences("", Some(false));
        }
        if self.document.has_selection() {
            return self.delete_selection();
        }

        let position =
            self.document.cursor.position;

        if position >= self.document.len() {
            return Ok(());
        }

        let next =
            self.document.next_char_boundary(
                position
            )?;

        self.delete_range(
            position,
            next - position,
        )
    }

    fn backspace(&mut self) -> io::Result<()> {
        if !self.document.secondary_cursors.is_empty() {
            return self.edit_occurrences("", Some(true));
        }
        if self.document.has_selection() {
            return self.delete_selection();
        }

        let position =
            self.document.cursor.position;

        if position == 0 {
            return Ok(());
        }

        let previous =
            self.document.previous_char_boundary(
                position
            )?;

        self.delete_range(
            previous,
            position - previous,
        )
    }

    fn delete_selection(&mut self) -> io::Result<()> {
        if !self.document.secondary_cursors.is_empty() {
            return self.edit_occurrences("", None);
        }
        if !self.document.has_selection() {
            return Ok(());
        }

        let start =
            self.document.selection_start();

        let end =
            self.document.selection_end();

        self.delete_range(
            start,
            end - start,
        )
    }

    // ----------------------------------------------------------------------
    // Undo / redo
    // ----------------------------------------------------------------------

    fn undo(&mut self) -> io::Result<()> {
        if self.undo_stack.last().is_some_and(|e| matches!(e.kind, HistoryKind::Workspace(_))) {
            return Err(io::Error::other("Workspace undo requires both panes"));
        }
        self.multi_edit_group = None;
        if self.read_only {
            return Ok(());
        }

        let mut entry =
            match self.undo_stack.pop() {
                Some(entry) => entry,
                None => return Ok(()),
            };

        self.applying_history = true;

        let result: io::Result<()> = (|| {
            self.apply_history_kind(
                &mut entry.kind,
                true,
            )?;
            self.restore_cursor(entry.before.clone());

            Ok(())
        })();

        self.applying_history = false;

        result?;

        self.redo_stack.push(entry);
        self.dirty = true;

        Ok(())
    }

    fn redo(&mut self) -> io::Result<()> {
        if self.redo_stack.last().is_some_and(|e| matches!(e.kind, HistoryKind::Workspace(_))) {
            return Err(io::Error::other("Workspace undo requires both panes"));
        }
        self.multi_edit_group = None;
        if self.read_only {
            return Ok(());
        }

        let mut entry =
            match self.redo_stack.pop() {
                Some(entry) => entry,
                None => return Ok(()),
            };

        self.applying_history = true;

        let result: io::Result<()> = (|| {
            self.apply_history_kind(
                &mut entry.kind,
                false,
            )?;
            self.restore_cursor(entry.after.clone());

            Ok(())
        })();

        self.applying_history = false;

        result?;

        self.undo_stack.push(entry);
        self.dirty = true;

        Ok(())
    }
}

// ==========================================================================
// Search keyboard handling
// ==========================================================================

fn ctrl_pressed(
    keymod: Mod,
) -> bool {
    keymod.intersects(
        Mod::LCTRLMOD | Mod::RCTRLMOD
    )
}

fn shift_pressed(
    keymod: Mod,
) -> bool {
    keymod.intersects(
        Mod::LSHIFTMOD | Mod::RSHIFTMOD
    )
}

fn alt_pressed(
    keymod: Mod,
) -> bool {
    keymod.intersects(
        Mod::LALTMOD | Mod::RALTMOD
    )
}

fn handle_search_key(
    search_ui: &mut SearchUi,
    editor: &mut Editor,
    key: Keycode,
    keymod: Mod,
    repeat: bool,
) -> io::Result<SearchKeyResult> {
    /*
     * The main event loop handles opening/collapsing the panel before this
     * modal key handler runs. Consume the shortcuts here as a safeguard.
     */
    if (key == Keycode::F
        || key == Keycode::H)
        && ctrl_pressed(keymod)
    {
        return Ok(SearchKeyResult::Consumed);
    }

    if search_ui.is_replace_mode()
        && !repeat
    {
        let enter =
            key == Keycode::Return
                || key == Keycode::KpEnter;

        if (key == Keycode::R
                && alt_pressed(keymod))
            || (enter
                && ctrl_pressed(keymod)
                && !shift_pressed(keymod))
        {
            return Ok(
                SearchKeyResult::ReplaceCurrent
            );
        }

        if (key == Keycode::A
                && alt_pressed(keymod))
            || (enter
                && ctrl_pressed(keymod)
                && shift_pressed(keymod))
        {
            return Ok(
                SearchKeyResult::ReplaceAll
            );
        }

        if key == Keycode::M
            && alt_pressed(keymod)
        {
            search_ui.cycle_mode(
                &mut editor.document,
                shift_pressed(keymod),
            )?;

            return Ok(SearchKeyResult::Consumed);
        }
    }

    match key {
        Keycode::Escape => {
            search_ui.close();

            Ok(SearchKeyResult::Closed)
        }

        Keycode::Tab => {
            if !repeat {
                if search_ui.is_replace_mode() {
                    search_ui.focus_next_field(
                        shift_pressed(keymod)
                    );
                } else {
                    search_ui.cycle_mode(
                        &mut editor.document,
                        shift_pressed(keymod),
                    )?;
                }
            }

            Ok(SearchKeyResult::Consumed)
        }

Keycode::Return | Keycode::KpEnter => {
    if shift_pressed(keymod) {
        search_ui.previous(
            &mut editor.document
        )?;
    } else {
        search_ui.next(
            &mut editor.document
        )?;
    }

    // Jump the editor cursor to the found match.
    if let Some(current) = search_ui.current_match() {
        editor.document.move_cursor(current.start)?;
    }

    Ok(SearchKeyResult::Consumed)
}

        Keycode::Backspace => {
            search_ui.backspace_focused(
                &mut editor.document
            )?;

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::Delete => {
            search_ui.delete_focused(
                &mut editor.document
            )?;

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::Left => {
            search_ui.move_focused_left();

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::Right => {
            search_ui.move_focused_right();

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::Home => {
            search_ui.move_focused_home();

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::End => {
            search_ui.move_focused_end();

            Ok(SearchKeyResult::Consumed)
        }

        _ => Ok(SearchKeyResult::Ignored),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchKeyResult {
    Consumed,
    Closed,
    Ignored,
    ReplaceCurrent,
    ReplaceAll,
}

fn replacement_requires_cursor_refresh(
    result: SearchKeyResult,
    has_current_match: bool,
) -> bool {
    !has_current_match
        && matches!(
            result,
            SearchKeyResult::ReplaceCurrent
                | SearchKeyResult::ReplaceAll
        )
}

fn replace_current_match(
    search_ui: &mut SearchUi,
    editor: &mut Editor,
) -> io::Result<usize> {
    let current =
        match search_ui.current_match() {
            Some(current) => current,
            None => {
                search_ui.set_last_replace_count(
                    Some(0)
                );

                return Ok(0);
            }
        };

    let replacement =
        search_ui.replacement().to_owned();

    // Calculate progress against the original document before mutation.
    // For a zero-width regex result, advancing one original Unicode scalar
    // prevents repeatedly inserting at the same byte position.
    let original_resume =
        if current.start == current.end {
            if current.end < editor.document.len() {
                Some(
                    editor.document
                        .next_char_boundary(
                            current.end
                        )?
                )
            } else {
                None
            }
        } else {
            Some(current.end)
        };

    let changed =
        editor.replace_range(
            SearchResult {
                start: current.start,
                end: current.end,
            },
            &replacement,
        )?;

    let resume =
        if current.start == current.end {
            match original_resume {
                Some(position) => Some(
                    position
                        .checked_add(
                            replacement.len()
                        )
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "replacement search position overflow",
                            )
                        })?
                ),
                None => None,
            }
        } else {
            Some(
                current.start
                    .checked_add(
                        replacement.len()
                    )
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "replacement search position overflow",
                        )
                    })?
            )
        };

    search_ui.refresh_after_replace(
        &mut editor.document,
        resume,
    )?;

    let count =
        usize::from(changed);

    search_ui.set_last_replace_count(
        Some(count)
    );

    Ok(count)
}

fn replace_all_matches(
    search_ui: &mut SearchUi,
    editor: &mut Editor,
) -> io::Result<usize> {
    let replacement =
        search_ui.replacement().to_owned();

    let count =
        match search_ui.searcher() {
            Some(searcher) =>
                editor.replace_all(
                    searcher,
                    &replacement,
                )?,
            None => 0,
        };

    search_ui.finish_replace_all(count);

    Ok(count)
}

fn search_mode_option(
    mode: SearchMode,
) -> &'static str {
    match mode {
        SearchMode::CaseSensitive => "",
        SearchMode::CaseInsensitive =>
            " --ignore-case",
        SearchMode::Regex => " --regex",
    }
}

fn find_command_text(
    search_ui: &SearchUi,
) -> String {
    if search_ui.query().is_empty() {
        return ":find ".to_string();
    }

    format!(
        ":find {}{}",
        quote_argument(search_ui.query()),
        search_mode_option(search_ui.mode()),
    )
}

fn replace_command_text(
    search_ui: &SearchUi,
) -> String {
    format!(
        ":replace {} {}{}",
        quote_argument(search_ui.query()),
        quote_argument(
            search_ui.replacement()
        ),
        search_mode_option(search_ui.mode()),
    )
}

fn sync_command_search(
    command_bar: &CommandBar,
    search_ui: &mut SearchUi,
    table: &mut PieceTable,
    origin: Option<usize>,
) -> io::Result<()> {
    match command_bar.parse() {
        Ok(ParsedCommand::Find {
            query,
            mode,
            backward,
        }) => {
            let origin = origin.unwrap_or(
                table.cursor.position
            );

            search_ui.configure_command_from(
                table,
                &query,
                None,
                mode,
                origin,
                backward,
            )?;
        }

        Ok(ParsedCommand::Replace {
            query,
            replacement,
            mode,
            ..
        }) => {
            search_ui.configure_command(
                table,
                &query,
                Some(&replacement),
                mode,
            )?;
        }

        _ => search_ui.close(),
    }

    Ok(())
}

#[derive(Default)]
struct CommandOutcome {
    quit: bool,
    toggle_split: bool,
    focus_other: bool,
    document_changed: bool,
    document_reloaded: bool,
    path_changed: bool,
    cursor_changed: bool,
    keybinding_mode: Option<KeybindingMode>,
}

fn goto_line_index(
    table: &mut PieceTable,
    line: isize,
    mode: GotoMode,
    configured_mode: LineNumberMode,
) -> Result<usize, String> {
    let current_line = table.cursor.line;
    if mode == GotoMode::Automatic
        && configured_mode != LineNumberMode::Normal
        && line >= 0
    {
        let label = line as usize;
        let mut matches = Vec::new();
        // Only the cursor line and the two lines at this distance can
        // carry this label. Reuse the gutter's numbering rules exactly.
        for candidate in [
            Some(current_line),
            current_line.checked_sub(label),
            current_line.checked_add(label),
        ].into_iter().flatten() {
            if matches.contains(&candidate)
                || line_numbers::LineNumbers::display_number(
                    configured_mode, candidate, current_line,
                ) != label
            {
                continue;
            }
            table.ensure_line_cached(candidate)
                .map_err(|error| error.to_string())?;
            if candidate < table.cached_line_count() {
                matches.push(candidate);
            }
        }
        return match matches.as_slice() {
            [destination] => Ok(*destination),
            [] => Err(format!("No line is labelled {label}.")),
            _ => Err(format!(
                "Label {label} matches multiple lines. Use :goto -{label} (up), :goto +{label} (down), or --abs."
            )),
        };
    }

    // Signed arguments and --rel are both resolved to Relative by the parser.
    // Automatic here is an absolute label in normal mode, never an offset.
    let relative = mode == GotoMode::Relative;

    let destination = if relative {
        if line >= 0 {
            current_line
                .checked_add(line as usize)
                .ok_or_else(|| {
                    "Relative line is out of range"
                        .to_string()
                })
        } else {
            current_line
                .checked_sub(line.unsigned_abs())
                .ok_or_else(|| {
                    "Relative line is before the start of the document"
                        .to_string()
                })
        }
    } else {
        usize::try_from(line)
            .ok()
            .and_then(|line| line.checked_sub(1))
            .ok_or_else(|| {
                "line numbers start at 1"
                    .to_string()
            })
    }?;

    table.ensure_line_cached(destination)
        .map_err(|error| error.to_string())?;
    if destination >= table.cached_line_count() {
        return Err(format!(
            "Line {} is past the last line ({}).",
            destination + 1,
            table.cached_line_count(),
        ));
    }
    Ok(destination)
}

fn execute_command_bar(
    command_bar: &mut CommandBar,
    search_ui: &mut SearchUi,
    editor: &mut Editor,
    other_editor: &mut Editor,
    renderer: &mut Renderer<'_>,
    terminal: &mut Terminal,
    vim: &mut VimController,
    reverse_find: bool,
    other_vim: &mut VimController,
    lsp_ui: &mut lsp_ui::LspUi,
) -> CommandOutcome {
    let mut outcome =
        CommandOutcome::default();

    let other_history_len = other_editor.undo_stack.len();
    if let Some(result) = lsp_ui.review(editor, other_editor, command_bar) {
        match result {
            Ok(true) => {
                vim.finish_formatting(editor);
                if other_editor.undo_stack.len() > other_history_len {
                    other_vim.finish_formatting(other_editor);
                }
                search_ui.close();
                outcome.document_changed = true;
                outcome.cursor_changed = true;
                outcome.path_changed = true;
            }
            Ok(false) => {},
            Err(error) => command_bar.show_info(&error),
        }
        return outcome;
    }
    editor.clear_secondary_cursors();
    let previous_epoch = command_bar.epoch();
    let execute = command_bar.prepare_execute();
    if previous_epoch != command_bar.epoch() {
        if let Err(error) = sync_command_search(command_bar, search_ui, &mut editor.document, vim.search_origin()) {
            command_bar.set_status(error.to_string());
            return outcome;
        }
        outcome.cursor_changed = search_ui.current_match().is_some();
    }
    if !execute {
        return outcome;
    }

    match command_bar.parse() {
        Ok(ParsedCommand::Find {
            backward,
            ..
        }) => {
            let result =
                if backward || reverse_find {
                    search_ui.previous(
                        &mut editor.document
                    )
                } else {
                    search_ui.next(
                        &mut editor.document
                    )
                };

            match result {
                Ok(()) => {
                    outcome.cursor_changed = true;
                }

                Err(error) => {
                    command_bar.set_status(
                        error.to_string()
                    );
                }
            }
        }

        Ok(ParsedCommand::Replace {
            all,
            ..
        }) => {
            let result =
                if all {
                    replace_all_matches(
                        search_ui,
                        editor,
                    )
                } else {
                    replace_current_match(
                        search_ui,
                        editor,
                    )
                };

            match result {
                Ok(count) => {
                    outcome.document_changed =
                        count > 0;
                    outcome.cursor_changed = true;
                }

                Err(error) => {
                    command_bar.set_status(
                        error.to_string()
                    );
                }
            }
        }

        Ok(ParsedCommand::Goto {
            line,
            column,
            mode,
        }) => {
            let result = goto_line_index(
                &mut editor.document,
                line,
                mode,
                editor.config.line_numbers,
            ).and_then(|line| {
                editor.document
                    .move_cursor_to_line_column(
                        line,
                        column.unwrap_or(1) - 1,
                    )
                    .map_err(|error| error.to_string())
            });

            match result {
                Ok(()) => {
                    command_bar.close();
                    search_ui.close();
                    outcome.cursor_changed = true;
                }

                Err(error) => {
                    command_bar.set_status(
                        error
                    );
                }
            }
        }

        Ok(ParsedCommand::New { path }) => {
            let result = if path.as_deref().is_some_and(|path| file_is_open_in(path, other_editor)) {
                Err(io::Error::other("This file is already open in the other pane"))
            } else {
                editor.new_document(path.as_deref())
            };
            match result {
                Ok(()) => {
                    command_bar.close();
                    search_ui.close();
                    outcome.document_reloaded = true;
                    outcome.path_changed = true;
                    outcome.cursor_changed = true;
                }
                Err(error) => command_bar.set_status(error.to_string()),
            }
        }

        Ok(ParsedCommand::Open {
            path,
        }) => {
            if file_is_open_in(&path, other_editor) {
                command_bar.close();
                search_ui.close();
                outcome.focus_other = true;
                return outcome;
            }

            match editor.open(&path) {
                Ok(()) => {
                    command_bar.close();
                    search_ui.close();
                    outcome.document_reloaded = true;
                    outcome.path_changed = true;
                    outcome.cursor_changed = true;
                }

                Err(error) => {
                    command_bar.set_status(
                        error.to_string()
                    );
                }
            }
        }

        Ok(ParsedCommand::SetFontSize {
            points,
        }) => {
            match renderer.set_font_size(
                points as f32
            ) {
                Ok(()) => {
                    editor.config.font_size =
                        points;
                    other_editor.config.font_size =
                        points;
                    command_bar.close();
                    search_ui.close();
                    outcome.cursor_changed = true;
                }

                Err(error) => {
                    command_bar.set_status(error);
                }
            }
        }

        Ok(ParsedCommand::SetLineNumbers {
            mode,
        }) => {
            editor.config.line_numbers = mode;
            other_editor.config.line_numbers = mode;
            renderer.set_line_number_mode(mode);
            command_bar.close();
            search_ui.close();
            outcome.cursor_changed = true;
        }

        Ok(ParsedCommand::SetKeybindings {
            mode,
        }) => {
            editor.config.keybinding_mode = mode;
            other_editor.config.keybinding_mode = mode;
            command_bar.close();
            search_ui.close();
            outcome.cursor_changed = true;
            outcome.keybinding_mode = Some(mode);
        }

        Ok(ParsedCommand::Term) => {
            command_bar.close();
            search_ui.close();
            terminal.open(
                editor.path.as_deref()
            );
        }

        Ok(ParsedCommand::Split) => {
            command_bar.close();
            search_ui.close();
            outcome.toggle_split = true;
        }

        Ok(ParsedCommand::Format { provider }) => {
            outcome.document_changed = run_format_command(editor, search_ui, command_bar, vim, provider.as_deref());
            outcome.cursor_changed = outcome.document_changed;
            if outcome.document_changed && editor.config.keybinding_mode == KeybindingMode::Vim {
                renderer.set_mode_label(Some(vim.mode_label()));
            }
        }

        Ok(ParsedCommand::Hover) | Ok(ParsedCommand::Definition) => {
            let action = if matches!(command_bar.parse(), Ok(ParsedCommand::Hover)) { lsp::Action::Hover } else { lsp::Action::Definition };
            if let Err(error) = lsp_ui.request(action, editor, other_editor, command_bar) {
                command_bar.show_info(&error);
            }
        }
        Ok(ParsedCommand::Actions { refactor_only }) => {
            let action = lsp::Action::CodeActions { anchor: editor.document.cursor.anchor, refactor_only };
            if let Err(error) = lsp_ui.request(action, editor, other_editor, command_bar) {
                command_bar.show_info(&error);
            }
        }
        Ok(ParsedCommand::Rename { name }) => {
            if let Err(error) = lsp_ui.request(lsp::Action::Rename(name), editor, other_editor, command_bar) {
                command_bar.show_info(&error);
            }
        }
        Ok(ParsedCommand::LspBack) => {
            match lsp_ui.go_back(editor, other_editor) {
                Ok(result) => { command_bar.close(); search_ui.close(); outcome = result; }
                Err(error) => command_bar.show_info(&error),
            }
        }
        Ok(ParsedCommand::LspStart | ParsedCommand::LspRestart) => {
            let restart = matches!(command_bar.parse(), Ok(ParsedCommand::LspRestart));
            if let Err(error) = lsp_ui.start(restart, editor, other_editor, command_bar) {
                command_bar.show_info(&error);
            }
        }
        Ok(ParsedCommand::LspStatus) => command_bar.show_info(&lsp_ui.status(editor)),
        Ok(ParsedCommand::LspInstall { server }) => {
            if let Err(error) = lsp_ui.setup(true, Some(&server), editor, command_bar) { command_bar.show_info(&error); }
        }
        Ok(ParsedCommand::LspDoctor { server }) => {
            if let Err(error) = lsp_ui.setup(false, server.as_deref(), editor, command_bar) { command_bar.show_info(&error); }
        }
        Ok(ParsedCommand::LspStop) => { lsp_ui.stop(); command_bar.show_info("LSP stopped; any setup is being cancelled. Autocomplete is paused. Use :lsp start to connect again."); }

        Ok(ParsedCommand::Formatters) => {
            match formatting::Formatters::load(std::path::Path::new("config/formatters.toml")) {
                Ok(config) => command_bar.show_formatters(config.choices(editor.path.as_deref())),
                Err(error) => command_bar.set_status(error.to_string()),
            }
        }

        Ok(ParsedCommand::ExtractConfig) => {
            match embedded_config::extract_defaults(
                std::path::Path::new("config")
            ) {
                Ok(summary) if summary.created == 0 => {
                    command_bar.set_status(format!(
                        "All {} config files already exist; nothing was overwritten",
                        summary.existing,
                    ));
                }

                Ok(summary) => {
                    command_bar.set_status(format!(
                        "Created {} config files; preserved {} existing files",
                        summary.created,
                        summary.existing,
                    ));
                }

                Err(error) => {
                    command_bar.set_status(format!(
                        "Failed to extract config: {error}"
                    ));
                }
            }
        }

        Ok(ParsedCommand::Save) => {
            match editor.save() {
                Ok(()) => {
                    command_bar.close();
                    search_ui.close();
                    outcome.path_changed = true;
                }

                Err(error) => {
                    command_bar.set_status(
                        error.to_string()
                    );
                }
            }
        }

        Ok(ParsedCommand::SaveAs { path, overwrite }) => {
            if let Some(path) = path {
                match save_as_in_pane(editor, other_editor, &path, overwrite) {
                    Ok(()) => {
                        command_bar.close();
                        search_ui.close();
                        outcome.path_changed = true;
                    }
                    Err(error) => command_bar.set_status(error.to_string()),
                }
            } else {
                command_bar.open(if overwrite { ":save-as! " } else { ":save-as " });
                command_bar.set_status("Enter a destination path; quote paths containing spaces");
                search_ui.close();
            }
        }

        Ok(ParsedCommand::Recover { number }) => {
            if let Some(number) = number {
                if editor.dirty {
                    command_bar.show_info("Save the current document before opening recovered work, or switch to an empty pane.");
                } else {
                    let result = piece_table::recovery::root().and_then(|root| editor.recover_from(&root, number));
                    match result {
                        Ok(incomplete) => {
                            search_ui.close();
                            outcome.document_reloaded = true; outcome.path_changed = true; outcome.cursor_changed = true;
                            command_bar.show_info(if incomplete { "Recovered through the last complete edit; an interrupted journal tail was ignored. Save As chooses the destination. The original file is unchanged." } else { "Recovered into a separate file. Save As chooses the destination; Save keeps this recovery copy. The original file is unchanged." });
                        }
                        Err(error) => command_bar.show_info(&format!("Recovery could not finish: {error}. The recovery files have been kept.")),
                    }
                }
            } else {
                command_bar.show_info(&piece_table::recovery::describe().unwrap_or_else(|error| format!("Could not list recovery sessions: {error}")));
            }
        }

        Ok(ParsedCommand::Quit) => {
            outcome.quit = true;
        }

        Err(error) => {
            command_bar.set_status(error);
        }
    }

    outcome
}

fn run_format_command(
    editor: &mut Editor,
    search_ui: &mut SearchUi,
    command_bar: &mut CommandBar,
    vim: &mut VimController,
    provider: Option<&str>,
) -> bool {
    match editor.format_document(provider) {
        Ok((name, changed)) => {
            if changed && editor.config.keybinding_mode == KeybindingMode::Vim {
                vim.finish_formatting(editor);
            }
            if changed { search_ui.close(); }
            command_bar.set_status(if changed {
                format!("Formatted with {name}; Ctrl+Z undoes the change")
            } else {
                format!("Already formatted ({name})")
            });
            changed
        }
        Err(error) => { command_bar.set_status(error.to_string()); false }
    }
}

fn save_as_in_pane(
    editor: &mut Editor,
    other_editor: &Editor,
    path: &str,
    overwrite: bool,
) -> io::Result<()> {
    if file_is_open_in(path, other_editor) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "That file is open in the other pane. Choose a different destination.",
        ));
    }
    editor.save_as(path, overwrite)
}

fn parse_location(arg: &str) -> Option<(&str, usize, Option<usize>)> {
    let (before_last, last) =
        arg.rsplit_once(':')?;

    let last_number = last.parse::<usize>()
        .ok()?;

    if let Some((path, line)) =
        before_last.rsplit_once(':')
        && let Ok(line) = line.parse::<usize>()
    {
        return Some((
            path,
            line,
            Some(last_number),
        ));
    }

    Some((before_last, last_number, None))
}

fn file_is_open_in(
    path: &str,
    editor: &Editor,
) -> bool {
    let Some(open_path) = editor.path.as_deref()
    else {
        return false;
    };

    let requested = std::path::Path::new(path);

    match (
        std::fs::canonicalize(requested),
        std::fs::canonicalize(open_path),
    ) {
        (Ok(requested), Ok(open_path)) =>
            requested == open_path,
        _ => requested == open_path,
    }
}

fn focus_pane(
    target: usize,
    active_pane: &mut usize,
    editor: &mut Editor,
    other_editor: &mut Editor,
    vim: &mut VimController,
    other_vim: &mut VimController,
    renderer: &mut Renderer<'_>,
) {
    let target = target.min(1);

    if target == *active_pane {
        return;
    }

    std::mem::swap(editor, other_editor);
    std::mem::swap(vim, other_vim);
    renderer.swap_view();
    *active_pane = target;
    renderer.set_active_pane(target);
    renderer.set_file_path(editor.path.as_deref());
    renderer.invalidate_scroll_cache();
    renderer.update_cursor(&editor.document);
    renderer.ensure_cursor_visible(&mut editor.document);
    renderer.set_mode_label(
        (editor.config.keybinding_mode == KeybindingMode::Vim)
            .then_some(vim.mode_label())
    );
}

fn apply_keybinding_mode(
    mode: KeybindingMode,
    vim_enabled: &mut bool,
    editor: &mut Editor,
    other_editor: &mut Editor,
    vim: &mut VimController,
    other_vim: &mut VimController,
    renderer: &mut Renderer<'_>,
) {
    editor.clear_secondary_cursors();
    other_editor.clear_secondary_cursors();
    *vim_enabled = mode == KeybindingMode::Vim;

    if *vim_enabled {
        for target in [&mut *editor, &mut *other_editor] {
            let cursor = &mut target.document.cursor;
            cursor.anchor = cursor.position;
            cursor.anchor_line = cursor.line;
            cursor.anchor_column = cursor.column;
        }
        vim.reset();
        other_vim.reset();
        renderer.set_mode_label(Some(vim.mode_label()));
    } else {
        vim.deactivate(editor);
        other_vim.deactivate(other_editor);
        renderer.set_mode_label(None);
    }
}

/// Return whether the requested file belongs in the other pane. Reuse an
/// already-open document before considering replacement, preserving its edits.
fn open_terminal_document(
    path: &str,
    read_only: bool,
    editor: &mut Editor,
    other_editor: &mut Editor,
) -> io::Result<bool> {
    if file_is_open_in(path, editor) {
        return Ok(false);
    }
    if file_is_open_in(path, other_editor) {
        return Ok(true);
    }
    let focus_other = editor.dirty;
    let target = if focus_other { other_editor } else { editor };
    if target.dirty {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Both panes have unsaved changes. Save one file before opening another.",
        ));
    }
    target.open(path)?;
    target.read_only = read_only;
    Ok(focus_other)
}

fn move_to_terminal_location(
    document: &mut PieceTable,
    line: usize,
    column: Option<usize>,
    byte_column: bool,
) -> io::Result<()> {
    let column = column.unwrap_or(1).saturating_sub(1);
    let column = if byte_column {
        let text = document.line_text(line.saturating_sub(1))?;
        text.char_indices().take_while(|(offset, _)| *offset < column).count()
    } else { column };
    document.move_cursor_to_line_column(line.saturating_sub(1), column)
}

fn handle_terminal_action(
    action: TerminalAction,
    terminal: &mut Terminal,
    editor: &mut Editor,
    other_editor: &mut Editor,
    renderer: &mut Renderer<'_>,
) -> Result<bool, String> {
    let read_only = matches!(action, TerminalAction::View(_));
    let (path, line, column, byte_column, read_only) = match action {
        TerminalAction::None => return Ok(false),
        TerminalAction::Commit(commit) => {
            if let Err(error) = terminal.open_commit(commit) { terminal.set_status(error.to_string()); }
            return Ok(false);
        }
        TerminalAction::EnterDirectory(path) => {
            if let Err(error) = terminal.enter_directory(&path) {
                terminal.set_status(error.to_string());
            }
            return Ok(false);
        }
        TerminalAction::ListedFile(path) => (path, None, None, false, false),
        TerminalAction::Location(path, location) =>
            (path, Some(location.line), location.column, location.byte_column, false),
        TerminalAction::Edit(value) | TerminalAction::View(value) => {
            let (path_text, line, column) = match parse_location(&value) {
                Some((path, line, column)) => (path, Some(line), column),
                None => (value.as_str(), None, None),
            };
            let path = match terminal.resolve_path(path_text) {
                Ok(path) => path,
                Err(error) => {
                    terminal.set_status(error.to_string());
                    return Ok(false);
                }
            };
            (path, line, column, false, read_only)
        }
    };

    let display_path =
        path.to_string_lossy().into_owned();

    let focus_other = match open_terminal_document(
        &display_path,
        read_only,
        editor,
        other_editor,
    ) {
        Ok(focus_other) => focus_other,
        Err(error) => {
            terminal.set_status(format!("Could not open {}: {error}", terminal::display_path(&path)));
            return Ok(false);
        }
    };
    let target = if focus_other { other_editor } else { editor };
    target.clear_secondary_cursors();

    if let Some(line) = line {
        if let Err(error) = move_to_terminal_location(&mut target.document, line, column, byte_column) {
            terminal.set_status(error.to_string());
        }
    }

    if !focus_other {
        renderer.set_file_path(target.path.as_deref());
        renderer.invalidate_scroll_cache();
        renderer.update_cursor(&target.document);
        renderer.ensure_cursor_visible(&mut target.document);
    }
    terminal.close_to_editor();

    Ok(focus_other)
}

// ==========================================================================
// Main
// ==========================================================================

fn main() -> Result<(), String> {
    piece_table::recovery::enable_for_application();
    let argument = std::env::args().nth(1);
    let launch = startup::LaunchTarget::resolve(
        argument.as_deref(),
        &std::env::current_dir().map_err(|error| error.to_string())?,
    ).map_err(|error| format!("Could not open launch path: {error}"))?;
    if let Some(root) = launch.workspace_root() {
        // Set once, before configuration loads and worker threads start.
        // Terminal navigation has its own cwd and does not change this root.
        std::env::set_current_dir(root).map_err(|error| {
            format!("Could not open folder {}: {error}", root.display())
        })?;
    }

let editor_config =
    EditorConfig::load("config/editor.toml")
        .map_err(|e| e.to_string())?;

println!(
    "EDITOR CONFIG: tab_width={}, insert_spaces={}, line_numbers={:?}, keybindings={:?}",
    editor_config.tab_width,
    editor_config.insert_spaces,
    editor_config.line_numbers,
    editor_config.keybinding_mode,
);
    let key_bindings =
        KeyBindings::load(
            "config/keybindings.toml"
        )
        .map_err(|e| e.to_string())?;

    let startup_start =
        Instant::now();


let mut editor =
    Editor::new(editor_config.clone())
        .map_err(|e| e.to_string())?;

let mut other_editor =
    Editor::new(editor_config)
        .map_err(|e| e.to_string())?;

let mut vim_enabled =
    editor.config.keybinding_mode == KeybindingMode::Vim;

if let startup::LaunchTarget::File { path, location } = &launch {
    editor.open(path).map_err(|e| e.to_string())?;
    if let Some((line, column)) = location {
        editor.document.move_cursor_to_line_column(
            line.saturating_sub(1),
            column.unwrap_or(0),
        ).map_err(|e| e.to_string())?;
    }
}
        
    println!(
        "Editor startup / PieceTable open: {:?}",
        startup_start.elapsed()
    );

    let sdl =
        sdl3::init()
            .map_err(|e| e.to_string())?;

let video =
        sdl.video()
            .map_err(|e| e.to_string())?;

let clipboard =
    video.clipboard();

let (
    window,
    window_hit_test,
) =
    window::create_window(
        &video,
        "Pötyi",
        800,
        600,
    )?;

    video
        .text_input()
        .start(&window);

    let canvas =
        window.into_canvas();

let ttf_context =
    sdl3::ttf::init()
        .map_err(|e| e.to_string())?;

let logical_font_size =
    editor.config.font_size as f32;

let logical_font_stream =
    sdl3::iostream::IOStream::from_bytes(FONT_DATA)
        .map_err(|e| e.to_string())?;

let logical_font =
    ttf_context
        .load_font_from_iostream(
            logical_font_stream,
            logical_font_size,
        )
        .map_err(|e| e.to_string())?;

let raster_font_stream =
    sdl3::iostream::IOStream::from_bytes(FONT_DATA)
        .map_err(|e| e.to_string())?;

let raster_font =
    ttf_context
        .load_font_from_iostream(
            raster_font_stream,
            logical_font_size,
        )
        .map_err(|e| e.to_string())?;

let texture_creator = canvas.texture_creator();
let mut renderer =
    Renderer::new(
        canvas,
        &texture_creator,
        logical_font,
        raster_font,
        logical_font_size,
        (
            ttf_context.load_font_from_iostream(
                sdl3::iostream::IOStream::from_bytes(FONT_DATA).map_err(|e| e.to_string())?,
                command_bar::COMMAND_FONT_SIZE,
            ).map_err(|e| e.to_string())?,
            ttf_context.load_font_from_iostream(
                sdl3::iostream::IOStream::from_bytes(FONT_DATA).map_err(|e| e.to_string())?,
                command_bar::COMMAND_FONT_SIZE,
            ).map_err(|e| e.to_string())?,
        ),
        window_hit_test,
    )?;

renderer.set_tab_width(
    editor.config.tab_width,
);

renderer.set_line_number_mode(
    editor.config.line_numbers,
);

if let Some(path) = editor.path.as_deref() {
    renderer.set_file_path(Some(path));
}
    let mut search_ui =
        SearchUi::new();

    let mut command_bar =
        CommandBar::new();
    match piece_table::recovery::root().and_then(|root| piece_table::recovery::list(&root)) {
        Ok(entries) if !entries.is_empty() => {
            command_bar.open(":recover");
            command_bar.show_info(&piece_table::recovery::describe().unwrap_or_else(|e| format!("Could not list recovered work: {e}")));
        }
        Err(error) => { command_bar.open(":recover"); command_bar.show_info(&format!("Could not check crash recovery: {error}")); }
        _ => (),
    }

    let mut vim = VimController::new();
    let mut other_vim = VimController::new();

    renderer.set_mode_label(
        vim_enabled.then_some(vim.mode_label())
    );

    let terminal_directory =
        editor.path.as_deref()
            .and_then(|path| path.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| {
                        PathBuf::from(".")
                    })
            });

    let mut terminal =
        Terminal::new(terminal_directory)
            .map_err(|error| {
                error.to_string()
            })?;

    let event_subsystem =
        sdl.event()
            .map_err(|error| {
                error.to_string()
            })?;

    event_subsystem
        .register_custom_event::<TerminalEvent>()
        .map_err(|error| {
            error.to_string()
        })?;

    terminal.set_events(event_subsystem.clone());
    if let Some(root) = launch.workspace_root() {
        terminal.open(None);
        terminal.enter_directory(root).map_err(|error| error.to_string())?;
    }
    if command_bar.is_info() && matches!(command_bar.parse(), Ok(ParsedCommand::Recover { .. })) { terminal.close_to_editor(); }
    event_subsystem.register_custom_event::<lsp::Event>().map_err(|e| e.to_string())?;
    event_subsystem.register_custom_event::<lsp_setup::Event>().map_err(|e| e.to_string())?;
    let mut lsp_ui = lsp_ui::LspUi::new(event_subsystem.clone());

    let mut event_pump =
        sdl.event_pump()
            .map_err(|e| e.to_string())?;

    if !renderer.window_mut().show() {
        return Err(
            sdl3::get_error().to_string(),
        );
    }

    let mut dirty = true;
    let mut split_mode = false;
    let mut active_pane = 0usize;
    let mut pending_events =
        Vec::with_capacity(32);

    // ----------------------------------------------------------------------
    // Event loop
    // ----------------------------------------------------------------------

    let mut terminal_frames = terminal::FrameSchedule::default();
    'event_loop: loop {
        pending_events.clear();
        if !terminal.is_active() { terminal_frames.clear(); }
        if terminal.poll_background().map_err(|error| error.to_string())? && terminal.is_active() {
            terminal_frames.changed();
        }

        if !dirty
            && let Some(event) = event_pump
                .wait_event_timeout(
                    terminal_frames.wait(Instant::now(), dirty, terminal.has_pending_work())
                )
        {
            pending_events.push(event);
        }

        pending_events.extend(
            event_pump.poll_iter().take(64)
        );

        for mut event in pending_events.drain(..) {
            if let Some(terminal_event) = event
                .as_user_event_type::<TerminalEvent>()
            {
                if terminal.handle_event(terminal_event).map_err(|error| error.to_string())? && terminal.is_active() {
                    terminal_frames.changed();
                }
                continue;
            }

            if let Some(event) = event.as_user_event_type::<lsp_setup::Event>() {
                lsp_ui.accept_setup(event, &editor, &other_editor, &mut command_bar);
                dirty = true;
                continue;
            }
            if let Some(event) = event.as_user_event_type::<lsp::Event>() {
                lsp_ui.validate_completion(&editor, &other_editor,
                    !terminal.is_active() && !command_bar.is_active() && (!vim_enabled || vim.mode() == vim::VimMode::Insert));
                let outcome = lsp_ui.accept(event, &mut editor, &mut other_editor, &mut command_bar);
                if outcome.document_changed && vim_enabled { vim.finish_formatting(&mut editor); }
                if outcome.focus_other {
                    focus_pane(1 - active_pane, &mut active_pane, &mut editor, &mut other_editor,
                        &mut vim, &mut other_vim, &mut renderer);
                }
                if outcome.cursor_changed {
                    search_ui.close();
                    if vim_enabled && outcome.document_reloaded { vim.reset(); }
                    renderer.set_file_path(editor.path.as_deref());
                    renderer.set_mode_label(vim_enabled.then_some(vim.mode_label()));
                    renderer.invalidate_scroll_cache();
                    renderer.update_cursor(&editor.document);
                    renderer.ensure_cursor_visible(&mut editor.document);
                }
                dirty = true;
                continue;
            }

            let coordinates_converted =
                renderer
                    .convert_event_coordinates(
                        &mut event,
                    );

            match event {
                Event::DropFile {
                    filename,
                    ..
                } => {
                    /*
                     * Opening a document also terminates the active search.
                     * The query itself is preserved by SearchUi.
                     */
                    search_ui.close();
                    command_bar.close();

                    if file_is_open_in(
                        &filename,
                        &other_editor,
                    ) {
                        focus_pane(
                            1 - active_pane,
                            &mut active_pane,
                            &mut editor,
                            &mut other_editor,
                            &mut vim,
                            &mut other_vim,
                            &mut renderer,
                        );
                        dirty = true;
                        continue;
                    }

if let Err(error) = editor.open(&filename) {
    command_bar.open(":");
    command_bar.show_info(&format!("Could not open {filename}: {error}"));
    dirty = true;
    continue;
}

if vim_enabled {
    vim.reset();
    renderer.set_mode_label(Some(vim.mode_label()));
}

if let Some(path) = editor.path.as_deref() {
    renderer.set_file_path(Some(path));
}

renderer
    .invalidate_scroll_cache();

renderer
    .update_cursor(
        &editor.document
    );

renderer
    .ensure_cursor_visible(
        &mut editor.document
    );

dirty = true;
                }


Event::MouseButtonDown {
    mouse_btn: MouseButton::Left,
    clicks: 1,
    x,
    y,
    ..
} => {
    if !coordinates_converted {
        continue;
    }

    match renderer.window_control_at(
        x as i32,
        y as i32,
    ) {
        WindowControl::Minimize => {
            renderer.window_mut().minimize();
        }

        WindowControl::Maximize => {
            let window =
                renderer.window_mut();

            if window.is_maximized() {
                window.restore();
            } else {
                window.maximize();
            }

            dirty = true;
        }

        WindowControl::Close => {
            break 'event_loop;
        }

        WindowControl::None => {
            if terminal.is_active() {
                match renderer.terminal_hit_at(
                    x as i32,
                    y as i32,
                ) {
                    TerminalHit::Input => {
                        let cursor = renderer
                            .terminal_cursor_at(
                                &terminal,
                                x as i32,
                            );
                        terminal.focus_prompt();
                        terminal.set_cursor(cursor);
                    }

                    TerminalHit::StopOrRunAgain => {
                        if terminal.is_running() {
                            if let Err(error) = terminal.stop() {
                                terminal.set_status(
                                    error.to_string()
                                );
                            }
                        } else {
                            match terminal.run_again(
                                &event_subsystem
                            ) {
                                Ok(action) => {
                                    if handle_terminal_action(
                                        action,
                                        &mut terminal,
                                        &mut editor,
                                        &mut other_editor,
                                        &mut renderer,
                                    )? {
                                        focus_pane(
                                            1 - active_pane,
                                            &mut active_pane,
                                            &mut editor,
                                            &mut other_editor,
                                            &mut vim,
                                            &mut other_vim,
                                            &mut renderer,
                                        );
                                    }
                                }
                                Err(error) => {
                                    terminal.set_status(
                                        error.to_string()
                                    );
                                }
                            }
                        }
                    }

                    TerminalHit::Clear => {
                        let result = if terminal.can_go_back() { terminal.go_back() } else { terminal.clear() };
                        if let Err(error) = result {
                            terminal.set_status(
                                error.to_string()
                            );
                        }
                    }

                    TerminalHit::Editor => {
                        terminal.close_to_editor();
                    }

                    TerminalHit::Output => {
                        let action = renderer.terminal_action_at(&mut terminal, x as i32, y as i32)?;
                        let offset = renderer.terminal_output_offset_at(&mut terminal, x as i32, y as i32)?;
                        terminal.begin_output_drag(offset, x as i32, y as i32, action)
                            .map_err(|error| error.to_string())?;
                    }

                    TerminalHit::Outside => {}
                }

                dirty = true;
            } else if let Some(index) = renderer.completion_hit_at(x as i32, y as i32) {
                lsp_ui.choose_completion(index, &editor, &other_editor, &mut command_bar);
                dirty = true;
            } else if command_bar.is_active()
                && renderer.command_bar_hit_at(&command_bar, x as i32, y as i32) != CommandBarHit::Outside
            {
                let hit = renderer.command_bar_hit_at(&command_bar, x as i32, y as i32);
                let hit = if let CommandBarHit::Suggestion(index) = hit
                    && !command_bar.is_info()
                {
                    command_bar.select_suggestion(index);
                    CommandBarHit::Execute
                } else {
                    hit
                };
                match hit {
                    CommandBarHit::Suggestion(_) => {},

                    CommandBarHit::Input => {
                        let cursor =
                            renderer
                                .command_bar_cursor_at(
                                    &command_bar,
                                    x as i32,
                                );

                        command_bar.set_cursor(
                            cursor
                        );
                        dirty = true;
                    }

                    CommandBarHit::Execute => {
                        let close_vim_search =
                            vim_enabled
                            && !command_bar.selects_option_on_enter()
                            && matches!(
                                command_bar.parse(),
                                Ok(ParsedCommand::Find { .. })
                            );

                        let outcome = if close_vim_search {
                            let outcome = CommandOutcome {
                                cursor_changed: search_ui
                                    .current_match()
                                    .is_some(),
                                ..CommandOutcome::default()
                            };

                            vim.accept_search();
                            command_bar.close();
                            outcome
                        } else {
                            execute_command_bar(
                                &mut command_bar,
                                &mut search_ui,
                                &mut editor,
                                &mut other_editor,
                                &mut renderer,
                                &mut terminal,
                                &mut vim,
                                false,
                                &mut other_vim,
                                &mut lsp_ui,
                            )
                        };

                        if outcome.quit {
                            break 'event_loop;
                        }

                        if outcome.toggle_split {
                            split_mode = !split_mode;
                            renderer.set_split_mode(
                                split_mode
                            );
                        }

                        if outcome.focus_other {
                            focus_pane(
                                1 - active_pane,
                                &mut active_pane,
                                &mut editor,
                                &mut other_editor,
                                &mut vim,
                                &mut other_vim,
                                &mut renderer,
                            );
                        }

                        if outcome.path_changed {
                            renderer.set_file_path(
                                editor.path.as_deref()
                            );
                        }

                        if outcome.document_changed
                            || outcome.document_reloaded
                        {
                            renderer
                                .invalidate_scroll_cache();
                        }

                        if let Some(mode) = outcome.keybinding_mode {
                            apply_keybinding_mode(
                                mode,
                                &mut vim_enabled,
                                &mut editor,
                                &mut other_editor,
                                &mut vim,
                                &mut other_vim,
                                &mut renderer,
                            );
                        }

                        if vim_enabled
                            && outcome.document_reloaded
                        {
                            vim.reset();
                            renderer.set_mode_label(
                                Some(vim.mode_label())
                            );
                        }

                        if outcome.cursor_changed {
                            renderer.update_cursor(
                                &editor.document
                            );

                            if search_ui.current_match()
                                .is_some()
                            {
                                renderer.ensure_search_match_visible(
                                    &mut editor.document,
                                    &search_ui,
                                    &command_bar,
                                );
                            } else {
                                renderer.ensure_cursor_visible(
                                    &mut editor.document
                                );
                            }
                        }

                        dirty = true;
                    }

                    CommandBarHit::Outside => {}
                }
            } else {
                lsp_ui.dismiss_completion();
                if let Some(clicked_pane) =
                    renderer.pane_at_point(
                        x as i32,
                        y as i32,
                    )
                    && clicked_pane != active_pane
                {
                    focus_pane(
                        clicked_pane,
                        &mut active_pane,
                        &mut editor,
                        &mut other_editor,
                        &mut vim,
                        &mut other_vim,
                        &mut renderer,
                    );
                    search_ui.close();
                }

                if vim_enabled {
                    vim.handle_document_click();
                    renderer.set_mode_label(
                        Some(vim.mode_label())
                    );
                }

                let target =
                    renderer
                        .cursor_target_at(
                            &mut editor.document,
                            x as i32,
                            y as i32,
                        )?;

                if let Some((line, column)) = target {
                    editor.clear_secondary_cursors();
                    editor
                        .document
                        .move_cursor_to_line_column(
                            line,
                            column,
                        )
                        .map_err(|error| {
                            error.to_string()
                        })?;

                    if vim_enabled {
                        vim.settle_cursor(&mut editor)?;
                    }

                    renderer.ensure_cursor_visible(
                        &mut editor.document,
                    );
                    lsp_ui.document_clicked(&editor, &other_editor, &mut command_bar);

                    dirty = true;
                }
            }
        }
    }
}

Event::MouseButtonUp {
    mouse_btn: MouseButton::Left, x, y, ..
} => {
    if terminal.is_active() && terminal.output_dragging() && coordinates_converted {
        let offset = renderer.terminal_output_offset_at(&mut terminal, x as i32, y as i32)?;
        terminal.drag_output_to(offset, x as i32, y as i32).map_err(|error| error.to_string())?;
        if let Some(action) = terminal.finish_output_drag() {
            terminal.focus_prompt();
            if handle_terminal_action(action, &mut terminal, &mut editor, &mut other_editor, &mut renderer)? {
                focus_pane(1 - active_pane, &mut active_pane, &mut editor, &mut other_editor,
                    &mut vim, &mut other_vim, &mut renderer);
            }
        }
        dirty = true;
    }
}

Event::MouseMotion {
    x,
    y,
    ..
} => {
    if terminal.is_active() && terminal.output_dragging() && coordinates_converted {
        let offset = renderer.terminal_output_offset_at(&mut terminal, x as i32, y as i32)?;
        terminal.drag_output_to(offset, x as i32, y as i32).map_err(|error| error.to_string())?;
        dirty = true;
        continue;
    }
    if coordinates_converted
        && command_bar.is_active()
        && let CommandBarHit::Suggestion(index) =
            renderer.command_bar_hit_at(
                &command_bar,
                x as i32,
                y as i32,
            )
        && command_bar.select_suggestion(
                index
            )
    {
        dirty = true;
    }
}

                Event::KeyDown {
                    keycode: Some(key),
                    keymod,
                    repeat,
                    ..
                } => {
                    if ctrl_pressed(keymod)
                        && key == Keycode::Grave
                        && !repeat
                    {
                        editor.clear_secondary_cursors();
                        command_bar.close();
                        search_ui.close();
                        terminal.toggle(
                            editor.path.as_deref()
                        );
                        dirty = true;
                        continue;
                    }

                    if terminal.is_active() {
                        if terminal.can_go_back() && alt_pressed(keymod) && key == Keycode::Left && !repeat {
                            if let Err(error) = terminal.go_back() { terminal.set_status(error.to_string()); }
                            dirty = true; continue;
                        }

                        let clipboard_command = key_bindings.terminal_clipboard_command(key, keymod, repeat);
                        if clipboard_command == Some(Command::Paste) {
                            match read_text(&clipboard) {
                                Ok(text) => {
                                    terminal.focus_prompt();
                                    terminal.insert_text(&text);
                                }
                                Err(error) => terminal
                                    .set_status(format!(
                                        "Clipboard paste failed: {error}"
                                    )),
                            }

                            dirty = true;
                            continue;
                        }

                        let selected_copy = ctrl_pressed(keymod) && key == Keycode::C
                            && !repeat && !terminal.output_selection().is_empty();
                        if clipboard_command == Some(Command::Copy) || selected_copy {
                            let all = shift_pressed(keymod) || terminal.output_selection().is_empty();
                            if let Err(error) = terminal.copy_output(&clipboard, all, false) {
                                terminal.set_status(format!("Clipboard copy failed: {error}"));
                            } else {
                                vim.set_clipboard_linewise(!all && terminal.output_linewise());
                                other_vim.set_clipboard_linewise(!all && terminal.output_linewise());
                            }
                            dirty = true;
                            continue;
                        }

                        if ctrl_pressed(keymod)
                            && key == Keycode::C
                            && !repeat
                            && terminal.is_running()
                        {
                            if let Err(error) = terminal.stop() {
                                terminal.set_status(
                                    error.to_string()
                                );
                            }

                            dirty = true;
                            continue;
                        }

                        if ctrl_pressed(keymod)
                            && key == Keycode::L
                            && !repeat
                        {
                            if let Err(error) = terminal.clear() {
                                terminal.set_status(
                                    error.to_string()
                                );
                            }

                            dirty = true;
                            continue;
                        }

                        let select_output = !terminal.output_focused()
                            && key == Keycode::Up && shift_pressed(keymod)
                            && !keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD
                                | Mod::LGUIMOD | Mod::RGUIMOD | Mod::LALTMOD | Mod::RALTMOD);
                        if (key == Keycode::F6 && !repeat) || select_output {
                            if terminal.output_focused() {
                                terminal.focus_prompt();
                            } else {
                                let end = terminal.output_mut().len();
                                terminal.move_output_cursor(end, false).map_err(|error| error.to_string())?;
                                renderer.navigate_terminal_output(&mut terminal, if select_output {
                                    OutputCommand::Rows(-1, true)
                                } else { OutputCommand::None })?;
                            }
                            dirty = true;
                            continue;
                        }

                        if terminal.output_focused() {
                            let command = terminal.output_key(key, keymod, vim_enabled)
                                .map_err(|error| error.to_string())?;
                            if matches!(command, OutputCommand::Copy) {
                                let linewise = terminal.output_linewise();
                                if !terminal.output_selection().is_empty() {
                                    if let Err(error) = terminal.copy_output(&clipboard, false, true) {
                                        terminal.set_status(format!("Clipboard copy failed: {error}"));
                                    } else {
                                        vim.set_clipboard_linewise(linewise);
                                        other_vim.set_clipboard_linewise(linewise);
                                    }
                                }
                            } else {
                                renderer.navigate_terminal_output(&mut terminal, command)?;
                            }
                            dirty = true;
                            continue;
                        }

                        match key {
                            Keycode::Escape => {
                                terminal.close_to_editor();
                            }
                            Keycode::Up => {
                                terminal.history_previous();
                            }
                            Keycode::Down => {
                                terminal.history_next();
                            }
                            Keycode::Backspace => {
                                terminal.backspace();
                            }
                            Keycode::Delete => {
                                terminal.delete();
                            }
                            Keycode::Left => {
                                terminal.move_left();
                            }
                            Keycode::Right => {
                                terminal.move_right();
                            }
                            Keycode::Home => {
                                terminal.move_home();
                            }
                            Keycode::End => {
                                terminal.move_end();
                            }
                            Keycode::Tab if !repeat => {
                                terminal.complete_path(shift_pressed(keymod));
                            }
                            Keycode::Return
                            | Keycode::KpEnter
                                if !repeat =>
                            {
                                match terminal.submit(
                                    &event_subsystem
                                ) {
                                    Ok(action) => {
                                        if handle_terminal_action(
                                            action,
                                            &mut terminal,
                                            &mut editor,
                                            &mut other_editor,
                                            &mut renderer,
                                        )? {
                                            focus_pane(
                                                1 - active_pane,
                                                &mut active_pane,
                                                &mut editor,
                                                &mut other_editor,
                                                &mut vim,
                                                &mut other_vim,
                                                &mut renderer,
                                            );
                                        }
                                    }
                                    Err(error) => {
                                        terminal.set_status(
                                            error.to_string()
                                        );
                                    }
                                }
                            }
                            _ => {}
                        }

                        dirty = true;
                        continue;
                    }

                    if lsp_ui.completion_key(key, repeat, &editor, &other_editor, &mut command_bar) {
                        dirty = true;
                        continue;
                    }

                    /*
                     * Familiar shortcuts open the shared command bar with
                     * their command already selected. Ctrl+P opens the full
                     * command list.
                     */
                    if ctrl_pressed(keymod)
                        && !(vim_enabled
                            && vim.mode() != vim::VimMode::Insert
                            && !command_bar.is_active()
                            && !search_ui.is_active()
                            && VimController::page_motion(key, keymod).is_some())
                        && (key == Keycode::F
                            || key == Keycode::H
                            || key == Keycode::P)
                    {
                        let initial =
                            match key {
                                Keycode::F =>
                                    find_command_text(
                                        &search_ui
                                    ),

                                Keycode::H =>
                                    replace_command_text(
                                        &search_ui
                                    ),

                                _ => ":".to_string(),
                            };

                        editor.clear_secondary_cursors();
                        command_bar.open(&initial);

                        sync_command_search(
                            &command_bar,
                            &mut search_ui,
                            &mut editor.document,
                            vim.search_origin(),
                        )
                        .map_err(|error| {
                            error.to_string()
                        })?;

                        if search_ui.current_match()
                            .is_some()
                        {
                            renderer.update_cursor(
                                &editor.document,
                            );

                            renderer
                                .ensure_search_match_visible(
                                    &mut editor.document,
                                    &search_ui,
                                    &command_bar,
                                );
                        }

                        dirty = true;
                        continue;
                    }

                    let bound_command =
                        key_bindings.command_for(
                            key,
                            keymod,
                            repeat,
                        );

                    /*
                     * Command mode is modal. Its editing, suggestions, and
                     * execution never fall through to the document.
                     */
                    if command_bar.is_active() {
                        if bound_command
                            == Some(Command::Paste)
                        {
                            match read_text(&clipboard) {
                                Ok(text)
                                    if !text.is_empty() =>
                                {
                                    command_bar
                                        .insert_text(&text);

                                    sync_command_search(
                                        &command_bar,
                                        &mut search_ui,
                                        &mut editor.document,
                                        vim.search_origin(),
                                    )
                                    .map_err(|error| {
                                        error.to_string()
                                    })?;

                                    dirty = true;
                                }

                                Ok(_) => {}

                                Err(error) => {
                                    eprintln!(
                                        "Clipboard paste failed: {error}"
                                    );
                                }
                            }

                            continue;
                        }

                        let mut input_changed = false;
                        let mut outcome =
                            CommandOutcome::default();

                        match key {
                            Keycode::Escape => {
                                if vim_enabled {
                                    vim.cancel_search(&mut editor)?;
                                }
                                command_bar.close();
                                search_ui.close();
                            }

                            Keycode::Up => {
                                command_bar
                                    .move_selection(-1);
                            }

                            Keycode::Down => {
                                command_bar
                                    .move_selection(1);
                            }

                            Keycode::Tab => {
                                input_changed =
                                    command_bar
                                        .apply_selected();
                            }

                            Keycode::Backspace => {
                                command_bar.backspace();
                                input_changed = true;
                            }

                            Keycode::Delete => {
                                command_bar.delete();
                                input_changed = true;
                            }

                            Keycode::Left => {
                                command_bar.move_left();
                            }

                            Keycode::Right => {
                                command_bar.move_right();
                            }

                            Keycode::Home => {
                                command_bar.move_home();
                            }

                            Keycode::End => {
                                command_bar.move_end();
                            }

                            Keycode::Return
                            | Keycode::KpEnter
                                if !repeat =>
                            {
                                let close_vim_search =
                                    vim_enabled
                                    && !command_bar.selects_option_on_enter()
                                    && matches!(
                                        command_bar.parse(),
                                        Ok(ParsedCommand::Find { .. })
                                    );

                                if close_vim_search {
                                    outcome.cursor_changed = search_ui
                                        .current_match()
                                        .is_some();
                                    vim.accept_search();
                                    command_bar.close();
                                } else {
                                    outcome = execute_command_bar(
                                        &mut command_bar,
                                        &mut search_ui,
                                        &mut editor,
                                        &mut other_editor,
                                        &mut renderer,
                                        &mut terminal,
                                        &mut vim,
                                        shift_pressed(
                                            keymod
                                        ),
                                        &mut other_vim,
                                        &mut lsp_ui,
                                    );
                                }
                            }

                            _ => {}
                        }

                        if input_changed {
                            sync_command_search(
                                &command_bar,
                                &mut search_ui,
                                &mut editor.document,
                                vim.search_origin(),
                            )
                            .map_err(|error| {
                                error.to_string()
                            })?;
                        }

                        if outcome.quit {
                            break 'event_loop;
                        }

                        if outcome.toggle_split {
                            split_mode = !split_mode;
                            renderer.set_split_mode(
                                split_mode
                            );
                        }

                        if outcome.focus_other {
                            focus_pane(
                                1 - active_pane,
                                &mut active_pane,
                                &mut editor,
                                &mut other_editor,
                                &mut vim,
                                &mut other_vim,
                                &mut renderer,
                            );
                        }

                        if outcome.path_changed {
                            renderer.set_file_path(
                                editor.path.as_deref()
                            );
                        }

                        if outcome.document_changed
                            || outcome.document_reloaded
                        {
                            renderer
                                .invalidate_scroll_cache();
                        }

                        if let Some(mode) = outcome.keybinding_mode {
                            apply_keybinding_mode(
                                mode,
                                &mut vim_enabled,
                                &mut editor,
                                &mut other_editor,
                                &mut vim,
                                &mut other_vim,
                                &mut renderer,
                            );
                        }

                        if vim_enabled
                            && outcome.document_reloaded
                        {
                            vim.reset();
                            renderer.set_mode_label(
                                Some(vim.mode_label())
                            );
                        }

                        if outcome.cursor_changed {
                            renderer.update_cursor(
                                &editor.document
                            );
                        }

                        if search_ui.current_match()
                            .is_some()
                        {
                            renderer
                                .ensure_search_match_visible(
                                    &mut editor.document,
                                    &search_ui,
                                    &command_bar,
                                );
                        } else if outcome.cursor_changed {
                            renderer.ensure_cursor_visible(
                                &mut editor.document
                            );
                        }

                        dirty = true;
                        continue;
                    }

                    /*
                     * Normal editor key handling.
                     */
                    let workspace_redo = if vim_enabled {
                        vim.workspace_history_key(key, keymod).or_else(|| match bound_command {
                            Some(Command::Undo) => Some(false), Some(Command::Redo) => Some(true), _ => None
                        })
                    } else {
                        match bound_command { Some(Command::Undo) => Some(false), Some(Command::Redo) => Some(true), _ => None }
                    };
                    if let Some(redo) = workspace_redo {
                        match workspace_edit::history(&mut editor, &mut other_editor, redo) {
                            Ok(false) => {},
                            result => {
                                if result.is_ok() {
                                    lsp_ui.files_changed(workspace_edit::recent_disk_changes(&editor, !redo));
                                    renderer.set_file_path(editor.path.as_deref());
                                }
                                if vim_enabled {
                                    vim.deactivate(&mut editor); other_vim.deactivate(&mut other_editor);
                                    vim.consume_workspace_history_key();
                                    renderer.set_mode_label(Some(vim.mode_label()));
                                }
                                if let Err(error) = result { command_bar.open(":"); command_bar.show_info(&error); }
                                search_ui.close();
                                renderer.invalidate_scroll_cache();
                                renderer.update_cursor(&editor.document);
                                dirty = true;
                                continue;
                            }
                        }
                    }
                    if !vim_enabled && key == Keycode::Escape {
                        editor.clear_secondary_cursors();
                        dirty = true;
                        continue;
                    }
                    if vim_enabled && bound_command == Some(Command::SelectNextOccurrence) {
                        // Ctrl+D still reaches Vim's half-page motion; Cmd+D has no Vim action.
                        if VimController::page_motion(key, keymod).is_none() { continue; }
                    }
                    if vim_enabled {
                        let outcome = vim.handle_key(
                            &mut editor,
                            &clipboard,
                            key,
                            keymod,
                            repeat,
                            renderer.visible_line_count(),
                        )?;

                        renderer.set_mode_label(
                            Some(vim.mode_label())
                        );

                        match outcome.ui_action {
                            VimUiAction::None => {}

                            VimUiAction::OpenCommandBar => {
                                command_bar.open(":");
                                search_ui.close();
                            }

                            VimUiAction::OpenSearch { backward } => {
                                if backward {
                                    command_bar.open(
                                        ":find  --backward"
                                    );
                                    command_bar.set_cursor(6);
                                } else {
                                    command_bar.open(":find ");
                                }
                                search_ui.close();
                            }

                            VimUiAction::RepeatSearch { backward } => {
                                let result = if backward {
                                    search_ui.previous(
                                        &mut editor.document
                                    )
                                } else {
                                    search_ui.next(
                                        &mut editor.document
                                    )
                                };
                                result.map_err(|error| error.to_string())?;
                            }

                            VimUiAction::SearchWord { query, backward } => {
                                if !query.is_empty() {
                                    search_ui.configure_command(
                                        &mut editor.document,
                                        &query,
                                        None,
                                        SearchMode::CaseSensitive,
                                    ).map_err(|error| error.to_string())?;

                                    let result = if backward {
                                        search_ui.previous(
                                            &mut editor.document
                                        )
                                    } else {
                                        search_ui.next(
                                            &mut editor.document
                                        )
                                    };
                                    result.map_err(|error| error.to_string())?;
                                }
                            }
                        }

                        if outcome.document_changed {
                            renderer.invalidate_scroll_cache();
                        }

                        if outcome.cursor_changed {
                            renderer.ensure_cursor_visible(
                                &mut editor.document
                            );
                        }

                        if outcome.consumed {
                            dirty = true;
                            continue;
                        }
                    }

                    if let Some(command) =
                        bound_command
                    {
                        let start =
                            Instant::now();

                        let mut document_changed =
                            false;

                        let redraw =
                            match command {
                                Command::FormatDocument => {
                                    editor.clear_secondary_cursors();
                                    command_bar.open(":format");
                                    document_changed = run_format_command(
                                        &mut editor, &mut search_ui, &mut command_bar, &mut vim, None,
                                    );
                                    if document_changed && vim_enabled {
                                        renderer.set_mode_label(Some(vim.mode_label()));
                                    }
                                    true
                                }

                                Command::NewFile => {
                                    editor.clear_secondary_cursors();
                                    command_bar.open(":new ");
                                    search_ui.close();
                                    true
                                }

                                Command::SaveAs => {
                                    editor.clear_secondary_cursors();
                                    command_bar.open(":save-as ");
                                    search_ui.close();
                                    true
                                }

                                Command::Save => {
                                    match editor.save() {
                                        Ok(()) => renderer.set_file_path(editor.path.as_deref()),
                                        Err(error) => {
                                            command_bar.open(":save");
                                            command_bar.set_status(error.to_string());
                                        }
                                    }
                                    true
                                }

                                Command::Copy => {
                                    if let Err(error) =
                                        copy_selection(
                                            &clipboard,
                                            &editor.document,
                                        )
                                    {
                                        eprintln!(
                                            "Clipboard copy failed: {error}"
                                        );
                                    }

                                    false
                                }

                                Command::Paste => {
                                    let vim_insert_text =
                                        (vim_enabled
                                            && vim.mode()
                                                == vim::VimMode::Insert)
                                            .then(|| {
                                                read_text(&clipboard)
                                                    .unwrap_or_default()
                                            });

                                    match paste(
                                        &clipboard,
                                        &mut editor,
                                    ) {
                                        Ok(true) => {
                                            document_changed = true;

                                            if let Some(text) =
                                                vim_insert_text
                                            {
                                                vim.record_text(&text);
                                            }

                                            true
                                        }

                                        Ok(false) => false,

                                        Err(error) => {
                                            eprintln!(
                                                "Clipboard paste failed: {error}"
                                            );

                                            false
                                        }
                                    }
                                }

                                Command::Cut => {
                                    match cut_selection(
                                        &clipboard,
                                        &mut editor,
                                    ) {
                                        Ok(true) => {
                                            document_changed = true;
                                            true
                                        }
                                        Ok(false) => false,
                                        Err(error) => {
                                            eprintln!(
                                                "Clipboard cut failed: {error}"
                                            );
                                            false
                                        }
                                    }
                                }

                                _ => {
                                    editor
                                        .execute(command, renderer.visible_line_count())
                                        .map_err(
                                            |e| e.to_string()
                                        )?;

                                    true
                                }
                            };

                        let command_time =
                            start.elapsed();

                        if command_time
                            > Duration::from_millis(20)
                        {
                            println!(
                                "Command {:?}: {:?}",
                                command,
                                command_time
                            );
                        }

                        if matches!(
                            command,
                            Command::Delete
                                | Command::Backspace
                                | Command::Newline
                        ) || document_changed
                        {
                            renderer
                                .invalidate_scroll_cache();
                        }

                        if redraw {
                            renderer
                                .ensure_cursor_visible(
                                    &mut editor.document
                                );

                            dirty = true;
                        }

                        if matches!(
                            command,
                            Command::Quit
                        ) {
                            break 'event_loop;
                        }
                    }
                }

                Event::TextInput {
                    text,
                    ..
                } => {
                    if terminal.is_active() {
                        if !terminal.output_focused() {
                            terminal.insert_text(&text);
                        }
                        dirty = true;
                        continue;
                    }

                    if vim_enabled
                        && vim.consume_suppressed_text_input()
                    {
                        continue;
                    }

                    if command_bar.is_active() {
                        if !text.is_empty() {
                            if command_bar.input() == ":"
                                && text == ":"
                            {
                                command_bar.close();
                                search_ui.close();

                                editor.insert(":")
                                    .map_err(|error| {
                                        error.to_string()
                                    })?;

                                renderer
                                    .invalidate_scroll_cache();

                                renderer.ensure_cursor_visible(
                                    &mut editor.document
                                );

                                dirty = true;
                                continue;
                            }

                            command_bar
                                .insert_text(&text);

                            sync_command_search(
                                &command_bar,
                                &mut search_ui,
                                &mut editor.document,
                                vim.search_origin(),
                            )
                            .map_err(|error| {
                                error.to_string()
                            })?;

                            if search_ui
                                .current_match()
                                .is_some()
                            {
                                renderer.update_cursor(
                                    &editor.document,
                                );

                                renderer
                                    .ensure_search_match_visible(
                                        &mut editor.document,
                                        &search_ui,
                                        &command_bar,
                                    );
                            }

                            dirty = true;
                        }

                        continue;
                    }

                    if vim_enabled
                        && vim.mode() != vim::VimMode::Insert
                    {
                        continue;
                    }

                    if !vim_enabled && text == ":" && editor.document.secondary_cursors.is_empty() {
                        let line = editor
                            .document
                            .cursor
                            .line;

                        let empty_line = editor
                            .document
                            .line_length(line)
                            .map_err(|error| {
                                error.to_string()
                            })? == 0;

                        if empty_line {
                            command_bar.open(":");
                            search_ui.close();
                            dirty = true;
                            continue;
                        }
                    }

                    if !text.is_empty() {
                        editor
                            .insert(&text)
                            .map_err(
                                |e| e.to_string()
                            )?;

                        if vim_enabled {
                            vim.record_text(&text);
                        }
                        lsp_ui.typed_member_trigger(&text, &editor, &other_editor, &mut command_bar);

                        renderer
                            .invalidate_scroll_cache();

                        renderer
                            .ensure_cursor_visible(
                                &mut editor.document
                            );

                        dirty = true;
                    }
                }

                Event::MouseWheel {
                    x,
                    y,
                    mouse_x,
                    mouse_y,
                    ..
                } => {
                    if terminal.is_active() {
                        terminal.scroll(
                            y as isize * 3
                        );
                        dirty = true;
                        continue;
                    }

                    if command_bar.is_active()
                        && renderer.command_bar_hit_at(&command_bar, mouse_x as i32, mouse_y as i32) != CommandBarHit::Outside
                        && !matches!(command_bar.parse(), Ok(ParsedCommand::Find { .. } | ParsedCommand::Replace { .. }))
                    {
                        if y != 0.0 {
                            command_bar.scroll_suggestions(if y > 0.0 { -3 } else { 3 });
                            dirty = true;
                        }
                        continue;
                    }

                    /*
                     * Mouse wheel still controls the document while search
                     * is open. This lets the user inspect nearby matches
                     * without closing the search bar.
                     */
                    renderer.scroll_by(
                        -(y as isize),
                        &mut editor.document,
                    );

                    renderer.scroll_horizontal(
                        -(x as i32) * 40,
                    );

                    dirty = true;
                }

                Event::Quit { .. } => {
                    break 'event_loop;
                }

                Event::Window {
                    win_event:
                        WindowEvent::Resized(_, _)
                        | WindowEvent::PixelSizeChanged(_, _)
                        | WindowEvent::DisplayChanged(_)
                        | WindowEvent::Exposed
                        | WindowEvent::Restored
                        | WindowEvent::Maximized,
                    ..
                } => {
                    renderer.update_window_size()?;
                    dirty = true;
                }

                Event::Display {
                    display_event:
                        DisplayEvent::ContentScaleChanged,
                    ..
                } => {
                    renderer.update_window_size()?;
                    dirty = true;
                }

                _ => {}
            }
        }

        for document in [&mut editor.document, &mut other_editor.document] {
            if let Some(warning) = document.take_recovery_warning() {
                command_bar.open(":recover"); command_bar.show_info(&warning); dirty = true;
            }
        }

        lsp_ui.validate_completion(&editor, &other_editor,
            !terminal.is_active() && !command_bar.is_active() && (!vim_enabled || vim.mode() == vim::VimMode::Insert));
        lsp_ui.discard_dismissed_preview(&command_bar);
        lsp_ui.reconcile(&editor, &other_editor);

        dirty |= terminal.is_active() && terminal_frames.due(Instant::now());
        if dirty {
            let start =
                Instant::now();

            renderer.update_window_size()?;
            renderer.set_completion(lsp_ui.completion_display());

            if terminal.is_active() {
                renderer.render_terminal(
                    &mut terminal
                )?;
                terminal_frames.rendered(Instant::now());
            } else if split_mode {
                renderer.render_split(
                    &mut editor.document,
                    &mut other_editor.document,
                    &search_ui,
                    &command_bar,
                )?;
            } else {
                renderer.render(
                    &mut editor.document,
                    &search_ui,
                    &command_bar,
                )?;
            }

            let render_time =
                start.elapsed();

            if render_time
                > Duration::from_millis(20)
            {
                println!(
                    "Render: {:?}",
                    render_time
                );
            }

            dirty = false;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
