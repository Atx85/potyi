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
        other_editor,
        renderer,
        terminal,
        vim,
        other_vim,
        emacs,
        lsp_ui,
        search_ui,
        command_bar,
        clipboard,
        dirty,
        split_mode,
        active_pane,
        vim_enabled,
        ..
    } = context;
    /*
     * Command mode is modal. Its editing, suggestions, and
     * execution never fall through to the document.
     */
    if command_bar.is_active() {
        if bound_command == Some(Command::Paste) {
            match read_text(clipboard) {
                Ok(text) if !text.is_empty() => {
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

                    *dirty = true;
                }

                Ok(_) => {}

                Err(error) => {
                    eprintln!("Clipboard paste failed: {error}");
                }
            }

            return Ok(Some(EventFlow::Continue));
        }

        let mut input_changed = false;
        let mut outcome = CommandOutcome::default();

        match key {
            Keycode::Escape => {
                if *vim_enabled {
                    vim.cancel_search(&mut *editor)?;
                }
                command_bar.close();
                search_ui.close();
            }

            Keycode::Up => {
                command_bar.move_selection(-1);
            }

            Keycode::Down => {
                command_bar.move_selection(1);
            }

            Keycode::Tab => {
                input_changed = command_bar.apply_selected();
            }

            Keycode::Backspace => {
                command_bar.backspace();
                input_changed = true;
            }

            Keycode::Delete => {
                command_bar.delete();
                input_changed = true;
            }

            Keycode::Left => {
                command_bar.move_left();
            }

            Keycode::Right => {
                command_bar.move_right();
            }

            Keycode::Home => {
                command_bar.move_home();
            }

            Keycode::End => {
                command_bar.move_end();
            }

            Keycode::Return | Keycode::KpEnter if !repeat => {
                let close_vim_search = *vim_enabled
                    && !command_bar.selects_option_on_enter()
                    && matches!(command_bar.parse(), Ok(ParsedCommand::Find { .. }));

                if close_vim_search {
                    outcome.cursor_changed = search_ui.current_match().is_some();
                    vim.accept_search();
                    command_bar.close();
                } else {
                    outcome = execute_command_bar(
                        &mut *command_bar,
                        &mut *search_ui,
                        &mut *editor,
                        &mut *other_editor,
                        &mut *renderer,
                        &mut *terminal,
                        &mut *vim,
                        shift_pressed(keymod),
                        &mut *other_vim,
                        &mut *lsp_ui,
                    );
                }
            }

            _ => {}
        }

        if input_changed {
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
        }

        if outcome.quit {
            return Ok(Some(EventFlow::Quit));
        }

        if outcome.toggle_split {
            *split_mode = !*split_mode;
            renderer.set_split_mode(*split_mode);
        }

        if outcome.focus_other {
            focus_pane(
                1 - *active_pane,
                &mut *active_pane,
                &mut *editor,
                &mut *other_editor,
                &mut *vim,
                &mut *other_vim,
                &mut *renderer,
            );
        }

        if outcome.path_changed {
            renderer.set_file_path(editor.path.as_deref());
        }

        if outcome.document_changed || outcome.document_reloaded {
            renderer.invalidate_scroll_cache();
        }

        if let Some(mode) = outcome.keybinding_mode {
            apply_keybinding_mode(
                mode,
                &mut *vim_enabled,
                &mut *editor,
                &mut *other_editor,
                &mut *vim,
                &mut *other_vim,
                &mut *renderer,
            );
        }

        if *vim_enabled && outcome.document_reloaded {
            vim.reset();
            renderer.set_mode_label(Some(vim.mode_label()));
        }

        if outcome.cursor_changed {
            renderer.update_cursor(&editor.document);
        }

        if search_ui.current_match().is_some() {
            renderer.ensure_search_match_visible(&mut editor.document, &*search_ui, &*command_bar);
        } else if outcome.cursor_changed {
            renderer.ensure_cursor_visible(&mut editor.document);
        }

        *dirty = true;
        return Ok(Some(EventFlow::Continue));
    }

    Ok(None)
}
