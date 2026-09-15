// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Pane focus, keyboard mode, and terminal file navigation.
use super::*;

pub(crate) fn save_as_in_pane(
    editor: &mut Editor,
    other_editor: &Editor,
    path: &str,
    overwrite: bool,
) -> io::Result<()> {
    if file_is_open_in(path, other_editor) {
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
    let target = target.min(1);

    if target == *active_pane {
        return;
    }

    editor.emacs.cancel_sequence();
    other_editor.emacs.cancel_sequence();
    std::mem::swap(editor, other_editor);
    std::mem::swap(vim, other_vim);
    renderer.swap_view();
    *active_pane = target;
    renderer.set_active_pane(target);
    renderer.set_file_path(editor.path.as_deref());
    renderer.invalidate_scroll_cache();
    renderer.update_cursor(&editor.document);
    renderer.ensure_cursor_visible(&mut editor.document);
    renderer.set_mode_label(editor_mode_label(editor, vim));
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
    let target = if focus_other { other_editor } else { editor };
    if target.dirty {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Both panes have unsaved changes. Save one file before opening another.",
        ));
    }
    target.open(path)?;
    target.read_only = read_only;
    Ok(focus_other)
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

    let focus_other = match open_terminal_document(&display_path, read_only, editor, other_editor) {
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
    terminal.close_to_editor();

    Ok(focus_other)
}

// ==========================================================================
// Main
// ==========================================================================
