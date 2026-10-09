// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! One per-event boundary: background delivery, coordinate conversion, input.
use super::*;

pub(super) fn dispatch(
    context: input::InputContext<'_, '_>,
    mut event: Event,
    terminal_frames: &mut terminal::FrameSchedule,
) -> Result<input::EventFlow, String> {
    context
        .command_bar
        .refresh_formatters(context.editor.path().as_deref());
    let background = background::dispatch(
        background::BackgroundContext {
            editor: &mut *context.editor,
            other_editor: &mut *context.other_editor,
            renderer: &mut *context.renderer,
            terminal: &mut *context.terminal,
            vim: &mut *context.vim,
            other_vim: &mut *context.other_vim,
            lsp_ui: &mut *context.lsp_ui,
            search_ui: &mut *context.search_ui,
            command_bar: &mut *context.command_bar,
            dirty: &mut *context.dirty,
            active_pane: &mut *context.active_pane,
            vim_enabled: *context.vim_enabled,
            terminal_frames,
        },
        &event,
    )?;
    if background == background::BackgroundFlow::Handled {
        return Ok(input::EventFlow::Continue);
    }
    let coordinates_converted = context.renderer.convert_event_coordinates(&mut event);
    input::dispatch(context, event, coordinates_converted)
}
