// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Ordered SDL input routing. Handlers borrow live state without copying
//! documents or histories; Continue and Quit replace the former loop jumps.
use super::*;
mod drop_file;
mod keyboard;
mod mouse;
mod text;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EventFlow {
    Continue,
    Quit,
}

pub(super) struct InputContext<'a, 'font> {
    pub editor: &'a mut Editor,
    pub other_editor: &'a mut Editor,
    pub renderer: &'a mut Renderer<'font>,
    pub terminal: &'a mut Terminal,
    pub vim: &'a mut VimController,
    pub other_vim: &'a mut VimController,
    pub emacs: &'a mut emacs::Controller,
    pub lsp_ui: &'a mut lsp_ui::LspUi,
    pub search_ui: &'a mut SearchUi,
    pub command_bar: &'a mut CommandBar,
    pub key_bindings: &'a KeyBindings,
    pub clipboard: &'a sdl3::clipboard::ClipboardUtil,
    pub event_subsystem: &'a sdl3::EventSubsystem,
    pub dirty: &'a mut bool,
    pub split_mode: &'a mut bool,
    pub active_pane: &'a mut usize,
    pub vim_enabled: &'a mut bool,
}

impl<'a, 'font> InputContext<'a, 'font> {
    // Sequential stages borrow the same state; no documents or controllers are copied.
    fn reborrow(&mut self) -> InputContext<'_, 'font> {
        InputContext {
            editor: &mut *self.editor,
            other_editor: &mut *self.other_editor,
            renderer: &mut *self.renderer,
            terminal: &mut *self.terminal,
            vim: &mut *self.vim,
            other_vim: &mut *self.other_vim,
            emacs: &mut *self.emacs,
            lsp_ui: &mut *self.lsp_ui,
            search_ui: &mut *self.search_ui,
            command_bar: &mut *self.command_bar,
            key_bindings: self.key_bindings,
            clipboard: self.clipboard,
            event_subsystem: self.event_subsystem,
            dirty: &mut *self.dirty,
            split_mode: &mut *self.split_mode,
            active_pane: &mut *self.active_pane,
            vim_enabled: &mut *self.vim_enabled,
        }
    }
}

pub(super) fn dispatch(
    context: InputContext<'_, '_>,
    event: Event,
    coordinates_converted: bool,
) -> Result<EventFlow, String> {
    match event {
        Event::DropFile { filename, .. } => drop_file::drop_file(context, &filename),
        Event::KeyDown {
            keycode: Some(key),
            keymod,
            repeat,
            ..
        } => keyboard::key(context, key, keymod, repeat),
        Event::TextInput { text, .. } => text::text(context, &text),
        event @ (Event::MouseButtonDown { .. }
        | Event::MouseButtonUp { .. }
        | Event::MouseMotion { .. }
        | Event::MouseWheel { .. }) => mouse::mouse(context, event, coordinates_converted),
        Event::Quit { .. } => Ok(EventFlow::Quit),
        Event::Window {
            win_event:
                WindowEvent::Resized(_, _)
                | WindowEvent::PixelSizeChanged(_, _)
                | WindowEvent::DisplayChanged(_)
                | WindowEvent::Exposed
                | WindowEvent::Restored
                | WindowEvent::Maximized,
            ..
        }
        | Event::Display {
            display_event: DisplayEvent::ContentScaleChanged,
            ..
        } => {
            context.renderer.update_window_size()?;
            *context.dirty = true;
            Ok(EventFlow::Continue)
        }
        _ => Ok(EventFlow::Continue),
    }
}
