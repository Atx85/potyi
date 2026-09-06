// Pötyi - Lightweight text editor
// Copyright (C) 2026  Attila Banko

use sdl3::keyboard::{Keycode, Mod};

use crate::Editor;
use crate::clipboard::TextClipboard;
use crate::piece_table::PieceTable;
use crate::search::SearchResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VimMode {
    Normal,
    Insert,
    Visual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operator {
    Delete,
    Change,
    Yank,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Motion {
    Left,
    Right,
    Up,
    Down,
    WordForward,
    WordBackward,
    WordEnd,
    LineStart,
    FirstNonBlank,
    LineEnd,
    DocumentStart,
    DocumentEnd,
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InsertPlacement {
    Before,
    After,
    FirstNonBlank,
    LineEnd,
    OpenAbove,
    OpenBelow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EditCommand {
    Delete {
        motion: Motion,
        count: usize,
    },
    Change {
        motion: Motion,
        count: usize,
        text: String,
    },
    DeleteChars {
        count: usize,
    },
    Insert {
        placement: InsertPlacement,
        text: String,
    },
    Paste {
        before: bool,
        linewise: bool,
        text: String,
        count: usize,
    },
}

struct InsertSession {
    history_start: usize,
    placement: InsertPlacement,
    change: Option<(Motion, usize)>,
    text: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum VimUiAction {
    None,
    OpenCommandBar,
    OpenSearch { backward: bool },
    RepeatSearch { backward: bool },
    SearchWord { query: String, backward: bool },
}

pub(crate) struct VimOutcome {
    pub consumed: bool,
    pub document_changed: bool,
    pub cursor_changed: bool,
    pub ui_action: VimUiAction,
}

impl VimOutcome {
    fn ignored() -> Self {
        Self {
            consumed: false,
            document_changed: false,
            cursor_changed: false,
            ui_action: VimUiAction::None,
        }
    }

    fn consumed(cursor_changed: bool) -> Self {
        Self {
            consumed: true,
            document_changed: false,
            cursor_changed,
            ui_action: VimUiAction::None,
        }
    }
}

pub(crate) struct VimController {
    mode: VimMode,
    count: usize,
    pending_operator: Option<(Operator, usize)>,
    pending_g: bool,
    insert_session: Option<InsertSession>,
    last_change: Option<EditCommand>,
    last_search_backward: bool,
    search_origin: Option<usize>,
    register_linewise: bool,
    suppress_text_input: bool,
}

impl VimController {
    pub(crate) fn new() -> Self {
        Self {
            mode: VimMode::Normal,
            count: 0,
            pending_operator: None,
            pending_g: false,
            insert_session: None,
            last_change: None,
            last_search_backward: false,
            search_origin: None,
            register_linewise: false,
            suppress_text_input: false,
        }
    }

    pub(crate) fn mode(&self) -> VimMode {
        self.mode
    }

    pub(crate) fn mode_label(&self) -> &'static str {
        match self.mode {
            VimMode::Normal => "-- NORMAL --",
            VimMode::Insert => "-- INSERT --",
            VimMode::Visual => "-- VISUAL --",
        }
    }

    pub(crate) fn cancel_pending(&mut self) {
        self.count = 0;
        self.pending_operator = None;
        self.pending_g = false;
    }

    pub(crate) fn handle_document_click(&mut self) {
        self.cancel_pending();
        if self.mode == VimMode::Visual {
            self.mode = VimMode::Normal;
        }
    }

    pub(crate) fn settle_cursor(&self, editor: &mut Editor) -> Result<(), String> {
        if self.mode == VimMode::Normal {
            settle_normal_cursor(editor)?;
        }
        Ok(())
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::new();
    }

    pub(crate) fn deactivate(&mut self, editor: &mut Editor) {
        self.finish_insert(editor);
        self.reset();
    }

    pub(crate) fn consume_suppressed_text_input(&mut self) -> bool {
        std::mem::take(&mut self.suppress_text_input)
    }

    pub(crate) fn record_text(&mut self, text: &str) {
        if let Some(session) = self.insert_session.as_mut() {
            session.text.push_str(text);
        }
    }

    pub(crate) fn accept_search(&mut self) {
        self.search_origin = None;
    }

    pub(crate) fn search_origin(&self) -> Option<usize> {
        self.search_origin
    }

    pub(crate) fn cancel_search(&mut self, editor: &mut Editor) -> Result<(), String> {
        if let Some(origin) = self.search_origin.take() {
            editor
                .document
                .move_cursor(origin)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub(crate) fn handle_key<C: TextClipboard>(
        &mut self,
        editor: &mut Editor,
        clipboard: &C,
        key: Keycode,
        keymod: Mod,
        repeat: bool,
    ) -> Result<VimOutcome, String> {
        if self.mode == VimMode::Insert {
            return self.handle_insert_key(editor, key, keymod, repeat);
        }

        if has_command_modifier(keymod) {
            if self.mode == VimMode::Normal && ctrl_pressed(keymod) && key == Keycode::R && !repeat
            {
                editor.redo().map_err(|error| error.to_string())?;
                return Ok(VimOutcome::consumed(true));
            }

            return Ok(VimOutcome::ignored());
        }

        if key == Keycode::Escape {
            self.cancel_pending();

            if self.mode == VimMode::Visual {
                let position = editor.document.cursor.position;
                editor
                    .document
                    .move_cursor(position)
                    .map_err(|error| error.to_string())?;
                self.mode = VimMode::Normal;
                return Ok(VimOutcome::consumed(true));
            }

            self.mode = VimMode::Normal;
            return Ok(VimOutcome::consumed(false));
        }

        if !shift_pressed(keymod)
            && let Some(digit) = digit_for_key(key)
            && (digit != 0 || self.count > 0)
        {
            self.count = self.count.saturating_mul(10).saturating_add(digit);
            return Ok(VimOutcome::consumed(false));
        }

        if self.mode == VimMode::Visual {
            return self.handle_visual_key(editor, clipboard, key, keymod);
        }

        self.handle_normal_key(editor, clipboard, key, keymod)
    }

    fn handle_insert_key(
        &mut self,
        editor: &mut Editor,
        key: Keycode,
        keymod: Mod,
        repeat: bool,
    ) -> Result<VimOutcome, String> {
        if key == Keycode::Escape && !repeat {
            self.finish_insert(editor);
            self.mode = VimMode::Normal;
            return Ok(VimOutcome::consumed(true));
        }

        if has_command_modifier(keymod) {
            return Ok(VimOutcome::ignored());
        }

        match key {
            Keycode::Return | Keycode::KpEnter if !repeat => {
                editor.newline().map_err(|error| error.to_string())?;
                if let Some(session) = self.insert_session.as_mut() {
                    session.text.push('\n');
                }
                Ok(changed_outcome(true))
            }

            Keycode::Tab if !repeat => {
                editor.insert_tab().map_err(|error| error.to_string())?;
                if let Some(session) = self.insert_session.as_mut() {
                    if editor.config.insert_spaces {
                        session.text.push_str(&" ".repeat(editor.config.tab_width));
                    } else {
                        session.text.push('\t');
                    }
                }
                Ok(changed_outcome(true))
            }

            Keycode::Backspace => {
                editor.backspace().map_err(|error| error.to_string())?;

                if let Some(session) = self.insert_session.as_mut() {
                    session.text.pop();
                }

                Ok(VimOutcome {
                    consumed: true,
                    document_changed: true,
                    cursor_changed: true,
                    ui_action: VimUiAction::None,
                })
            }

            Keycode::Delete => {
                editor.delete().map_err(|error| error.to_string())?;
                Ok(VimOutcome {
                    consumed: true,
                    document_changed: true,
                    cursor_changed: true,
                    ui_action: VimUiAction::None,
                })
            }

            _ => Ok(VimOutcome::ignored()),
        }
    }

    fn handle_normal_key<C: TextClipboard>(
        &mut self,
        editor: &mut Editor,
        clipboard: &C,
        key: Keycode,
        keymod: Mod,
    ) -> Result<VimOutcome, String> {
        if self.pending_g {
            self.pending_g = false;

            if key == Keycode::G {
                if let Some((operator, operator_count)) = self.pending_operator.take() {
                    let count = operator_count.saturating_mul(self.take_count());
                    return self.apply_operator(
                        editor,
                        clipboard,
                        operator,
                        Motion::DocumentStart,
                        count,
                    );
                }
                return self.apply_motion(editor, Motion::DocumentStart);
            }

            self.cancel_pending();
            return Ok(VimOutcome::consumed(false));
        }

        if let Some((operator, operator_count)) = self.pending_operator {
            if operator_key(key, keymod) == Some(operator) {
                self.pending_operator = None;
                let count = operator_count.saturating_mul(self.take_count());
                return self.apply_operator(editor, clipboard, operator, Motion::Line, count);
            }

            if key == Keycode::G && !shift_pressed(keymod) {
                self.pending_g = true;
                return Ok(VimOutcome::consumed(false));
            }

            if let Some(motion) = motion_for_key(key, keymod) {
                self.pending_operator = None;
                let count = operator_count.saturating_mul(self.take_count());
                return self.apply_operator(editor, clipboard, operator, motion, count);
            }

            self.cancel_pending();
            return Ok(VimOutcome::consumed(false));
        }

        if shift_pressed(keymod) {
            let shortcut = match key {
                Keycode::D => Some((Operator::Delete, Motion::LineEnd)),
                Keycode::C => Some((Operator::Change, Motion::LineEnd)),
                Keycode::Y => Some((Operator::Yank, Motion::Line)),
                Keycode::S => Some((Operator::Change, Motion::Line)),
                Keycode::X => Some((Operator::Delete, Motion::Left)),
                _ => None,
            };

            if let Some((operator, motion)) = shortcut {
                let count = self.take_count();
                return self.apply_operator(editor, clipboard, operator, motion, count);
            }
        }

        if key == Keycode::S {
            let count = self.take_count();
            return self.apply_operator(editor, clipboard, Operator::Change, Motion::Right, count);
        }

        if let Some(operator) = operator_key(key, keymod) {
            let count = self.take_count();
            self.pending_operator = Some((operator, count));
            self.suppress_text_input = true;
            return Ok(VimOutcome::consumed(false));
        }

        if key == Keycode::G && !shift_pressed(keymod) {
            self.pending_g = true;
            self.suppress_text_input = true;
            return Ok(VimOutcome::consumed(false));
        }

        if let Some(motion) = motion_for_key(key, keymod) {
            return self.apply_motion(editor, motion);
        }

        let count = self.take_count();

        match key {
            Keycode::I => {
                let placement = if shift_pressed(keymod) {
                    InsertPlacement::FirstNonBlank
                } else {
                    InsertPlacement::Before
                };
                self.begin_insert(editor, placement, None)?;
                self.suppress_text_input = true;
                Ok(VimOutcome::consumed(true))
            }

            Keycode::A => {
                let placement = if shift_pressed(keymod) {
                    InsertPlacement::LineEnd
                } else {
                    InsertPlacement::After
                };
                self.begin_insert(editor, placement, None)?;
                self.suppress_text_input = true;
                Ok(VimOutcome::consumed(true))
            }

            Keycode::O => {
                let placement = if shift_pressed(keymod) {
                    InsertPlacement::OpenAbove
                } else {
                    InsertPlacement::OpenBelow
                };
                self.begin_insert(editor, placement, None)?;
                self.suppress_text_input = true;
                Ok(VimOutcome {
                    consumed: true,
                    document_changed: true,
                    cursor_changed: true,
                    ui_action: VimUiAction::None,
                })
            }

            Keycode::V => {
                self.enter_visual(editor)?;
                self.suppress_text_input = true;
                Ok(VimOutcome::consumed(true))
            }

            Keycode::X => {
                let start = editor.document.cursor.position;
                let mut end = start;
                for _ in 0..count {
                    end = editor
                        .document
                        .next_char_boundary(end)
                        .map_err(|error| error.to_string())?;
                }
                if end > start {
                    copy_range(clipboard, &editor.document, start, end)?;
                    self.register_linewise = false;
                }

                let command = EditCommand::DeleteChars { count };
                let changed = self.execute_edit(editor, &command)?;
                if changed {
                    self.last_change = Some(command);
                }
                self.suppress_text_input = true;
                Ok(changed_outcome(changed))
            }

            Keycode::U => {
                editor.undo().map_err(|error| error.to_string())?;
                self.suppress_text_input = true;
                Ok(VimOutcome::consumed(true))
            }

            Keycode::P => {
                let before = shift_pressed(keymod);
                let text = clipboard.text()?;
                if text.is_empty() {
                    return Ok(VimOutcome::consumed(false));
                }

                let command = EditCommand::Paste {
                    before,
                    linewise: self.register_linewise,
                    text,
                    count,
                };
                let changed = self.execute_edit(editor, &command)?;
                if changed {
                    self.last_change = Some(command);
                }
                self.suppress_text_input = true;
                Ok(changed_outcome(changed))
            }

            Keycode::Period => {
                let Some(command) = self.last_change.clone() else {
                    return Ok(VimOutcome::consumed(false));
                };

                let command = command.with_count(count);
                let changed = self.execute_edit(editor, &command)?;
                Ok(changed_outcome(changed))
            }

            Keycode::Slash => {
                self.last_search_backward = shift_pressed(keymod);
                self.search_origin = Some(editor.document.cursor.position);
                self.suppress_text_input = true;
                Ok(VimOutcome {
                    consumed: true,
                    document_changed: false,
                    cursor_changed: false,
                    ui_action: VimUiAction::OpenSearch {
                        backward: self.last_search_backward,
                    },
                })
            }

            Keycode::Question => {
                self.last_search_backward = true;
                self.search_origin = Some(editor.document.cursor.position);
                self.suppress_text_input = true;
                Ok(VimOutcome {
                    consumed: true,
                    document_changed: false,
                    cursor_changed: false,
                    ui_action: VimUiAction::OpenSearch { backward: true },
                })
            }

            Keycode::N => {
                let backward = if shift_pressed(keymod) {
                    !self.last_search_backward
                } else {
                    self.last_search_backward
                };
                self.suppress_text_input = true;
                Ok(VimOutcome {
                    consumed: true,
                    document_changed: false,
                    cursor_changed: true,
                    ui_action: VimUiAction::RepeatSearch { backward },
                })
            }

            Keycode::Asterisk | Keycode::_8 if shift_pressed(keymod) => {
                let query = word_under_cursor(&editor.document)?;
                self.last_search_backward = false;
                self.suppress_text_input = true;
                Ok(VimOutcome {
                    consumed: true,
                    document_changed: false,
                    cursor_changed: !query.is_empty(),
                    ui_action: VimUiAction::SearchWord {
                        query,
                        backward: false,
                    },
                })
            }

            Keycode::Hash | Keycode::_3 if shift_pressed(keymod) => {
                let query = word_under_cursor(&editor.document)?;
                self.last_search_backward = true;
                self.suppress_text_input = true;
                Ok(VimOutcome {
                    consumed: true,
                    document_changed: false,
                    cursor_changed: !query.is_empty(),
                    ui_action: VimUiAction::SearchWord {
                        query,
                        backward: true,
                    },
                })
            }

            Keycode::Colon | Keycode::Semicolon if shift_pressed(keymod) => {
                self.suppress_text_input = true;
                Ok(VimOutcome {
                    consumed: true,
                    document_changed: false,
                    cursor_changed: false,
                    ui_action: VimUiAction::OpenCommandBar,
                })
            }

            _ => Ok(VimOutcome::ignored()),
        }
    }

    fn handle_visual_key<C: TextClipboard>(
        &mut self,
        editor: &mut Editor,
        clipboard: &C,
        key: Keycode,
        keymod: Mod,
    ) -> Result<VimOutcome, String> {
        if key == Keycode::V {
            let position = editor.document.cursor.position;
            editor
                .document
                .move_cursor(position)
                .map_err(|error| error.to_string())?;
            self.mode = VimMode::Normal;
            self.suppress_text_input = true;
            return Ok(VimOutcome::consumed(true));
        }

        if let Some(motion) = motion_for_key(key, keymod) {
            return self.apply_motion(editor, motion);
        }

        let operator =
            operator_key(key, keymod).or_else(|| (key == Keycode::X).then_some(Operator::Delete));

        let Some(operator) = operator else {
            return Ok(VimOutcome::ignored());
        };

        let start = editor.document.selection_start();
        let end = editor.document.selection_end();
        let selected_characters = editor
            .document
            .read_range(start, end.saturating_sub(start))
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .map(|text| text.chars().count())
            .unwrap_or_else(|| end.saturating_sub(start));
        let copied = copy_range(clipboard, &editor.document, start, end)?;
        self.register_linewise = false;

        match operator {
            Operator::Yank => {
                editor
                    .document
                    .move_cursor(start)
                    .map_err(|error| error.to_string())?;
                self.mode = VimMode::Normal;
                self.suppress_text_input = true;
                Ok(VimOutcome::consumed(true))
            }

            Operator::Delete | Operator::Change => {
                if editor.read_only {
                    self.mode = VimMode::Normal;
                    self.suppress_text_input = true;
                    return Ok(VimOutcome::consumed(true));
                }

                let history_start = editor.begin_history_group();
                editor
                    .delete_range(start, end.saturating_sub(start))
                    .map_err(|error| error.to_string())?;

                if operator == Operator::Change {
                    self.mode = VimMode::Insert;
                    self.insert_session = Some(InsertSession {
                        history_start,
                        placement: InsertPlacement::Before,
                        change: None,
                        text: String::new(),
                    });
                } else {
                    editor.end_history_group(history_start);
                    self.mode = VimMode::Normal;
                    settle_normal_cursor(editor)?;
                    self.last_change = Some(EditCommand::DeleteChars {
                        count: selected_characters.max(1),
                    });
                }

                self.suppress_text_input = true;
                Ok(VimOutcome {
                    consumed: true,
                    document_changed: copied && end > start,
                    cursor_changed: true,
                    ui_action: VimUiAction::None,
                })
            }
        }
    }

    fn apply_motion(&mut self, editor: &mut Editor, motion: Motion) -> Result<VimOutcome, String> {
        let count = self.take_count();
        let target = motion_target(&mut editor.document, motion, count)?;

        if self.mode == VimMode::Visual {
            let anchor = editor.document.cursor.anchor;
            editor
                .set_cursor_and_anchor(target, anchor)
                .map_err(|error| error.to_string())?;
        } else {
            let target = normal_cursor_target(&mut editor.document, target)?;
            editor
                .document
                .move_cursor(target)
                .map_err(|error| error.to_string())?;
        }

        Ok(VimOutcome::consumed(true))
    }

    fn apply_operator<C: TextClipboard>(
        &mut self,
        editor: &mut Editor,
        clipboard: &C,
        operator: Operator,
        motion: Motion,
        count: usize,
    ) -> Result<VimOutcome, String> {
        // Vim treats `cw` like `ce`: change the word itself, while `dw`
        // also consumes the separating whitespace.
        let motion = if operator == Operator::Change && motion == Motion::WordForward {
            Motion::WordEnd
        } else {
            motion
        };

        let (start, mut end, linewise) = motion_range(&mut editor.document, motion, count)?;

        if start == end {
            return Ok(VimOutcome::consumed(false));
        }

        copy_range(clipboard, &editor.document, start, end)?;
        self.register_linewise = linewise;

        if operator == Operator::Yank {
            editor
                .document
                .move_cursor(start)
                .map_err(|error| error.to_string())?;
            return Ok(VimOutcome::consumed(true));
        }

        if editor.read_only {
            return Ok(VimOutcome::consumed(false));
        }

        if operator == Operator::Change {
            if linewise
                && end > start
                && editor
                    .document
                    .byte_at(end - 1)
                    .map_err(|error| error.to_string())?
                    == Some(b'\n')
            {
                end -= 1;
            }

            let history_start = editor.begin_history_group();
            editor
                .delete_range(start, end - start)
                .map_err(|error| error.to_string())?;
            self.mode = VimMode::Insert;
            self.insert_session = Some(InsertSession {
                history_start,
                placement: InsertPlacement::Before,
                change: Some((motion, count)),
                text: String::new(),
            });
            self.suppress_text_input = true;
            return Ok(changed_outcome(true));
        }

        let command = EditCommand::Delete { motion, count };
        let (start, end) = deletion_range(&mut editor.document, start, end, linewise)?;
        editor
            .delete_range(start, end - start)
            .map_err(|error| error.to_string())?;
        settle_normal_cursor(editor)?;
        self.last_change = Some(command);
        Ok(changed_outcome(true))
    }

    fn begin_insert(
        &mut self,
        editor: &mut Editor,
        placement: InsertPlacement,
        change: Option<(Motion, usize)>,
    ) -> Result<(), String> {
        if editor.read_only {
            return Ok(());
        }

        let history_start = editor.begin_history_group();
        apply_insert_placement(editor, placement)?;
        self.mode = VimMode::Insert;
        self.insert_session = Some(InsertSession {
            history_start,
            placement,
            change,
            text: String::new(),
        });
        Ok(())
    }

    fn finish_insert(&mut self, editor: &mut Editor) {
        let Some(session) = self.insert_session.take() else {
            return;
        };

        editor.end_history_group(session.history_start);
        let _ = settle_normal_cursor(editor);

        if let Some((motion, count)) = session.change {
            self.last_change = Some(EditCommand::Change {
                motion,
                count,
                text: session.text,
            });
        } else if !session.text.is_empty()
            || matches!(
                session.placement,
                InsertPlacement::OpenAbove | InsertPlacement::OpenBelow
            )
        {
            self.last_change = Some(EditCommand::Insert {
                placement: session.placement,
                text: session.text,
            });
        }
    }

    fn enter_visual(&mut self, editor: &mut Editor) -> Result<(), String> {
        self.mode = VimMode::Visual;
        let start = editor.document.cursor.position;
        let end = editor
            .document
            .next_char_boundary(start)
            .map_err(|error| error.to_string())?;
        editor
            .set_cursor_and_anchor(end, start)
            .map_err(|error| error.to_string())
    }

    fn execute_edit(&mut self, editor: &mut Editor, command: &EditCommand) -> Result<bool, String> {
        if editor.read_only {
            return Ok(false);
        }

        match command {
            EditCommand::Delete { motion, count } => {
                let (start, end, linewise) = motion_range(&mut editor.document, *motion, *count)?;
                if start == end {
                    return Ok(false);
                }
                let (start, end) = deletion_range(&mut editor.document, start, end, linewise)?;
                editor
                    .delete_range(start, end - start)
                    .map_err(|error| error.to_string())?;
                settle_normal_cursor(editor)?;
                Ok(true)
            }

            EditCommand::Change {
                motion,
                count,
                text,
            } => {
                let (start, mut end, linewise) =
                    motion_range(&mut editor.document, *motion, *count)?;
                if linewise
                    && end > start
                    && editor
                        .document
                        .byte_at(end - 1)
                        .map_err(|error| error.to_string())?
                        == Some(b'\n')
                {
                    end -= 1;
                }
                if start == end {
                    return Ok(false);
                }
                let group = editor.begin_history_group();
                editor
                    .replace_range(SearchResult { start, end }, text)
                    .map_err(|error| error.to_string())?;
                editor.end_history_group(group);
                settle_normal_cursor(editor)?;
                Ok(true)
            }

            EditCommand::DeleteChars { count } => {
                let start = editor.document.cursor.position;
                let mut end = start;
                for _ in 0..*count {
                    end = editor
                        .document
                        .next_char_boundary(end)
                        .map_err(|error| error.to_string())?;
                }
                if start == end {
                    return Ok(false);
                }
                editor
                    .delete_range(start, end - start)
                    .map_err(|error| error.to_string())?;
                settle_normal_cursor(editor)?;
                Ok(true)
            }

            EditCommand::Insert { placement, text } => {
                let group = editor.begin_history_group();
                apply_insert_placement(editor, *placement)?;
                editor.insert(text).map_err(|error| error.to_string())?;
                editor.end_history_group(group);
                settle_normal_cursor(editor)?;
                Ok(!text.is_empty()
                    || matches!(
                        placement,
                        InsertPlacement::OpenAbove | InsertPlacement::OpenBelow
                    ))
            }

            EditCommand::Paste {
                before,
                linewise,
                text,
                count,
            } => paste_text(editor, text, *before, *linewise, *count),
        }
    }

    fn take_count(&mut self) -> usize {
        let count = self.count.max(1);
        self.count = 0;
        count
    }
}

impl EditCommand {
    fn with_count(mut self, count: usize) -> Self {
        if count <= 1 {
            return self;
        }

        match &mut self {
            EditCommand::Delete { count: stored, .. }
            | EditCommand::Change { count: stored, .. }
            | EditCommand::DeleteChars { count: stored }
            | EditCommand::Paste { count: stored, .. } => *stored = count,
            EditCommand::Insert { .. } => {}
        }
        self
    }
}

fn changed_outcome(changed: bool) -> VimOutcome {
    VimOutcome {
        consumed: true,
        document_changed: changed,
        cursor_changed: changed,
        ui_action: VimUiAction::None,
    }
}

fn ctrl_pressed(keymod: Mod) -> bool {
    keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)
}

fn shift_pressed(keymod: Mod) -> bool {
    keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD)
}

fn has_command_modifier(keymod: Mod) -> bool {
    keymod.intersects(
        Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LALTMOD | Mod::RALTMOD | Mod::LGUIMOD | Mod::RGUIMOD,
    )
}

fn digit_for_key(key: Keycode) -> Option<usize> {
    match key {
        Keycode::_0 => Some(0),
        Keycode::_1 => Some(1),
        Keycode::_2 => Some(2),
        Keycode::_3 => Some(3),
        Keycode::_4 => Some(4),
        Keycode::_5 => Some(5),
        Keycode::_6 => Some(6),
        Keycode::_7 => Some(7),
        Keycode::_8 => Some(8),
        Keycode::_9 => Some(9),
        _ => None,
    }
}

fn operator_key(key: Keycode, keymod: Mod) -> Option<Operator> {
    if shift_pressed(keymod) {
        return None;
    }

    match key {
        Keycode::D => Some(Operator::Delete),
        Keycode::C => Some(Operator::Change),
        Keycode::Y => Some(Operator::Yank),
        _ => None,
    }
}

fn motion_for_key(key: Keycode, keymod: Mod) -> Option<Motion> {
    match key {
        Keycode::H | Keycode::Left => Some(Motion::Left),
        Keycode::L | Keycode::Right => Some(Motion::Right),
        Keycode::K | Keycode::Up => Some(Motion::Up),
        Keycode::J | Keycode::Down => Some(Motion::Down),
        Keycode::W => Some(Motion::WordForward),
        Keycode::B => Some(Motion::WordBackward),
        Keycode::E => Some(Motion::WordEnd),
        Keycode::_0 | Keycode::Home => Some(Motion::LineStart),
        Keycode::Caret | Keycode::_6 if shift_pressed(keymod) => Some(Motion::FirstNonBlank),
        Keycode::Dollar | Keycode::_4 if shift_pressed(keymod) => Some(Motion::LineEnd),
        Keycode::End => Some(Motion::LineEnd),
        Keycode::G if shift_pressed(keymod) => Some(Motion::DocumentEnd),
        _ => None,
    }
}

fn motion_target(table: &mut PieceTable, motion: Motion, count: usize) -> Result<usize, String> {
    let mut position = table.cursor.position;

    match motion {
        Motion::Left => {
            let line_start = table
                .line_start(table.cursor.line)
                .map_err(|error| error.to_string())?;
            for _ in 0..count {
                if position <= line_start {
                    break;
                }
                position = table
                    .previous_char_boundary(position)
                    .map_err(|error| error.to_string())?;
            }
        }
        Motion::Right => {
            let line_end = line_end(table, table.cursor.line)?;
            for _ in 0..count {
                if position >= line_end {
                    break;
                }
                position = table
                    .next_char_boundary(position)
                    .map_err(|error| error.to_string())?;
            }
        }
        Motion::WordForward => {
            for _ in 0..count {
                position = word_forward(table, position)?;
            }
        }
        Motion::WordBackward => {
            for _ in 0..count {
                position = word_backward(table, position)?;
            }
        }
        Motion::WordEnd => {
            for index in 0..count {
                position = word_end(table, position)?;
                if index + 1 < count && position < table.len() {
                    position = table
                        .next_char_boundary(position)
                        .map_err(|error| error.to_string())?;
                }
            }
        }
        Motion::LineStart => {
            position = table
                .line_start(table.cursor.line)
                .map_err(|error| error.to_string())?;
        }
        Motion::FirstNonBlank => {
            position = first_nonblank(table, table.cursor.line)?;
        }
        Motion::LineEnd => {
            position = line_end(table, table.cursor.line)?;
        }
        Motion::Up | Motion::Down => {
            let line_count = table.line_count().map_err(|error| error.to_string())?;
            let line = if motion == Motion::Up {
                table.cursor.line.saturating_sub(count)
            } else {
                table
                    .cursor
                    .line
                    .saturating_add(count)
                    .min(line_count.saturating_sub(1))
            };
            position = position_on_line(table, line, table.cursor.column)?;
        }
        Motion::DocumentStart => {
            position = table
                .line_start(count.saturating_sub(1))
                .map_err(|error| error.to_string())?;
        }
        Motion::DocumentEnd => {
            let lines = table.line_count().map_err(|error| error.to_string())?;
            let line = if count > 1 {
                count.saturating_sub(1).min(lines.saturating_sub(1))
            } else {
                lines.saturating_sub(1)
            };
            position = first_nonblank(table, line)?;
        }
        Motion::Line => {}
    }

    Ok(position)
}

fn motion_range(
    table: &mut PieceTable,
    motion: Motion,
    count: usize,
) -> Result<(usize, usize, bool), String> {
    if motion == Motion::Line {
        return linewise_range(table, table.cursor.line, count);
    }

    if matches!(motion, Motion::Up | Motion::Down) {
        let lines = table.line_count().map_err(|error| error.to_string())?;
        let target_line = if motion == Motion::Up {
            table.cursor.line.saturating_sub(count)
        } else {
            table
                .cursor
                .line
                .saturating_add(count)
                .min(lines.saturating_sub(1))
        };
        let first = table.cursor.line.min(target_line);
        let number = table.cursor.line.max(target_line) - first + 1;
        return linewise_range(table, first, number);
    }

    if matches!(motion, Motion::DocumentStart | Motion::DocumentEnd) {
        let lines = table.line_count().map_err(|error| error.to_string())?;
        let target_line = match motion {
            Motion::DocumentStart => count.saturating_sub(1),
            Motion::DocumentEnd if count > 1 => count.saturating_sub(1),
            Motion::DocumentEnd => lines.saturating_sub(1),
            _ => unreachable!(),
        }
        .min(lines.saturating_sub(1));
        let first = table.cursor.line.min(target_line);
        let number = table.cursor.line.max(target_line) - first + 1;
        return linewise_range(table, first, number);
    }

    let origin = table.cursor.position;
    let target = motion_target(table, motion, count)?;
    let (start, mut end) = if origin <= target {
        (origin, target)
    } else {
        (target, origin)
    };

    if motion == Motion::WordEnd && end < table.len() {
        end = table
            .next_char_boundary(end)
            .map_err(|error| error.to_string())?;
    }

    Ok((start, end, false))
}

fn linewise_range(
    table: &mut PieceTable,
    first_line: usize,
    count: usize,
) -> Result<(usize, usize, bool), String> {
    let start = table
        .line_start(first_line)
        .map_err(|error| error.to_string())?;
    let end_line = first_line.saturating_add(count);
    let mut end = table
        .line_start(end_line)
        .map_err(|error| error.to_string())?;

    if end == start {
        end = table.len();
    }

    Ok((start, end, true))
}

fn deletion_range(
    table: &mut PieceTable,
    start: usize,
    end: usize,
    linewise: bool,
) -> Result<(usize, usize), String> {
    if linewise
        && end == table.len()
        && start > 0
        && table
            .byte_at(start - 1)
            .map_err(|error| error.to_string())?
            == Some(b'\n')
        && table
            .byte_at(end.saturating_sub(1))
            .map_err(|error| error.to_string())?
            != Some(b'\n')
    {
        return Ok((start - 1, end));
    }

    Ok((start, end))
}

fn position_on_line(table: &mut PieceTable, line: usize, column: usize) -> Result<usize, String> {
    let start = table.line_start(line).map_err(|error| error.to_string())?;
    let text = table.line_text(line).map_err(|error| error.to_string())?;
    let byte = text
        .char_indices()
        .nth(column)
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    Ok(start + byte)
}

fn line_end(table: &mut PieceTable, line: usize) -> Result<usize, String> {
    let start = table.line_start(line).map_err(|error| error.to_string())?;
    let text = table.line_text(line).map_err(|error| error.to_string())?;
    Ok(start + text.len())
}

fn normal_cursor_target(table: &mut PieceTable, target: usize) -> Result<usize, String> {
    let (line, _) = table
        .line_column_at(target)
        .map_err(|error| error.to_string())?;
    let start = table.line_start(line).map_err(|error| error.to_string())?;
    let end = line_end(table, line)?;

    if target >= end && end > start {
        table
            .previous_char_boundary(end)
            .map_err(|error| error.to_string())
    } else {
        Ok(target)
    }
}

fn settle_normal_cursor(editor: &mut Editor) -> Result<(), String> {
    let position = editor.document.cursor.position;
    let target = normal_cursor_target(&mut editor.document, position)?;
    editor
        .document
        .move_cursor(target)
        .map_err(|error| error.to_string())
}

fn first_nonblank(table: &mut PieceTable, line: usize) -> Result<usize, String> {
    let start = table.line_start(line).map_err(|error| error.to_string())?;
    let text = table.line_text(line).map_err(|error| error.to_string())?;
    let byte = text
        .char_indices()
        .find(|(_, character)| !character.is_whitespace())
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    Ok(start + byte)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordClass {
    Whitespace,
    Keyword,
    Punctuation,
}

fn word_class(character: char) -> WordClass {
    if character.is_whitespace() {
        WordClass::Whitespace
    } else if character.is_alphanumeric() || character == '_' {
        WordClass::Keyword
    } else {
        WordClass::Punctuation
    }
}

fn char_at(table: &PieceTable, position: usize) -> Result<Option<char>, String> {
    if position >= table.len() {
        return Ok(None);
    }
    let end = table
        .next_char_boundary(position)
        .map_err(|error| error.to_string())?;
    let bytes = table
        .read_range(position, end - position)
        .map_err(|error| error.to_string())?;
    let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    Ok(text.chars().next())
}

fn word_forward(table: &PieceTable, mut position: usize) -> Result<usize, String> {
    let Some(character) = char_at(table, position)? else {
        return Ok(position);
    };
    let class = word_class(character);

    while let Some(character) = char_at(table, position)? {
        if word_class(character) != class {
            break;
        }
        position = table
            .next_char_boundary(position)
            .map_err(|error| error.to_string())?;
    }

    while let Some(character) = char_at(table, position)? {
        if word_class(character) != WordClass::Whitespace {
            break;
        }
        position = table
            .next_char_boundary(position)
            .map_err(|error| error.to_string())?;
    }

    Ok(position)
}

fn word_backward(table: &PieceTable, mut position: usize) -> Result<usize, String> {
    if position == 0 {
        return Ok(0);
    }

    position = table
        .previous_char_boundary(position)
        .map_err(|error| error.to_string())?;

    while position > 0 {
        let character = char_at(table, position)?.unwrap();
        if word_class(character) != WordClass::Whitespace {
            break;
        }
        position = table
            .previous_char_boundary(position)
            .map_err(|error| error.to_string())?;
    }

    let class = char_at(table, position)?
        .map(word_class)
        .unwrap_or(WordClass::Whitespace);

    while position > 0 {
        let previous = table
            .previous_char_boundary(position)
            .map_err(|error| error.to_string())?;
        let Some(character) = char_at(table, previous)? else {
            break;
        };
        if word_class(character) != class {
            break;
        }
        position = previous;
    }

    Ok(position)
}

fn word_end(table: &PieceTable, mut position: usize) -> Result<usize, String> {
    while let Some(character) = char_at(table, position)? {
        if word_class(character) != WordClass::Whitespace {
            break;
        }
        position = table
            .next_char_boundary(position)
            .map_err(|error| error.to_string())?;
    }

    let Some(character) = char_at(table, position)? else {
        return Ok(position);
    };
    let class = word_class(character);
    let mut last = position;

    while let Some(character) = char_at(table, position)? {
        if word_class(character) != class {
            break;
        }
        last = position;
        position = table
            .next_char_boundary(position)
            .map_err(|error| error.to_string())?;
    }
    Ok(last)
}

fn word_under_cursor(table: &PieceTable) -> Result<String, String> {
    let mut start = table.cursor.position.min(table.len());
    let Some(character) = char_at(table, start)? else {
        return Ok(String::new());
    };
    let class = word_class(character);
    if class == WordClass::Whitespace {
        return Ok(String::new());
    }

    while start > 0 {
        let previous = table
            .previous_char_boundary(start)
            .map_err(|error| error.to_string())?;
        let Some(character) = char_at(table, previous)? else {
            break;
        };
        if word_class(character) != class {
            break;
        }
        start = previous;
    }

    let mut end = table.cursor.position;
    while let Some(character) = char_at(table, end)? {
        if word_class(character) != class {
            break;
        }
        end = table
            .next_char_boundary(end)
            .map_err(|error| error.to_string())?;
    }

    let bytes = table
        .read_range(start, end - start)
        .map_err(|error| error.to_string())?;
    String::from_utf8(bytes).map_err(|error| error.to_string())
}

fn copy_range<C: TextClipboard>(
    clipboard: &C,
    table: &PieceTable,
    start: usize,
    end: usize,
) -> Result<bool, String> {
    if start >= end {
        return Ok(false);
    }
    let bytes = table
        .read_range(start, end - start)
        .map_err(|error| error.to_string())?;
    let text = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    if text.contains('\0') {
        return Err("selection contains a NUL byte and cannot be copied as text".to_string());
    }
    clipboard.set_text(&text)?;
    Ok(true)
}

fn apply_insert_placement(editor: &mut Editor, placement: InsertPlacement) -> Result<(), String> {
    match placement {
        InsertPlacement::Before => {}
        InsertPlacement::After => {
            let line = editor.document.cursor.line;
            let end = line_end(&mut editor.document, line)?;
            if editor.document.cursor.position < end {
                editor
                    .document
                    .cursor_right()
                    .map_err(|error| error.to_string())?;
            }
        }
        InsertPlacement::FirstNonBlank => {
            let line = editor.document.cursor.line;
            let target = first_nonblank(&mut editor.document, line)?;
            editor
                .document
                .move_cursor(target)
                .map_err(|error| error.to_string())?;
        }
        InsertPlacement::LineEnd => {
            let line = editor.document.cursor.line;
            let target = line_end(&mut editor.document, line)?;
            editor
                .document
                .move_cursor(target)
                .map_err(|error| error.to_string())?;
        }
        InsertPlacement::OpenAbove => {
            let target = editor
                .document
                .line_start(editor.document.cursor.line)
                .map_err(|error| error.to_string())?;
            editor
                .document
                .move_cursor(target)
                .map_err(|error| error.to_string())?;
            editor.insert("\n").map_err(|error| error.to_string())?;
            editor
                .document
                .move_cursor(target)
                .map_err(|error| error.to_string())?;
        }
        InsertPlacement::OpenBelow => {
            let line = editor.document.cursor.line;
            let target = line_end(&mut editor.document, line)?;
            editor
                .document
                .move_cursor(target)
                .map_err(|error| error.to_string())?;
            editor.insert("\n").map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn paste_text(
    editor: &mut Editor,
    text: &str,
    before: bool,
    linewise: bool,
    count: usize,
) -> Result<bool, String> {
    if text.is_empty() {
        return Ok(false);
    }

    let group = editor.begin_history_group();
    let repeated = text.repeat(count.max(1));

    if linewise {
        let line = editor.document.cursor.line;
        let target = if before {
            editor
                .document
                .line_start(line)
                .map_err(|error| error.to_string())?
        } else {
            editor
                .document
                .line_start(line.saturating_add(1))
                .map_err(|error| error.to_string())?
        };
        editor
            .document
            .move_cursor(target)
            .map_err(|error| error.to_string())?;

        let mut value = repeated;
        if !value.ends_with('\n') {
            value.push('\n');
        }
        editor.insert(&value).map_err(|error| error.to_string())?;
    } else {
        if !before {
            let line = editor.document.cursor.line;
            let end = line_end(&mut editor.document, line)?;
            if editor.document.cursor.position < end {
                editor
                    .document
                    .cursor_right()
                    .map_err(|error| error.to_string())?;
            }
        }
        editor
            .insert(&repeated)
            .map_err(|error| error.to_string())?;
    }

    editor.end_history_group(group);
    settle_normal_cursor(editor)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::TextClipboard;
    use crate::config::{EditorConfig, KeybindingMode, LineNumberMode};
    use std::cell::RefCell;

    #[derive(Default)]
    struct Clipboard(RefCell<String>);

    impl TextClipboard for Clipboard {
        fn set_text(&self, text: &str) -> Result<(), String> {
            self.0.replace(text.to_string());
            Ok(())
        }

        fn text(&self) -> Result<String, String> {
            Ok(self.0.borrow().clone())
        }
    }

    fn editor(text: &str) -> Editor {
        let mut editor = Editor::new(EditorConfig {
            tab_width: 2,
            insert_spaces: false,
            line_numbers: LineNumberMode::Dynamic,
            font_size: 18,
            keybinding_mode: KeybindingMode::Vim,
        })
        .unwrap();
        editor.insert(text).unwrap();
        editor.undo_stack.clear();
        editor.document.move_cursor(0).unwrap();
        editor
    }

    fn key(vim: &mut VimController, editor: &mut Editor, clipboard: &Clipboard, key: Keycode) {
        vim.handle_key(editor, clipboard, key, Mod::NOMOD, false)
            .unwrap();
    }

    fn shifted_key(
        vim: &mut VimController,
        editor: &mut Editor,
        clipboard: &Clipboard,
        key: Keycode,
    ) -> VimOutcome {
        vim.handle_key(editor, clipboard, key, Mod::LSHIFTMOD, false)
            .unwrap()
    }

    fn type_text(vim: &mut VimController, editor: &mut Editor, text: &str) {
        editor.insert(text).unwrap();
        vim.record_text(text);
    }

    #[test]
    fn dw_is_semantic_and_dot_repeats_it() {
        let mut editor = editor("one two three");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::D);
        key(&mut vim, &mut editor, &clipboard, Keycode::W);
        assert_eq!(editor.document.text().unwrap(), "two three");

        key(&mut vim, &mut editor, &clipboard, Keycode::Period);
        assert_eq!(editor.document.text().unwrap(), "three");
    }

    #[test]
    fn counted_motion_moves_by_words() {
        let mut editor = editor("one two three");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::_2);
        key(&mut vim, &mut editor, &clipboard, Keycode::W);
        assert_eq!(editor.document.cursor.position, 8);
    }

    #[test]
    fn insert_session_is_one_undo_step() {
        let mut editor = editor("start");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::I);
        editor.insert("a").unwrap();
        vim.record_text("a");
        editor.insert("b").unwrap();
        vim.record_text("b");
        key(&mut vim, &mut editor, &clipboard, Keycode::Escape);

        assert_eq!(editor.undo_stack.len(), 1);
        editor.undo().unwrap();
        assert_eq!(editor.document.text().unwrap(), "start");
        editor.redo().unwrap();
        assert_eq!(editor.document.text().unwrap(), "abstart");
    }

    #[test]
    fn dd_and_dot_delete_complete_lines() {
        let mut editor = editor("one\ntwo\nthree");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::D);
        key(&mut vim, &mut editor, &clipboard, Keycode::D);
        assert_eq!(editor.document.text().unwrap(), "two\nthree");
        assert_eq!(clipboard.text().unwrap(), "one\n");

        key(&mut vim, &mut editor, &clipboard, Keycode::Period);
        assert_eq!(editor.document.text().unwrap(), "three");
    }

    #[test]
    fn yy_and_p_use_linewise_clipboard_semantics() {
        let mut editor = editor("one\ntwo");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::Y);
        key(&mut vim, &mut editor, &clipboard, Keycode::Y);
        key(&mut vim, &mut editor, &clipboard, Keycode::P);

        assert_eq!(editor.document.text().unwrap(), "one\none\ntwo");
    }

    #[test]
    fn cw_preserves_space_and_dot_changes_next_word() {
        let mut editor = editor("one two");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::C);
        key(&mut vim, &mut editor, &clipboard, Keycode::W);
        type_text(&mut vim, &mut editor, "X");
        key(&mut vim, &mut editor, &clipboard, Keycode::Escape);
        assert_eq!(editor.document.text().unwrap(), "X two");

        key(&mut vim, &mut editor, &clipboard, Keycode::W);
        key(&mut vim, &mut editor, &clipboard, Keycode::Period);
        assert_eq!(editor.document.text().unwrap(), "X X");
    }

    #[test]
    fn insert_and_dot_share_the_semantic_edit_path() {
        let mut editor = editor("z");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::I);
        type_text(&mut vim, &mut editor, "ab");
        key(&mut vim, &mut editor, &clipboard, Keycode::Escape);
        key(&mut vim, &mut editor, &clipboard, Keycode::Period);

        assert_eq!(editor.document.text().unwrap(), "ababz");
    }

    #[test]
    fn search_keys_return_actions_without_editing() {
        let mut editor = editor("one two one");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        let outcome = vim
            .handle_key(&mut editor, &clipboard, Keycode::Slash, Mod::NOMOD, false)
            .unwrap();
        assert_eq!(
            outcome.ui_action,
            VimUiAction::OpenSearch { backward: false },
        );

        let outcome = shifted_key(&mut vim, &mut editor, &clipboard, Keycode::_8);
        assert_eq!(
            outcome.ui_action,
            VimUiAction::SearchWord {
                query: "one".to_string(),
                backward: false,
            },
        );
    }

    #[test]
    fn visual_delete_removes_selected_characters() {
        let mut editor = editor("abcde");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::V);
        key(&mut vim, &mut editor, &clipboard, Keycode::L);
        key(&mut vim, &mut editor, &clipboard, Keycode::D);

        assert_eq!(editor.document.text().unwrap(), "cde");
        assert_eq!(clipboard.text().unwrap(), "ab");
        assert_eq!(vim.mode(), VimMode::Normal);
    }

    #[test]
    fn shifted_d_deletes_to_end_of_line() {
        let mut editor = editor("one two\nnext");
        editor.document.move_cursor(4).unwrap();
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        shifted_key(&mut vim, &mut editor, &clipboard, Keycode::D);

        assert_eq!(editor.document.text().unwrap(), "one \nnext");
        assert_eq!(clipboard.text().unwrap(), "two");
    }

    #[test]
    fn read_only_editor_allows_motion_but_not_changes() {
        let mut editor = editor("one two");
        editor.read_only = true;
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::W);
        assert_eq!(editor.document.cursor.position, 4);

        key(&mut vim, &mut editor, &clipboard, Keycode::X);
        key(&mut vim, &mut editor, &clipboard, Keycode::I);
        assert_eq!(editor.document.text().unwrap(), "one two");
        assert_eq!(vim.mode(), VimMode::Normal);
    }

    #[test]
    fn insert_newline_is_grouped_and_repeated() {
        let mut editor = editor("ab");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::I);
        key(&mut vim, &mut editor, &clipboard, Keycode::Return);
        type_text(&mut vim, &mut editor, "x");
        key(&mut vim, &mut editor, &clipboard, Keycode::Escape);
        assert_eq!(editor.document.text().unwrap(), "\nxab");

        key(&mut vim, &mut editor, &clipboard, Keycode::Period);
        assert_eq!(editor.document.text().unwrap(), "\nx\nxab");

        editor.undo().unwrap();
        assert_eq!(editor.document.text().unwrap(), "\nxab");
    }

    #[test]
    fn open_above_types_on_the_new_line_and_repeats() {
        let mut editor = editor("abc");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        shifted_key(&mut vim, &mut editor, &clipboard, Keycode::O);
        type_text(&mut vim, &mut editor, "x");
        key(&mut vim, &mut editor, &clipboard, Keycode::Escape);
        assert_eq!(editor.document.text().unwrap(), "x\nabc");
        assert_eq!(editor.document.cursor.position, 0);

        key(&mut vim, &mut editor, &clipboard, Keycode::Period);
        assert_eq!(editor.document.text().unwrap(), "x\nx\nabc");
    }

    #[test]
    fn line_end_and_append_stay_before_the_newline() {
        let mut editor = editor("abc\nnext");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        shifted_key(&mut vim, &mut editor, &clipboard, Keycode::_4);
        assert_eq!(editor.document.cursor.position, 2);

        key(&mut vim, &mut editor, &clipboard, Keycode::A);
        type_text(&mut vim, &mut editor, "x");
        key(&mut vim, &mut editor, &clipboard, Keycode::Escape);

        assert_eq!(editor.document.text().unwrap(), "abcx\nnext");
        assert_eq!(editor.document.cursor.position, 3);
    }

    #[test]
    fn cancelling_search_restores_the_original_cursor() {
        let mut editor = editor("one two one");
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        editor.document.move_cursor(4).unwrap();
        key(&mut vim, &mut editor, &clipboard, Keycode::Slash);
        editor.document.move_cursor(8).unwrap();
        vim.cancel_search(&mut editor).unwrap();

        assert_eq!(editor.document.cursor.position, 4);
        assert_eq!(vim.search_origin(), None);
    }

    #[test]
    fn deleting_the_last_line_removes_its_leading_newline() {
        let mut editor = editor("one\ntwo");
        editor.document.move_cursor(4).unwrap();
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::D);
        key(&mut vim, &mut editor, &clipboard, Keycode::D);

        assert_eq!(editor.document.text().unwrap(), "one");
        assert_eq!(clipboard.text().unwrap(), "two");
        assert_eq!(editor.document.cursor.position, 2);
    }

    #[test]
    fn operator_document_motions_are_linewise() {
        let mut editor = editor("one\ntwo\nthree");
        editor.document.move_cursor(4).unwrap();
        let clipboard = Clipboard::default();
        let mut vim = VimController::new();

        key(&mut vim, &mut editor, &clipboard, Keycode::D);
        shifted_key(&mut vim, &mut editor, &clipboard, Keycode::G);

        assert_eq!(editor.document.text().unwrap(), "one");
        assert_eq!(clipboard.text().unwrap(), "two\nthree");

        editor.undo().unwrap();
        editor.document.move_cursor(8).unwrap();
        key(&mut vim, &mut editor, &clipboard, Keycode::D);
        key(&mut vim, &mut editor, &clipboard, Keycode::G);
        key(&mut vim, &mut editor, &clipboard, Keycode::G);

        assert_eq!(editor.document.text().unwrap(), "");
    }
}
