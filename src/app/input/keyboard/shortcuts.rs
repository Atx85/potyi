// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn terminal_toggle(
    context: InputContext<'_, '_>,
    key: Keycode,
    keymod: Mod,
    repeat: bool,
) -> KeyResult {
    let InputContext {
        editor,
        terminal,
        renderer,
        search_ui,
        command_bar,
        dirty,
        ..
    } = context;
    if ctrl_pressed(keymod) && key == Keycode::Grave && !repeat {
        editor.clear_secondary_cursors();
        command_bar.close();
        search_ui.close();
        if renderer.terminal_focused(terminal) {
            terminal.toggle(editor.path.as_deref());
        } else {
            renderer.place_terminal_in_active_pane();
            terminal.open(editor.path.as_deref());
        }
        *dirty = true;
        return Ok(Some(EventFlow::Continue));
    }

    Ok(None)
}

pub(super) fn open_command(
    context: InputContext<'_, '_>,
    key: Keycode,
    keymod: Mod,
    _repeat: bool,
) -> KeyResult {
    let InputContext {
        editor,
        renderer,
        vim,
        emacs,
        search_ui,
        command_bar,
        dirty,
        vim_enabled,
        ..
    } = context;
    /*
     * Familiar shortcuts open the shared command bar with
     * their command already selected. Ctrl+P opens the full
     * command list.
     */
    if editor.config.keybinding_mode != KeybindingMode::Emacs
        && ctrl_pressed(keymod)
        && !(*vim_enabled
            && vim.mode() != vim::VimMode::Insert
            && !command_bar.is_active()
            && !search_ui.is_active()
            && VimController::page_motion(key, keymod).is_some())
        && (key == Keycode::F || key == Keycode::H || key == Keycode::P)
    {
        let initial = match key {
            Keycode::F => find_command_text(&*search_ui),

            Keycode::H => replace_command_text(&*search_ui),

            _ => ":".to_string(),
        };

        editor.clear_secondary_cursors();
        command_bar.open(&initial);

        sync_command_search(
            &*command_bar,
            &mut *search_ui,
            &mut editor.document,
            if editor.config.keybinding_mode == KeybindingMode::Emacs {
                emacs.search_origin
            } else {
                vim.search_origin()
            },
        )
        .map_err(|error| error.to_string())?;

        if search_ui.current_match().is_some() {
            renderer.update_cursor(&editor.document);

            renderer.ensure_search_match_visible(&mut editor.document, &*search_ui, &*command_bar);
        }

        *dirty = true;
        return Ok(Some(EventFlow::Continue));
    }

    Ok(None)
}
