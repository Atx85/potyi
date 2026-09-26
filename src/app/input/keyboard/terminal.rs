// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn handle(
    context: InputContext<'_, '_>,
    key: Keycode,
    keymod: Mod,
    repeat: bool,
) -> KeyResult {
    let InputContext {
        editor,
        other_editor,
        renderer,
        terminal,
        vim,
        other_vim,
        key_bindings,
        clipboard,
        event_subsystem,
        dirty,
        active_pane,
        split_mode,
        vim_enabled,
        ..
    } = context;
    if renderer.terminal_focused(terminal) {
        if terminal.can_go_back() && alt_pressed(keymod) && key == Keycode::Left && !repeat {
            if let Err(error) = terminal.go_back() {
                terminal.set_status(error.to_string());
            }
            *dirty = true;
            return Ok(Some(EventFlow::Continue));
        }

        let clipboard_command = key_bindings.terminal_clipboard_command(key, keymod, repeat);
        if clipboard_command == Some(Command::Paste) {
            match read_text(clipboard) {
                Ok(text) => {
                    terminal.focus_prompt();
                    terminal.insert_text(&text);
                }
                Err(error) => terminal.set_status(format!("Clipboard paste failed: {error}")),
            }

            *dirty = true;
            return Ok(Some(EventFlow::Continue));
        }

        let selected_copy = ctrl_pressed(keymod)
            && key == Keycode::C
            && !repeat
            && !terminal.output_selection().is_empty();
        if clipboard_command == Some(Command::Copy) || selected_copy {
            let all = shift_pressed(keymod) || terminal.output_selection().is_empty();
            if let Err(error) = terminal.copy_output(clipboard, all, false) {
                terminal.set_status(format!("Clipboard copy failed: {error}"));
            } else {
                vim.set_clipboard_linewise(!all && terminal.output_linewise());
                other_vim.set_clipboard_linewise(!all && terminal.output_linewise());
            }
            *dirty = true;
            return Ok(Some(EventFlow::Continue));
        }

        if ctrl_pressed(keymod) && key == Keycode::C && !repeat && terminal.is_running() {
            if let Err(error) = terminal.stop() {
                terminal.set_status(error.to_string());
            }

            *dirty = true;
            return Ok(Some(EventFlow::Continue));
        }

        if ctrl_pressed(keymod) && key == Keycode::L && !repeat {
            if let Err(error) = terminal.clear() {
                terminal.set_status(error.to_string());
            }

            *dirty = true;
            return Ok(Some(EventFlow::Continue));
        }

        let select_output = !terminal.output_focused()
            && key == Keycode::Up
            && shift_pressed(keymod)
            && !keymod.intersects(
                Mod::LCTRLMOD
                    | Mod::RCTRLMOD
                    | Mod::LGUIMOD
                    | Mod::RGUIMOD
                    | Mod::LALTMOD
                    | Mod::RALTMOD,
            );
        if (key == Keycode::F6 && !repeat) || select_output {
            if terminal.output_focused() {
                terminal.focus_prompt();
            } else {
                let end = terminal.output_mut().len();
                terminal
                    .move_output_cursor(end, false)
                    .map_err(|error| error.to_string())?;
                renderer.navigate_terminal_output(
                    &mut *terminal,
                    if select_output {
                        OutputCommand::Rows(-1, true)
                    } else {
                        OutputCommand::None
                    },
                )?;
            }
            *dirty = true;
            return Ok(Some(EventFlow::Continue));
        }

        if terminal.output_focused() {
            let command = terminal
                .output_key(key, keymod, *vim_enabled)
                .map_err(|error| error.to_string())?;
            if matches!(command, OutputCommand::Copy) {
                let linewise = terminal.output_linewise();
                if !terminal.output_selection().is_empty() {
                    if let Err(error) = terminal.copy_output(clipboard, false, true) {
                        terminal.set_status(format!("Clipboard copy failed: {error}"));
                    } else {
                        vim.set_clipboard_linewise(linewise);
                        other_vim.set_clipboard_linewise(linewise);
                    }
                }
            } else {
                renderer.navigate_terminal_output(&mut *terminal, command)?;
            }
            *dirty = true;
            return Ok(Some(EventFlow::Continue));
        }

        match key {
            Keycode::Escape => {
                terminal.close_to_editor();
            }
            Keycode::Up => {
                terminal.history_previous();
            }
            Keycode::Down => {
                terminal.history_next();
            }
            Keycode::Backspace => {
                terminal.backspace();
            }
            Keycode::Delete => {
                terminal.delete();
            }
            Keycode::Left => {
                terminal.move_left();
            }
            Keycode::Right => {
                terminal.move_right();
            }
            Keycode::Home => {
                terminal.move_home();
            }
            Keycode::End => {
                terminal.move_end();
            }
            Keycode::Tab if !repeat => {
                terminal.complete_path(shift_pressed(keymod));
            }
            Keycode::Return | Keycode::KpEnter if !repeat => {
                if terminal.input().trim() == ":exit" {
                    if close_focused_pane(split_mode, active_pane, editor, other_editor,
                        vim, other_vim, renderer, terminal) {
                        terminal.clear_input();
                    } else {
                        terminal.set_status("No split pane to close. Use :quit in the editor to close Pötyi.");
                    }
                    *dirty = true;
                    return Ok(Some(EventFlow::Continue));
                }
                match terminal.submit(event_subsystem) {
                    Ok(action) => {
                        if handle_terminal_action_in_pane(
                            action,
                            &mut *terminal,
                            &mut *editor,
                            &mut *other_editor,
                            &mut *renderer,
                            TerminalOpenTarget::CurrentPane,
                        )? {
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
                    }
                    Err(error) => {
                        terminal.set_status(error.to_string());
                    }
                }
            }
            _ => {}
        }

        *dirty = true;
        return Ok(Some(EventFlow::Continue));
    }

    Ok(None)
}
