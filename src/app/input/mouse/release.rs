// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn handle(
    context: MouseContext<'_, '_>,
    x: f32,
    y: f32,
    coordinates_converted: bool,
) -> Result<EventFlow, String> {
    let MouseContext {
        mouse_state,
        editor,
        other_editor,
        renderer,
        terminal,
        vim,
        other_vim,
        dirty,
        split_mode,
        active_pane,
        vim_enabled,
        ..
    } = context;
    if mouse_state.split_drag {
        if coordinates_converted {
            renderer.resize_split(x as i32);
            *dirty = true;
        }
        mouse_state.cancel_drag();
        return Ok(EventFlow::Continue);
    }
    if coordinates_converted {
        *dirty |= mouse_state.drag_document(
            editor,
            renderer,
            vim,
            *vim_enabled,
            *active_pane,
            x as i32,
            y as i32,
        )?;
    }
    mouse_state.document_drag = None;
    if terminal.is_active() && terminal.output_dragging() && coordinates_converted {
        let offset = renderer.terminal_output_offset_at(&mut *terminal, x as i32, y as i32)?;
        terminal
            .drag_output_to(offset, x as i32, y as i32)
            .map_err(|error| error.to_string())?;
        let open_other_pane = terminal.output_click_opens_other_pane();
        if let Some(action) = terminal.finish_output_drag() {
            terminal.focus_prompt();
            if handle_terminal_action_in_pane(
                action,
                &mut *terminal,
                &mut *editor,
                &mut *other_editor,
                &mut *renderer,
                if open_other_pane {
                    TerminalOpenTarget::OtherPane
                } else {
                    TerminalOpenTarget::CurrentPane
                },
            )? {
                if open_other_pane {
                    *split_mode = true;
                    renderer.set_split_mode(true);
                }
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
        *dirty = true;
    }
    Ok(EventFlow::Continue)
}
