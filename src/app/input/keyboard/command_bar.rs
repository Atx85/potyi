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

            return Ok(KeyFlow::Handled);
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

        let flow = command_outcome::apply(
            CommandOutcomeContext {
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
            },
            outcome,
            CommandSource::Keyboard,
        );
        return Ok(flow.into());
    }

    Ok(KeyFlow::Pass)
}
