// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Route the new pane separately from the original terminal handlers.
use super::*;
use crate::experimental_terminal::{links::FileLink, pane::Action};

/// SDL accepts whole milliseconds. Rounding a positive frame deadline down to
/// zero repeatedly polls the event queue for the last fraction of every frame.
pub(super) fn wait_timeout(timeout: std::time::Duration) -> std::time::Duration {
    let milliseconds = timeout.as_millis() + u128::from(timeout.subsec_nanos() % 1_000_000 != 0);
    std::time::Duration::from_millis(milliseconds.min(i32::MAX as u128) as u64)
}

pub(super) fn poll(
    renderer: &mut Renderer<'_>,
    vim: &mut VimController,
    other_vim: &mut VimController,
) -> bool {
    let changed = renderer.poll_experimental();
    if let Some(linewise) = renderer.experimental_clipboard_mode() {
        vim.set_clipboard_linewise(linewise);
        other_vim.set_clipboard_linewise(linewise);
    }
    changed
}

pub(super) fn open_link(
    link: FileLink,
    editor: &mut Editor,
    other: &mut Editor,
    renderer: &mut Renderer<'_>,
    terminal: &mut Terminal,
    active: &mut usize,
    split: bool,
    vim: &mut VimController,
    other_vim: &mut VimController,
) {
    if link.path.is_dir() {
        if let Err(error) = renderer.experimental_enter_directory(&link.path) {
            renderer.experimental_status(error);
        }
        return;
    }
    let terminal_pane = renderer.experimental_pane().unwrap_or(*active);
    let destination = if split {
        if link.other_pane {
            1 - terminal_pane
        } else {
            terminal_pane
        }
    } else {
        *active
    };
    let (target, source) = if destination == *active {
        (&mut *editor, &mut *other)
    } else {
        (&mut *other, &mut *editor)
    };
    let opened =
        replace_terminal_document(&link.path.to_string_lossy(), link.read_only, target, source)
            .and_then(|()| {
                target.clear_secondary_cursors();
                if let Some(line) = link.line {
                    move_to_terminal_location(
                        &mut target.document,
                        line,
                        link.column,
                        link.byte_column,
                    )?;
                }
                Ok(())
            });
    if let Err(error) = opened {
        renderer.experimental_status(format!("Could not open {}: {error}", link.path.display()));
        return;
    }
    if !split || destination == terminal_pane {
        renderer.hide_experimental();
    }
    if renderer.terminal_pane() == destination && terminal.is_active() {
        terminal.close_to_editor();
    }
    focus_pane(destination, active, editor, other, vim, other_vim, renderer);
    renderer.set_file_path(editor.path().as_deref());
    renderer.invalidate_scroll_cache();
    renderer.update_cursor(&editor.document);
    renderer.ensure_cursor_visible(&mut editor.document);
}
fn action(context: input::InputContext<'_, '_>, action: Action) {
    match action {
        Action::Handled => {}
        Action::Editor => context.renderer.hide_experimental(),
        Action::CommandBar => context.command_bar.open(":"),
        Action::ClosePane => {
            if !close_focused_pane(
                context.split_mode,
                context.active_pane,
                context.editor,
                context.other_editor,
                context.vim,
                context.other_vim,
                context.renderer,
                context.terminal,
            ) {
                context.renderer.experimental_status(
                    "No split pane to close. Use :quit to close Pötyi.".into(),
                );
            }
        }
        Action::Open(link) => open_link(
            link,
            context.editor,
            context.other_editor,
            context.renderer,
            context.terminal,
            context.active_pane,
            *context.split_mode,
            context.vim,
            context.other_vim,
        ),
    }
    *context.dirty = true;
}
pub(super) fn key(
    context: input::InputContext<'_, '_>,
    key: Keycode,
    mods: Mod,
    repeat: bool,
) -> bool {
    if key == Keycode::Grave
        && ctrl_pressed(mods)
        && !repeat
        && context.renderer.has_experimental()
        && !context.renderer.experimental_focused()
    {
        context.command_bar.close();
        context.search_ui.close();
        let directory = std::env::current_dir().unwrap_or_default();
        if let Err(error) = context.renderer.open_experimental(&directory, None) {
            context.command_bar.show_info(&error);
        }
        *context.dirty = true;
        return true;
    }
    if !context.renderer.experimental_focused() || context.command_bar.is_active() {
        return false;
    }
    if let Some(command) = context
        .key_bindings
        .terminal_clipboard_command(key, mods, false)
    {
        if !repeat {
            context.renderer.experimental_clipboard(
                command == Command::Paste,
                shift_pressed(mods),
                context.clipboard,
            );
            *context.dirty = true;
        }
        return true;
    }
    let event = Event::KeyDown {
        timestamp: 0,
        window_id: 0,
        keycode: Some(key),
        scancode: None,
        keymod: mods,
        repeat,
        which: 0,
        raw: 0,
    };
    let result =
        context
            .renderer
            .experimental_event(&event, mods, context.clipboard, *context.vim_enabled);
    if let Some(result) = result {
        action(context, result);
        return true;
    }
    false
}
#[cfg(test)]
mod wait_tests {
    use super::wait_timeout;
    use std::time::Duration;

    #[test]
    fn positive_sdl_waits_round_up_without_delaying_pending_work() {
        assert_eq!(wait_timeout(Duration::ZERO), Duration::ZERO);
        assert_eq!(
            wait_timeout(Duration::from_nanos(1)),
            Duration::from_millis(1)
        );
        assert_eq!(
            wait_timeout(Duration::from_micros(999)),
            Duration::from_millis(1)
        );
        assert_eq!(
            wait_timeout(Duration::from_millis(16)),
            Duration::from_millis(16)
        );
        assert_eq!(
            wait_timeout(Duration::from_micros(16_001)),
            Duration::from_millis(17)
        );
        assert_eq!(
            wait_timeout(Duration::MAX),
            Duration::from_millis(i32::MAX as u64)
        );
    }
}
pub(super) fn pointer_and_text(
    context: input::InputContext<'_, '_>,
    event: &Event,
    converted: bool,
) -> bool {
    let focused = context.renderer.experimental_focused();
    if focused && matches!(event, Event::TextInput { .. }) && context.pane_keys.consume_text() {
        return true;
    }
    let mods = context.keyboard.mod_state();
    let dragging = context.renderer.experimental_dragging();
    let hit = match event {
        Event::TextInput { .. } => focused && !context.command_bar.is_active(),
        Event::MouseButtonDown { x, y, .. } => {
            converted
                && context
                    .renderer
                    .experimental_hit(*x as i32, *y as i32, context.command_bar)
        }
        Event::MouseButtonUp { x, y, .. } | Event::MouseMotion { x, y, .. } => {
            converted
                && (dragging
                    || context
                        .renderer
                        .experimental_hit(*x as i32, *y as i32, context.command_bar))
        }
        Event::Window {
            win_event: WindowEvent::FocusLost,
            ..
        } => true,
        Event::MouseWheel {
            mouse_x, mouse_y, ..
        } => {
            converted
                && context.renderer.experimental_hit(
                    *mouse_x as i32,
                    *mouse_y as i32,
                    context.command_bar,
                )
        }
        _ => false,
    };
    if !hit {
        return false;
    }
    if let Event::MouseButtonDown { x, y, .. } = event {
        focus_pane_preserving_view(
            context.renderer.experimental_pane().unwrap(),
            context.active_pane,
            context.editor,
            context.other_editor,
            context.vim,
            context.other_vim,
            context.renderer,
        );
        if let Some(result) =
            context
                .renderer
                .experimental_button(*x as i32, *y as i32, context.command_bar)
        {
            action(context, result);
            return true;
        }
    }
    if let Some(result) =
        context
            .renderer
            .experimental_event(event, mods, context.clipboard, *context.vim_enabled)
    {
        action(context, result);
        return true;
    }
    false
}
