// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

use crate::piece_table::MouseSelection;

mod command_bar;
mod document;
mod motion;
mod release;
mod terminal;
mod wheel;

struct DocumentDrag {
    pane: usize,
    revision: u64,
    selection: MouseSelection,
    start: (i32, i32),
    moved: bool,
    pointer: (i32, i32),
    next_scroll: Option<Instant>,
}

#[derive(Default)]
struct WheelRemainder {
    x: f64,
    y: f64,
}

impl WheelRemainder {
    fn take(&mut self, x: f64, y: f64) -> (i32, isize) {
        fn accumulate(remainder: &mut f64, delta: f64) -> f64 {
            if !delta.is_finite() {
                return 0.0;
            }
            if delta != 0.0 && delta.signum() != remainder.signum() {
                *remainder = 0.0;
            }
            *remainder += delta;
            let whole = remainder.trunc();
            *remainder -= whole;
            whole
        }
        (
            accumulate(&mut self.x, x) as i32,
            accumulate(&mut self.y, y) as isize,
        )
    }
}

#[derive(Default)]
pub(crate) struct MouseState {
    document_drag: Option<DocumentDrag>,
    split_drag: bool,
    wheel: [WheelRemainder; 4], // left editor, right editor, terminal, command bar
    cursors: [Option<sdl3::mouse::Cursor>; 5],
    cursor_kind: Option<sdl3::mouse::SystemCursor>,
}

impl MouseState {
    #[cfg(test)]
    pub(super) fn cursor_kind(&self) -> Option<sdl3::mouse::SystemCursor> {
        self.cursor_kind
    }

    pub(crate) fn wait_timeout(&self) -> Duration {
        self.document_drag
            .as_ref()
            .and_then(|drag| drag.next_scroll)
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::MAX)
    }

    pub(crate) fn tick(
        &mut self,
        editor: &mut Editor,
        renderer: &mut Renderer<'_>,
        vim: &mut VimController,
        vim_enabled: bool,
        active_pane: usize,
    ) -> Result<bool, String> {
        let Some(drag) = &self.document_drag else {
            return Ok(false);
        };
        if drag
            .next_scroll
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            let (x, y) = drag.pointer;
            return self.drag_document(editor, renderer, vim, vim_enabled, active_pane, x, y);
        }
        Ok(false)
    }

    pub(super) fn cancel_drag(&mut self) {
        self.document_drag = None;
        self.split_drag = false;
        self.reset_cursor();
    }

    pub(super) fn reset_cursor(&mut self) {
        self.show_cursor(None);
    }

    fn show_resize_cursor(&mut self, resize: bool) {
        self.show_cursor(if resize {
            Some(sdl3::mouse::SystemCursor::SizeWE)
        } else {
            None
        });
    }

    fn show_cursor(&mut self, kind: Option<sdl3::mouse::SystemCursor>) {
        use sdl3::mouse::{Cursor, SystemCursor::*};
        let kind = kind.unwrap_or(Arrow);
        if self.cursor_kind == Some(kind) {
            return;
        }
        self.cursor_kind = Some(kind);
        let index = match kind {
            SizeWE => 1,
            SizeNS => 2,
            SizeNWSE => 3,
            SizeNESW => 4,
            _ => 0,
        };
        let cursor = &mut self.cursors[index];
        if cursor.is_none() {
            *cursor = Cursor::from_system(kind).ok();
        }
        if let Some(cursor) = cursor {
            cursor.set();
        }
    }

    fn drag_document(
        &mut self,
        editor: &mut Editor,
        renderer: &mut Renderer<'_>,
        vim: &mut VimController,
        vim_enabled: bool,
        active_pane: usize,
        x: i32,
        y: i32,
    ) -> Result<bool, String> {
        let Some(drag) = &mut self.document_drag else {
            return Ok(false);
        };
        if drag.pane != active_pane || drag.revision != editor.document.revision() {
            self.document_drag = None;
            return Ok(false);
        }
        drag.moved |= (x - drag.start.0).abs() >= 3 || (y - drag.start.1).abs() >= 3;
        if !drag.moved {
            return Ok(true);
        }
        drag.pointer = (x, y);
        drag.next_scroll = renderer
            .selection_drag_outside(x, y)
            .then(|| Instant::now() + Duration::from_millis(40));
        if let Some((line, column)) = renderer.drag_cursor_target(&mut editor.document, x, y)? {
            editor
                .document
                .move_cursor_to_line_column(line, column)
                .map_err(|e| e.to_string())?;
            let position = editor.document.cursor.position;
            let (cursor, anchor) = drag
                .selection
                .endpoints(&mut editor.document, position)
                .map_err(|e| e.to_string())?;
            editor
                .set_cursor_and_anchor(cursor, anchor)
                .map_err(|e| e.to_string())?;
            if vim_enabled {
                vim.mouse_selection_changed(editor);
                renderer.set_mode_label(Some(vim.mode_label()));
            }
            renderer.update_cursor(&editor.document);
        }
        Ok(true)
    }
}

pub(super) struct MouseContext<'a, 'font> {
    pub mouse_state: &'a mut MouseState,
    pub editor: &'a mut Editor,
    pub other_editor: &'a mut Editor,
    pub renderer: &'a mut Renderer<'font>,
    pub terminal: &'a mut Terminal,
    pub vim: &'a mut VimController,
    pub other_vim: &'a mut VimController,
    pub lsp_ui: &'a mut lsp_ui::LspUi,
    pub search_ui: &'a mut SearchUi,
    pub command_bar: &'a mut CommandBar,
    pub event_subsystem: &'a sdl3::EventSubsystem,
    pub keyboard: &'a sdl3::keyboard::KeyboardUtil,
    pub dirty: &'a mut bool,
    pub split_mode: &'a mut bool,
    pub active_pane: &'a mut usize,
    pub vim_enabled: &'a mut bool,
}

impl<'a, 'font> From<InputContext<'a, 'font>> for MouseContext<'a, 'font> {
    fn from(context: InputContext<'a, 'font>) -> Self {
        Self {
            mouse_state: context.mouse_state,
            editor: context.editor,
            other_editor: context.other_editor,
            renderer: context.renderer,
            terminal: context.terminal,
            vim: context.vim,
            other_vim: context.other_vim,
            lsp_ui: context.lsp_ui,
            search_ui: context.search_ui,
            command_bar: context.command_bar,
            event_subsystem: context.event_subsystem,
            keyboard: context.keyboard,
            dirty: context.dirty,
            split_mode: context.split_mode,
            active_pane: context.active_pane,
            vim_enabled: context.vim_enabled,
        }
    }
}

pub(super) fn mouse(
    context: MouseContext<'_, '_>,
    event: Event,
    coordinates_converted: bool,
) -> Result<EventFlow, String> {
    match event {
        Event::MouseButtonDown {
            mouse_btn: MouseButton::Left,
            clicks,
            x,
            y,
            ..
        } => press(context, x, y, clicks, coordinates_converted),
        Event::MouseButtonUp {
            mouse_btn: MouseButton::Left,
            x,
            y,
            ..
        } => release::handle(context, x, y, coordinates_converted),
        Event::MouseMotion {
            x, y, mousestate, ..
        } => motion::handle(context, x, y, mousestate, coordinates_converted),
        Event::MouseWheel {
            x,
            y,
            mouse_x,
            mouse_y,
            direction,
            ..
        } => wheel::handle(
            context,
            x,
            y,
            mouse_x,
            mouse_y,
            direction,
            coordinates_converted,
        ),
        _ => Ok(EventFlow::Continue),
    }
}

fn press(
    context: MouseContext<'_, '_>,
    x: f32,
    y: f32,
    clicks: u8,
    coordinates_converted: bool,
) -> Result<EventFlow, String> {
    if !coordinates_converted {
        return Ok(EventFlow::Continue);
    }
    context.mouse_state.cancel_drag();
    context.terminal.cancel_output_drag();
    if let Some(cursor) = context.renderer.window_resize_cursor(x as i32, y as i32) {
        context.mouse_state.show_cursor(Some(cursor));
        return Ok(EventFlow::Continue);
    }
    if context.renderer.split_divider_hit(x as i32, y as i32) {
        context.mouse_state.split_drag = true;
        context.mouse_state.show_resize_cursor(true);
        return Ok(EventFlow::Continue);
    }
    let extend = context
        .keyboard
        .mod_state()
        .intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
    let control = context.renderer.window_control_at(x as i32, y as i32);
    if clicks > 1 && !matches!(control, WindowControl::None) {
        return Ok(EventFlow::Continue);
    }
    match control {
        WindowControl::Minimize => {
            context.renderer.window_mut().minimize();
        }
        WindowControl::Maximize => {
            let window = context.renderer.window_mut();
            if window.is_maximized() {
                window.restore();
            } else {
                window.maximize();
            }
            *context.dirty = true;
        }
        WindowControl::Close => return Ok(EventFlow::Quit),
        WindowControl::None => {
            // Preserve hit-test priority: terminal, completion, command bar, document.
            if context.renderer.terminal_visible(context.terminal)
                && y >= window::TITLE_BAR_HEIGHT as f32
                && context.renderer.pane_at(x as i32) == context.renderer.terminal_pane()
                && (!context.command_bar.is_active()
                    || context
                        .renderer
                        .command_bar_hit_at(context.command_bar, x as i32, y as i32)
                        == CommandBarHit::Outside)
            {
                return terminal::press(context, x, y, clicks, extend);
            }
            if let Some(index) = context.renderer.completion_hit_at(x as i32, y as i32) {
                context.lsp_ui.choose_completion(
                    index,
                    context.editor,
                    context.other_editor,
                    context.command_bar,
                );
                *context.dirty = true;
            } else if context.command_bar.is_active()
                && context
                    .renderer
                    .command_bar_hit_at(context.command_bar, x as i32, y as i32)
                    != CommandBarHit::Outside
            {
                return command_bar::press(context, x, y);
            } else {
                return document::press(context, x, y, clicks, extend);
            }
        }
    }
    Ok(EventFlow::Continue)
}
