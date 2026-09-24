// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn text(context: InputContext<'_, '_>, text: &str) -> Result<EventFlow, String> {
    let InputContext {
        pane_keys,
        editor,
        other_editor,
        renderer,
        terminal,
        vim,
        emacs,
        lsp_ui,
        search_ui,
        command_bar,
        dirty,
        vim_enabled,
        ..
    } = context;

    if pane_keys.consume_text() || emacs.consume_text() {
        return Ok(EventFlow::Continue);
    }
    if renderer.terminal_focused(terminal) {
        if !terminal.output_focused() {
            terminal.insert_text(&text);
        }
        *dirty = true;
        return Ok(EventFlow::Continue);
    }

    if *vim_enabled && vim.consume_suppressed_text_input() {
        return Ok(EventFlow::Continue);
    }

    if command_bar.is_active() {
        if !text.is_empty() {
            if editor.config.keybinding_mode != KeybindingMode::Emacs
                && command_bar.input() == ":"
                && text == ":"
            {
                command_bar.close();
                search_ui.close();

                editor.insert(":").map_err(|error| error.to_string())?;

                renderer.invalidate_scroll_cache();

                renderer.ensure_cursor_visible(&mut editor.document);

                *dirty = true;
                return Ok(EventFlow::Continue);
            }

            command_bar.insert_text(&text);

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

                renderer.ensure_search_match_visible(
                    &mut editor.document,
                    &*search_ui,
                    &*command_bar,
                );
            }

            *dirty = true;
        }

        return Ok(EventFlow::Continue);
    }

    if *vim_enabled && vim.mode() != vim::VimMode::Insert {
        return Ok(EventFlow::Continue);
    }

    if editor.config.keybinding_mode == KeybindingMode::Conventional
        && text == ":"
        && editor.document.secondary_cursors.is_empty()
    {
        let line = editor.document.cursor.line;

        let empty_line = editor
            .document
            .line_length(line)
            .map_err(|error| error.to_string())?
            == 0;

        if empty_line {
            command_bar.open(":");
            search_ui.close();
            *dirty = true;
            return Ok(EventFlow::Continue);
        }
    }

    if !text.is_empty() {
        if editor.config.keybinding_mode == KeybindingMode::Emacs {
            if let Err(error) = emacs.insert(&mut *editor, &text) {
                command_bar.open(":");
                command_bar.show_info(&error.to_string());
            }
            renderer.set_mode_label(editor_mode_label(&*editor, &*vim));
        } else {
            editor.insert(&text).map_err(|e| e.to_string())?;
        }

        if *vim_enabled {
            vim.record_text(&text);
        }
        lsp_ui.typed_member_trigger(&text, &*editor, &*other_editor, &mut *command_bar);

        renderer.invalidate_scroll_cache();

        renderer.ensure_cursor_visible(&mut editor.document);

        *dirty = true;
    }

    Ok(EventFlow::Continue)
}
