// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Pane focus, keyboard mode, and terminal file navigation.
use super::*;

/// Refresh both renderer caches after synchronizing another view of this buffer.
pub(crate) fn synchronize_pane_views(
    editor: &mut Editor,
    other: &mut Editor,
    renderer: &mut Renderer<'_>,
) -> io::Result<()> {
    if !editor.shares_document_with(other) {
        return Ok(());
    }
    let text_changed = editor.document.revision() != other.document.revision();
    let path_changed = editor.path != other.path;
    Editor::synchronize_views(editor, other)?;
    if text_changed || path_changed {
        renderer.invalidate_scroll_cache();
        renderer.update_cursor(&editor.document);
        if path_changed {
            renderer.set_file_path(editor.path.as_deref());
        }
        renderer.swap_view();
        renderer.invalidate_scroll_cache();
        renderer.update_cursor(&other.document);
        if path_changed {
            renderer.set_file_path(other.path.as_deref());
        }
        renderer.swap_view();
    }
    Ok(())
}

pub(crate) fn save_as_in_pane(
    editor: &mut Editor,
    other_editor: &Editor,
    path: &str,
    overwrite: bool,
) -> io::Result<()> {
    if file_is_open_in(path, other_editor) && !editor.shares_document_with(other_editor) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "That file is open in the other pane. Choose a different destination.",
        ));
    }
    editor.save_as(path, overwrite)
}

pub(crate) fn parse_location(arg: &str) -> Option<(&str, usize, Option<usize>)> {
    let (before_last, last) = arg.rsplit_once(':')?;

    let last_number = last.parse::<usize>().ok()?;

    if let Some((path, line)) = before_last.rsplit_once(':')
        && let Ok(line) = line.parse::<usize>()
    {
        return Some((path, line, Some(last_number)));
    }

    Some((before_last, last_number, None))
}

pub(crate) fn editor_mode_label(editor: &Editor, vim: &VimController) -> Option<&'static str> {
    match editor.config.keybinding_mode {
        KeybindingMode::Vim => Some(vim.mode_label()),
        KeybindingMode::Emacs => Some(editor.emacs.label()),
        KeybindingMode::Conventional => None,
    }
}

pub(crate) fn focus_pane(
    target: usize,
    active_pane: &mut usize,
    editor: &mut Editor,
    other_editor: &mut Editor,
    vim: &mut VimController,
    other_vim: &mut VimController,
    renderer: &mut Renderer<'_>,
) {
    if target.min(1) == *active_pane {
        return;
    }
    focus_pane_preserving_view(
        target,
        active_pane,
        editor,
        other_editor,
        vim,
        other_vim,
        renderer,
    );
    renderer.ensure_cursor_visible(&mut editor.document);
}

/// Pointer focus keeps the document's independent viewport; navigation can
/// explicitly reveal the cursor via focus_pane instead.
pub(crate) fn focus_pane_preserving_view(
    target: usize,
    active_pane: &mut usize,
    editor: &mut Editor,
    other_editor: &mut Editor,
    vim: &mut VimController,
    other_vim: &mut VimController,
    renderer: &mut Renderer<'_>,
) {
    let target = target.min(1);

    if target == *active_pane {
        return;
    }

    let shared = editor.shares_document_with(other_editor);
    if shared {
        vim.pause_shared_view(editor);
        // Input dispatch has already synchronized the text. Finish the outgoing
        // Vim group before giving the incoming view its shared undo history.
        other_editor.undo_stack.clone_from(&editor.undo_stack);
        other_editor.redo_stack.clone_from(&editor.redo_stack);
        editor.multi_edit_group = None;
        other_editor.multi_edit_group = None;
    }
    editor.emacs.cancel_sequence();
    other_editor.emacs.cancel_sequence();
    std::mem::swap(editor, other_editor);
    std::mem::swap(vim, other_vim);
    if shared {
        vim.resume_shared_view(editor);
    }
    renderer.swap_view();
    *active_pane = target;
    renderer.set_active_pane(target);
    renderer.set_file_path(editor.path.as_deref());
    renderer.invalidate_scroll_cache();
    renderer.update_cursor(&editor.document);
    renderer.set_mode_label(editor_mode_label(editor, vim));
}

/// Hide the focused pane without discarding its document or undo history.
pub(crate) fn close_focused_pane(
    split_mode: &mut bool,
    active_pane: &mut usize,
    editor: &mut Editor,
    other_editor: &mut Editor,
    vim: &mut VimController,
    other_vim: &mut VimController,
    renderer: &mut Renderer<'_>,
    terminal: &mut Terminal,
) -> bool {
    if !*split_mode {
        return false;
    }
    if renderer.terminal_focused(terminal) {
        terminal.close_to_editor();
    }
    *split_mode = false;
    renderer.set_split_mode(false);
    focus_pane_preserving_view(
        1 - *active_pane,
        active_pane,
        editor,
        other_editor,
        vim,
        other_vim,
        renderer,
    );
    true
}

/// Open a folder as a left-hand listing and a right-hand editor. A dropped
/// folder may replace a clean right document, but never an unsaved one.
pub(crate) fn open_folder_workspace(
    root: &std::path::Path,
    split_mode: &mut bool,
    active_pane: &mut usize,
    editor: &mut Editor,
    other_editor: &mut Editor,
    vim: &mut VimController,
    other_vim: &mut VimController,
    renderer: &mut Renderer<'_>,
    terminal: &mut Terminal,
) -> Result<(), String> {
    if terminal.is_running() {
        return Err("Wait for the running terminal command before opening a folder".into());
    }
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    // Check access before replacing a document or changing the pane layout.
    std::fs::read_dir(&root).map_err(|error| error.to_string())?;
    let right = if *active_pane == 1 {
        &*editor
    } else {
        &*other_editor
    };
    let empty = if right.dirty {
        None
    } else {
        Some(Editor::new(right.config.clone()).map_err(|error| error.to_string())?)
    };

    terminal.clear().map_err(|error| error.to_string())?;
    *split_mode = true;
    renderer.set_split_mode(true);
    focus_pane(
        1,
        active_pane,
        editor,
        other_editor,
        vim,
        other_vim,
        renderer,
    );
    if let Some(empty) = empty {
        *editor = empty;
        vim.reset();
        renderer.set_file_path(None);
        renderer.set_mode_label(editor_mode_label(editor, vim));
        renderer.update_cursor(&editor.document);
        renderer.ensure_cursor_visible(&mut editor.document);
    }
    focus_pane(
        0,
        active_pane,
        editor,
        other_editor,
        vim,
        other_vim,
        renderer,
    );
    renderer.place_terminal_in_active_pane();
    terminal.set_listing_width(renderer.terminal_columns());
    terminal.open(None);
    terminal
        .enter_directory(&root)
        .map_err(|error| error.to_string())?;
    focus_pane(
        1,
        active_pane,
        editor,
        other_editor,
        vim,
        other_vim,
        renderer,
    );
    Ok(())
}

pub(crate) fn apply_keybinding_mode(
    mode: KeybindingMode,
    vim_enabled: &mut bool,
    editor: &mut Editor,
    other_editor: &mut Editor,
    vim: &mut VimController,
    other_vim: &mut VimController,
    renderer: &mut Renderer<'_>,
) {
    editor.clear_secondary_cursors();
    other_editor.clear_secondary_cursors();
    editor.emacs.reset();
    other_editor.emacs.reset();
    *vim_enabled = mode == KeybindingMode::Vim;

    if *vim_enabled {
        for target in [&mut *editor, &mut *other_editor] {
            let cursor = &mut target.document.cursor;
            cursor.anchor = cursor.position;
            cursor.anchor_line = cursor.line;
            cursor.anchor_column = cursor.column;
        }
        vim.reset();
        other_vim.reset();
        renderer.set_mode_label(Some(vim.mode_label()));
    } else {
        vim.deactivate(editor);
        other_vim.deactivate(other_editor);
        renderer.set_mode_label(if mode == KeybindingMode::Emacs {
            Some("Emacs")
        } else {
            None
        });
    }
}

/// Return whether the requested file belongs in the other pane. Reuse an
/// already-open document before considering replacement, preserving its edits.
pub(crate) fn open_terminal_document(
    path: &str,
    read_only: bool,
    editor: &mut Editor,
    other_editor: &mut Editor,
) -> io::Result<bool> {
    if file_is_open_in(path, editor) {
        return Ok(false);
    }
    if file_is_open_in(path, other_editor) {
        return Ok(true);
    }
    let focus_other = editor.dirty;
    let (target, source) = if focus_other { (other_editor, editor) } else { (editor, other_editor) };
    if target.dirty && target.path.is_none() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Both panes have unsaved changes. Give the selected document a filename with :save-as before opening another.",
        ));
    }
    replace_terminal_document(path, read_only, target, source)?;
    Ok(focus_other)
}

/// Stage the destination before saving, so broken links leave the current file
/// untouched. Save failures must also leave the document and undo history intact.
pub(crate) fn replace_terminal_document(
    path: &str,
    read_only: bool,
    target: &mut Editor,
    source: &mut Editor,
) -> io::Result<()> {
    if file_is_open_in(path, target) {
        return Ok(());
    }
    if target.dirty && target.path.is_none() {
        return Err(io::Error::other(
            "The selected pane has unsaved changes in an unnamed document. Give it a filename with :save-as before opening another file.",
        ));
    }
    let replacement = if file_is_open_in(path, source) {
        source.duplicate_view()
    } else {
        let mut replacement = Editor::new(target.config.clone())?;
        replacement.open(path)?;
        replacement.read_only = read_only;
        replacement
    };
    if target.dirty {
        target.save().map_err(|error| io::Error::new(error.kind(), format!(
            "Could not save {} before switching: {error}. Your edits are still open.",
            terminal::display_path(target.path.as_deref().expect("named document")),
        )))?;
        // Propagate the saved state before this view detaches from a shared file.
        Editor::synchronize_views(target, source)?;
    }
    *target = replacement;
    Ok(())
}

pub(crate) fn move_to_terminal_location(
    document: &mut PieceTable,
    line: usize,
    column: Option<usize>,
    byte_column: bool,
) -> io::Result<()> {
    let column = column.unwrap_or(1).saturating_sub(1);
    let column = if byte_column {
        let text = document.line_text(line.saturating_sub(1))?;
        text.char_indices()
            .take_while(|(offset, _)| *offset < column)
            .count()
    } else {
        column
    };
    document.move_cursor_to_line_column(line.saturating_sub(1), column)
}

pub(crate) fn handle_terminal_action(
    action: TerminalAction,
    terminal: &mut Terminal,
    editor: &mut Editor,
    other_editor: &mut Editor,
    renderer: &mut Renderer<'_>,
) -> Result<bool, String> {
    handle_terminal_action_in_pane(
        action,
        terminal,
        editor,
        other_editor,
        renderer,
        TerminalOpenTarget::Automatic,
    )
}

#[derive(Clone, Copy)]
pub(crate) enum TerminalOpenTarget {
    Automatic,
    CurrentPane,
    OtherPane,
}

pub(crate) fn handle_terminal_action_in_pane(
    action: TerminalAction,
    terminal: &mut Terminal,
    editor: &mut Editor,
    other_editor: &mut Editor,
    renderer: &mut Renderer<'_>,
    destination: TerminalOpenTarget,
) -> Result<bool, String> {
    let read_only = matches!(action, TerminalAction::View(_));
    let (path, line, column, byte_column, read_only) = match action {
        TerminalAction::None => return Ok(false),
        TerminalAction::Commit(commit) => {
            if let Err(error) = terminal.open_commit(commit) {
                terminal.set_status(error.to_string());
            }
            return Ok(false);
        }
        TerminalAction::EnterDirectory(path) => {
            if let Err(error) = terminal.enter_directory(&path) {
                terminal.set_status(error.to_string());
            }
            return Ok(false);
        }
        TerminalAction::ListedFile(path) => (path, None, None, false, false),
        TerminalAction::Location(path, location) => (
            path,
            Some(location.line),
            location.column,
            location.byte_column,
            false,
        ),
        TerminalAction::Edit(value) | TerminalAction::View(value) => {
            let (path_text, line, column) = match parse_location(&value) {
                Some((path, line, column)) => (path, Some(line), column),
                None => (value.as_str(), None, None),
            };
            let path = match terminal.resolve_path(path_text) {
                Ok(path) => path,
                Err(error) => {
                    terminal.set_status(error.to_string());
                    return Ok(false);
                }
            };
            (path, line, column, false, read_only)
        }
    };

    let display_path = path.to_string_lossy().into_owned();

    let split_terminal = renderer.terminal_is_split(terminal);
    let opened = match destination {
        TerminalOpenTarget::Automatic if split_terminal => {
            open_terminal_document(&display_path, read_only, other_editor, editor)
                .map(|in_terminal_pane| !in_terminal_pane)
        }
        TerminalOpenTarget::Automatic => {
            open_terminal_document(&display_path, read_only, editor, other_editor)
        }
        TerminalOpenTarget::CurrentPane | TerminalOpenTarget::OtherPane => {
            let other = matches!(destination, TerminalOpenTarget::OtherPane);
            let (target, source) = if other {
                (&mut *other_editor, &mut *editor)
            } else {
                (&mut *editor, &mut *other_editor)
            };
            replace_terminal_document(&display_path, read_only, target, source).map(|()| other)
        }
    };
    let focus_other = match opened {
        Ok(focus_other) => focus_other,
        Err(error) => {
            terminal.set_status(format!(
                "Could not open {}: {error}",
                terminal::display_path(&path)
            ));
            return Ok(false);
        }
    };
    let target = if focus_other { other_editor } else { editor };
    target.clear_secondary_cursors();

    if let Some(line) = line {
        if let Err(error) =
            move_to_terminal_location(&mut target.document, line, column, byte_column)
        {
            terminal.set_status(error.to_string());
        }
    }

    if !focus_other {
        renderer.set_file_path(target.path.as_deref());
        renderer.invalidate_scroll_cache();
        renderer.update_cursor(&target.document);
        renderer.ensure_cursor_visible(&mut target.document);
    }
    if !focus_other || !(split_terminal || matches!(destination, TerminalOpenTarget::OtherPane)) {
        terminal.close_to_editor();
    }

    Ok(focus_other)
}

// ==========================================================================
// Main
// ==========================================================================
