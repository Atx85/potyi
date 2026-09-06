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
mod keybindings;
mod line_numbers;
mod piece_table;
mod renderer;
mod search;
mod search_ui;
mod window;
mod syntax;
mod config;
mod terminal;
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
use config::{EditorConfig, KeybindingMode};
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
    ParsedCommand,
};
use terminal::{
    Terminal,
    TerminalAction,
    TerminalEvent,
};
use vim::{VimController, VimUiAction};

// ==========================================================================
// Cursor / history
// ==========================================================================

#[derive(Clone, Copy)]
struct CursorState {
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

    applying_history: bool,
    dirty: bool,
    read_only: bool,
}

impl Editor {

    
    fn new(config: EditorConfig) -> io::Result<Self> {
        Ok(Self {
            document: PieceTable::empty()?,
            path: None,
            config,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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

        let before = entries.first().unwrap().before;
        let after = entries.last().unwrap().after;
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
    }

    fn open(
        &mut self,
        path: &str,
    ) -> io::Result<()> {
        let start = Instant::now();

        let document =
            PieceTable::open(path)?;

        println!(
            "PieceTable::open({}): {:?}",
            path,
            start.elapsed()
        );

        self.document = document;
        self.path = Some(PathBuf::from(path));

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

    self.path = Some(path);
    self.dirty = false;

    Ok(())
}

    fn execute(
        &mut self,
        command: Command,
    ) -> io::Result<()> {
        match command {
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
            self.restore_cursor(entry.before);

            Ok(())
        })();

        self.applying_history = false;

        result?;

        self.redo_stack.push(entry);
        self.dirty = true;

        Ok(())
    }

    fn redo(&mut self) -> io::Result<()> {
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
            self.restore_cursor(entry.after);

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

fn execute_command_bar(
    command_bar: &mut CommandBar,
    search_ui: &mut SearchUi,
    editor: &mut Editor,
    other_editor: &mut Editor,
    renderer: &mut Renderer<'_>,
    terminal: &mut Terminal,
    reverse_find: bool,
) -> CommandOutcome {
    let mut outcome =
        CommandOutcome::default();

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
        }) => {
            match editor.document
                .move_cursor_to_line_column(
                    line - 1,
                    column.unwrap_or(1) - 1,
                )
            {
                Ok(()) => {
                    command_bar.close();
                    search_ui.close();
                    outcome.cursor_changed = true;
                }

                Err(error) => {
                    command_bar.set_status(
                        error.to_string()
                    );
                }
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

        Ok(ParsedCommand::Quit) => {
            outcome.quit = true;
        }

        Err(error) => {
            command_bar.set_status(error);
        }
    }

    outcome
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
    *vim_enabled = mode == KeybindingMode::Vim;

    if *vim_enabled {
        vim.reset();
        other_vim.reset();
        renderer.set_mode_label(Some(vim.mode_label()));
    } else {
        vim.deactivate(editor);
        other_vim.deactivate(other_editor);
        renderer.set_mode_label(None);
    }
}

fn handle_terminal_action(
    action: TerminalAction,
    terminal: &mut Terminal,
    editor: &mut Editor,
    other_editor: &Editor,
    renderer: &mut Renderer<'_>,
) -> Result<bool, String> {
    let (value, read_only) = match action {
        TerminalAction::None => return Ok(false),
        TerminalAction::Edit(value) => (value, false),
        TerminalAction::View(value) => (value, true),
    };

    if editor.dirty {
        terminal.set_status(
            "Save the current file before opening another one"
        );
        return Ok(false);
    }

    let (path_text, line, column) =
        match parse_location(&value) {
            Some((path, line, column)) =>
                (path, Some(line), column),
            None =>
                (value.as_str(), None, None),
        };

    let path = match terminal.resolve_path(
        path_text
    ) {
        Ok(path) => path,
        Err(error) => {
            terminal.set_status(error.to_string());
            return Ok(false);
        }
    };

    let display_path =
        path.to_string_lossy().into_owned();

    if file_is_open_in(
        &display_path,
        other_editor,
    ) {
        terminal.set_status(
            "That file is already open in the other pane"
        );
        terminal.close_to_editor();
        return Ok(true);
    }

    if let Err(error) = editor.open(&display_path) {
        terminal.set_status(format!(
            "Could not open {}: {error}",
            path.display(),
        ));
        return Ok(false);
    }

    editor.read_only = read_only;

    if let Some(line) = line
        && let Err(error) = editor.document
            .move_cursor_to_line_column(
                line.saturating_sub(1),
                column.unwrap_or(1)
                    .saturating_sub(1),
            )
    {
        terminal.set_status(error.to_string());
    }

    renderer.set_file_path(
        editor.path.as_deref()
    );
    renderer.invalidate_scroll_cache();
    renderer.update_cursor(&editor.document);
    renderer.ensure_cursor_visible(
        &mut editor.document
    );
    terminal.close_to_editor();

    Ok(false)
}

// ==========================================================================
// Main
// ==========================================================================

fn main() -> Result<(), String> {


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

if let Some(arg) = std::env::args().nth(1) {
    if let Some((path, line, column)) = parse_location(&arg) {
        editor.open(path)
            .map_err(|e| e.to_string())?;

        editor.document.move_cursor_to_line_column(
            line.saturating_sub(1),
            column.unwrap_or(0),
        ).map_err(|e| e.to_string())?;
    } else {
        editor.open(&arg)
            .map_err(|e| e.to_string())?;
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

let mut renderer =
    Renderer::new(
        canvas,
        logical_font,
        raster_font,
        logical_font_size,
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

    'event_loop: loop {
        pending_events.clear();

        if !dirty
            && let Some(event) = event_pump
                .wait_event_timeout(
                    Duration::from_secs(1)
                )
        {
            pending_events.push(event);
        }

        pending_events.extend(
            event_pump.poll_iter()
        );

        for mut event in pending_events.drain(..) {
            if let Some(terminal_event) = event
                .as_user_event_type::<TerminalEvent>()
            {
                terminal.handle_event(terminal_event)
                    .map_err(|error| {
                        error.to_string()
                    })?;
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

editor
    .open(&filename)
    .map_err(|e| e.to_string())?;

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
                                        &other_editor,
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
                        if let Err(error) = terminal.clear() {
                            terminal.set_status(
                                error.to_string()
                            );
                        }
                    }

                    TerminalHit::Editor => {
                        terminal.close_to_editor();
                    }

                    TerminalHit::Output => {
                        if let Some(path) = renderer
                            .terminal_path_at(
                                &mut terminal,
                                x as i32,
                                y as i32,
                            )?
                        {
                            if handle_terminal_action(
                                TerminalAction::Edit(path),
                                &mut terminal,
                                &mut editor,
                                &other_editor,
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
                    }

                    TerminalHit::Outside => {}
                }

                dirty = true;
            } else if command_bar.is_active() {
                match renderer.command_bar_hit_at(
                    &command_bar,
                    x as i32,
                    y as i32,
                ) {
                    CommandBarHit::Suggestion(index) => {
                        if command_bar.apply_suggestion(
                            index
                        ) {
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
                                    &editor.document
                                );

                                renderer.ensure_search_match_visible(
                                    &mut editor.document,
                                    &search_ui,
                                    &command_bar,
                                );
                            }
                        }

                        dirty = true;
                    }

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
                                false,
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

                    dirty = true;
                }
            }
        }
    }
}

Event::MouseMotion {
    x,
    y,
    ..
} => {
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
                        command_bar.close();
                        search_ui.close();
                        terminal.toggle(
                            editor.path.as_deref()
                        );
                        dirty = true;
                        continue;
                    }

                    if terminal.is_active() {
                        if ctrl_pressed(keymod)
                            && key == Keycode::V
                            && !repeat
                        {
                            match read_text(&clipboard) {
                                Ok(text) => terminal
                                    .insert_text(&text),
                                Err(error) => terminal
                                    .set_status(format!(
                                        "Clipboard paste failed: {error}"
                                    )),
                            }

                            dirty = true;
                            continue;
                        }

                        if ctrl_pressed(keymod)
                            && shift_pressed(keymod)
                            && key == Keycode::C
                            && !repeat
                        {
                            match terminal.output_text() {
                                Ok(output) => match clipboard
                                    .set_clipboard_text(
                                        &output
                                    )
                                {
                                    Ok(()) => terminal
                                        .set_status(
                                            "Output copied"
                                        ),
                                    Err(error) => terminal
                                        .set_status(format!(
                                            "Clipboard copy failed: {error}"
                                        )),
                                },
                                Err(error) => terminal
                                    .set_status(
                                        error.to_string()
                                    ),
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
                                            &other_editor,
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

                    /*
                     * Familiar shortcuts open the shared command bar with
                     * their command already selected. Ctrl+P opens the full
                     * command list.
                     */
                    if ctrl_pressed(keymod)
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
                                        shift_pressed(
                                            keymod
                                        ),
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
                    if vim_enabled {
                        let outcome = vim.handle_key(
                            &mut editor,
                            &clipboard,
                            key,
                            keymod,
                            repeat,
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
                                        .execute(command)
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
                        terminal.insert_text(&text);
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

                    if !vim_enabled && text == ":" {
                        let line = editor
                            .document
                            .cursor
                            .line;

                        let empty_line = editor
                            .document
                            .line_text(line)
                            .map_err(|error| {
                                error.to_string()
                            })?
                            .is_empty();

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
                    ..
                } => {
                    if terminal.is_active() {
                        terminal.scroll(
                            y as isize * 3
                        );
                        dirty = true;
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

        if dirty {
            let start =
                Instant::now();

            renderer.update_window_size()?;

            if terminal.is_active() {
                renderer.render_terminal(
                    &mut terminal
                )?;
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
