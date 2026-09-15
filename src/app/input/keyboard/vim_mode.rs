// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn handle(
    context: InputContext<'_, '_>,
    key: Keycode,
    keymod: Mod,
    repeat: bool,
    bound_command: Option<Command>,
) -> KeyResult {
    let InputContext {
        editor,
        renderer,
        vim,
        search_ui,
        command_bar,
        clipboard,
        dirty,
        vim_enabled,
        ..
    } = context;
    if *vim_enabled && bound_command == Some(Command::SelectNextOccurrence) {
        // Ctrl+D still reaches Vim's half-page motion; Cmd+D has no Vim action.
        if VimController::page_motion(key, keymod).is_none() {
            return Ok(Some(EventFlow::Continue));
        }
    }
    if *vim_enabled {
        let outcome = vim.handle_key(
            &mut *editor,
            clipboard,
            key,
            keymod,
            repeat,
            renderer.visible_line_count(),
        )?;

        renderer.set_mode_label(Some(vim.mode_label()));

        match outcome.ui_action {
            VimUiAction::None => {}

            VimUiAction::OpenCommandBar => {
                command_bar.open(":");
                search_ui.close();
            }

            VimUiAction::OpenSearch { backward } => {
                if backward {
                    command_bar.open(":find  --backward");
                    command_bar.set_cursor(6);
                } else {
                    command_bar.open(":find ");
                }
                search_ui.close();
            }

            VimUiAction::RepeatSearch { backward } => {
                let result = if backward {
                    search_ui.previous(&mut editor.document)
                } else {
                    search_ui.next(&mut editor.document)
                };
                result.map_err(|error| error.to_string())?;
            }

            VimUiAction::SearchWord { query, backward } => {
                if !query.is_empty() {
                    search_ui
                        .configure_command(
                            &mut editor.document,
                            &query,
                            None,
                            SearchMode::CaseSensitive,
                        )
                        .map_err(|error| error.to_string())?;

                    let result = if backward {
                        search_ui.previous(&mut editor.document)
                    } else {
                        search_ui.next(&mut editor.document)
                    };
                    result.map_err(|error| error.to_string())?;
                }
            }
        }

        if outcome.document_changed {
            renderer.invalidate_scroll_cache();
        }

        if outcome.cursor_changed {
            renderer.ensure_cursor_visible(&mut editor.document);
        }

        if outcome.consumed {
            *dirty = true;
            return Ok(Some(EventFlow::Continue));
        }
    }

    Ok(None)
}
