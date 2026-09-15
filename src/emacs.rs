// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Practical Emacs editing over the existing file-backed document and undo history.
use crate::{
    Editor,
    clipboard::{TextClipboard, read_text},
    search::SearchResult,
};
use sdl3::keyboard::{Keycode, Mod};
use std::{collections::VecDeque, io};

const MAX_COUNT: i32 = 1000;
const RING_BYTES: usize = 256 * 1024;
const RING_ENTRIES: usize = 32;

#[derive(Default)]
pub(crate) struct State {
    mark: Option<usize>,
    active_mark: bool,
    prefix: bool,
    meta: bool,
    count: Option<i32>,
    digits: bool,
    last_kill: bool,
    yank: Option<(usize, usize, usize, usize)>, // start, end, ring index, undo length
}
impl State {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn cancel_sequence(&mut self) {
        self.prefix = false;
        self.meta = false;
        self.count = None;
        self.digits = false;
        self.last_kill = false;
        self.yank = None;
    }
    pub fn label(&self) -> &'static str {
        if self.prefix {
            "Emacs · C-x"
        } else if self.meta {
            "Emacs · Esc"
        } else if self.count.is_some() {
            "Emacs · argument"
        } else if self.active_mark {
            "Emacs · mark"
        } else {
            "Emacs"
        }
    }
    fn take_count(&mut self) -> i32 {
        self.digits = false;
        self.count.take().unwrap_or(1)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Ui {
    None,
    Command(&'static str, bool),
    Search(bool),
    History(bool, usize),
    OtherPane,
    Help,
    Split,
    OnlyPane,
    Cancel,
}
pub(crate) struct Outcome {
    pub consumed: bool,
    pub changed: bool,
    pub ui: Ui,
}
impl Outcome {
    fn handled(ui: Ui) -> Self {
        Self {
            consumed: true,
            changed: false,
            ui,
        }
    }
}
#[derive(Default)]
pub(crate) struct Controller {
    ring: VecDeque<String>,
    bytes: usize,
    suppress_text: bool,
    pub search_origin: Option<usize>,
}
impl Controller {
    // SDL text follows its key event. Reset on the next key so an Alt shortcut
    // which produced no text never swallows the next ordinary character.
    pub fn begin_key(&mut self) {
        self.suppress_text = false;
    }
    pub fn consume_text(&mut self) -> bool {
        std::mem::take(&mut self.suppress_text)
    }

    pub fn insert(&mut self, editor: &mut Editor, text: &str) -> io::Result<()> {
        let count = editor.emacs.take_count().max(0) as usize;
        editor.emacs.last_kill = false;
        editor.emacs.yank = None;
        if text.len().saturating_mul(count) > 1024 * 1024 {
            return Err(io::Error::other("Repeated insertion is limited to 1 MiB"));
        }
        let start = editor.document.selection_start();
        editor.insert(&text.repeat(count))?;
        editor.emacs.mark = Some(start);
        editor.emacs.active_mark = false;
        Ok(())
    }

    pub fn key<C: TextClipboard>(
        &mut self,
        editor: &mut Editor,
        clipboard: &C,
        key: Keycode,
        mods: Mod,
        page: usize,
    ) -> Result<Outcome, String> {
        let mut state = std::mem::take(&mut editor.emacs);
        let result = self.key_inner(editor, &mut state, clipboard, key, mods, page);
        editor.emacs = state;
        result
    }

    fn key_inner<C: TextClipboard>(
        &mut self,
        editor: &mut Editor,
        s: &mut State,
        clipboard: &C,
        key: Keycode,
        mods: Mod,
        page: usize,
    ) -> Result<Outcome, String> {
        if matches!(
            key,
            Keycode::LCtrl
                | Keycode::RCtrl
                | Keycode::LAlt
                | Keycode::RAlt
                | Keycode::LShift
                | Keycode::RShift
                | Keycode::LGui
                | Keycode::RGui
        ) {
            return Ok(Outcome::handled(Ui::None));
        }
        let ctrl = mods.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
        let alt = mods.intersects(Mod::LALTMOD | Mod::RALTMOD);
        let shift = mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
        if mods.intersects(Mod::LGUIMOD | Mod::RGUIMOD) {
            s.last_kill = false;
            s.yank = None;
            s.mark = None;
            s.active_mark = false;
            return Ok(Outcome {
                consumed: false,
                changed: false,
                ui: Ui::None,
            });
        }
        if ctrl && key == Keycode::G {
            s.reset();
            editor
                .document
                .move_cursor(editor.document.cursor.position)
                .map_err(err)?;
            return Ok(Outcome::handled(Ui::Cancel));
        }
        if key == Keycode::Escape {
            s.meta = true;
            return Ok(Outcome::handled(Ui::None));
        }
        let meta = std::mem::take(&mut s.meta) || alt;
        let prefixed = std::mem::take(&mut s.prefix);
        self.suppress_text = meta || prefixed;
        if prefixed {
            s.last_kill = false;
            s.yank = None;
            let count = s.take_count().unsigned_abs() as usize;
            let ui = match key {
                Keycode::S if ctrl => Ui::Command(":save", true),
                Keycode::F if ctrl => Ui::Command(":open ", false),
                Keycode::W if ctrl => Ui::Command(":save-as ", false),
                Keycode::C if ctrl => Ui::Command(":quit", true),
                Keycode::O if !ctrl => Ui::OtherPane,
                Keycode::_2 | Keycode::_3 if !ctrl => Ui::Split,
                Keycode::_1 if !ctrl => Ui::OnlyPane,
                Keycode::U if !ctrl => Ui::History(false, count),
                Keycode::H if !ctrl => {
                    editor.document.select_all().map_err(err)?;
                    s.mark = Some(editor.document.cursor.anchor);
                    s.active_mark = true;
                    Ui::None
                }
                Keycode::X if ctrl => {
                    let mark = s
                        .mark
                        .ok_or("No mark set; use Ctrl+Space")?
                        .min(editor.document.len());
                    let point = editor.document.cursor.position;
                    editor.set_cursor_and_anchor(mark, point).map_err(err)?;
                    s.mark = Some(point);
                    s.active_mark = true;
                    Ui::None
                }
                _ => return Err("Unsupported Emacs C-x sequence; Ctrl+G cancels".into()),
            };
            return Ok(Outcome::handled(ui));
        }
        if ctrl && key == Keycode::X {
            s.prefix = true;
            return Ok(Outcome::handled(Ui::None));
        }
        if ctrl && key == Keycode::U {
            s.count = Some(
                s.count
                    .unwrap_or(1)
                    .saturating_mul(4)
                    .clamp(-MAX_COUNT, MAX_COUNT),
            );
            s.digits = false;
            return Ok(Outcome::handled(Ui::None));
        }
        if meta || s.count.is_some() {
            if key == Keycode::Minus && !ctrl {
                s.count = Some(-s.count.unwrap_or(1));
                return Ok(Outcome::handled(Ui::None));
            }
            if let Some(digit) = digit(key) {
                let old = s.count.unwrap_or(1);
                let value = if s.digits {
                    old.abs().saturating_mul(10) + digit
                } else {
                    digit
                };
                s.count = Some(value.min(MAX_COUNT) * if old < 0 { -1 } else { 1 });
                s.digits = true;
                self.suppress_text = true;
                return Ok(Outcome::handled(Ui::None));
            }
        }
        // Ordinary text is delivered by SDL TextInput; retain its numeric argument.
        let editing_key = matches!(
            key,
            Keycode::Left
                | Keycode::Right
                | Keycode::Up
                | Keycode::Down
                | Keycode::Home
                | Keycode::End
                | Keycode::PageUp
                | Keycode::PageDown
                | Keycode::Backspace
                | Keycode::Delete
                | Keycode::Return
                | Keycode::KpEnter
                | Keycode::Tab
        );
        if !ctrl && !meta && !editing_key {
            s.last_kill = false;
            s.yank = None;
            return Ok(Outcome {
                consumed: false,
                changed: false,
                ui: Ui::None,
            });
        }
        editor.clear_secondary_cursors();
        let explicit = s.count.is_some();
        let count = s.take_count();
        let n = count.unsigned_abs() as usize;
        let reverse = count < 0;
        let was_kill = s.last_kill;
        s.last_kill = false;
        let old_yank = s.yank.take();
        if meta && key == Keycode::X {
            return Ok(Outcome::handled(Ui::Command(":", false)));
        }
        if ctrl && matches!(key, Keycode::S | Keycode::R) {
            return Ok(Outcome::handled(Ui::Search(key == Keycode::R)));
        }
        if ctrl && key == Keycode::H {
            return Ok(Outcome::handled(Ui::Help));
        }
        if ctrl
            && (matches!(key, Keycode::Slash | Keycode::Underscore)
                || shift && key == Keycode::Minus)
        {
            return Ok(Outcome::handled(Ui::History(
                shift && key == Keycode::Slash,
                n,
            )));
        }
        if ctrl && key == Keycode::Space {
            s.mark = Some(editor.document.cursor.position);
            s.active_mark = true;
            editor
                .document
                .move_cursor(editor.document.cursor.position)
                .map_err(err)?;
            return Ok(Outcome::handled(Ui::None));
        }
        let motion = match (ctrl, meta, key) {
            (true, false, Keycode::F) | (false, false, Keycode::Right) => Some(Motion::Char(true)),
            (true, false, Keycode::B) | (false, false, Keycode::Left) => Some(Motion::Char(false)),
            (true, false, Keycode::N) | (false, false, Keycode::Down) => Some(Motion::Line(true)),
            (true, false, Keycode::P) | (false, false, Keycode::Up) => Some(Motion::Line(false)),
            (true, false, Keycode::A) | (false, false, Keycode::Home) => Some(Motion::Edge(false)),
            (true, false, Keycode::E) | (false, false, Keycode::End) => Some(Motion::Edge(true)),
            (false, true, Keycode::F) => Some(Motion::Word(true)),
            (false, true, Keycode::B) => Some(Motion::Word(false)),
            (true, false, Keycode::V) | (false, false, Keycode::PageDown) => {
                Some(Motion::Page(true))
            }
            (false, true, Keycode::V) | (false, false, Keycode::PageUp) => {
                Some(Motion::Page(false))
            }
            (false, true, Keycode::Less) => Some(Motion::Buffer(false)),
            (false, true, Keycode::Greater) => Some(Motion::Buffer(true)),
            (false, true, Keycode::Comma) if shift => Some(Motion::Buffer(false)),
            (false, true, Keycode::Period) if shift => Some(Motion::Buffer(true)),
            _ => None,
        };
        if let Some(motion) = motion {
            let selecting = s.active_mark || editor.document.has_selection() || shift && !meta;
            if selecting && !s.active_mark {
                s.mark = Some(editor.document.cursor.anchor);
            }
            for _ in 0..n {
                move_point(editor, motion, reverse, selecting, page).map_err(err)?;
            }
            s.active_mark = selecting;
            return Ok(Outcome::handled(Ui::None));
        }
        let copy = meta && key == Keycode::W;
        let kill_region = ctrl && key == Keycode::W;
        let kill_line = ctrl && key == Keycode::K;
        let kill_word = meta && matches!(key, Keycode::D | Keycode::Backspace | Keycode::Delete);
        if copy || kill_region || kill_line || kill_word {
            if !copy && editor.read_only {
                return Err("This file is open for viewing".into());
            }
            let point = editor.document.cursor.position;
            let (start, end, backward) = if copy || kill_region {
                if !editor.document.has_selection() {
                    return Err("No selected region; use Ctrl+Space and move".into());
                }
                (
                    editor.document.selection_start(),
                    editor.document.selection_end(),
                    point < editor.document.cursor.anchor,
                )
            } else if kill_word {
                let backward = (key != Keycode::D) ^ reverse;
                let mut end = point;
                for _ in 0..n {
                    end = word_boundary(&editor.document, end, !backward).map_err(err)?;
                }
                (point.min(end), point.max(end), backward)
            } else {
                let saved = editor.document.cursor.clone();
                let mut end = point;
                if reverse {
                    for _ in 0..n {
                        end = editor.document.current_line_start().map_err(err)?;
                        if end == editor.document.cursor.position && end > 0 {
                            end = editor.document.previous_char_boundary(end).map_err(err)?;
                        }
                        editor.document.move_cursor(end).map_err(err)?;
                    }
                } else {
                    for _ in 0..n {
                        end = editor.document.current_line_end().map_err(err)?;
                        if (explicit || end == point) && end < editor.document.len() {
                            end = editor.document.next_char_boundary(end).map_err(err)?;
                        }
                        editor.document.move_cursor(end).map_err(err)?;
                    }
                }
                editor.document.cursor = saved;
                (point.min(end), point.max(end), reverse)
            };
            if start == end {
                return Ok(Outcome::handled(Ui::None));
            }
            let text = String::from_utf8(
                editor
                    .document
                    .read_range(start, end - start)
                    .map_err(err)?,
            )
            .map_err(err)?;
            if text.contains('\0') {
                return Err("Selection contains NUL and cannot be copied as text".into());
            }
            let combined = if !copy
                && was_kill
                && self
                    .ring
                    .front()
                    .is_some_and(|v| v.len() + text.len() <= RING_BYTES)
            {
                let previous = self.ring.front().unwrap();
                Some(if backward {
                    format!("{text}{previous}")
                } else {
                    format!("{previous}{text}")
                })
            } else {
                None
            };
            clipboard.set_text(combined.as_deref().unwrap_or(&text))?;
            if !copy {
                editor.delete_range(start, end - start).map_err(err)?;
            }
            if let Some(combined) = combined {
                self.bytes -= self.ring.pop_front().unwrap().len();
                self.remember(combined);
            } else {
                self.remember(text);
            }
            s.active_mark = false;
            s.mark = Some(editor.document.cursor.position);
            s.last_kill = !copy;
            if copy {
                editor.document.move_cursor(point).map_err(err)?;
            }
            return Ok(Outcome {
                consumed: true,
                changed: !copy,
                ui: Ui::None,
            });
        }
        if ctrl && key == Keycode::Y || meta && key == Keycode::Y {
            if editor.read_only {
                return Err("This file is open for viewing".into());
            }
            let (start, end, index, text) = if meta {
                let (start, end, index, history) =
                    old_yank.ok_or("Alt+Y must immediately follow a yank")?;
                if editor.undo_stack.len() != history || self.ring.is_empty() {
                    return Err("The previous yank is no longer current".into());
                }
                let index = (index + n.max(1)) % self.ring.len();
                (start, end, index, self.ring[index].clone())
            } else {
                let text = read_text(clipboard)?;
                if self.ring.front() != Some(&text) {
                    self.remember(text.clone());
                }
                let start = editor.document.selection_start();
                (start, editor.document.selection_end(), 0, text)
            };
            editor
                .replace_range(SearchResult { start, end }, &text)
                .map_err(err)?;
            s.mark = Some(start);
            s.active_mark = false;
            if self.ring.get(index) == Some(&text) {
                s.yank = Some((start, start + text.len(), index, editor.undo_stack.len()));
            }
            return Ok(Outcome {
                consumed: true,
                changed: true,
                ui: Ui::None,
            });
        }
        let command = match (ctrl, meta, key) {
            (true, false, Keycode::D) | (false, false, Keycode::Delete) => {
                Some(crate::keybindings::Command::Delete)
            }
            (false, false, Keycode::Backspace) => Some(crate::keybindings::Command::Backspace),
            (true, false, Keycode::J | Keycode::M)
            | (false, false, Keycode::Return | Keycode::KpEnter) => {
                Some(crate::keybindings::Command::Newline)
            }
            (false, false, Keycode::Tab) => Some(crate::keybindings::Command::InsertTab),
            _ => None,
        };
        if let Some(command) = command {
            if editor.read_only {
                return Err("This file is open for viewing".into());
            }
            let history = editor.begin_history_group();
            for _ in 0..n {
                editor.execute(command, page).map_err(err)?;
            }
            editor.end_history_group(history);
            s.mark = Some(editor.document.cursor.position);
            s.active_mark = false;
            return Ok(Outcome {
                consumed: true,
                changed: true,
                ui: Ui::None,
            });
        }
        if ctrl && key == Keycode::O {
            if editor.read_only {
                return Err("This file is open for viewing".into());
            }
            let point = editor.document.cursor.position;
            editor.insert_text(&"\n".repeat(n)).map_err(err)?;
            editor.document.move_cursor(point).map_err(err)?;
            s.active_mark = false;
            return Ok(Outcome {
                consumed: true,
                changed: true,
                ui: Ui::None,
            });
        }
        Err("Emacs key sequence is not supported; Alt+X opens commands, Ctrl+G cancels".into())
    }

    fn remember(&mut self, text: String) {
        if text.len() > RING_BYTES {
            self.ring.clear();
            self.bytes = 0;
            return;
        }
        while self.ring.len() >= RING_ENTRIES || self.bytes + text.len() > RING_BYTES {
            if let Some(old) = self.ring.pop_back() {
                self.bytes -= old.len();
            } else {
                break;
            }
        }
        self.bytes += text.len();
        self.ring.push_front(text);
    }
}
#[derive(Clone, Copy)]
enum Motion {
    Char(bool),
    Word(bool),
    Line(bool),
    Edge(bool),
    Page(bool),
    Buffer(bool),
}
fn move_point(
    e: &mut Editor,
    motion: Motion,
    reverse: bool,
    selecting: bool,
    page: usize,
) -> io::Result<()> {
    let anchor = if selecting {
        e.document.cursor.anchor
    } else {
        e.document.cursor.position
    };
    let point = e.document.cursor.position;
    match motion {
        Motion::Line(forward) => {
            e.document.move_page(forward ^ reverse, 1, selecting)?;
            return Ok(());
        }
        Motion::Page(forward) => {
            e.document.move_page(forward ^ reverse, page, selecting)?;
            return Ok(());
        }
        _ => (),
    }
    let target = match motion {
        Motion::Char(forward) => {
            if forward ^ reverse {
                e.document.next_char_boundary(point)?
            } else {
                e.document.previous_char_boundary(point)?
            }
        }
        Motion::Word(forward) => word_boundary(&e.document, point, forward ^ reverse)?,
        Motion::Edge(end) => {
            if end ^ reverse {
                e.document.current_line_end()?
            } else {
                e.document.current_line_start()?
            }
        }
        Motion::Buffer(end) => {
            if end ^ reverse {
                e.document.len()
            } else {
                0
            }
        }
        _ => unreachable!(),
    };
    e.set_cursor_and_anchor(target, if selecting { anchor } else { target })
}
fn word_boundary(
    table: &crate::piece_table::PieceTable,
    mut point: usize,
    forward: bool,
) -> io::Result<usize> {
    let word = |p| -> io::Result<bool> {
        let end = table.next_char_boundary(p)?;
        let bytes = table.read_range(p, end - p)?;
        let c = std::str::from_utf8(&bytes)
            .map_err(io::Error::other)?
            .chars()
            .next()
            .unwrap();
        Ok(c.is_alphanumeric() || c == '_')
    };
    if forward {
        while point < table.len() && !word(point)? {
            point = table.next_char_boundary(point)?;
        }
        while point < table.len() && word(point)? {
            point = table.next_char_boundary(point)?;
        }
    } else {
        while point > 0 {
            let prev = table.previous_char_boundary(point)?;
            if word(prev)? {
                break;
            }
            point = prev;
        }
        while point > 0 {
            let prev = table.previous_char_boundary(point)?;
            if !word(prev)? {
                break;
            }
            point = prev;
        }
    }
    Ok(point)
}
fn digit(key: Keycode) -> Option<i32> {
    [
        Keycode::_0,
        Keycode::_1,
        Keycode::_2,
        Keycode::_3,
        Keycode::_4,
        Keycode::_5,
        Keycode::_6,
        Keycode::_7,
        Keycode::_8,
        Keycode::_9,
    ]
    .iter()
    .position(|k| *k == key)
    .map(|v| v as i32)
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
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
    fn editor(text: &str) -> Editor {
        let mut e = Editor::new(crate::config::EditorConfig::default()).unwrap();
        e.config.keybinding_mode = crate::config::KeybindingMode::Emacs;
        e.insert_text(text).unwrap();
        e.document.move_cursor(0).unwrap();
        e.undo_stack.clear();
        e.redo_stack.clear();
        e.dirty = false;
        e
    }
    fn key(c: &mut Controller, e: &mut Editor, b: &Clipboard, k: Keycode, m: Mod) -> Outcome {
        c.begin_key();
        c.key(e, b, k, m, 3).unwrap()
    }
    const C: Mod = Mod::LCTRLMOD;
    const M: Mod = Mod::LALTMOD;
    const NONE: Mod = Mod::NOMOD;

    #[test]
    fn emacs_unicode_words_marks_and_exchange() {
        let (mut c, mut e, b) = (
            Controller::default(),
            editor("éclair 東京\nlast line"),
            Clipboard::default(),
        );
        key(&mut c, &mut e, &b, Keycode::F, C);
        assert_eq!(e.document.cursor.position, "é".len());
        key(&mut c, &mut e, &b, Keycode::A, C);
        key(&mut c, &mut e, &b, Keycode::Space, C);
        key(&mut c, &mut e, &b, Keycode::F, M);
        assert_eq!(e.document.selection_end(), "éclair".len());
        key(&mut c, &mut e, &b, Keycode::W, M);
        assert_eq!(b.text().unwrap(), "éclair");
        assert!(!e.document.has_selection());
        key(&mut c, &mut e, &b, Keycode::F, M);
        assert_eq!(e.document.cursor.position, "éclair 東京".len());
        key(&mut c, &mut e, &b, Keycode::X, C);
        key(&mut c, &mut e, &b, Keycode::X, C);
        assert!(e.document.has_selection());
        key(&mut c, &mut e, &b, Keycode::G, C);
        assert!(!e.document.has_selection());
        assert_eq!(
            c.key(&mut e, &b, Keycode::S, C, 3).unwrap().ui,
            Ui::Search(false)
        );
    }

    #[test]
    fn emacs_prefixes_counts_and_text_suppression() {
        let (mut c, mut e, b) = (
            Controller::default(),
            editor("abcdefghij"),
            Clipboard::default(),
        );
        key(&mut c, &mut e, &b, Keycode::U, C);
        key(&mut c, &mut e, &b, Keycode::F, C);
        assert_eq!(e.document.cursor.position, 4);
        key(&mut c, &mut e, &b, Keycode::Minus, M);
        key(&mut c, &mut e, &b, Keycode::_2, NONE);
        key(&mut c, &mut e, &b, Keycode::F, C);
        assert_eq!(e.document.cursor.position, 2);
        key(&mut c, &mut e, &b, Keycode::Escape, NONE);
        key(&mut c, &mut e, &b, Keycode::LShift, Mod::LSHIFTMOD);
        assert_eq!(
            key(&mut c, &mut e, &b, Keycode::X, NONE).ui,
            Ui::Command(":", false)
        );
        assert!(c.consume_text());
        key(&mut c, &mut e, &b, Keycode::X, C);
        assert_eq!(
            key(&mut c, &mut e, &b, Keycode::S, C).ui,
            Ui::Command(":save", true)
        );
        key(&mut c, &mut e, &b, Keycode::U, C);
        key(&mut c, &mut e, &b, Keycode::Z, NONE);
        assert!(!c.consume_text());
        c.insert(&mut e, "z").unwrap();
        assert_eq!(e.document.text().unwrap(), "abzzzzcdefghij");
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "abcdefghij");
        key(&mut c, &mut e, &b, Keycode::X, C);
        key(&mut c, &mut e, &b, Keycode::G, C);
        assert!(!e.emacs.prefix);
    }

    #[test]
    fn emacs_kills_coalesce_yanks_cycle_and_undo() {
        let (mut c, mut e, b) = (
            Controller::default(),
            editor("first\nsecond\nthird"),
            Clipboard::default(),
        );
        key(&mut c, &mut e, &b, Keycode::K, C);
        assert_eq!(b.text().unwrap(), "first");
        key(&mut c, &mut e, &b, Keycode::K, C);
        assert_eq!(b.text().unwrap(), "first\n");
        assert_eq!(e.document.text().unwrap(), "second\nthird");
        key(&mut c, &mut e, &b, Keycode::F, C);
        key(&mut c, &mut e, &b, Keycode::A, C);
        key(&mut c, &mut e, &b, Keycode::K, C);
        assert_eq!(c.ring.len(), 2);
        key(&mut c, &mut e, &b, Keycode::Y, C);
        assert_eq!(e.document.text().unwrap(), "second\nthird");
        key(&mut c, &mut e, &b, Keycode::Y, M);
        assert_eq!(e.document.text().unwrap(), "first\n\nthird");
        e.undo().unwrap();
        assert_eq!(e.document.text().unwrap(), "second\nthird");
        // A changed document invalidates yank-pop even if the last key was Alt+Y.
        assert!(c.key(&mut e, &b, Keycode::Y, M, 3).is_err());
    }

    #[test]
    fn emacs_ring_is_shared_bounded_and_readonly_kills_do_nothing() {
        let (mut c, mut e, b) = (
            Controller::default(),
            editor("word next"),
            Clipboard::default(),
        );
        key(&mut c, &mut e, &b, Keycode::D, M);
        let mut other = editor("");
        key(&mut c, &mut other, &b, Keycode::Y, C);
        assert_eq!(other.document.text().unwrap(), "word");
        e.read_only = true;
        let before = e.document.text().unwrap();
        let clip = b.text().unwrap();
        assert!(c.key(&mut e, &b, Keycode::K, C, 3).is_err());
        assert_eq!(e.document.text().unwrap(), before);
        assert_eq!(b.text().unwrap(), clip);
        for i in 0..100 {
            c.remember(format!("{i}{}", "x".repeat(20_000)));
        }
        assert!(c.bytes <= RING_BYTES && c.ring.len() <= RING_ENTRIES);
        c.remember("x".repeat(RING_BYTES + 1));
        assert_eq!(c.bytes, 0);
        assert!(c.ring.is_empty());
    }

    #[test]
    fn emacs_line_page_and_whole_document_selection() {
        let (mut c, mut e, b) = (
            Controller::default(),
            editor("one\ntwo\nthree\nfour\nfive"),
            Clipboard::default(),
        );
        key(&mut c, &mut e, &b, Keycode::V, C);
        assert_eq!(e.document.cursor.line, 3);
        key(&mut c, &mut e, &b, Keycode::V, M);
        assert_eq!(e.document.cursor.line, 0);
        key(&mut c, &mut e, &b, Keycode::X, C);
        key(&mut c, &mut e, &b, Keycode::H, NONE);
        assert_eq!(e.document.selection_end(), e.document.len());
        key(&mut c, &mut e, &b, Keycode::W, C);
        assert_eq!(e.document.len(), 0);
        e.undo().unwrap();
        assert!(e.document.text().unwrap().ends_with("five"));
    }
}

pub(crate) const HELP: &str = "Emacs keys · C = Ctrl; M = Alt (or Escape, then key)\n\
C-f/b/n/p: move; M-f/b: words; C-a/e: line edges; M-</>: document edges\n\
C-v / M-v: page down/up; C-Space: set mark; C-x C-x: exchange point and mark\n\
C-w: kill region; M-w: copy region; C-k: kill line; M-d / M-Backspace: kill word\n\
C-y: yank; M-y: cycle kills; C-/ or C-_: undo; C-Shift-/: redo\n\
C-u: argument ×4; M-digits: repeat count (up to 1000); C-g: cancel\n\
C-s/r: search forward/backward; Enter accepts search; C-g restores its start\n\
C-x C-f: open; C-x C-s: save; C-x C-w: Save As; C-x C-c: quit if saved\n\
C-x 2/3: show the two panes; C-x o: switch pane; C-x 1: show only this pane\n\
C-x h: select document; C-x u: undo; C-o: open line; M-x: Potyi commands\n\
Backspace/C-d: delete backward/forward; C-j or C-m: newline; C-h: this help\n\
Kill ring: at most 32 entries / 256 KiB. Larger kills use the system clipboard.";

#[cfg(test)]
mod edge_tests {
    use super::*;
    struct Clipboard;
    impl TextClipboard for Clipboard {
        fn set_text(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn text(&self) -> Result<String, String> {
            Ok(String::new())
        }
    }
    #[test]
    fn emacs_empty_edges_prefix_actions_and_modifier_keys() {
        let mut e = Editor::new(crate::config::EditorConfig::default()).unwrap();
        let mut c = Controller::default();
        let b = Clipboard;
        for key in [
            Keycode::F,
            Keycode::B,
            Keycode::N,
            Keycode::P,
            Keycode::A,
            Keycode::E,
            Keycode::K,
        ] {
            c.key(&mut e, &b, key, Mod::LCTRLMOD, 10).unwrap();
            assert_eq!(e.document.cursor.position, 0);
        }
        for (key, mods, expected) in [
            (Keycode::F, Mod::LCTRLMOD, Ui::Command(":open ", false)),
            (Keycode::W, Mod::LCTRLMOD, Ui::Command(":save-as ", false)),
            (Keycode::_2, Mod::NOMOD, Ui::Split),
            (Keycode::O, Mod::NOMOD, Ui::OtherPane),
            (Keycode::_1, Mod::NOMOD, Ui::OnlyPane),
            (Keycode::C, Mod::LCTRLMOD, Ui::Command(":quit", true)),
        ] {
            c.key(&mut e, &b, Keycode::X, Mod::LCTRLMOD, 10).unwrap();
            c.key(&mut e, &b, Keycode::LCtrl, Mod::LCTRLMOD, 10)
                .unwrap();
            assert_eq!(c.key(&mut e, &b, key, mods, 10).unwrap().ui, expected);
        }
        c.key(&mut e, &b, Keycode::Escape, Mod::NOMOD, 10).unwrap();
        c.key(&mut e, &b, Keycode::X, Mod::LALTMOD, 10).unwrap();
        assert!(!e.emacs.meta);
        assert_eq!(
            c.key(&mut e, &b, Keycode::H, Mod::LCTRLMOD, 10).unwrap().ui,
            Ui::Help
        );
    }
}
