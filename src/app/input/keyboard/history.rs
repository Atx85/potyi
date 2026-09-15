// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn handle(
    context: InputContext<'_, '_>,
    key: Keycode,
    keymod: Mod,
    _repeat: bool,
    bound_command: Option<Command>,
) -> KeyResult {
    let InputContext {
        editor,
        other_editor,
        renderer,
        vim,
        other_vim,
        lsp_ui,
        search_ui,
        command_bar,
        dirty,
        vim_enabled,
        ..
    } = context;
    /*
     * Normal editor key handling.
     */
    let workspace_redo = if *vim_enabled {
        vim.workspace_history_key(key, keymod)
            .or_else(|| match bound_command {
                Some(Command::Undo) => Some(false),
                Some(Command::Redo) => Some(true),
                _ => None,
            })
    } else {
        match bound_command {
            Some(Command::Undo) => Some(false),
            Some(Command::Redo) => Some(true),
            _ => None,
        }
    };
    if let Some(redo) = workspace_redo {
        match workspace_edit::history(&mut *editor, &mut *other_editor, redo) {
            Ok(false) => {}
            result => {
                if result.is_ok() {
                    lsp_ui.files_changed(workspace_edit::recent_disk_changes(&*editor, !redo));
                    renderer.set_file_path(editor.path.as_deref());
                }
                if *vim_enabled {
                    vim.deactivate(&mut *editor);
                    other_vim.deactivate(&mut *other_editor);
                    vim.consume_workspace_history_key();
                    renderer.set_mode_label(Some(vim.mode_label()));
                }
                if let Err(error) = result {
                    command_bar.open(":");
                    command_bar.show_info(&error);
                }
                search_ui.close();
                renderer.invalidate_scroll_cache();
                renderer.update_cursor(&editor.document);
                *dirty = true;
                return Ok(Some(EventFlow::Continue));
            }
        }
    }

    Ok(None)
}
