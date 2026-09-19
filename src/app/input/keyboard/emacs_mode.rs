// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn handle(
    context: InputContext<'_, '_>,
    key: Keycode,
    keymod: Mod,
    _repeat: bool,
) -> KeyResult {
    let InputContext {
        editor,
        other_editor,
        renderer,
        terminal,
        vim,
        other_vim,
        emacs,
        lsp_ui,
        search_ui,
        command_bar,
        clipboard,
        dirty,
        split_mode,
        active_pane,
        ..
    } = context;
    if editor.config.keybinding_mode == KeybindingMode::Emacs {
        let control = ctrl_pressed(keymod);
        if command_bar.is_active() {
            let searching = emacs.search_origin.is_some()
                && matches!(command_bar.parse(), Ok(ParsedCommand::Find { .. }));
            if control && key == Keycode::G || key == Keycode::Escape {
                if let Some(origin) = emacs.search_origin.take() {
                    editor
                        .document
                        .move_cursor(origin.min(editor.document.len()))
                        .map_err(|e| e.to_string())?;
                }
                editor.emacs.reset();
                command_bar.close();
                search_ui.close();
                renderer.update_cursor(&editor.document);
                *dirty = true;
                return Ok(Some(EventFlow::Continue));
            }
            if searching && matches!(key, Keycode::Return | Keycode::KpEnter) {
                emacs.search_origin = None;
                command_bar.close();
                editor
                    .document
                    .move_cursor(editor.document.cursor.position)
                    .map_err(|e| e.to_string())?;
                *dirty = true;
                return Ok(Some(EventFlow::Continue));
            }
            if searching && control && matches!(key, Keycode::S | Keycode::R) {
                let result = if key == Keycode::R {
                    search_ui.previous(&mut editor.document)
                } else {
                    search_ui.next(&mut editor.document)
                };
                if let Err(error) = result {
                    command_bar.set_status(error.to_string());
                }
                renderer.ensure_cursor_visible(&mut editor.document);
                *dirty = true;
                return Ok(Some(EventFlow::Continue));
            }
            if control
                && matches!(
                    key,
                    Keycode::A
                        | Keycode::E
                        | Keycode::B
                        | Keycode::F
                        | Keycode::H
                        | Keycode::D
                        | Keycode::K
                        | Keycode::Y
                        | Keycode::N
                        | Keycode::P
                )
            {
                match key {
                    Keycode::A => command_bar.move_home(),
                    Keycode::E => command_bar.move_end(),
                    Keycode::B => command_bar.move_left(),
                    Keycode::F => command_bar.move_right(),
                    Keycode::H => command_bar.backspace(),
                    Keycode::D => command_bar.delete(),
                    Keycode::N => {
                        command_bar.move_selection(1);
                    }
                    Keycode::P => {
                        command_bar.move_selection(-1);
                    }
                    Keycode::K => {
                        let text = command_bar.input()[command_bar.cursor()..].to_string();
                        if !text.is_empty() {
                            match crate::clipboard::TextClipboard::set_text(clipboard, &text) {
                                Ok(()) => {
                                    while command_bar.cursor() < command_bar.input().len() {
                                        command_bar.delete();
                                    }
                                }
                                Err(error) => command_bar.set_status(error),
                            }
                        }
                    }
                    Keycode::Y => match read_text(clipboard) {
                        Ok(text) => command_bar.insert_text(&text),
                        Err(error) => command_bar.set_status(error),
                    },
                    _ => (),
                }
                if let Err(error) = sync_command_search(
                    &*command_bar,
                    &mut *search_ui,
                    &mut editor.document,
                    emacs.search_origin,
                ) {
                    command_bar.set_status(error.to_string());
                }
                renderer.ensure_cursor_visible(&mut editor.document);
                *dirty = true;
                return Ok(Some(EventFlow::Continue));
            }
        } else {
            let handled = emacs.key(
                &mut *editor,
                clipboard,
                key,
                keymod,
                renderer.visible_line_count(),
            );
            match handled {
                Err(error) => {
                    command_bar.open(":");
                    command_bar.show_info(&error);
                    *dirty = true;
                    return Ok(Some(EventFlow::Continue));
                }
                Ok(action) if action.consumed => {
                    let mut outcome = CommandOutcome::default();
                    match action.ui {
                        emacs::Ui::None => (),
                        emacs::Ui::Cancel => {
                            search_ui.close();
                            emacs.search_origin = None;
                        }
                        emacs::Ui::Command(text, execute) => {
                            emacs.search_origin = None;
                            search_ui.close();
                            command_bar.open(text);
                            if text == ":quit" && (editor.dirty || other_editor.dirty) {
                                command_bar.show_info(
                                    "Save modified documents before quitting with Ctrl+X Ctrl+C.",
                                );
                            } else if execute {
                                outcome = execute_command_bar(
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
                                );
                            }
                        }
                        emacs::Ui::Search(backward) => {
                            emacs.search_origin = Some(editor.document.cursor.position);
                            search_ui.close();
                            command_bar.open(if backward {
                                ":find  --backward"
                            } else {
                                ":find "
                            });
                            command_bar.set_cursor(6);
                        }
                        emacs::Ui::History(redo, count) => {
                            for _ in 0..count {
                                let result =
                                    workspace_edit::history(&mut *editor, &mut *other_editor, redo)
                                        .and_then(|handled| {
                                            if handled {
                                                Ok(())
                                            } else {
                                                if redo { editor.redo() } else { editor.undo() }
                                                    .map_err(|e| e.to_string())
                                            }
                                        });
                                if let Err(error) = result {
                                    command_bar.open(":");
                                    command_bar.show_info(&error);
                                    break;
                                }
                            }
                            editor.emacs.reset();
                            lsp_ui.files_changed(workspace_edit::recent_disk_changes(
                                &*editor, !redo,
                            ));
                            search_ui.close();
                            renderer.invalidate_scroll_cache();
                            renderer.set_file_path(editor.path.as_deref());
                        }
                        emacs::Ui::Help => {
                            command_bar.open(":");
                            command_bar.show_info(emacs::HELP);
                        }
                        emacs::Ui::OtherPane => {
                            if *split_mode {
                                outcome.focus_other = true;
                            }
                        }
                        emacs::Ui::Split => {
                            *split_mode = true;
                            renderer.set_split_mode(true);
                        }
                        emacs::Ui::OnlyPane => {
                            *split_mode = false;
                            renderer.set_split_mode(false);
                        }
                    }
                    if outcome.quit {
                        return Ok(Some(EventFlow::Quit));
                    }
                    if outcome.close_pane {
                        close_focused_pane(split_mode, active_pane, editor, other_editor,
                            vim, other_vim, renderer, terminal);
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
                    if action.changed || outcome.document_changed || outcome.document_reloaded {
                        renderer.invalidate_scroll_cache();
                    }
                    if outcome.path_changed {
                        renderer.set_file_path(editor.path.as_deref());
                    }
                    renderer.set_mode_label(editor_mode_label(&*editor, &*vim));
                    renderer.update_cursor(&editor.document);
                    renderer.ensure_cursor_visible(&mut editor.document);
                    *dirty = true;
                    return Ok(Some(EventFlow::Continue));
                }
                _ => (),
            }
        }
    }

    Ok(None)
}
