//! Conventional-mode occurrence editing. Work happens only on user input;
//! selections retain cursor metadata, never copies of the document.

use std::io;

use crate::config::KeybindingMode;
use crate::keybindings::Command;
use crate::{Editor, HistoryEntry, HistoryKind, Searcher, TextChange};

pub(crate) fn is_cursor_motion(command: Command) -> bool {
    matches!(
        command,
        Command::MoveLeft
            | Command::MoveRight
            | Command::MoveUp
            | Command::MoveDown
            | Command::MoveWordLeft
            | Command::MoveWordRight
            | Command::Home
            | Command::End
            | Command::PageUp
            | Command::PageDown
            | Command::SelectLeft
            | Command::SelectRight
            | Command::SelectUp
            | Command::SelectDown
            | Command::SelectWordLeft
            | Command::SelectWordRight
            | Command::SelectHome
            | Command::SelectEnd
            | Command::SelectPageUp
            | Command::SelectPageDown
    )
}

impl Editor {
    pub(crate) fn clear_secondary_cursors(&mut self) {
        self.document.secondary_cursors.clear();
        self.multi_edit_group = None;
    }

    pub(crate) fn move_occurrence_cursors(
        &mut self,
        command: Command,
        page_lines: usize,
    ) -> io::Result<()> {
        self.multi_edit_group = None;
        let primary = self.document.cursor.clone();
        // Use the ordinary navigation rules independently at each cursor,
        // including each cursor's preferred column on vertical movements.
        let result = (|| {
            let mut moved = Vec::with_capacity(self.document.secondary_cursors.len() + 1);
            for index in 0..=self.document.secondary_cursors.len() {
                let is_primary = index == self.document.secondary_cursors.len();
                self.document.cursor = if is_primary {
                    primary.clone()
                } else {
                    self.document.secondary_cursors[index].clone()
                };
                self.execute_single(command, page_lines)?;
                moved.push((self.document.cursor.clone(), is_primary));
            }
            moved.sort_unstable_by_key(|(c, _)| {
                (c.position.min(c.anchor), c.position.max(c.anchor))
            });

            // Converging cursors and overlapping selections become one edit
            // target. Adjacent selections remain separate until they overlap.
            let mut merged: Vec<(crate::piece_table::Cursor, bool)> =
                Vec::with_capacity(moved.len());
            for (cursor, is_primary) in moved {
                if let Some((last, last_primary)) = merged.last_mut() {
                    let start = last.position.min(last.anchor);
                    let end = last.position.max(last.anchor);
                    let next_start = cursor.position.min(cursor.anchor);
                    let next_end = cursor.position.max(cursor.anchor);
                    if next_start < end
                        || next_start == start
                        || (next_start == end && next_start == next_end)
                    {
                        let active = if is_primary { &cursor } else { &*last };
                        let desired = active.desired_column;
                        let (position, anchor) = if active.position < active.anchor {
                            (start, end.max(next_end))
                        } else {
                            (end.max(next_end), start)
                        };
                        self.set_cursor_and_anchor(position, anchor)?;
                        self.document.cursor.desired_column = desired;
                        *last = self.document.cursor.clone();
                        *last_primary |= is_primary;
                        continue;
                    }
                }
                merged.push((cursor, is_primary));
            }
            Ok::<_, io::Error>(merged)
        })();
        self.document.cursor = primary;
        let cursors = result?;
        self.document.secondary_cursors.clear();
        for (cursor, is_primary) in cursors {
            if is_primary {
                self.document.cursor = cursor;
            } else {
                self.document.secondary_cursors.push(cursor);
            }
        }
        Ok(())
    }

    fn character_at(&self, position: usize) -> io::Result<char> {
        let end = self.document.next_char_boundary(position)?;
        let bytes = self.document.read_range(position, end - position)?;
        std::str::from_utf8(&bytes)
            .ok()
            .and_then(|text| text.chars().next())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 character"))
    }

    pub(crate) fn select_next_occurrence(&mut self) -> io::Result<()> {
        if self.config.keybinding_mode != KeybindingMode::Conventional {
            return Ok(());
        }
        self.multi_edit_group = None;
        if !self.document.has_selection() {
            self.clear_secondary_cursors();
            if self.document.is_empty() {
                return Ok(());
            }
            let mut start = self.document.cursor.position;
            if start == self.document.len() {
                start = self.document.previous_char_boundary(start)?;
            }
            let character = self.character_at(start)?;
            if character.is_whitespace() {
                return Ok(());
            }
            let mut end = self.document.next_char_boundary(start)?;
            if character.is_alphanumeric() || character == '_' {
                while start > 0 {
                    let previous = self.document.previous_char_boundary(start)?;
                    let c = self.character_at(previous)?;
                    if !c.is_alphanumeric() && c != '_' {
                        break;
                    }
                    start = previous;
                }
                while end < self.document.len() {
                    let c = self.character_at(end)?;
                    if !c.is_alphanumeric() && c != '_' {
                        break;
                    }
                    end = self.document.next_char_boundary(end)?;
                }
            }
            return self.set_cursor_and_anchor(end, start);
        }

        let start = self.document.selection_start();
        let end = self.document.selection_end();
        let bytes = self.document.read_range(start, end - start)?;
        let query = String::from_utf8(bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let searcher = Searcher::new(&query);
        let mut occupied: Vec<_> = self
            .document
            .secondary_cursors
            .iter()
            .chain(std::iter::once(&self.document.cursor))
            .map(|cursor| {
                (
                    cursor.position.min(cursor.anchor),
                    cursor.position.max(cursor.anchor),
                )
            })
            .collect();
        occupied.sort_unstable();

        let mut position = end;
        let mut wrapped = false;
        loop {
            let Some(found) = searcher.find_forward_match(&self.document, position)? else {
                if wrapped {
                    return Ok(());
                }
                wrapped = true;
                position = 0;
                continue;
            };
            if wrapped && found.start >= end {
                return Ok(());
            }
            let index = occupied.partition_point(|&(_, end)| end <= found.start);
            if let Some(&(start, end)) = occupied.get(index)
                && start < found.end
            {
                position = end;
                continue;
            }
            let previous = self.document.cursor.clone();
            self.set_cursor_and_anchor(found.end, found.start)?;
            self.document.secondary_cursors.push(previous);
            return Ok(());
        }
    }

    /// Apply a single input event at every cursor. `backward` expands empty
    /// selections by one Unicode character for Backspace/Delete.
    pub(crate) fn edit_occurrences(
        &mut self,
        text: &str,
        backward: Option<bool>,
    ) -> io::Result<()> {
        if self.read_only {
            return Ok(());
        }
        let before = self.cursor_state();
        let mut ranges = Vec::with_capacity(self.document.secondary_cursors.len() + 1);
        for (cursor, primary) in self
            .document
            .secondary_cursors
            .iter()
            .map(|c| (c, false))
            .chain(std::iter::once((&self.document.cursor, true)))
        {
            let mut start = cursor.position.min(cursor.anchor);
            let mut end = cursor.position.max(cursor.anchor);
            if start == end {
                match backward {
                    Some(true) if start > 0 => {
                        start = self.document.previous_char_boundary(start)?
                    }
                    Some(false) if end < self.document.len() => {
                        end = self.document.next_char_boundary(end)?
                    }
                    _ => {}
                }
            }
            ranges.push((start, end, primary));
        }
        ranges.sort_unstable_by_key(|&(start, end, _)| (start, end));
        let mut merged: Vec<(usize, usize, bool)> = Vec::with_capacity(ranges.len());
        for (start, end, primary) in ranges {
            if let Some(last) = merged.last_mut()
                && (start < last.1 || start == last.0)
            {
                last.1 = last.1.max(end);
                last.2 |= primary;
            } else {
                merged.push((start, end, primary));
            }
        }

        let removed: usize = merged.iter().map(|&(start, end, _)| end - start).sum();
        text.len()
            .checked_mul(merged.len())
            .and_then(|added| (self.document.len() - removed).checked_add(added))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Edit would overflow document length",
                )
            })?;

        // Reuse one stored copy of the replacement at all cursors. Retain only
        // affected piece references in history, rather than document snapshots.
        let mut changes = Vec::new();
        for &(start, end, _) in &merged {
            if !self
                .document
                .range_equals(start, end - start, text.as_bytes())?
            {
                changes.push(TextChange {
                    position: start,
                    deleted: self.document.capture_range(start, end - start)?,
                    deleted_length: end - start,
                    inserted: None,
                });
            }
        }
        if !changes.is_empty() {
            let inserted = self.document.store_text(text)?;
            for change in changes.iter_mut().rev() {
                change.inserted = inserted;
                self.document
                    .delete(change.position, change.deleted_length)?;
                if let Some(piece) = inserted {
                    self.document
                        .insert_pieces(change.position, std::slice::from_ref(&piece))?;
                }
            }
        }

        let mut removed_before = 0;
        let mut added_before = 0;
        let mut cursors = Vec::with_capacity(merged.len());
        let mut primary_position = 0;
        for (start, end, primary) in merged {
            let position = start - removed_before + added_before + text.len();
            self.set_cursor_and_anchor(position, position)?;
            if primary {
                primary_position = position;
            }
            if cursors
                .last()
                .is_none_or(|c: &crate::piece_table::Cursor| c.position != position)
            {
                cursors.push(self.document.cursor.clone());
            }
            removed_before += end - start;
            added_before += text.len();
        }
        self.set_cursor_and_anchor(primary_position, primary_position)?;
        cursors.retain(|c| c.position != primary_position);
        self.document.secondary_cursors = cursors;

        if !changes.is_empty() {
            let after = self.cursor_state();
            let kind = HistoryKind::Changes(changes);
            if let Some(index) = self.multi_edit_group
                && index + 1 == self.undo_stack.len()
                && let HistoryKind::Sequence(kinds) = &mut self.undo_stack[index].kind
            {
                kinds.push(kind);
                self.undo_stack[index].after = after;
            } else {
                self.multi_edit_group = Some(self.undo_stack.len());
                self.undo_stack.push(HistoryEntry {
                    kind: HistoryKind::Sequence(vec![kind]),
                    before,
                    after,
                });
            }
            self.redo_stack.clear();
            self.dirty = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::{self, TextClipboard};
    use crate::{
        config::EditorConfig,
        keybindings::{Command, KeyBindings},
    };
    use sdl3::keyboard::{Keycode, Mod};
    use std::cell::RefCell;

    fn editor(text: &str) -> Editor {
        let mut editor = Editor::new(EditorConfig::default()).unwrap();
        editor.config.keybinding_mode = KeybindingMode::Conventional;
        editor.document.insert(0, text).unwrap();
        editor.document.move_cursor(0).unwrap();
        editor
    }

    fn select(editor: &mut Editor, count: usize) {
        for _ in 0..count {
            editor.execute(Command::SelectNextOccurrence, 20).unwrap();
        }
    }

    #[derive(Default)]
    struct Clipboard(RefCell<String>);
    impl TextClipboard for Clipboard {
        fn text(&self) -> Result<String, String> {
            Ok(self.0.borrow().clone())
        }
        fn set_text(&self, text: &str) -> Result<(), String> {
            *self.0.borrow_mut() = text.into();
            Ok(())
        }
    }

    #[test]
    fn occurrence_typing_is_one_group_and_restores_all_selections() {
        let mut e = editor("case case case");
        select(&mut e, 3);
        assert_eq!(e.document.secondary_cursors.len(), 2);
        assert!(e.undo_stack.is_empty());
        e.insert("é").unwrap();
        e.insert("中").unwrap();
        e.backspace().unwrap();
        e.insert("x").unwrap();
        assert_eq!(e.document.text().unwrap(), "éx éx éx");
        assert_eq!(e.undo_stack.len(), 1);
        e.clear_secondary_cursors();
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "case case case");
        assert_eq!(e.document.secondary_cursors.len(), 2);
        assert_eq!(e.document.selection_start(), 10);
        e.redo().unwrap();
        assert_eq!(e.document.text().unwrap(), "éx éx éx");
        assert_eq!(e.document.secondary_cursors.len(), 2);
        e.insert("!").unwrap();
        assert_eq!(e.undo_stack.len(), 2, "redo ends the previous typing group");
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "éx éx éx");
    }

    #[test]
    fn occurrence_selection_wraps_skips_duplicates_and_is_literal() {
        let mut e = editor("a.b A.B a.b aXb a.b");
        e.set_cursor_and_anchor(11, 8).unwrap();
        select(&mut e, 10);
        assert_eq!(e.document.secondary_cursors.len(), 2);
        assert_eq!(e.document.selection_start(), 0);
        e.insert("x").unwrap();
        assert_eq!(e.document.text().unwrap(), "x A.B x aXb x");
    }

    #[test]
    fn occurrence_selection_accepts_backward_and_multiline_text() {
        let mut e = editor("é\n中 / é\n中");
        e.set_cursor_and_anchor(0, "é\n中".len()).unwrap();
        select(&mut e, 1);
        e.insert("a\nb\n").unwrap();
        assert_eq!(e.document.text().unwrap(), "a\nb\n / a\nb\n");
        assert_eq!(e.document.cursor.line, 4);
        assert_eq!(e.document.secondary_cursors[0].line, 2);
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "é\n中 / é\n中");
    }

    #[test]
    fn occurrence_word_selection_uses_unicode_boundaries() {
        let mut e = editor("é_中 é_中");
        e.document.move_cursor(2).unwrap();
        select(&mut e, 1);
        assert_eq!(e.document.selection_start(), 0);
        assert_eq!(e.document.selection_end(), "é_中".len());
        select(&mut e, 1);
        e.insert("🙂").unwrap();
        e.backspace().unwrap();
        assert_eq!(e.document.text().unwrap(), " ");
        e.delete().unwrap();
        assert_eq!(e.document.text().unwrap(), "");
        assert!(e.document.secondary_cursors.is_empty());
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "é_中 é_中");
    }

    #[test]
    fn occurrence_overlaps_and_adjacent_deletions_do_not_duplicate_cursors() {
        let mut e = editor("aaaaa");
        e.set_cursor_and_anchor(2, 0).unwrap();
        select(&mut e, 8);
        assert_eq!(e.document.secondary_cursors.len(), 1);
        e.delete().unwrap();
        assert_eq!(e.document.text().unwrap(), "a");
        assert!(e.document.secondary_cursors.is_empty());
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "aaaaa");
    }

    #[test]
    fn occurrence_paste_cut_newline_tab_and_noop_history() {
        let mut e = editor("cat cat");
        select(&mut e, 2);
        e.insert("cat").unwrap();
        assert!(!e.dirty);
        assert!(e.undo_stack.is_empty());
        e.clear_secondary_cursors();
        e.document.move_cursor(0).unwrap();
        select(&mut e, 2);
        let clipboard = Clipboard::default();
        clipboard::copy_selection(&clipboard, &e.document).unwrap();
        assert_eq!(clipboard.text().unwrap(), "cat\ncat");
        clipboard::cut_selection(&clipboard, &mut e).unwrap();
        assert_eq!(e.document.text().unwrap(), " ");
        clipboard.set_text("é\r\nz").unwrap();
        clipboard::paste(&clipboard, &mut e).unwrap();
        e.newline().unwrap();
        e.config.insert_spaces = false;
        e.insert_tab().unwrap();
        assert_eq!(e.document.text().unwrap(), "é\nz\n\t é\nz\n\t");
        assert_eq!(e.undo_stack.len(), 1);
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "cat cat");
    }

    #[test]
    fn occurrence_navigation_preserves_cursors_and_vim_return_ends_them() {
        let mut e = editor("cat cat");
        select(&mut e, 2);
        e.insert("dog").unwrap();
        e.execute(Command::MoveLeft, 10).unwrap();
        assert_eq!(e.document.secondary_cursors.len(), 1);
        e.insert("!").unwrap();
        assert_eq!(e.document.text().unwrap(), "do!g do!g");
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "dog dog");
        e.config.keybinding_mode = KeybindingMode::Vim;
        e.clear_secondary_cursors();
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "cat cat");
        assert!(e.document.secondary_cursors.is_empty());
        select(&mut e, 3);
        assert!(e.document.secondary_cursors.is_empty());
        assert_eq!(
            crate::VimController::page_motion(Keycode::D, Mod::LCTRLMOD),
            Some((true, true))
        );
    }

    #[test]
    fn occurrence_arrows_select_and_replace_only_a_suffix() {
        let mut e = editor("foo_bar foo_bar foo_bar");
        select(&mut e, 3);
        e.execute(Command::MoveRight, 10).unwrap();
        assert!(!e.document.has_selection());
        assert_eq!(e.document.secondary_cursors.len(), 2);
        for _ in 0..3 {
            e.execute(Command::SelectLeft, 10).unwrap();
        }
        let clipboard = Clipboard::default();
        clipboard::copy_selection(&clipboard, &e.document).unwrap();
        assert_eq!(clipboard.text().unwrap(), "bar\nbar\nbar");
        e.insert("b").unwrap();
        e.insert("az").unwrap();
        assert_eq!(e.document.text().unwrap(), "foo_baz foo_baz foo_baz");
        assert_eq!(e.undo_stack.len(), 1);
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "foo_bar foo_bar foo_bar");
        assert_eq!(e.document.secondary_cursors.len(), 2);
        assert_eq!(e.document.cursor.anchor - e.document.cursor.position, 3);
        e.redo().unwrap();
        assert_eq!(e.document.text().unwrap(), "foo_baz foo_baz foo_baz");
    }

    #[test]
    fn occurrence_arrows_move_inside_unicode_words_without_losing_cursors() {
        let mut e = editor("é中x é中x");
        select(&mut e, 2);
        e.execute(Command::MoveLeft, 10).unwrap();
        e.execute(Command::MoveRight, 10).unwrap();
        e.execute(Command::SelectRight, 10).unwrap();
        e.insert("🙂").unwrap();
        assert_eq!(e.document.text().unwrap(), "é🙂x é🙂x");
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "é中x é中x");
    }

    #[test]
    fn occurrence_vertical_arrows_keep_each_preferred_column() {
        let mut e = editor("alpha\nx\nlonger\nalpha\nxy\nlonger");
        select(&mut e, 2);
        e.execute(Command::MoveRight, 10).unwrap();
        e.execute(Command::MoveDown, 10).unwrap();
        assert_eq!((e.document.cursor.line, e.document.cursor.column), (4, 2));
        assert_eq!(
            (
                e.document.secondary_cursors[0].line,
                e.document.secondary_cursors[0].column
            ),
            (1, 1)
        );
        e.execute(Command::MoveDown, 10).unwrap();
        assert_eq!((e.document.cursor.line, e.document.cursor.column), (5, 5));
        assert_eq!(
            (
                e.document.secondary_cursors[0].line,
                e.document.secondary_cursors[0].column
            ),
            (2, 5)
        );
        e.execute(Command::MoveUp, 10).unwrap();
        e.execute(Command::MoveUp, 10).unwrap();
        assert_eq!((e.document.cursor.line, e.document.cursor.column), (3, 5));
        assert_eq!(
            (
                e.document.secondary_cursors[0].line,
                e.document.secondary_cursors[0].column
            ),
            (0, 5)
        );
        assert!(e.undo_stack.is_empty());
        assert!(!e.dirty);
    }

    #[test]
    fn occurrence_home_end_and_word_selection_edit_each_line() {
        let mut e = editor("alpha tail\nalpha tail");
        select(&mut e, 2);
        e.execute(Command::End, 10).unwrap();
        e.execute(Command::SelectWordLeft, 10).unwrap();
        e.insert("end").unwrap();
        assert_eq!(e.document.text().unwrap(), "alpha end\nalpha end");
        e.execute(Command::Home, 10).unwrap();
        e.execute(Command::SelectWordRight, 10).unwrap();
        e.insert("first ").unwrap();
        assert_eq!(e.document.text().unwrap(), "first end\nfirst end");
    }

    #[test]
    fn occurrence_shift_vertical_arrows_select_at_each_cursor() {
        for command in [Command::SelectUp, Command::SelectDown] {
            let mut e = editor("cat\none\n\ncat\ntwo\n");
            select(&mut e, 2);
            e.execute(Command::MoveRight, 10).unwrap();
            if command == Command::SelectUp {
                e.execute(Command::MoveDown, 10).unwrap();
            }
            e.execute(command, 10).unwrap();
            e.insert("_").unwrap();
            assert_eq!(e.document.text().unwrap(), "cat_\n\ncat_\n");
            e.undo().unwrap();
            assert_eq!(e.document.text().unwrap(), "cat\none\n\ncat\ntwo\n");
        }
    }

    #[test]
    fn occurrence_page_movement_and_selection_keep_all_cursors() {
        let mut e = editor("cat\n111\n222\n333\n444\n555\ncat\n777\n888\n999");
        select(&mut e, 2);
        e.execute(Command::MoveRight, 2).unwrap();
        e.execute(Command::PageDown, 2).unwrap();
        assert_eq!((e.document.cursor.line, e.document.cursor.column), (8, 3));
        assert_eq!(
            (
                e.document.secondary_cursors[0].line,
                e.document.secondary_cursors[0].column
            ),
            (2, 3)
        );
        e.execute(Command::PageUp, 2).unwrap();
        assert_eq!(e.document.cursor.line, 6);
        assert_eq!(e.document.secondary_cursors[0].line, 0);
        e.execute(Command::SelectPageDown, 2).unwrap();
        e.insert("X").unwrap();
        assert_eq!(e.document.text().unwrap(), "catX\n333\n444\n555\ncatX\n999");
    }

    #[test]
    fn occurrence_converging_carets_and_overlapping_selections_merge() {
        let mut e = editor("cat cat");
        select(&mut e, 2);
        e.execute(Command::MoveRight, 10).unwrap();
        e.execute(Command::Home, 10).unwrap();
        assert!(e.document.secondary_cursors.is_empty());
        e.insert("!").unwrap();
        assert_eq!(e.document.text().unwrap(), "!cat cat");

        let mut e = editor("cat cat");
        select(&mut e, 2);
        e.execute(Command::MoveLeft, 10).unwrap();
        e.execute(Command::SelectEnd, 10).unwrap();
        assert!(e.document.secondary_cursors.is_empty());
        assert_eq!(e.document.selection_start(), 0);
        assert_eq!(e.document.selection_end(), 7);
        e.insert("dog").unwrap();
        assert_eq!(e.document.text().unwrap(), "dog");
    }

    #[test]
    fn occurrence_shortcuts_require_an_explicit_press() {
        let bindings = KeyBindings::default();
        for modifier in [Mod::LCTRLMOD, Mod::RCTRLMOD, Mod::LGUIMOD, Mod::RGUIMOD] {
            assert_eq!(
                bindings.command_for(Keycode::D, modifier, false),
                Some(Command::SelectNextOccurrence)
            );
            assert_eq!(bindings.command_for(Keycode::D, modifier, true), None);
        }
        assert_eq!(bindings.command_for(Keycode::D, Mod::NOMOD, false), None);
    }

    #[test]
    fn occurrence_fifty_matches_share_inserted_storage_and_undo() {
        let original = vec!["case"; 50].join("\n");
        let mut e = editor(&original);
        select(&mut e, 50);
        let before = e.document.edit_store_len();
        e.insert("replacement").unwrap();
        assert_eq!(e.document.edit_store_len() - before, "replacement".len());
        assert_eq!(
            e.document.text().unwrap(),
            vec!["replacement"; 50].join("\n")
        );
        assert_eq!(e.undo_stack.len(), 1);
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), original);
    }

    #[test]
    fn occurrence_empty_whitespace_and_read_only_are_safe() {
        let mut e = editor("");
        select(&mut e, 3);
        let mut e = editor("   ");
        select(&mut e, 3);
        assert!(!e.document.has_selection());
        let mut e = editor("cat cat");
        select(&mut e, 2);
        e.read_only = true;
        e.insert("dog").unwrap();
        e.delete().unwrap();
        e.backspace().unwrap();
        assert_eq!(e.document.text().unwrap(), "cat cat");
        assert!(e.undo_stack.is_empty());
    }
}
