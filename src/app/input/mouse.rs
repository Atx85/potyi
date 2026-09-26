// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

use crate::piece_table::MouseSelection;

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
struct WheelRemainder { x: f64, y: f64 }

impl WheelRemainder {
    fn take(&mut self, x: f64, y: f64) -> (i32, isize) {
        fn accumulate(remainder: &mut f64, delta: f64) -> f64 {
            if !delta.is_finite() { return 0.0; }
            if delta != 0.0 && delta.signum() != remainder.signum() { *remainder = 0.0; }
            *remainder += delta;
            let whole = remainder.trunc();
            *remainder -= whole;
            whole
        }
        (accumulate(&mut self.x, x) as i32, accumulate(&mut self.y, y) as isize)
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
    pub(super) fn cursor_kind(&self) -> Option<sdl3::mouse::SystemCursor> { self.cursor_kind }

    pub(crate) fn wait_timeout(&self) -> Duration {
        self.document_drag.as_ref().and_then(|drag| drag.next_scroll)
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::MAX)
    }

    pub(crate) fn tick(&mut self, editor: &mut Editor, renderer: &mut Renderer<'_>,
        vim: &mut VimController, vim_enabled: bool, active_pane: usize) -> Result<bool, String>
    {
        let Some(drag) = &self.document_drag else { return Ok(false); };
        if drag.next_scroll.is_some_and(|deadline| Instant::now() >= deadline) {
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

    pub(super) fn reset_cursor(&mut self) { self.show_cursor(None); }

    fn show_resize_cursor(&mut self, resize: bool) {
        self.show_cursor(if resize { Some(sdl3::mouse::SystemCursor::SizeWE) } else { None });
    }

    fn show_cursor(&mut self, kind: Option<sdl3::mouse::SystemCursor>) {
        use sdl3::mouse::{Cursor, SystemCursor::*};
        let kind = kind.unwrap_or(Arrow);
        if self.cursor_kind == Some(kind) { return; }
        self.cursor_kind = Some(kind);
        let index = match kind { SizeWE => 1, SizeNS => 2, SizeNWSE => 3, SizeNESW => 4, _ => 0 };
        let cursor = &mut self.cursors[index];
        if cursor.is_none() { *cursor = Cursor::from_system(kind).ok(); }
        if let Some(cursor) = cursor { cursor.set(); }
    }

    fn drag_document(&mut self, editor: &mut Editor, renderer: &mut Renderer<'_>,
        vim: &mut VimController, vim_enabled: bool, active_pane: usize, x: i32, y: i32)
        -> Result<bool, String>
    {
        let Some(drag) = &mut self.document_drag else { return Ok(false); };
        if drag.pane != active_pane || drag.revision != editor.document.revision() {
            self.document_drag = None;
            return Ok(false);
        }
        drag.moved |= (x - drag.start.0).abs() >= 3 || (y - drag.start.1).abs() >= 3;
        if !drag.moved { return Ok(true); }
        drag.pointer = (x, y);
        drag.next_scroll = renderer.selection_drag_outside(x, y)
            .then(|| Instant::now() + Duration::from_millis(40));
        if let Some((line, column)) = renderer.drag_cursor_target(&mut editor.document, x, y)? {
            editor.document.move_cursor_to_line_column(line, column).map_err(|e| e.to_string())?;
            let position = editor.document.cursor.position;
            let (cursor, anchor) = drag.selection.endpoints(&mut editor.document, position).map_err(|e| e.to_string())?;
            editor.set_cursor_and_anchor(cursor, anchor).map_err(|e| e.to_string())?;
            if vim_enabled {
                vim.mouse_selection_changed(editor);
                renderer.set_mode_label(Some(vim.mode_label()));
            }
            renderer.update_cursor(&editor.document);
        }
        Ok(true)
    }
}

pub(super) fn mouse(
    context: InputContext<'_, '_>,
    event: Event,
    coordinates_converted: bool,
) -> Result<EventFlow, String> {
    let InputContext {
        mouse_state,
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
        split_mode,
        active_pane,
        vim_enabled,
        ..
    } = context;
    match event {
        Event::MouseButtonDown {
            mouse_btn: MouseButton::Left,
            clicks,
            x,
            y,
            ..
        } => {
            if !coordinates_converted {
                return Ok(EventFlow::Continue);
            }

            mouse_state.cancel_drag();
            terminal.cancel_output_drag();
            if let Some(cursor) = renderer.window_resize_cursor(x as i32, y as i32) {
                mouse_state.show_cursor(Some(cursor));
                return Ok(EventFlow::Continue);
            }
            if renderer.split_divider_hit(x as i32, y as i32) {
                mouse_state.split_drag = true;
                mouse_state.show_resize_cursor(true);
                return Ok(EventFlow::Continue);
            }
            let extend = keyboard.mod_state().intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
            if clicks > 1 && !matches!(renderer.window_control_at(x as i32, y as i32), WindowControl::None) {
                return Ok(EventFlow::Continue);
            }
            match renderer.window_control_at(x as i32, y as i32) {
                WindowControl::Minimize => {
                    renderer.window_mut().minimize();
                }

                WindowControl::Maximize => {
                    let window = renderer.window_mut();

                    if window.is_maximized() {
                        window.restore();
                    } else {
                        window.maximize();
                    }

                    *dirty = true;
                }

                WindowControl::Close => {
                    return Ok(EventFlow::Quit);
                }

                WindowControl::None => {
                    if renderer.terminal_visible(terminal)
                        && y >= window::TITLE_BAR_HEIGHT as f32
                        && renderer.pane_at(x as i32) == renderer.terminal_pane()
                        && (!command_bar.is_active()
                            || renderer.command_bar_hit_at(command_bar, x as i32, y as i32) == CommandBarHit::Outside)
                    {
                        let clicked_pane = renderer.pane_at(x as i32);
                        focus_pane_preserving_view(clicked_pane, active_pane, editor, other_editor, vim, other_vim, renderer);
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
                                let action = renderer.terminal_action_at(
                                    &mut *terminal,
                                    x as i32,
                                    y as i32,
                                )?;
                                let offset = renderer.terminal_output_offset_at(
                                    &mut *terminal,
                                    x as i32,
                                    y as i32,
                                )?;
                                let open_other_pane = keyboard.mod_state().intersects(
                                    Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LGUIMOD | Mod::RGUIMOD,
                                );
                                terminal
                                    .begin_output_mouse_drag(offset, x as i32, y as i32, action, open_other_pane, clicks, extend)
                                    .map_err(|error| error.to_string())?;
                            }

                            TerminalHit::Outside => {}
                        }

                        *dirty = true;
                    } else if let Some(index) = renderer.completion_hit_at(x as i32, y as i32) {
                        lsp_ui.choose_completion(
                            index,
                            &*editor,
                            &*other_editor,
                            &mut *command_bar,
                        );
                        *dirty = true;
                    } else if command_bar.is_active()
                        && renderer.command_bar_hit_at(&*command_bar, x as i32, y as i32)
                            != CommandBarHit::Outside
                    {
                        let hit = renderer.command_bar_hit_at(&*command_bar, x as i32, y as i32);
                        let hit = if let CommandBarHit::Suggestion(index) = hit
                            && !command_bar.is_info()
                        {
                            command_bar.select_suggestion(index);
                            CommandBarHit::Execute
                        } else {
                            hit
                        };
                        match hit {
                            CommandBarHit::Suggestion(_) => {}

                            CommandBarHit::Input => {
                                let cursor =
                                    renderer.command_bar_cursor_at(&*command_bar, x as i32);

                                command_bar.set_cursor(cursor);
                                *dirty = true;
                            }

                            CommandBarHit::Execute => {
                                let close_vim_search = *vim_enabled
                                    && !command_bar.selects_option_on_enter()
                                    && matches!(
                                        command_bar.parse(),
                                        Ok(ParsedCommand::Find { .. })
                                    );

                                let outcome = if close_vim_search {
                                    let outcome = CommandOutcome {
                                        cursor_changed: search_ui.current_match().is_some(),
                                        ..CommandOutcome::default()
                                    };

                                    vim.accept_search();
                                    command_bar.close();
                                    outcome
                                } else {
                                    execute_command_bar(
                                        &mut *command_bar,
                                        &mut *search_ui,
                                        &mut *editor,
                                        &mut *other_editor,
                                        &mut *renderer,
                                        &mut *terminal,
                                        &mut *vim,
                                        false,
                                        &mut *other_vim,
                                        &mut *lsp_ui,
                                    )
                                };

                                if outcome.quit {
                                    return Ok(EventFlow::Quit);
                                }
                                if outcome.close_pane {
                                    close_focused_pane(split_mode, active_pane, editor, other_editor,
                                        vim, other_vim, renderer, terminal);
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

                                    if search_ui.current_match().is_some() {
                                        renderer.ensure_search_match_visible(
                                            &mut editor.document,
                                            &*search_ui,
                                            &*command_bar,
                                        );
                                    } else {
                                        renderer.ensure_cursor_visible(&mut editor.document);
                                    }
                                }

                                *dirty = true;
                            }

                            CommandBarHit::Outside => {}
                        }
                    } else {
                        lsp_ui.dismiss_completion();
                        if let Some(clicked_pane) = renderer.pane_at_point(x as i32, y as i32)
                            && clicked_pane != *active_pane
                        {
                            focus_pane_preserving_view(
                                clicked_pane,
                                &mut *active_pane,
                                &mut *editor,
                                &mut *other_editor,
                                &mut *vim,
                                &mut *other_vim,
                                &mut *renderer,
                            );
                            search_ui.close();
                        }

                        editor.emacs.reset();
                        if *vim_enabled {
                            vim.handle_document_click();
                            renderer.set_mode_label(Some(vim.mode_label()));
                        }

                        let target =
                            renderer.cursor_target_at(&mut editor.document, x as i32, y as i32)?;

                        if let Some((line, column)) = target {
                            let anchor = extend.then_some(editor.document.cursor.anchor);
                            editor.clear_secondary_cursors();
                            editor
                                .document
                                .move_cursor_to_line_column(line, column)
                                .map_err(|error| error.to_string())?;

                            if *vim_enabled && clicks == 1 && !extend {
                                vim.settle_cursor(&mut *editor)?;
                            }
                            let position = editor.document.cursor.position;
                            let selection = MouseSelection::begin(&mut editor.document, position, clicks, anchor)
                                .map_err(|e| e.to_string())?;
                            let (cursor, anchor) = selection.endpoints(&mut editor.document, position).map_err(|e| e.to_string())?;
                            editor.set_cursor_and_anchor(cursor, anchor).map_err(|e| e.to_string())?;
                            mouse_state.document_drag = Some(DocumentDrag { pane: *active_pane,
                                revision: editor.document.revision(), selection, start: (x as i32, y as i32),
                                moved: false, pointer: (x as i32, y as i32), next_scroll: None });
                            if *vim_enabled {
                                vim.mouse_selection_changed(editor);
                                renderer.set_mode_label(Some(vim.mode_label()));
                            }

                            renderer.ensure_cursor_visible(&mut editor.document);
                            lsp_ui.document_clicked(&*editor, &*other_editor, &mut *command_bar);

                            *dirty = true;
                        }
                    }
                }
            }
        }

        Event::MouseButtonUp {
            mouse_btn: MouseButton::Left,
            x,
            y,
            ..
        } => {
            if mouse_state.split_drag {
                if coordinates_converted {
                    renderer.resize_split(x as i32);
                    *dirty = true;
                }
                mouse_state.cancel_drag();
                return Ok(EventFlow::Continue);
            }
            if coordinates_converted {
                *dirty |= mouse_state.drag_document(editor, renderer, vim, *vim_enabled, *active_pane, x as i32, y as i32)?;
            }
            mouse_state.document_drag = None;
            if terminal.is_active() && terminal.output_dragging() && coordinates_converted {
                let offset =
                    renderer.terminal_output_offset_at(&mut *terminal, x as i32, y as i32)?;
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
                        if open_other_pane { TerminalOpenTarget::OtherPane } else { TerminalOpenTarget::CurrentPane },
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
        }

        Event::MouseMotion { x, y, mousestate, .. } => {
            if !mousestate.left() {
                if mouse_state.document_drag.is_some() || mouse_state.split_drag { mouse_state.cancel_drag(); }
                terminal.cancel_output_drag();
            }
            if coordinates_converted {
                let window_cursor = renderer.window_resize_cursor(x as i32, y as i32);
                let control = if window_cursor.is_some() { WindowControl::None }
                    else { renderer.window_control_at(x as i32, y as i32) };
                *dirty |= renderer.set_window_control_hover(control);
                let cursor = if mouse_state.split_drag {
                    Some(sdl3::mouse::SystemCursor::SizeWE)
                } else {
                    window_cursor.or_else(||
                        renderer.split_divider_hit(x as i32, y as i32).then_some(sdl3::mouse::SystemCursor::SizeWE))
                };
                mouse_state.show_cursor(cursor);
                if mouse_state.split_drag {
                    renderer.resize_split(x as i32);
                    terminal.set_listing_width(renderer.terminal_columns());
                    *dirty = true;
                    return Ok(EventFlow::Continue);
                }
                if mouse_state.drag_document(editor, renderer, vim, *vim_enabled, *active_pane, x as i32, y as i32)? {
                    *dirty = true;
                    return Ok(EventFlow::Continue);
                }
            }
            if terminal.is_active() && terminal.output_dragging() && coordinates_converted {
                let offset =
                    renderer.terminal_output_offset_at(&mut *terminal, x as i32, y as i32)?;
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
        }
        Event::MouseWheel {
            x,
            y,
            mouse_x,
            mouse_y,
            direction,
            ..
        } => {
            if !coordinates_converted {
                return Ok(EventFlow::Continue);
            }
            if renderer.terminal_visible(terminal)
                && renderer.pane_at(mouse_x as i32) == renderer.terminal_pane()
                && (!command_bar.is_active()
                    || renderer.command_bar_hit_at(command_bar, mouse_x as i32, mouse_y as i32) == CommandBarHit::Outside)
            {
                let sign = if direction == sdl3::mouse::MouseWheelDirection::Flipped { -1.0 } else { 1.0 };
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
                    let sign = if direction == sdl3::mouse::MouseWheelDirection::Flipped { -1.0 } else { 1.0 };
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
                focus_pane_preserving_view(pane, active_pane, editor, other_editor, vim, other_vim, renderer);
                search_ui.close();
            }
            let sign = if direction == sdl3::mouse::MouseWheelDirection::Flipped { -1.0 } else { 1.0 };
            let (x, y) = if x == 0.0 && keyboard.mod_state().intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) {
                (y, 0.0)
            } else { (x, y) };
            let (pixels, rows) = mouse_state.wheel[*active_pane].take(-x as f64 * sign * 40.0, -y as f64 * sign);
            renderer.scroll_by(rows, &mut editor.document);
            renderer.scroll_horizontal(pixels);

            *dirty = true;
        }
        _ => (),
    }

    Ok(EventFlow::Continue)
}
