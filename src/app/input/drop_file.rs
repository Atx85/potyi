// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn drop_file(
    context: InputContext<'_, '_>,
    filename: &str,
) -> Result<EventFlow, String> {
    let InputContext {
        editor,
        other_editor,
        renderer,
        vim,
        other_vim,
        search_ui,
        command_bar,
        dirty,
        active_pane,
        vim_enabled,
        ..
    } = context;

    /*
     * Opening a document also terminates the active search.
     * The query itself is preserved by SearchUi.
     */
    search_ui.close();
    command_bar.close();

    if file_is_open_in(&filename, &*other_editor) {
        focus_pane(
            1 - *active_pane,
            &mut *active_pane,
            &mut *editor,
            &mut *other_editor,
            &mut *vim,
            &mut *other_vim,
            &mut *renderer,
        );
        *dirty = true;
        return Ok(EventFlow::Continue);
    }

    if let Err(error) = editor.open(&filename) {
        command_bar.open(":");
        command_bar.show_info(&format!("Could not open {filename}: {error}"));
        *dirty = true;
        return Ok(EventFlow::Continue);
    }

    if *vim_enabled {
        vim.reset();
        renderer.set_mode_label(Some(vim.mode_label()));
    }

    if let Some(path) = editor.path.as_deref() {
        renderer.set_file_path(Some(path));
    }

    renderer.invalidate_scroll_cache();

    renderer.update_cursor(&editor.document);

    renderer.ensure_cursor_visible(&mut editor.document);

    *dirty = true;

    Ok(EventFlow::Continue)
}
