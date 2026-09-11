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


use std::io;

use sdl3::clipboard::ClipboardUtil;

use crate::piece_table::PieceTable;
use crate::Editor;


pub(crate) trait TextClipboard {
    fn set_text(
        &self,
        text: &str,
    ) -> Result<(), String>;

    fn text(&self) -> Result<String, String>;
}

impl TextClipboard for ClipboardUtil {
    fn set_text(
        &self,
        text: &str,
    ) -> Result<(), String> {
        self.set_clipboard_text(text)
            .map_err(|error| error.to_string())
    }

    fn text(&self) -> Result<String, String> {
        self.clipboard_text()
            .map_err(|error| error.to_string())
    }
}

/// Copies the current document selection to the platform clipboard.
///
/// The selection is materialized only for this call and is not retained by
/// Pötyi after SDL has handed it to the operating system.
pub(crate) fn copy_selection<C: TextClipboard>(
    clipboard: &C,
    table: &PieceTable,
) -> Result<bool, String> {
    let mut ranges: Vec<_> = table.secondary_cursors.iter()
        .chain(std::iter::once(&table.cursor))
        .filter(|cursor| cursor.position != cursor.anchor)
        .map(|cursor| (cursor.position.min(cursor.anchor), cursor.position.max(cursor.anchor)))
        .collect();
    if ranges.is_empty() { return Ok(false); }
    ranges.sort_unstable();
    let mut bytes = Vec::new();
    for (index, (start, end)) in ranges.into_iter().enumerate() {
        if index > 0 { bytes.push(b'\n'); }
        bytes.extend(table.read_range(start, end - start).map_err(|error| error.to_string())?);
    }

    let text =
        String::from_utf8(bytes)
            .map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "selection is not valid UTF-8: {error}"
                    ),
                )
                .to_string()
            })?;

    // SDL's text clipboard API is NUL-terminated. The Rust SDL wrapper
    // cannot represent an embedded NUL and would otherwise panic.
    if text.contains('\0') {
        return Err(
            "selection contains a NUL byte and cannot be copied as text"
                .to_string()
        );
    }

    clipboard.set_text(&text)?;

    Ok(true)
}

/// Reads text from the platform clipboard and normalizes line endings.
///
/// Windows exposes clipboard line endings as CRLF. Pötyi uses LF for
/// editing on every platform. Normalization reuses the String's existing
/// allocation, including for multiline Windows clipboard contents.
pub(crate) fn read_text<C: TextClipboard>(
    clipboard: &C,
) -> Result<String, String> {
    Ok(
        normalize_line_endings(
            clipboard.text()?
        )
    )
}

/// Pastes clipboard text as one editor operation so one Undo restores the
/// complete selection or insertion point from before the paste.
pub(crate) fn paste<C: TextClipboard>(
    clipboard: &C,
    editor: &mut Editor,
) -> Result<bool, String> {
    let text = read_text(clipboard)?;

    if text.is_empty() {
        return Ok(false);
    }

    editor.insert(&text)
        .map_err(|error| error.to_string())?;

    Ok(true)
}

/// Copies and removes the current selection as one undoable edit.
pub(crate) fn cut_selection<C: TextClipboard>(
    clipboard: &C,
    editor: &mut Editor,
) -> Result<bool, String> {
    if !copy_selection(clipboard, &editor.document)? {
        return Ok(false);
    }

    editor.delete_selection()
        .map_err(|error| error.to_string())?;

    Ok(true)
}

fn normalize_line_endings(
    text: String,
) -> String {
    if !text.as_bytes().contains(&b'\r') {
        return text;
    }

    let mut bytes =
        text.into_bytes();

    let mut read = 0usize;
    let mut write = 0usize;

    while read < bytes.len() {
        if bytes[read] == b'\r' {
            bytes[write] = b'\n';
            write += 1;
            read += 1;

            if read < bytes.len()
                && bytes[read] == b'\n'
            {
                read += 1;
            }
        } else {
            bytes[write] = bytes[read];
            write += 1;
            read += 1;
        }
    }

    bytes.truncate(write);

    // The input came from a valid String. Removing ASCII carriage returns
    // and replacing them with ASCII line feeds preserves UTF-8 validity.
    String::from_utf8(bytes)
        .expect(
            "clipboard newline normalization must preserve UTF-8"
        )
}


#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::config::{
        EditorConfig,
        LineNumberMode,
    };

    #[derive(Default)]
    struct MemoryClipboard {
        text: RefCell<String>,
    }

    impl TextClipboard for MemoryClipboard {
        fn set_text(
            &self,
            text: &str,
        ) -> Result<(), String> {
            text.clone_into(
                &mut self.text.borrow_mut()
            );

            Ok(())
        }

        fn text(&self) -> Result<String, String> {
            Ok(self.text.borrow().clone())
        }
    }

    #[test]
    fn copy_preserves_forward_utf8_selection() {
        let mut table =
            PieceTable::empty().unwrap();

        table.insert(0, "aé中z").unwrap();
        table.move_cursor(1).unwrap();
        table.select_right().unwrap();
        table.select_right().unwrap();

        let clipboard =
            MemoryClipboard::default();

        assert!(
            copy_selection(
                &clipboard,
                &table,
            )
            .unwrap()
        );

        assert_eq!(
            clipboard.text().unwrap(),
            "é中",
        );
    }

    #[test]
    fn copy_preserves_backward_selection() {
        let mut table =
            PieceTable::empty().unwrap();

        table.insert(0, "alpha").unwrap();
        table.move_cursor(4).unwrap();
        table.select_left().unwrap();
        table.select_left().unwrap();

        let clipboard =
            MemoryClipboard::default();

        copy_selection(
            &clipboard,
            &table,
        )
        .unwrap();

        assert_eq!(
            clipboard.text().unwrap(),
            "ph",
        );
    }

    #[test]
    fn copy_without_selection_leaves_clipboard_unchanged() {
        let mut table =
            PieceTable::empty().unwrap();

        table.insert(0, "alpha").unwrap();

        let clipboard = MemoryClipboard {
            text: RefCell::new(
                "existing".to_string()
            ),
        };

        assert!(
            !copy_selection(
                &clipboard,
                &table,
            )
            .unwrap()
        );

        assert_eq!(
            clipboard.text().unwrap(),
            "existing",
        );
    }

    #[test]
    fn read_text_keeps_lf_and_utf8_content() {
        let clipboard = MemoryClipboard {
            text: RefCell::new(
                "one\né中".to_string()
            ),
        };

        assert_eq!(
            read_text(&clipboard).unwrap(),
            "one\né中",
        );
    }

    #[test]
    fn read_text_normalizes_windows_and_classic_line_endings() {
        let clipboard = MemoryClipboard {
            text: RefCell::new(
                "one\r\ntwo\rthree\n"
                    .to_string()
            ),
        };

        assert_eq!(
            read_text(&clipboard).unwrap(),
            "one\ntwo\nthree\n",
        );
    }

    #[test]
    fn newline_normalization_reuses_the_clipboard_allocation() {
        let mut text =
            String::with_capacity(64);

        text.push_str("one\r\ntwo\rthree");

        let allocation =
            text.as_ptr();

        let normalized =
            normalize_line_endings(text);

        assert_eq!(
            normalized,
            "one\ntwo\nthree",
        );

        assert_eq!(
            normalized.as_ptr(),
            allocation,
        );
    }

    #[test]
    fn copy_rejects_embedded_nul_without_panicking() {
        let mut table =
            PieceTable::empty().unwrap();

        table.insert(0, "a\0b").unwrap();
        table.move_cursor(0).unwrap();
        table.select_right().unwrap();
        table.select_right().unwrap();
        table.select_right().unwrap();

        let clipboard =
            MemoryClipboard::default();

        assert!(
            copy_selection(
                &clipboard,
                &table,
            )
            .unwrap_err()
            .contains("NUL")
        );
    }

    #[test]
    fn paste_replaces_selection_as_one_undo_step() {
        let mut editor =
            Editor::new(
                EditorConfig {
                    tab_width: 4,
                    insert_spaces: false,
                    line_numbers:
                        LineNumberMode::Dynamic,
                    font_size: 18,
                    keybinding_mode:
                        crate::config::KeybindingMode::Conventional,
                }
            )
            .unwrap();

        editor
            .document
            .insert(0, "alpha")
            .unwrap();

        editor
            .document
            .move_cursor(1)
            .unwrap();

        editor
            .document
            .select_right()
            .unwrap();

        editor
            .document
            .select_right()
            .unwrap();

        let clipboard = MemoryClipboard {
            text: RefCell::new(
                "é\r\nx".to_string()
            ),
        };

        assert!(
            paste(
                &clipboard,
                &mut editor,
            )
            .unwrap()
        );

        let pasted =
            editor.document
                .read_range(
                    0,
                    editor.document.len(),
                )
                .unwrap();

        assert_eq!(
            String::from_utf8(pasted).unwrap(),
            "aé\nxha",
        );

        assert_eq!(
            editor.undo_stack.len(),
            1,
        );

        editor.undo().unwrap();

        let restored =
            editor.document
                .read_range(
                    0,
                    editor.document.len(),
                )
                .unwrap();

        assert_eq!(
            String::from_utf8(restored).unwrap(),
            "alpha",
        );
    }

    #[test]
    fn empty_paste_does_not_create_history() {
        let mut editor =
            Editor::new(
                EditorConfig {
                    tab_width: 4,
                    insert_spaces: false,
                    line_numbers:
                        LineNumberMode::Dynamic,
                    font_size: 18,
                    keybinding_mode:
                        crate::config::KeybindingMode::Conventional,
                }
            )
            .unwrap();

        let clipboard =
            MemoryClipboard::default();

        assert!(
            !paste(
                &clipboard,
                &mut editor,
            )
            .unwrap()
        );

        assert!(editor.undo_stack.is_empty());
        assert!(editor.redo_stack.is_empty());
    }

    #[test]
    fn cut_copies_and_removes_selection() {
        let mut editor =
            Editor::new(
                EditorConfig {
                    tab_width: 4,
                    insert_spaces: false,
                    line_numbers:
                        LineNumberMode::Dynamic,
                    font_size: 18,
                    keybinding_mode:
                        crate::config::KeybindingMode::Conventional,
                }
            )
            .unwrap();

        editor.document.insert(0, "alpha beta").unwrap();
        editor.set_cursor_and_anchor(5, 0).unwrap();
        let clipboard = MemoryClipboard::default();

        assert!(cut_selection(&clipboard, &mut editor).unwrap());
        assert_eq!(clipboard.text().unwrap(), "alpha");
        assert_eq!(editor.document.text().unwrap(), " beta");

        editor.undo().unwrap();
        assert_eq!(editor.document.text().unwrap(), "alpha beta");
    }
}
