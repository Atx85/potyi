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
        mouse_state,
        editor,
        other_editor,
        renderer,
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

    let target = renderer.cursor_target_at(&mut editor.document, x as i32, y as i32)?;

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
        let (cursor, anchor) = selection
            .endpoints(&mut editor.document, position)
            .map_err(|e| e.to_string())?;
        editor
            .set_cursor_and_anchor(cursor, anchor)
            .map_err(|e| e.to_string())?;
        mouse_state.document_drag = Some(DocumentDrag {
            pane: *active_pane,
            revision: editor.document.revision(),
            selection,
            start: (x as i32, y as i32),
            moved: false,
            pointer: (x as i32, y as i32),
            next_scroll: None,
        });
        if *vim_enabled {
            vim.mouse_selection_changed(editor);
            renderer.set_mode_label(Some(vim.mode_label()));
        }

        renderer.ensure_cursor_visible(&mut editor.document);
        lsp_ui.document_clicked(&*editor, &*other_editor, &mut *command_bar);

        *dirty = true;
    }
    Ok(EventFlow::Continue)
}
