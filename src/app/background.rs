// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Background events retain their distinct scheduling and LSP follow-up rules.
use super::*;

pub(super) struct BackgroundContext<'a, 'font> {
    pub editor: &'a mut Editor,
    pub other_editor: &'a mut Editor,
    pub renderer: &'a mut Renderer<'font>,
    pub terminal: &'a mut Terminal,
    pub vim: &'a mut VimController,
    pub other_vim: &'a mut VimController,
    pub lsp_ui: &'a mut lsp_ui::LspUi,
    pub search_ui: &'a mut SearchUi,
    pub command_bar: &'a mut CommandBar,
    pub dirty: &'a mut bool,
    pub active_pane: &'a mut usize,
    pub vim_enabled: bool,
    pub terminal_frames: &'a mut terminal::FrameSchedule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BackgroundFlow {
    Pass,
    Handled,
}

pub(super) fn dispatch(
    context: BackgroundContext<'_, '_>,
    event: &Event,
) -> Result<BackgroundFlow, String> {
    if let Some(event) = event.as_user_event_type::<TerminalEvent>() {
        if context
            .terminal
            .handle_event(event)
            .map_err(|error| error.to_string())?
            && context.renderer.terminal_visible(context.terminal)
        {
            context.terminal_frames.changed();
        }
        return Ok(BackgroundFlow::Handled);
    }
    if let Some(event) = event.as_user_event_type::<lsp_setup::Event>() {
        context.lsp_ui.accept_setup(
            event,
            context.editor,
            context.other_editor,
            context.command_bar,
        );
        *context.dirty = true;
        return Ok(BackgroundFlow::Handled);
    }
    if let Some(event) = event.as_user_event_type::<lsp::Event>() {
        accept_lsp(context, event)?;
        return Ok(BackgroundFlow::Handled);
    }
    Ok(BackgroundFlow::Pass)
}

fn accept_lsp(context: BackgroundContext<'_, '_>, event: lsp::Event) -> Result<(), String> {
    let BackgroundContext {
        editor,
        other_editor,
        renderer,
        terminal,
        vim,
        other_vim,
        lsp_ui,
        search_ui,
        command_bar,
        dirty,
        active_pane,
        vim_enabled,
        ..
    } = context;
    lsp_ui.validate_completion(
        editor,
        other_editor,
        !renderer.terminal_focused(terminal)
            && !command_bar.is_active()
            && (!vim_enabled || vim.mode() == vim::VimMode::Insert),
    );
    let outcome = lsp_ui.accept(event, editor, other_editor, command_bar);
    synchronize_pane_views(editor, other_editor, renderer).map_err(|e| e.to_string())?;
    if outcome.document_changed && vim_enabled {
        vim.finish_formatting(editor);
    }
    if outcome.focus_other {
        focus_pane(
            1 - *active_pane,
            active_pane,
            editor,
            other_editor,
            vim,
            other_vim,
            renderer,
        );
    }
    if outcome.cursor_changed {
        search_ui.close();
        if vim_enabled && outcome.document_reloaded {
            vim.reset();
        }
        renderer.set_file_path(editor.path().as_deref());
        renderer.set_mode_label(editor_mode_label(editor, vim));
        renderer.invalidate_scroll_cache();
        renderer.update_cursor(&editor.document);
        renderer.ensure_cursor_visible(&mut editor.document);
    }
    *dirty = true;
    Ok(())
}
