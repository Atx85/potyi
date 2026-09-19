// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io;
use std::ops::Range;

use sdl3::keyboard::{Keycode, Mod};

use super::{Terminal, TerminalAction};
use crate::clipboard::TextClipboard;
use crate::piece_table::PieceTable;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Visual {
    #[default]
    Off,
    Character,
    Line,
}

#[derive(Default)]
pub(super) struct Selection {
    pub focused: bool,
    pub cursor: usize,
    anchor: usize,
    range: Range<usize>,
    visual: Visual,
    pending_g: bool,
    pub desired_x: Option<i32>,
    drag: Option<(i32, i32, Option<TerminalAction>)>,
    dragged: bool,
    open_other_pane: bool,
}

pub(crate) enum OutputCommand {
    None,
    Copy,
    Rows(isize, bool),
    RowEdge(bool, bool),
}

impl Terminal {
    pub fn output_focused(&self) -> bool {
        self.selection.focused
    }

    pub fn output_cursor(&self) -> usize {
        self.selection.cursor
    }

    pub fn output_selection(&self) -> Range<usize> {
        self.selection.range.clone()
    }

    pub fn output_linewise(&self) -> bool {
        self.selection.visual == Visual::Line
    }

    pub fn focus_prompt(&mut self) {
        self.selection = Selection::default();
    }

    pub fn output_focus_hint(&self) -> Option<&'static str> {
        self.output_focused()
            .then_some(match self.selection.visual {
                Visual::Off => "Output · select and copy · Esc: command input",
                Visual::Character => "Output · VISUAL · y: copy · Esc: command input",
                Visual::Line => "Output · VISUAL LINE · y: copy · Esc: command input",
            })
    }

    pub fn begin_output_drag(
        &mut self,
        offset: usize,
        x: i32,
        y: i32,
        action: Option<TerminalAction>,
        open_other_pane: bool,
    ) -> io::Result<()> {
        self.selection = Selection {
            focused: true,
            open_other_pane,
            ..Selection::default()
        };
        self.move_output_cursor(offset, false)?;
        self.selection.drag = Some((x, y, action));
        Ok(())
    }

    pub fn output_dragging(&self) -> bool {
        self.selection.drag.is_some()
    }

    pub fn drag_output_to(&mut self, offset: usize, x: i32, y: i32) -> io::Result<()> {
        if let Some((start_x, start_y, _)) = &self.selection.drag {
            self.selection.dragged |= (x - start_x).abs() >= 3 || (y - start_y).abs() >= 3;
            if self.selection.dragged {
                self.move_output_cursor(offset, true)?;
            }
        }
        Ok(())
    }

    pub(crate) fn output_click_opens_other_pane(&self) -> bool {
        self.selection.open_other_pane
    }

    pub fn finish_output_drag(&mut self) -> Option<TerminalAction> {
        let (_, _, action) = self.selection.drag.take()?;
        if self.selection.dragged { None } else { action }
    }

    pub fn move_output_cursor(&mut self, offset: usize, extend: bool) -> io::Result<()> {
        let offset = offset.min(self.output.len());
        if self
            .output
            .byte_at(offset)?
            .is_some_and(|byte| byte & 0xc0 == 0x80)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "selection is not a UTF-8 boundary",
            ));
        }
        self.selection.focused = true;
        self.selection.cursor = offset;
        self.selection.desired_x = None;
        if !extend && self.selection.visual == Visual::Off {
            self.selection.anchor = offset;
        }
        self.refresh_output_selection()
    }

    fn refresh_output_selection(&mut self) -> io::Result<()> {
        let start = self.selection.anchor.min(self.selection.cursor);
        let end = self.selection.anchor.max(self.selection.cursor);
        self.selection.range = match self.selection.visual {
            Visual::Off => start..end,
            Visual::Character => start..self.output.next_char_boundary(end)?,
            Visual::Line => line_start(&self.output, start)?..line_end(&mut self.output, end)?,
        };
        Ok(())
    }

    pub fn output_desired_x(&self) -> Option<i32> {
        self.selection.desired_x
    }

    pub fn set_output_desired_x(&mut self, x: i32) {
        self.selection.desired_x = Some(x);
    }

    pub fn copy_output(
        &mut self,
        clipboard: &impl TextClipboard,
        all: bool,
        yank: bool,
    ) -> Result<(), String> {
        let range = if all {
            0..self.output.len()
        } else {
            self.output_selection()
        };
        if range.is_empty() {
            return Ok(());
        }
        let bytes = self
            .output
            .read_range(range.start, range.len())
            .map_err(|e| e.to_string())?;
        let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        clipboard.set_text(&text)?;
        if yank {
            self.selection.visual = Visual::Off;
            self.move_output_cursor(range.start, false)
                .map_err(|e| e.to_string())?;
        }
        self.set_status(if all {
            "Output copied"
        } else {
            "Selection copied"
        });
        Ok(())
    }

    /// Read-only navigation; editing and shell input never pass through here.
    pub fn output_key(
        &mut self,
        key: Keycode,
        modifiers: Mod,
        vim: bool,
    ) -> io::Result<OutputCommand> {
        let shift = modifiers.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
        let control = modifiers.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
        let gui = modifiers.intersects(Mod::LGUIMOD | Mod::RGUIMOD);
        let alt = modifiers.intersects(Mod::LALTMOD | Mod::RALTMOD);
        let selecting = self.selection.visual != Visual::Off
            || (shift
                && matches!(
                    key,
                    Keycode::Left
                        | Keycode::Right
                        | Keycode::Up
                        | Keycode::Down
                        | Keycode::Home
                        | Keycode::End
                        | Keycode::PageUp
                        | Keycode::PageDown
                ));
        let pending_g = std::mem::take(&mut self.selection.pending_g);
        if key == Keycode::Escape {
            self.focus_prompt();
            return Ok(OutputCommand::None);
        }
        if (control || gui) && key == Keycode::A {
            self.selection.visual = Visual::Off;
            self.selection.anchor = 0;
            self.move_output_cursor(self.output.len(), true)?;
            return Ok(OutputCommand::None);
        }
        if !control && !gui && !alt && vim {
            match key {
                Keycode::V => {
                    let mode = if shift {
                        Visual::Line
                    } else {
                        Visual::Character
                    };
                    self.selection.visual = if self.selection.visual == mode {
                        Visual::Off
                    } else {
                        mode
                    };
                    self.selection.anchor = self.selection.cursor;
                    self.refresh_output_selection()?;
                    return Ok(OutputCommand::None);
                }
                Keycode::Y => return Ok(OutputCommand::Copy),
                Keycode::G if shift || pending_g => {
                    let target = if shift {
                        line_start(&self.output, self.output.len().saturating_sub(1))?
                    } else {
                        0
                    };
                    self.move_output_cursor(target, self.selection.visual != Visual::Off)?;
                    return Ok(OutputCommand::None);
                }
                Keycode::G => {
                    self.selection.pending_g = true;
                    return Ok(OutputCommand::None);
                }
                _ => {}
            }
        }
        let plain_vim = vim && !control && !gui && !alt;
        let word = (control || alt) && !gui;
        let position = self.selection.cursor;
        let target = match key {
            Keycode::Up => return Ok(OutputCommand::Rows(-1, selecting)),
            Keycode::Down => return Ok(OutputCommand::Rows(1, selecting)),
            Keycode::K if plain_vim => {
                return Ok(OutputCommand::Rows(
                    -1,
                    self.selection.visual != Visual::Off,
                ));
            }
            Keycode::J if plain_vim => {
                return Ok(OutputCommand::Rows(1, self.selection.visual != Visual::Off));
            }
            Keycode::PageUp => return Ok(OutputCommand::Rows(-20, selecting)),
            Keycode::PageDown => return Ok(OutputCommand::Rows(20, selecting)),
            Keycode::Home if control || gui => 0,
            Keycode::End if control || gui => self.output.len(),
            Keycode::Home => return Ok(OutputCommand::RowEdge(false, selecting)),
            Keycode::End => return Ok(OutputCommand::RowEdge(true, selecting)),
            Keycode::_0 if plain_vim && !shift => line_start(&self.output, position)?,
            Keycode::_4 | Keycode::Dollar if plain_vim && shift => {
                let end = line_end(&mut self.output, position)?;
                let end = if end > position && self.output.byte_at(end - 1)? == Some(b'\n') {
                    end - 1
                } else {
                    end
                };
                if end > line_start(&self.output, position)? {
                    self.output.previous_char_boundary(end)?
                } else {
                    end
                }
            }
            Keycode::Left if word => word_start(&self.output, position, false)?,
            Keycode::Right if word => word_start(&self.output, position, true)?,
            Keycode::Left => self.output.previous_char_boundary(position)?,
            Keycode::Right => self.output.next_char_boundary(position)?,
            Keycode::H if plain_vim => self.output.previous_char_boundary(position)?,
            Keycode::L if plain_vim => self.output.next_char_boundary(position)?,
            Keycode::W if plain_vim => word_start(&self.output, position, true)?,
            Keycode::B if plain_vim => word_start(&self.output, position, false)?,
            _ => return Ok(OutputCommand::None),
        };
        self.move_output_cursor(target, selecting)?;
        Ok(OutputCommand::None)
    }

    pub(super) fn trim_selection(&mut self, removed: usize) {
        // Keep byte anchors on the retained text; no output-sized selection copy.
        let selection = &mut self.selection;
        selection.cursor = selection.cursor.saturating_sub(removed);
        selection.anchor = selection.anchor.saturating_sub(removed);
        selection.range = selection.range.start.saturating_sub(removed)
            ..selection.range.end.saturating_sub(removed);
        selection.drag = None;
    }
}

fn line_start(output: &PieceTable, mut offset: usize) -> io::Result<usize> {
    while offset > 0 {
        let start = offset.saturating_sub(4096);
        let bytes = output.read_range(start, offset - start)?;
        if let Some(index) = bytes.iter().rposition(|byte| *byte == b'\n') {
            return Ok(start + index + 1);
        }
        offset = start;
    }
    Ok(0)
}

fn line_end(output: &mut PieceTable, offset: usize) -> io::Result<usize> {
    Ok(output.next_line_start_from(offset)?.unwrap_or(output.len()))
}

fn class(output: &PieceTable, offset: usize) -> io::Result<u8> {
    let end = output.next_char_boundary(offset)?;
    let bytes = output.read_range(offset, end - offset)?;
    let text =
        std::str::from_utf8(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(match text.chars().next() {
        Some(c) if c.is_whitespace() => 0,
        Some(c) if c.is_alphanumeric() || c == '_' => 1,
        _ => 2,
    })
}

fn word_start(output: &PieceTable, mut offset: usize, forward: bool) -> io::Result<usize> {
    if forward && offset < output.len() {
        let kind = class(output, offset)?;
        while offset < output.len() && class(output, offset)? == kind {
            offset = output.next_char_boundary(offset)?;
        }
        while offset < output.len() && class(output, offset)? == 0 {
            offset = output.next_char_boundary(offset)?;
        }
    } else if !forward && offset > 0 {
        offset = output.previous_char_boundary(offset)?;
        while offset > 0 && class(output, offset)? == 0 {
            offset = output.previous_char_boundary(offset)?;
        }
        let kind = class(output, offset)?;
        while offset > 0 {
            let previous = output.previous_char_boundary(offset)?;
            if class(output, previous)? != kind {
                break;
            }
            offset = previous;
        }
    }
    Ok(offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Clipboard(RefCell<String>);
    impl TextClipboard for Clipboard {
        fn set_text(&self, text: &str) -> Result<(), String> {
            *self.0.borrow_mut() = text.into();
            Ok(())
        }
        fn text(&self) -> Result<String, String> {
            Ok(self.0.borrow().clone())
        }
    }

    fn terminal(text: &str) -> Terminal {
        let mut terminal = Terminal::new(std::env::temp_dir()).unwrap();
        terminal.clear().unwrap();
        terminal.append_text(text).unwrap();
        terminal.move_output_cursor(0, false).unwrap();
        terminal
    }

    fn selected(terminal: &mut Terminal) -> String {
        let clipboard = Clipboard::default();
        terminal.copy_output(&clipboard, false, false).unwrap();
        clipboard.text().unwrap()
    }

    #[test]
    fn conventional_selection_copies_unicode_in_both_directions() {
        let mut term = terminal("aé🙂 z\n");
        for _ in 0..3 {
            term.output_key(Keycode::Right, Mod::LSHIFTMOD, false)
                .unwrap();
        }
        assert_eq!(selected(&mut term), "aé🙂");
        term.move_output_cursor(7, false).unwrap();
        term.output_key(Keycode::Left, Mod::LSHIFTMOD, true)
            .unwrap();
        assert_eq!(selected(&mut term), "🙂");
        term.output_key(Keycode::Left, Mod::LSHIFTMOD, true)
            .unwrap();
        assert_eq!(selected(&mut term), "é🙂");
        term.output_key(Keycode::A, Mod::LGUIMOD, false).unwrap();
        assert_eq!(selected(&mut term), "aé🙂 z\n");
    }

    #[test]
    fn vim_visual_is_inclusive_and_yank_returns_to_output_navigation() {
        let mut term = terminal("é🙂 text\n");
        term.output_key(Keycode::V, Mod::NOMOD, true).unwrap();
        assert_eq!(selected(&mut term), "é");
        term.output_key(Keycode::L, Mod::NOMOD, true).unwrap();
        assert_eq!(selected(&mut term), "é🙂");
        assert!(matches!(
            term.output_key(Keycode::Y, Mod::NOMOD, true).unwrap(),
            OutputCommand::Copy
        ));
        let clipboard = Clipboard::default();
        term.copy_output(&clipboard, false, true).unwrap();
        assert_eq!(clipboard.text().unwrap(), "é🙂");
        assert!(term.output_focused());
        assert!(term.output_selection().is_empty());
        term.output_key(Keycode::Escape, Mod::NOMOD, true).unwrap();
        assert!(!term.output_focused());
    }

    #[test]
    fn visual_line_selects_whole_lines_in_both_directions() {
        let mut term = terminal("first\né🙂\nlast");
        term.move_output_cursor(8, false).unwrap();
        term.output_key(Keycode::V, Mod::LSHIFTMOD, true).unwrap();
        assert_eq!(selected(&mut term), "é🙂\n");
        term.move_output_cursor(2, true).unwrap();
        assert_eq!(selected(&mut term), "first\né🙂\n");
        term.move_output_cursor(15, true).unwrap();
        assert_eq!(selected(&mut term), "é🙂\nlast");
    }

    #[test]
    fn vim_words_and_line_end_leave_output_unchanged() {
        let original = "hello é🙂!\nnext\n";
        let mut term = terminal(original);
        term.output_key(Keycode::W, Mod::NOMOD, true).unwrap();
        assert_eq!(term.output_cursor(), 6);
        term.output_key(Keycode::B, Mod::NOMOD, true).unwrap();
        assert_eq!(term.output_cursor(), 0);
        term.output_key(Keycode::V, Mod::NOMOD, true).unwrap();
        term.output_key(Keycode::_4, Mod::LSHIFTMOD, true).unwrap();
        assert_eq!(selected(&mut term), "hello é🙂!");
        term.output_key(Keycode::Escape, Mod::NOMOD, true).unwrap();
        term.move_output_cursor(0, false).unwrap();
        term.output_key(Keycode::G, Mod::LSHIFTMOD, true).unwrap();
        assert_eq!(term.output_cursor(), 14);
        for key in [
            Keycode::Delete,
            Keycode::Backspace,
            Keycode::X,
            Keycode::D,
            Keycode::Return,
        ] {
            term.output_key(key, Mod::NOMOD, true).unwrap();
        }
        assert_eq!(term.output_text().unwrap(), original);
        assert!(term.input().is_empty());
    }

    #[test]
    fn streaming_preserves_selection_and_clear_resets_focus() {
        let mut term = terminal("before\nselected é🙂\n");
        term.move_output_cursor(7, false).unwrap();
        term.move_output_cursor(term.output.len() - 1, true)
            .unwrap();
        for _ in 0..100 {
            term.append_text("more output\n").unwrap();
        }
        assert_eq!(selected(&mut term), "selected é🙂");
        let clipboard = Clipboard::default();
        term.copy_output(&clipboard, true, false).unwrap();
        assert!(clipboard.text().unwrap().ends_with("more output\n"));
        term.clear().unwrap();
        assert!(term.output_selection().is_empty());
        assert!(!term.output_focused());
    }

    #[test]
    fn trimming_preserves_retained_selection_and_discards_old_selection() {
        let mut term = terminal(&"x\n".repeat(super::super::MAX_OUTPUT_BYTES / 2 - 20));
        let start = term.output.len();
        term.append_text("selected é🙂\n").unwrap();
        term.move_output_cursor(start, false).unwrap();
        term.move_output_cursor(term.output.len() - 1, true)
            .unwrap();
        term.append_text(&"more\n".repeat(30)).unwrap();
        assert!(term.output_cursor() < start);
        assert_eq!(selected(&mut term), "selected é🙂");
        term.move_output_cursor(0, false).unwrap();
        term.move_output_cursor(1, true).unwrap();
        term.append_text(&"x\n".repeat(super::super::MAX_OUTPUT_BYTES / 2))
            .unwrap();
        assert!(term.output_selection().is_empty());
        assert_eq!(term.output_cursor(), 0);
    }

    #[test]
    fn dragging_a_link_selects_without_opening_but_click_still_opens() {
        let mut term = terminal("Cargo.lock");
        let action = TerminalAction::ListedFile("Cargo.lock".into());
        term.begin_output_drag(0, 12, 50, Some(action.clone()), false)
            .unwrap();
        assert!(!term.output_click_opens_other_pane());
        term.drag_output_to(0, 13, 50).unwrap();
        assert_eq!(term.finish_output_drag(), Some(action.clone()));
        term.begin_output_drag(0, 12, 50, Some(action), true).unwrap();
        assert!(term.output_click_opens_other_pane());
        term.drag_output_to(10, 100, 50).unwrap();
        assert_eq!(term.finish_output_drag(), None);
        assert_eq!(selected(&mut term), "Cargo.lock");
    }

    #[test]
    fn empty_selection_does_not_overwrite_clipboard() {
        let mut term = terminal("output");
        let clipboard = Clipboard(RefCell::new("existing".into()));
        term.copy_output(&clipboard, false, true).unwrap();
        assert_eq!(clipboard.text().unwrap(), "existing");
    }
}
