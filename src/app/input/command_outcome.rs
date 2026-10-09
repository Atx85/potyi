// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Apply command follow-up work after execution, including pane swaps.
use super::*;

pub(super) struct CommandOutcomeContext<'a, 'font> {
    pub editor: &'a mut Editor,
    pub other_editor: &'a mut Editor,
    pub renderer: &'a mut Renderer<'font>,
    pub terminal: &'a mut Terminal,
    pub vim: &'a mut VimController,
    pub other_vim: &'a mut VimController,
    pub search_ui: &'a mut SearchUi,
    pub command_bar: &'a mut CommandBar,
    pub dirty: &'a mut bool,
    pub split_mode: &'a mut bool,
    pub active_pane: &'a mut usize,
    pub vim_enabled: &'a mut bool,
}

pub(super) enum CommandSource {
    Keyboard,
    Mouse,
    Emacs { document_changed: bool },
}

pub(super) fn apply(
    context: CommandOutcomeContext<'_, '_>,
    outcome: CommandOutcome,
    source: CommandSource,
) -> EventFlow {
    let CommandOutcomeContext {
        editor,
        other_editor,
        renderer,
        terminal,
        vim,
        other_vim,
        search_ui,
        command_bar,
        dirty,
        split_mode,
        active_pane,
        vim_enabled,
    } = context;
    if outcome.quit {
        return EventFlow::Quit;
    }
    if outcome.close_pane {
        close_focused_pane(
            split_mode,
            active_pane,
            editor,
            other_editor,
            vim,
            other_vim,
            renderer,
            terminal,
        );
    }
    // Emacs handles split/mode actions in its controller's UI action branch.
    if !matches!(source, CommandSource::Emacs { .. }) && outcome.toggle_split {
        *split_mode = !*split_mode;
        renderer.set_split_mode(*split_mode);
    }
    if outcome.focus_other {
        focus_pane(
            1 - *active_pane,
            active_pane,
            editor,
            other_editor,
            vim,
            other_vim,
            renderer,
        );
    }

    match source {
        CommandSource::Emacs { document_changed } => {
            if document_changed || outcome.document_changed || outcome.document_reloaded {
                renderer.invalidate_scroll_cache();
            }
            if outcome.path_changed {
                renderer.set_file_path(editor.path().as_deref());
            }
            renderer.set_mode_label(editor_mode_label(editor, vim));
            renderer.update_cursor(&editor.document);
            renderer.ensure_cursor_visible(&mut editor.document);
        }
        CommandSource::Keyboard | CommandSource::Mouse => {
            if outcome.path_changed {
                renderer.set_file_path(editor.path().as_deref());
            }
            if outcome.document_changed || outcome.document_reloaded {
                renderer.invalidate_scroll_cache();
            }
            if let Some(mode) = outcome.keybinding_mode {
                apply_keybinding_mode(
                    mode,
                    vim_enabled,
                    editor,
                    other_editor,
                    vim,
                    other_vim,
                    renderer,
                );
            }
            if *vim_enabled && outcome.document_reloaded {
                vim.reset();
                renderer.set_mode_label(Some(vim.mode_label()));
            }
            if outcome.cursor_changed {
                renderer.update_cursor(&editor.document);
            }
            // Keyboard editing also reveals an existing search match. Mouse
            // execution reveals it only when the command moved the cursor.
            if outcome.cursor_changed || matches!(source, CommandSource::Keyboard) {
                if search_ui.current_match().is_some() {
                    renderer.ensure_search_match_visible(
                        &mut editor.document,
                        search_ui,
                        command_bar,
                    );
                } else if outcome.cursor_changed {
                    renderer.ensure_cursor_visible(&mut editor.document);
                }
            }
        }
    }
    *dirty = true;
    EventFlow::Continue
}
