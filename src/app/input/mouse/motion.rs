// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn handle(
    context: MouseContext<'_, '_>,
    x: f32,
    y: f32,
    mousestate: sdl3::mouse::MouseState,
    coordinates_converted: bool,
) -> Result<EventFlow, String> {
    let MouseContext {
        mouse_state,
        editor,
        renderer,
        terminal,
        vim,
        command_bar,
        dirty,
        active_pane,
        vim_enabled,
        ..
    } = context;
    if !mousestate.left() {
        if mouse_state.document_drag.is_some() || mouse_state.split_drag {
            mouse_state.cancel_drag();
        }
        terminal.cancel_output_drag();
    }
    if coordinates_converted {
        let window_cursor = renderer.window_resize_cursor(x as i32, y as i32);
        let control = if window_cursor.is_some() {
            WindowControl::None
        } else {
            renderer.window_control_at(x as i32, y as i32)
        };
        *dirty |= renderer.set_window_control_hover(control);
        let cursor = if mouse_state.split_drag {
            Some(sdl3::mouse::SystemCursor::SizeWE)
        } else {
            window_cursor.or_else(|| {
                renderer
                    .split_divider_hit(x as i32, y as i32)
                    .then_some(sdl3::mouse::SystemCursor::SizeWE)
            })
        };
        mouse_state.show_cursor(cursor);
        if mouse_state.split_drag {
            renderer.resize_split(x as i32);
            terminal.set_listing_width(renderer.terminal_columns());
            *dirty = true;
            return Ok(EventFlow::Continue);
        }
        if mouse_state.drag_document(
            editor,
            renderer,
            vim,
            *vim_enabled,
            *active_pane,
            x as i32,
            y as i32,
        )? {
            *dirty = true;
            return Ok(EventFlow::Continue);
        }
    }
    if terminal.is_active() && terminal.output_dragging() && coordinates_converted {
        let offset = renderer.terminal_output_offset_at(&mut *terminal, x as i32, y as i32)?;
        terminal
            .drag_output_to(offset, x as i32, y as i32)
            .map_err(|error| error.to_string())?;
        *dirty = true;
        return Ok(EventFlow::Continue);
    }
    if coordinates_converted
        && command_bar.is_active()
        && let CommandBarHit::Suggestion(index) =
            renderer.command_bar_hit_at(&*command_bar, x as i32, y as i32)
        && command_bar.select_suggestion(index)
    {
        *dirty = true;
    }
    Ok(EventFlow::Continue)
}
