// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn escape(
    context: InputContext<'_, '_>,
    key: Keycode,
    _keymod: Mod,
    _repeat: bool,
) -> KeyResult {
    let InputContext {
        editor,
        dirty,
        vim_enabled,
        ..
    } = context;
    if !*vim_enabled && key == Keycode::Escape {
        editor.clear_secondary_cursors();
        *dirty = true;
        return Ok(Some(EventFlow::Continue));
    }

    Ok(None)
}

pub(super) fn handle(
    context: InputContext<'_, '_>,
    _key: Keycode,
    _keymod: Mod,
    _repeat: bool,
    bound_command: Option<Command>,
) -> Result<EventFlow, String> {
    let InputContext {
        editor,
        renderer,
        vim,
        search_ui,
        command_bar,
        clipboard,
        dirty,
        vim_enabled,
        ..
    } = context;
    if let Some(command) = bound_command {
        let start = Instant::now();

        let mut document_changed = false;

        let redraw = match command {
            Command::FormatDocument => {
                editor.clear_secondary_cursors();
                command_bar.open(":format");
                document_changed = run_format_command(
                    &mut *editor,
                    &mut *search_ui,
                    &mut *command_bar,
                    &mut *vim,
                    None,
                );
                if document_changed && *vim_enabled {
                    renderer.set_mode_label(Some(vim.mode_label()));
                }
                true
            }

            Command::NewFile => {
                editor.clear_secondary_cursors();
                command_bar.open(":new ");
                search_ui.close();
                true
            }

            Command::SaveAs => {
                editor.clear_secondary_cursors();
                command_bar.open(":save-as ");
                search_ui.close();
                true
            }

            Command::Save => {
                match editor.save() {
                    Ok(()) => renderer.set_file_path(editor.path.as_deref()),
                    Err(error) => {
                        command_bar.open(":save");
                        command_bar.set_status(error.to_string());
                    }
                }
                true
            }

            Command::Copy => {
                if let Err(error) = copy_selection(clipboard, &editor.document) {
                    eprintln!("Clipboard copy failed: {error}");
                }

                false
            }

            Command::Paste => {
                let vim_insert_text = (*vim_enabled && vim.mode() == vim::VimMode::Insert)
                    .then(|| read_text(clipboard).unwrap_or_default());

                match paste(clipboard, &mut *editor) {
                    Ok(true) => {
                        document_changed = true;

                        if let Some(text) = vim_insert_text {
                            vim.record_text(&text);
                        }

                        true
                    }

                    Ok(false) => false,

                    Err(error) => {
                        eprintln!("Clipboard paste failed: {error}");

                        false
                    }
                }
            }

            Command::Cut => match cut_selection(clipboard, &mut *editor) {
                Ok(true) => {
                    document_changed = true;
                    true
                }
                Ok(false) => false,
                Err(error) => {
                    eprintln!("Clipboard cut failed: {error}");
                    false
                }
            },

            _ => {
                editor
                    .execute(command, renderer.visible_line_count())
                    .map_err(|e| e.to_string())?;

                true
            }
        };

        let command_time = start.elapsed();

        if command_time > Duration::from_millis(20) {
            println!("Command {:?}: {:?}", command, command_time);
        }

        if matches!(
            command,
            Command::Delete | Command::Backspace | Command::Newline
        ) || document_changed
        {
            renderer.invalidate_scroll_cache();
        }

        if redraw {
            renderer.ensure_cursor_visible(&mut editor.document);

            *dirty = true;
        }

        if matches!(command, Command::Quit) {
            return Ok(EventFlow::Quit);
        }
    }

    Ok(EventFlow::Continue)
}
