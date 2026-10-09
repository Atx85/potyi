// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn handle(
    context: MouseContext<'_, '_>,
    x: f32,
    y: f32,
    mouse_x: f32,
    mouse_y: f32,
    direction: sdl3::mouse::MouseWheelDirection,
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
        search_ui,
        command_bar,
        keyboard,
        dirty,
        active_pane,
        ..
    } = context;
    if !coordinates_converted {
        return Ok(EventFlow::Continue);
    }
    if renderer.terminal_visible(terminal)
        && renderer.pane_at(mouse_x as i32) == renderer.terminal_pane()
        && (!command_bar.is_active()
            || renderer.command_bar_hit_at(command_bar, mouse_x as i32, mouse_y as i32)
                == CommandBarHit::Outside)
    {
        let sign = if direction == sdl3::mouse::MouseWheelDirection::Flipped {
            -1.0
        } else {
            1.0
        };
        let (_, rows) = mouse_state.wheel[2].take(0.0, y as f64 * sign * 3.0);
        terminal.scroll(rows);
        *dirty = true;
        return Ok(EventFlow::Continue);
    }

    if command_bar.is_active()
        && renderer.command_bar_hit_at(&*command_bar, mouse_x as i32, mouse_y as i32)
            != CommandBarHit::Outside
        && !matches!(
            command_bar.parse(),
            Ok(ParsedCommand::Find { .. } | ParsedCommand::Replace { .. })
        )
    {
        if y != 0.0 {
            let sign = if direction == sdl3::mouse::MouseWheelDirection::Flipped {
                -1.0
            } else {
                1.0
            };
            let (_, rows) = mouse_state.wheel[3].take(0.0, -y as f64 * sign * 3.0);
            command_bar.scroll_suggestions(rows);
            *dirty = true;
        }
        return Ok(EventFlow::Continue);
    }

    /*
     * Mouse wheel still controls the document while search
     * is open. This lets the user inspect nearby matches
     * without closing the search bar.
     */
    if let Some(pane) = renderer.pane_at_point(mouse_x as i32, mouse_y as i32)
        && pane != *active_pane
    {
        focus_pane_preserving_view(
            pane,
            active_pane,
            editor,
            other_editor,
            vim,
            other_vim,
            renderer,
        );
        search_ui.close();
    }
    let sign = if direction == sdl3::mouse::MouseWheelDirection::Flipped {
        -1.0
    } else {
        1.0
    };
    let (x, y) = if x == 0.0
        && keyboard
            .mod_state()
            .intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD)
    {
        (y, 0.0)
    } else {
        (x, y)
    };
    let (pixels, rows) =
        mouse_state.wheel[*active_pane].take(-x as f64 * sign * 40.0, -y as f64 * sign);
    renderer.scroll_by(rows, &mut editor.document);
    renderer.scroll_horizontal(pixels);

    *dirty = true;
    Ok(EventFlow::Continue)
}
