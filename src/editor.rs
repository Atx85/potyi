// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Document state and editing operations. Window and input routing live in app.
use crate::{
    config::{EditorConfig, KeybindingMode},
    emacs, formatting,
    keybindings::Command,
    multi_cursor,
    piece_table::{self, Piece, PieceTable, PieceTableSnapshot},
    search::{SearchResult, Searcher},
    workspace_edit,
};
use std::{io, path::PathBuf, time::Instant};

#[derive(Clone)]
pub(crate) struct CursorState {
    pub(crate) secondary_cursors: Vec<piece_table::Cursor>,
    pub(crate) position: usize,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) desired_column: Option<usize>,
    pub(crate) anchor: usize,
    pub(crate) anchor_line: usize,
    pub(crate) anchor_column: usize,
}

#[derive(Clone)]
pub(crate) struct TextChange {
    pub(crate) position: usize,
    pub(crate) deleted: Vec<Piece>,
    pub(crate) deleted_length: usize,
    pub(crate) inserted: Option<Piece>,
}

#[derive(Clone)]
pub(crate) enum HistoryKind {
    Changes(Vec<TextChange>),
    Snapshot(PieceTableSnapshot),
    Workspace(workspace_edit::Marker),
    Sequence(Vec<HistoryKind>),
}

#[derive(Clone)]
pub(crate) struct HistoryEntry {
    pub(crate) kind: HistoryKind,
    pub(crate) before: CursorState,
    pub(crate) after: CursorState,
}

#[derive(Clone, Copy)]
pub(crate) struct ReplacementRun {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) repetitions: usize,
}

// ==========================================================================
// Editor
// ==========================================================================

pub(crate) struct Editor {
    pub(crate) document: PieceTable,
    pub(crate) emacs: emacs::State,
    pub(crate) path: Option<PathBuf>,
    pub(crate) config: EditorConfig,

    pub(crate) undo_stack: Vec<HistoryEntry>,
    pub(crate) redo_stack: Vec<HistoryEntry>,

    pub(crate) multi_edit_group: Option<usize>,
    pub(crate) applying_history: bool,
    pub(crate) dirty: bool,
    pub(crate) read_only: bool,
}

impl Editor {
    pub(crate) fn new(config: EditorConfig) -> io::Result<Self> {
        let mut document = PieceTable::empty()?;
        piece_table::recovery::arm(&mut document, None);
        Ok(Self {
            document,
            emacs: emacs::State::default(),
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

    /// Make another view of the live buffer, including unsaved edits and history.
    pub(crate) fn duplicate_view(&self) -> Self {
        Self {
            document: self.document.duplicate_view(),
            emacs: emacs::State::default(),
            path: self.path.clone(),
            config: self.config.clone(),
            undo_stack: self.undo_stack.clone(),
            redo_stack: self.redo_stack.clone(),
            multi_edit_group: None,
            applying_history: false,
            dirty: self.dirty,
            read_only: self.read_only,
        }
    }

    pub(crate) fn shares_document_with(&self, other: &Self) -> bool {
        self.document.shares_storage_with(&other.document)
    }

    /// Called at input/async-event boundaries: views have independent cursors,
    /// but the newest buffer revision, history and saved state must agree.
    pub(crate) fn synchronize_views(active: &mut Self, other: &mut Self) -> io::Result<()> {
        if !active.shares_document_with(other) { return Ok(()); }
        if other.document.revision() > active.document.revision() {
            active.refresh_view_from(other)
        } else {
            other.refresh_view_from(active)
        }
    }

    fn refresh_view_from(&mut self, source: &Self) -> io::Result<()> {
        if self.document.revision() != source.document.revision()
            || self.undo_stack.len() != source.undo_stack.len()
            || self.redo_stack.len() != source.redo_stack.len()
        {
            self.document.refresh_view_from(&source.document)?;
            self.undo_stack.clone_from(&source.undo_stack);
            self.redo_stack.clone_from(&source.redo_stack);
            self.multi_edit_group = None;
        }
        self.path.clone_from(&source.path);
        self.dirty = source.dirty;
        self.read_only = source.read_only;
        Ok(())
    }

    pub(crate) fn insert_tab(&mut self) -> io::Result<()> {
        if self.config.insert_spaces {
            let spaces = " ".repeat(self.config.tab_width);
            self.insert_text(&spaces)
        } else {
            self.insert_text("\t")
        }
    }
    pub(crate) fn cursor_state(&self) -> CursorState {
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

    pub(crate) fn begin_history_group(&self) -> usize {
        self.undo_stack.len()
    }

    pub(crate) fn end_history_group(&mut self, start: usize) {
        // A workspace transaction must remain a top-level, coordinated undo step.
        let start = self
            .undo_stack
            .iter()
            .enumerate()
            .skip(start)
            .rfind(|(_, entry)| matches!(entry.kind, HistoryKind::Workspace(_)))
            .map_or(start, |(index, _)| index + 1);
        if start >= self.undo_stack.len() {
            return;
        }

        let entries: Vec<HistoryEntry> = self.undo_stack.drain(start..).collect();

        if entries.len() == 1 {
            self.undo_stack.push(entries.into_iter().next().unwrap());
            return;
        }

        let before = entries.first().unwrap().before.clone();
        let after = entries.last().unwrap().after.clone();
        let kinds = entries.into_iter().map(|entry| entry.kind).collect();

        self.undo_stack.push(HistoryEntry {
            kind: HistoryKind::Sequence(kinds),
            before,
            after,
        });
    }

    pub(crate) fn apply_history_kind(
        &mut self,
        kind: &mut HistoryKind,
        undo: bool,
    ) -> io::Result<()> {
        match kind {
            HistoryKind::Changes(changes) => {
                if undo {
                    for change in changes {
                        if let Some(inserted) = change.inserted {
                            self.document.delete(change.position, inserted.length)?;
                        }

                        if !change.deleted.is_empty() {
                            self.document
                                .insert_pieces(change.position, &change.deleted)?;
                        }
                    }
                } else {
                    for change in changes.iter_mut().rev() {
                        if !change.deleted.is_empty() {
                            self.document
                                .delete(change.position, change.deleted_length)?;
                        }

                        if let Some(inserted) = change.inserted {
                            self.document
                                .insert_pieces(change.position, std::slice::from_ref(&inserted))?;
                        }
                    }
                }
            }

            HistoryKind::Workspace(_) => {
                return Err(io::Error::other("Workspace undo requires both panes"));
            }

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

    pub(crate) fn restore_cursor(&mut self, state: CursorState) {
        let collapse_occurrence_selection = self.config.keybinding_mode == KeybindingMode::Vim
            && !state.secondary_cursors.is_empty();
        self.document.secondary_cursors =
            if self.config.keybinding_mode == KeybindingMode::Conventional {
                state.secondary_cursors
            } else {
                Vec::new()
            };
        self.document.cursor.position = state.position;

        self.document.cursor.line = state.line;

        self.document.cursor.column = state.column;

        self.document.cursor.desired_column = state.desired_column;

        self.document.cursor.anchor = state.anchor;

        self.document.cursor.anchor_line = state.anchor_line;

        self.document.cursor.anchor_column = state.anchor_column;
        if collapse_occurrence_selection {
            self.document.cursor.anchor = state.position;
            self.document.cursor.anchor_line = state.line;
            self.document.cursor.anchor_column = state.column;
        }
    }

    pub(crate) fn new_document(&mut self, path: Option<&str>) -> io::Result<()> {
        if self.dirty {
            return Err(io::Error::other(
                "Save the current document before creating a new file",
            ));
        }
        if path == Some("") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Enter a file path",
            ));
        }

        let mut document = PieceTable::empty()?;
        if let Some(path) = path {
            document
                .write_to_new(std::path::Path::new(path))
                .map_err(|error| {
                    if error.kind() == io::ErrorKind::AlreadyExists {
                        io::Error::new(error.kind(), "File already exists. Use :open to edit it.")
                    } else {
                        error
                    }
                })?;
        }
        piece_table::recovery::arm(&mut document, path.map(std::path::Path::new));
        self.document = document;
        self.emacs.reset();
        self.path = path.map(PathBuf::from);
        self.multi_edit_group = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.dirty = false;
        self.read_only = false;
        Ok(())
    }

    pub(crate) fn open(&mut self, path: &str) -> io::Result<()> {
        let start = Instant::now();

        let mut document = PieceTable::open(path)?;
        piece_table::recovery::arm(&mut document, Some(std::path::Path::new(path)));

        println!("PieceTable::open({}): {:?}", path, start.elapsed());

        self.document = document;
        self.emacs.reset();
        self.path = Some(PathBuf::from(path));

        self.multi_edit_group = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.dirty = false;
        self.read_only = false;

        Ok(())
    }

    pub(crate) fn save(&mut self) -> io::Result<()> {
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
            let filename = format!("untitled_{}.txt", number);

            let path = std::env::current_dir()?.join(filename);

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

    pub(crate) fn save_as(&mut self, path: &str, overwrite: bool) -> io::Result<()> {
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

    pub(crate) fn recover_from(
        &mut self,
        root: &std::path::Path,
        number: usize,
    ) -> io::Result<bool> {
        if self.dirty {
            return Err(io::Error::other(
                "Save the current document before opening recovered work, or switch to an empty pane",
            ));
        }
        let (path, incomplete, session) = piece_table::recovery::restore_number_at(root, number)?;
        self.open(
            path.to_str()
                .ok_or_else(|| io::Error::other("Recovery path is not UTF-8"))?,
        )?;
        self.document.recovered_from(session);
        self.dirty = true;
        Ok(incomplete)
    }

    pub(crate) fn apply_formatted(
        &mut self,
        output: &mut (impl io::Read + io::Seek),
    ) -> io::Result<bool> {
        if self.read_only {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "This file is open for viewing",
            ));
        }
        let before = self.cursor_state();
        output.rewind()?;
        let [cursor, anchor] = formatting::map_positions(
            output,
            [
                (before.line, before.column),
                (before.anchor_line, before.anchor_column),
            ],
        )?;
        if output.stream_position()? == 0 && self.document.len() != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Formatter returned empty output; document preserved",
            ));
        }
        output.rewind()?;
        let Some(snapshot) = self
            .document
            .replace_from_reader(output, formatting::MAX_OUTPUT_BYTES)?
        else {
            return Ok(false);
        };
        self.clear_secondary_cursors();
        let after = CursorState {
            secondary_cursors: Vec::new(),
            position: cursor.byte,
            line: cursor.line,
            column: cursor.column,
            anchor: anchor.byte,
            anchor_line: anchor.line,
            anchor_column: anchor.column,
            desired_column: None,
        };
        self.restore_cursor(after.clone());
        self.undo_stack.push(HistoryEntry {
            kind: HistoryKind::Snapshot(snapshot),
            before,
            after,
        });
        self.redo_stack.clear();
        self.dirty = true;
        Ok(true)
    }

    pub(crate) fn format_document(&mut self, name: Option<&str>) -> io::Result<(String, bool)> {
        if self.read_only {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "This file is open for viewing",
            ));
        }
        if self.document.len() > formatting::MAX_INPUT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Formatting is limited to documents up to 2 MiB",
            ));
        }
        let scratch = formatting::Scratch::create()?;
        if name == Some("builtin") || (name.is_none() && self.path.is_none()) {
            let mut output = formatting::builtin::run(
                self.path.as_deref(),
                &self.document,
                &scratch,
                self.config.tab_width,
                self.config.insert_spaces,
            )?;
            return Ok((
                "built-in indentation".into(),
                self.apply_formatted(&mut output)?,
            ));
        }
        let config = formatting::Formatters::load(std::path::Path::new("config/formatters.toml"))?;
        let (provider, executable) = match config.select(self.path.as_deref(), name) {
            Ok(selected) => selected,
            Err(_) if name.is_none() => {
                let mut output = formatting::builtin::run(
                    self.path.as_deref(),
                    &self.document,
                    &scratch,
                    self.config.tab_width,
                    self.config.insert_spaces,
                )?;
                return Ok((
                    "built-in indentation".into(),
                    self.apply_formatted(&mut output)?,
                ));
            }
            Err(error) => return Err(error),
        };
        let mut output = formatting::run(
            provider,
            &executable,
            self.path.as_deref(),
            &self.document,
            &scratch,
        )?;
        let changed = self.apply_formatted(&mut output)?;
        Ok((provider.name.clone(), changed))
    }

    pub(crate) fn execute(&mut self, command: Command, page_lines: usize) -> io::Result<()> {
        if self.config.keybinding_mode == KeybindingMode::Conventional
            && !self.document.secondary_cursors.is_empty()
            && multi_cursor::is_cursor_motion(command)
        {
            return self.move_occurrence_cursors(command, page_lines);
        }
        if !matches!(
            command,
            Command::SelectNextOccurrence
                | Command::Delete
                | Command::Backspace
                | Command::Newline
                | Command::InsertTab
                | Command::Copy
                | Command::Cut
                | Command::Paste
                | Command::Undo
                | Command::Redo
        ) {
            self.clear_secondary_cursors();
        }
        self.execute_single(command, page_lines)
    }

    pub(crate) fn execute_single(&mut self, command: Command, page_lines: usize) -> io::Result<()> {
        match command {
            Command::SelectNextOccurrence => self.select_next_occurrence(),
            Command::MoveLeft => self.move_left(),

            Command::InsertTab => self.insert_tab(),

            Command::MoveRight => self.move_right(),

            Command::MoveUp => self.move_up(),

            Command::MoveDown => self.move_down(),

            Command::MoveWordLeft | Command::SelectWordLeft => self
                .document
                .move_word(false, command == Command::SelectWordLeft),

            Command::MoveWordRight | Command::SelectWordRight => self
                .document
                .move_word(true, command == Command::SelectWordRight),

            Command::PageUp | Command::SelectPageUp => {
                self.document
                    .move_page(false, page_lines, command == Command::SelectPageUp)
            }

            Command::PageDown | Command::SelectPageDown => {
                self.document
                    .move_page(true, page_lines, command == Command::SelectPageDown)
            }

            Command::SelectAll => self.document.select_all(),
            Command::SelectHome => self.document.select_home(),
            Command::SelectEnd => self.document.select_end(),

            Command::SelectLeft => self.select_left(),

            Command::SelectRight => self.select_right(),

            Command::SelectUp => self.select_up(),

            Command::SelectDown => self.select_down(),

            Command::Home => self.home(),

            Command::End => self.end(),

            Command::Delete => self.delete(),

            Command::Backspace => self.backspace(),

            Command::Newline => self.newline(),

            Command::Save => self.save(),

            Command::Undo => self.undo(),

            Command::Redo => self.redo(),

            Command::Copy | Command::Cut | Command::Paste => Ok(()),

            Command::Quit => Ok(()),

            Command::NewFile | Command::SaveAs | Command::FormatDocument => Ok(()), // Handled in the event loop.
        }
    }

    // ----------------------------------------------------------------------
    // Cursor movement
    // ----------------------------------------------------------------------

    pub(crate) fn move_left(&mut self) -> io::Result<()> {
        if self.document.has_selection() {
            return self.document.move_cursor(self.document.selection_start());
        }

        self.document.cursor_left()
    }

    pub(crate) fn move_right(&mut self) -> io::Result<()> {
        if self.document.has_selection() {
            return self.document.move_cursor(self.document.selection_end());
        }

        self.document.cursor_right()
    }

    pub(crate) fn move_up(&mut self) -> io::Result<()> {
        self.document.cursor_up()
    }

    pub(crate) fn move_down(&mut self) -> io::Result<()> {
        self.document.cursor_down()
    }

    pub(crate) fn select_left(&mut self) -> io::Result<()> {
        self.document.select_left()
    }

    pub(crate) fn select_right(&mut self) -> io::Result<()> {
        self.document.select_right()
    }

    pub(crate) fn select_up(&mut self) -> io::Result<()> {
        self.document.select_up()
    }

    pub(crate) fn select_down(&mut self) -> io::Result<()> {
        self.document.select_down()
    }

    pub(crate) fn home(&mut self) -> io::Result<()> {
        if self.document.has_selection() {
            let position = self.document.selection_start();

            self.document.move_cursor(position)?;
        }

        self.document.cursor_home()
    }

    pub(crate) fn end(&mut self) -> io::Result<()> {
        if self.document.has_selection() {
            let position = self.document.selection_end();

            self.document.move_cursor(position)?;
        }

        self.document.cursor_end()
    }

    // ----------------------------------------------------------------------
    // Editing
    // ----------------------------------------------------------------------

    pub(crate) fn newline(&mut self) -> io::Result<()> {
        self.insert_text("\n")
    }

    pub(crate) fn insert(&mut self, text: &str) -> io::Result<()> {
        self.insert_text(text)
    }

    pub(crate) fn insert_text(&mut self, text: &str) -> io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        if self.read_only {
            return Ok(());
        }

        if !self.document.secondary_cursors.is_empty() {
            return self.edit_occurrences(text, None);
        }

        let range = if self.document.has_selection() {
            SearchResult {
                start: self.document.selection_start(),
                end: self.document.selection_end(),
            }
        } else {
            let position = self.document.cursor.position;

            SearchResult {
                start: position,
                end: position,
            }
        };

        self.replace_range(range, text)?;

        Ok(())
    }

    /// Replace one UTF-8 byte range and record it as a single undo step.
    ///
    /// Returns `true` when the document contents changed. Replacing text
    /// with identical bytes still moves the caret past the range, but does
    /// not allocate a history entry or discard redo history.
    pub(crate) fn replace_range(
        &mut self,
        range: SearchResult,
        replacement: &str,
    ) -> io::Result<bool> {
        if self.read_only {
            return Ok(false);
        }

        if range.end < range.start || range.end > self.document.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "replacement range is outside document",
            ));
        }

        let deleted_length = range.end - range.start;

        self.document
            .len()
            .checked_sub(deleted_length)
            .and_then(|length| length.checked_add(replacement.len()))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "replacement would overflow document length",
                )
            })?;

        let before = self.cursor_state();

        self.clear_secondary_cursors();

        let changed =
            !self
                .document
                .range_equals(range.start, deleted_length, replacement.as_bytes())?;

        let mut deleted = Vec::new();
        let mut inserted = None;

        if changed {
            deleted = self.document.capture_range(range.start, deleted_length)?;

            inserted = self.document.store_text(replacement)?;

            if deleted_length > 0 {
                self.document.delete(range.start, deleted_length)?;
            }

            if let Some(piece) = inserted {
                self.document
                    .insert_pieces(range.start, std::slice::from_ref(&piece))?;
            }
        }

        let new_position = range.start.checked_add(replacement.len()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "replacement cursor overflow")
        })?;

        self.set_cursor_and_anchor(new_position, new_position)?;

        if changed && !self.applying_history {
            let after = self.cursor_state();

            self.undo_stack.push(HistoryEntry {
                kind: HistoryKind::Changes(vec![TextChange {
                    position: range.start,
                    deleted,
                    deleted_length,
                    inserted,
                }]),
                before,
                after,
            });

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
    pub(crate) fn replace_all(
        &mut self,
        searcher: &Searcher,
        replacement: &str,
    ) -> io::Result<usize> {
        if self.read_only {
            return Ok(0);
        }

        let before = self.cursor_state();

        self.clear_secondary_cursors();

        let mut runs: Vec<ReplacementRun> = Vec::new();

        let mut mapped_position = None;
        let mut mapped_anchor = None;
        let mut added_before = 0usize;
        let mut removed_before = 0usize;

        let count = {
            let document = &self.document;

            searcher.visit_non_overlapping_replacements(document, replacement, |found| {
                if found.end < found.start || found.end > document.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "search returned an invalid replacement range",
                    ));
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

                let deleted_length = found.end - found.start;

                if replacement.len() >= deleted_length {
                    added_before = added_before
                        .checked_add(replacement.len() - deleted_length)
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "replacement position overflow",
                            )
                        })?;
                } else {
                    removed_before = removed_before
                        .checked_add(deleted_length - replacement.len())
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "replacement position overflow",
                            )
                        })?;
                }

                if let Some(last) = runs.last_mut() {
                    if last.end == found.start {
                        last.end = found.end;

                        last.repetitions = last.repetitions.checked_add(1).ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidData, "replacement count overflow")
                        })?;

                        return Ok(());
                    }
                }

                runs.push(ReplacementRun {
                    start: found.start,
                    end: found.end,
                    repetitions: 1,
                });

                Ok(())
            })?
        };

        if runs.is_empty() {
            return Ok(0);
        }

        let position = Self::finish_position_mapping(
            before.position,
            mapped_position,
            added_before,
            removed_before,
        )?;

        let anchor = Self::finish_position_mapping(
            before.anchor,
            mapped_anchor,
            added_before,
            removed_before,
        )?;

        let snapshot = self
            .document
            .replace_runs(
                runs.iter().map(|run| (run.start, run.end, run.repetitions)),
                replacement,
            )?
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "non-empty replacement plan produced no snapshot",
                )
            })?;

        self.set_cursor_and_anchor(position, anchor)?;

        let after = self.cursor_state();

        if !self.applying_history {
            self.undo_stack.push(HistoryEntry {
                kind: HistoryKind::Snapshot(snapshot),
                before,
                after,
            });

            self.redo_stack.clear();
        }

        self.dirty = true;

        Ok(count)
    }

    pub(crate) fn observe_position_change(
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

        let base = if position < range.start {
            position
        } else if position <= range.end {
            range.start.checked_add(replacement_length).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "mapped replacement position overflow",
                )
            })?
        } else {
            return Ok(());
        };

        *mapped = Some(
            base.checked_add(added_before)
                .and_then(|value| value.checked_sub(removed_before))
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "mapped replacement position overflow",
                    )
                })?,
        );

        Ok(())
    }

    pub(crate) fn finish_position_mapping(
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
            .and_then(|value| value.checked_sub(removed_before))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "mapped replacement position overflow",
                )
            })
    }

    pub(crate) fn set_cursor_and_anchor(
        &mut self,
        position: usize,
        anchor: usize,
    ) -> io::Result<()> {
        let (line, column) = self.document.line_column_at(position)?;

        let (anchor_line, anchor_column) = self.document.line_column_at(anchor)?;

        self.document.cursor.position = position;

        self.document.cursor.line = line;

        self.document.cursor.column = column;

        self.document.cursor.anchor = anchor;

        self.document.cursor.anchor_line = anchor_line;

        self.document.cursor.anchor_column = anchor_column;

        self.document.cursor.desired_column = None;

        Ok(())
    }

    pub(crate) fn delete_range(&mut self, position: usize, length: usize) -> io::Result<()> {
        if length == 0 {
            return Ok(());
        }

        let end = position
            .checked_add(length)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "delete range overflow"))?;

        self.replace_range(
            SearchResult {
                start: position,
                end,
            },
            "",
        )?;

        Ok(())
    }

    pub(crate) fn delete(&mut self) -> io::Result<()> {
        if !self.document.secondary_cursors.is_empty() {
            return self.edit_occurrences("", Some(false));
        }
        if self.document.has_selection() {
            return self.delete_selection();
        }

        let position = self.document.cursor.position;

        if position >= self.document.len() {
            return Ok(());
        }

        let next = self.document.next_char_boundary(position)?;

        self.delete_range(position, next - position)
    }

    pub(crate) fn backspace(&mut self) -> io::Result<()> {
        if !self.document.secondary_cursors.is_empty() {
            return self.edit_occurrences("", Some(true));
        }
        if self.document.has_selection() {
            return self.delete_selection();
        }

        let position = self.document.cursor.position;

        if position == 0 {
            return Ok(());
        }

        let previous = self.document.previous_char_boundary(position)?;

        self.delete_range(previous, position - previous)
    }

    pub(crate) fn delete_selection(&mut self) -> io::Result<()> {
        if !self.document.secondary_cursors.is_empty() {
            return self.edit_occurrences("", None);
        }
        if !self.document.has_selection() {
            return Ok(());
        }

        let start = self.document.selection_start();

        let end = self.document.selection_end();

        self.delete_range(start, end - start)
    }

    // ----------------------------------------------------------------------
    // Undo / redo
    // ----------------------------------------------------------------------

    pub(crate) fn undo(&mut self) -> io::Result<()> {
        if self
            .undo_stack
            .last()
            .is_some_and(|e| matches!(e.kind, HistoryKind::Workspace(_)))
        {
            return Err(io::Error::other("Workspace undo requires both panes"));
        }
        self.multi_edit_group = None;
        if self.read_only {
            return Ok(());
        }

        let mut entry = match self.undo_stack.pop() {
            Some(entry) => entry,
            None => return Ok(()),
        };

        self.applying_history = true;

        let result: io::Result<()> = (|| {
            self.apply_history_kind(&mut entry.kind, true)?;
            self.restore_cursor(entry.before.clone());

            Ok(())
        })();

        self.applying_history = false;

        result?;

        self.redo_stack.push(entry);
        self.dirty = true;

        Ok(())
    }

    pub(crate) fn redo(&mut self) -> io::Result<()> {
        if self
            .redo_stack
            .last()
            .is_some_and(|e| matches!(e.kind, HistoryKind::Workspace(_)))
        {
            return Err(io::Error::other("Workspace undo requires both panes"));
        }
        self.multi_edit_group = None;
        if self.read_only {
            return Ok(());
        }

        let mut entry = match self.redo_stack.pop() {
            Some(entry) => entry,
            None => return Ok(()),
        };

        self.applying_history = true;

        let result: io::Result<()> = (|| {
            self.apply_history_kind(&mut entry.kind, false)?;
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

pub(crate) fn file_is_open_in(path: &str, editor: &Editor) -> bool {
    let Some(open_path) = editor.path.as_deref() else {
        return false;
    };

    let requested = std::path::Path::new(path);

    match (
        std::fs::canonicalize(requested),
        std::fs::canonicalize(open_path),
    ) {
        (Ok(requested), Ok(open_path)) => requested == open_path,
        _ => requested == open_path,
    }
}

#[cfg(test)]
mod shared_view_tests;
