// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn press(
    context: MouseContext<'_, '_>,
    x: f32,
    y: f32,
    clicks: u8,
    extend: bool,
) -> Result<EventFlow, String> {
    let MouseContext {
        editor,
        other_editor,
        renderer,
        terminal,
        vim,
        other_vim,
        lsp_ui,
        search_ui,
        command_bar,
        event_subsystem,
        keyboard,
        dirty,
        active_pane,
        ..
    } = context;
    let clicked_pane = renderer.pane_at(x as i32);
    focus_pane_preserving_view(
        clicked_pane,
        active_pane,
        editor,
        other_editor,
        vim,
        other_vim,
        renderer,
    );
    search_ui.close();
    command_bar.close();
    lsp_ui.dismiss_completion();
    match renderer.terminal_hit_at(x as i32, y as i32) {
        TerminalHit::Input => {
            let cursor = renderer.terminal_cursor_at(&*terminal, x as i32);
            terminal.focus_prompt();
            terminal.set_cursor(cursor);
        }

        TerminalHit::StopOrRunAgain => {
            if terminal.is_running() {
                if let Err(error) = terminal.stop() {
                    terminal.set_status(error.to_string());
                }
            } else {
                match terminal.run_again(event_subsystem) {
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
        }

        TerminalHit::Clear => {
            let result = if terminal.can_go_back() {
                terminal.go_back()
            } else {
                terminal.clear()
            };
            if let Err(error) = result {
                terminal.set_status(error.to_string());
            }
        }

        TerminalHit::Editor => {
            terminal.close_to_editor();
        }

        TerminalHit::Output => {
            let action = renderer.terminal_action_at(&mut *terminal, x as i32, y as i32)?;
            let offset = renderer.terminal_output_offset_at(&mut *terminal, x as i32, y as i32)?;
            let open_other_pane = keyboard
                .mod_state()
                .intersects(Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LGUIMOD | Mod::RGUIMOD);
            terminal
                .begin_output_mouse_drag(
                    offset,
                    x as i32,
                    y as i32,
                    action,
                    open_other_pane,
                    clicks,
                    extend,
                )
                .map_err(|error| error.to_string())?;
        }

        TerminalHit::Outside => {}
    }

    *dirty = true;
    Ok(EventFlow::Continue)
}
