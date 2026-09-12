// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;
use sdl3::keyboard::Keycode;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(super) struct Menu {
    guard: Pending,
    anchor: usize,
    items: Vec<lsp::completion::Item>,
    selected: usize,
}

#[derive(Clone)]
pub(crate) struct Display {
    pub labels: Vec<String>,
    pub selected: usize,
    pub first: usize,
    pub total: usize,
}

impl LspUi {
    pub fn dismiss_completion(&mut self) {
        self.completion = None;
        if let Some(cancel) = self.completion_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
            self.pending = None;
            self.pending_anchor = None;
        }
    }

    pub fn validate_completion(&mut self, editor: &Editor, other: &Editor, visible: bool) {
        let valid = visible
            && !editor.read_only
            && editor.document.secondary_cursors.is_empty()
            && self.completion.as_ref().is_none_or(|m| {
                m.guard.matches_document(editor, other) && m.anchor == editor.document.cursor.anchor
            })
            && (self.completion_cancel.is_none()
                || self.pending.as_ref().is_some_and(|p| {
                    p.matches_document(editor, other)
                        && self.pending_anchor == Some(editor.document.cursor.anchor)
                }));
        if !valid {
            self.dismiss_completion();
        }
    }

    /// Trigger-only: no snapshots or requests for ordinary typing, paste, or frames.
    pub fn typed_member_trigger(
        &mut self,
        text: &str,
        editor: &Editor,
        other: &Editor,
        bar: &mut CommandBar,
    ) {
        self.dismiss_completion();
        if !matches!(text, "." | ":" | ">")
            || bar.is_active()
            || editor.read_only
            || !editor.document.secondary_cursors.is_empty()
            || editor.path.is_none()
            || editor.document.len() > lsp::MAX_DOCUMENT_BYTES
            || self.pending.is_some()
        {
            return;
        }
        let Ok(config) = lsp::Config::load() else {
            return;
        };
        if !self.is_enabled(config.enabled) {
            return;
        }
        let Some(server) = editor.path.as_deref().and_then(|p| config.server_for(p)) else {
            return;
        };
        if !member_trigger(text, &server.language_id, &editor.document) {
            return;
        }
        self.completion_error = None;
        let action = lsp::Action::Complete {
            trigger: text.into(),
            cancel: Arc::new(AtomicBool::new(false)),
        };
        if let Err(error) = self.request(action, editor, other, bar) {
            self.completion_error = Some(error);
        }
    }

    pub fn completion_display(&self) -> Option<Display> {
        let Some(menu) = self.completion.as_ref() else {
            return self.completion_cancel.as_ref().map(|_| Display {
                labels: vec!["Loading suggestions…".into()],
                selected: 0,
                first: 0,
                total: 0,
            });
        };
        let first = menu.selected.saturating_sub(7);
        Some(Display {
            labels: menu
                .items
                .iter()
                .skip(first)
                .take(8)
                .map(|i| i.label.clone())
                .collect(),
            selected: menu.selected - first,
            first,
            total: menu.items.len(),
        })
    }

    pub fn choose_completion(
        &mut self,
        index: usize,
        editor: &Editor,
        other: &Editor,
        bar: &mut CommandBar,
    ) {
        let Some(menu) = self.completion.take() else {
            return;
        };
        if !menu.guard.matches_document(editor, other)
            || menu.anchor != editor.document.cursor.anchor
            || bar.is_active()
        {
            return;
        }
        let Some(item) = menu.items.get(index) else {
            return;
        };
        let action = lsp::Action::ResolveCompletion {
            item: item.value.clone(),
            cancel: Arc::new(AtomicBool::new(false)),
        };
        if let Err(error) = self.request(action, editor, other, bar) {
            self.completion_error = Some(error);
        }
    }

    pub fn completion_key(
        &mut self,
        key: Keycode,
        repeat: bool,
        editor: &Editor,
        other: &Editor,
        bar: &mut CommandBar,
    ) -> bool {
        self.validate_completion(editor, other, !bar.is_active());
        let Some(menu) = self.completion.as_mut() else {
            if key == Keycode::Escape && self.completion_cancel.is_some() {
                self.dismiss_completion();
                return true;
            }
            return false;
        };
        match key {
            Keycode::Up => menu.selected = menu.selected.saturating_sub(1),
            Keycode::Down => menu.selected = (menu.selected + 1).min(menu.items.len() - 1),
            Keycode::Return | Keycode::KpEnter | Keycode::Tab => {
                let selected = menu.selected;
                if !repeat {
                    self.choose_completion(selected, editor, other, bar);
                }
            }
            Keycode::Escape => self.dismiss_completion(),
            _ => {
                self.dismiss_completion();
                return false;
            }
        }
        true
    }

    pub(super) fn accept_completion(
        &mut self,
        result: Result<lsp::Reply, String>,
        pending: Pending,
        editor: &mut Editor,
        other: &Editor,
        bar: &CommandBar,
    ) -> CommandOutcome {
        let anchor = self.pending_anchor.take();
        let mut outcome = CommandOutcome::default();
        if bar.is_active()
            || editor.read_only
            || !pending.matches_document(editor, other)
            || anchor != Some(editor.document.cursor.anchor)
            || !editor.document.secondary_cursors.is_empty()
        {
            return outcome;
        }
        match result {
            Ok(lsp::Reply::Completions(items)) if !items.is_empty() => {
                self.completion = Some(Menu {
                    guard: pending,
                    anchor: anchor.unwrap(),
                    items,
                    selected: 0,
                });
            }
            Ok(lsp::Reply::CompletionEdit(edit)) => {
                let result = (|| -> std::io::Result<bool> {
                    let changed = editor
                        .apply_formatted(&mut std::io::Cursor::new(edit.text.into_bytes()))?;
                    editor.document.move_cursor(edit.cursor)?;
                    editor.document.cursor.anchor = editor.document.cursor.position;
                    editor.document.cursor.anchor_line = editor.document.cursor.line;
                    editor.document.cursor.anchor_column = editor.document.cursor.column;
                    editor.document.cursor.desired_column = None;
                    let after = editor.cursor_state();
                    if changed {
                        if let Some(entry) = editor.undo_stack.last_mut() {
                            entry.after = after;
                        }
                    }
                    Ok(changed)
                })();
                match result {
                    Ok(changed) => {
                        outcome.document_changed = changed;
                        outcome.cursor_changed = true;
                    }
                    Err(error) => self.completion_error = Some(error.to_string()),
                }
            }
            Err(error) => self.completion_error = Some(error),
            _ => {}
        }
        outcome
    }
}

fn member_trigger(text: &str, language: &str, document: &crate::piece_table::PieceTable) -> bool {
    match text {
        "." => true,
        ":" if matches!(language, "lua" | "luau") => true,
        ":" | ">" => {
            let cursor = document.cursor.position;
            let wanted = if text == ":" { b':' } else { b'-' };
            cursor >= 2 && document.read_range(cursor - 2, 1).ok().as_deref() == Some(&[wanted])
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_member_operators_exclude_comparisons_and_type_annotations() {
        for (language, text, expected) in [
            ("lua", "object:", true),
            ("luau", "object:", true),
            ("rust", "Type:", false),
            ("rust", "Type::", true),
            ("cpp", "object->", true),
            ("cpp", "a >", false),
            ("csharp", "transform.", true),
        ] {
            let mut document = crate::piece_table::PieceTable::empty().unwrap();
            document.insert(0, text).unwrap();
            document.move_cursor(text.len()).unwrap();
            assert_eq!(
                member_trigger(&text[text.len() - 1..], language, &document),
                expected,
                "{language}: {text}"
            );
        }
    }
}
