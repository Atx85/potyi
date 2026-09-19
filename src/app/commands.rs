// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Command execution and its UI outcomes.
use super::*;

#[derive(Default)]
pub(crate) struct CommandOutcome {
    pub(crate) quit: bool,
    pub(crate) toggle_split: bool,
    pub(crate) close_pane: bool,
    pub(crate) focus_other: bool,
    pub(crate) document_changed: bool,
    pub(crate) document_reloaded: bool,
    pub(crate) path_changed: bool,
    pub(crate) cursor_changed: bool,
    pub(crate) keybinding_mode: Option<KeybindingMode>,
}

pub(crate) fn goto_line_index(
    table: &mut PieceTable,
    line: isize,
    mode: GotoMode,
    configured_mode: LineNumberMode,
) -> Result<usize, String> {
    let current_line = table.cursor.line;
    if mode == GotoMode::Automatic && configured_mode != LineNumberMode::Normal && line >= 0 {
        let label = line as usize;
        let mut matches = Vec::new();
        // Only the cursor line and the two lines at this distance can
        // carry this label. Reuse the gutter's numbering rules exactly.
        for candidate in [
            Some(current_line),
            current_line.checked_sub(label),
            current_line.checked_add(label),
        ]
        .into_iter()
        .flatten()
        {
            if matches.contains(&candidate)
                || line_numbers::LineNumbers::display_number(
                    configured_mode,
                    candidate,
                    current_line,
                ) != label
            {
                continue;
            }
            table
                .ensure_line_cached(candidate)
                .map_err(|error| error.to_string())?;
            if candidate < table.cached_line_count() {
                matches.push(candidate);
            }
        }
        return match matches.as_slice() {
            [destination] => Ok(*destination),
            [] => Err(format!("No line is labelled {label}.")),
            _ => Err(format!(
                "Label {label} matches multiple lines. Use :goto -{label} (up), :goto +{label} (down), or --abs."
            )),
        };
    }

    // Signed arguments and --rel are both resolved to Relative by the parser.
    // Automatic here is an absolute label in normal mode, never an offset.
    let relative = mode == GotoMode::Relative;

    let destination = if relative {
        if line >= 0 {
            current_line
                .checked_add(line as usize)
                .ok_or_else(|| "Relative line is out of range".to_string())
        } else {
            current_line
                .checked_sub(line.unsigned_abs())
                .ok_or_else(|| "Relative line is before the start of the document".to_string())
        }
    } else {
        usize::try_from(line)
            .ok()
            .and_then(|line| line.checked_sub(1))
            .ok_or_else(|| "line numbers start at 1".to_string())
    }?;

    table
        .ensure_line_cached(destination)
        .map_err(|error| error.to_string())?;
    if destination >= table.cached_line_count() {
        return Err(format!(
            "Line {} is past the last line ({}).",
            destination + 1,
            table.cached_line_count(),
        ));
    }
    Ok(destination)
}

pub(crate) fn execute_command_bar(
    command_bar: &mut CommandBar,
    search_ui: &mut SearchUi,
    editor: &mut Editor,
    other_editor: &mut Editor,
    renderer: &mut Renderer<'_>,
    terminal: &mut Terminal,
    vim: &mut VimController,
    reverse_find: bool,
    other_vim: &mut VimController,
    lsp_ui: &mut lsp_ui::LspUi,
) -> CommandOutcome {
    let mut outcome = CommandOutcome::default();

    let other_history_len = other_editor.undo_stack.len();
    if let Some(result) = lsp_ui.review(editor, other_editor, command_bar) {
        match result {
            Ok(true) => {
                vim.finish_formatting(editor);
                if other_editor.undo_stack.len() > other_history_len {
                    other_vim.finish_formatting(other_editor);
                }
                search_ui.close();
                outcome.document_changed = true;
                outcome.cursor_changed = true;
                outcome.path_changed = true;
            }
            Ok(false) => {}
            Err(error) => command_bar.show_info(&error),
        }
        return outcome;
    }
    editor.clear_secondary_cursors();
    let previous_epoch = command_bar.epoch();
    let execute = command_bar.prepare_execute();
    if previous_epoch != command_bar.epoch() {
        if let Err(error) = sync_command_search(
            command_bar,
            search_ui,
            &mut editor.document,
            vim.search_origin(),
        ) {
            command_bar.set_status(error.to_string());
            return outcome;
        }
        outcome.cursor_changed = search_ui.current_match().is_some();
    }
    if !execute {
        return outcome;
    }

    match command_bar.parse() {
        Ok(ParsedCommand::Find { backward, .. }) => {
            let result = if backward || reverse_find {
                search_ui.previous(&mut editor.document)
            } else {
                search_ui.next(&mut editor.document)
            };

            match result {
                Ok(()) => {
                    outcome.cursor_changed = true;
                }

                Err(error) => {
                    command_bar.set_status(error.to_string());
                }
            }
        }

        Ok(ParsedCommand::Replace { all, .. }) => {
            let result = if all {
                replace_all_matches(search_ui, editor)
            } else {
                replace_current_match(search_ui, editor)
            };

            match result {
                Ok(count) => {
                    outcome.document_changed = count > 0;
                    outcome.cursor_changed = true;
                }

                Err(error) => {
                    command_bar.set_status(error.to_string());
                }
            }
        }

        Ok(ParsedCommand::Goto { line, column, mode }) => {
            let result =
                goto_line_index(&mut editor.document, line, mode, editor.config.line_numbers)
                    .and_then(|line| {
                        editor
                            .document
                            .move_cursor_to_line_column(line, column.unwrap_or(1) - 1)
                            .map_err(|error| error.to_string())
                    });

            match result {
                Ok(()) => {
                    command_bar.close();
                    search_ui.close();
                    outcome.cursor_changed = true;
                }

                Err(error) => {
                    command_bar.set_status(error);
                }
            }
        }

        Ok(ParsedCommand::New { path }) => {
            let result = if path
                .as_deref()
                .is_some_and(|path| file_is_open_in(path, other_editor))
            {
                Err(io::Error::other(
                    "This file is already open in the other pane",
                ))
            } else {
                editor.new_document(path.as_deref())
            };
            match result {
                Ok(()) => {
                    command_bar.close();
                    search_ui.close();
                    outcome.document_reloaded = true;
                    outcome.path_changed = true;
                    outcome.cursor_changed = true;
                }
                Err(error) => command_bar.set_status(error.to_string()),
            }
        }

        Ok(ParsedCommand::Open { path }) => {
            if file_is_open_in(&path, other_editor) {
                command_bar.close();
                search_ui.close();
                outcome.focus_other = true;
                return outcome;
            }

            match editor.open(&path) {
                Ok(()) => {
                    command_bar.close();
                    search_ui.close();
                    outcome.document_reloaded = true;
                    outcome.path_changed = true;
                    outcome.cursor_changed = true;
                }

                Err(error) => {
                    command_bar.set_status(error.to_string());
                }
            }
        }

        Ok(ParsedCommand::SetFontSize { points }) => match renderer.set_font_size(points as f32) {
            Ok(()) => {
                editor.config.font_size = points;
                other_editor.config.font_size = points;
                command_bar.close();
                search_ui.close();
                outcome.cursor_changed = true;
            }

            Err(error) => {
                command_bar.set_status(error);
            }
        },

        Ok(ParsedCommand::SetLineNumbers { mode }) => {
            editor.config.line_numbers = mode;
            other_editor.config.line_numbers = mode;
            renderer.set_line_number_mode(mode);
            command_bar.close();
            search_ui.close();
            outcome.cursor_changed = true;
        }

        Ok(ParsedCommand::SetKeybindings { mode }) => {
            editor.config.keybinding_mode = mode;
            other_editor.config.keybinding_mode = mode;
            command_bar.close();
            search_ui.close();
            outcome.cursor_changed = true;
            outcome.keybinding_mode = Some(mode);
        }

        Ok(ParsedCommand::Term { command }) => {
            command_bar.close();
            search_ui.close();
            renderer.place_terminal_in_active_pane();
            terminal.open(editor.path.as_deref());
            if let Some(command) = command {
                terminal.set_listing_width(renderer.terminal_columns());
                match terminal.run_command(&command) {
                    Ok(action) => {
                        let opens_document = matches!(
                            &action,
                            TerminalAction::ListedFile(_)
                                | TerminalAction::Location(_, _)
                                | TerminalAction::Edit(_)
                                | TerminalAction::View(_)
                        );
                        match handle_terminal_action(
                            action,
                            terminal,
                            editor,
                            other_editor,
                            renderer,
                        ) {
                            Ok(focus_other) => {
                                outcome.focus_other = focus_other;
                                if opens_document && (!terminal.is_active() || focus_other) {
                                    outcome.document_reloaded = true;
                                    outcome.path_changed = true;
                                    outcome.cursor_changed = true;
                                }
                            }
                            Err(error) => terminal.set_status(error),
                        }
                    }
                    Err(error) => terminal.set_status(error.to_string()),
                }
            }
        }

        Ok(ParsedCommand::Split) => {
            command_bar.close();
            search_ui.close();
            outcome.toggle_split = true;
        }

        Ok(ParsedCommand::Format { provider }) => {
            outcome.document_changed =
                run_format_command(editor, search_ui, command_bar, vim, provider.as_deref());
            outcome.cursor_changed = outcome.document_changed;
            if outcome.document_changed && editor.config.keybinding_mode == KeybindingMode::Vim {
                renderer.set_mode_label(Some(vim.mode_label()));
            }
        }

        Ok(ParsedCommand::Hover) | Ok(ParsedCommand::Definition) => {
            let action = if matches!(command_bar.parse(), Ok(ParsedCommand::Hover)) {
                lsp::Action::Hover
            } else {
                lsp::Action::Definition
            };
            if let Err(error) = lsp_ui.request(action, editor, other_editor, command_bar) {
                command_bar.show_info(&error);
            }
        }
        Ok(ParsedCommand::Actions { refactor_only }) => {
            let action = lsp::Action::CodeActions {
                anchor: editor.document.cursor.anchor,
                refactor_only,
            };
            if let Err(error) = lsp_ui.request(action, editor, other_editor, command_bar) {
                command_bar.show_info(&error);
            }
        }
        Ok(ParsedCommand::Rename { name }) => {
            if let Err(error) =
                lsp_ui.request(lsp::Action::Rename(name), editor, other_editor, command_bar)
            {
                command_bar.show_info(&error);
            }
        }
        Ok(ParsedCommand::LspBack) => match lsp_ui.go_back(editor, other_editor) {
            Ok(result) => {
                command_bar.close();
                search_ui.close();
                outcome = result;
            }
            Err(error) => command_bar.show_info(&error),
        },
        Ok(ParsedCommand::LspStart | ParsedCommand::LspRestart) => {
            let restart = matches!(command_bar.parse(), Ok(ParsedCommand::LspRestart));
            if let Err(error) = lsp_ui.start(restart, editor, other_editor, command_bar) {
                command_bar.show_info(&error);
            }
        }
        Ok(ParsedCommand::LspStatus) => command_bar.show_info(&lsp_ui.status(editor)),
        Ok(ParsedCommand::LspInstall { server }) => {
            if let Err(error) = lsp_ui.setup(true, Some(&server), editor, command_bar) {
                command_bar.show_info(&error);
            }
        }
        Ok(ParsedCommand::LspDoctor { server }) => {
            if let Err(error) = lsp_ui.setup(false, server.as_deref(), editor, command_bar) {
                command_bar.show_info(&error);
            }
        }
        Ok(ParsedCommand::LspStop) => {
            lsp_ui.stop();
            command_bar.show_info("LSP stopped; any setup is being cancelled. Autocomplete is paused. Use :lsp start to connect again.");
        }

        Ok(ParsedCommand::Formatters) => {
            match formatting::Formatters::load(std::path::Path::new("config/formatters.toml")) {
                Ok(config) => command_bar.show_formatters(config.choices(editor.path.as_deref())),
                Err(error) => command_bar.set_status(error.to_string()),
            }
        }

        Ok(ParsedCommand::ExtractConfig) => {
            match embedded_config::extract_defaults(std::path::Path::new("config")) {
                Ok(summary) if summary.created == 0 => {
                    command_bar.set_status(format!(
                        "All {} config files already exist; nothing was overwritten",
                        summary.existing,
                    ));
                }

                Ok(summary) => {
                    command_bar.set_status(format!(
                        "Created {} config files; preserved {} existing files",
                        summary.created, summary.existing,
                    ));
                }

                Err(error) => {
                    command_bar.set_status(format!("Failed to extract config: {error}"));
                }
            }
        }

        Ok(ParsedCommand::Save) => match editor.save() {
            Ok(()) => {
                command_bar.close();
                search_ui.close();
                outcome.path_changed = true;
            }

            Err(error) => {
                command_bar.set_status(error.to_string());
            }
        },

        Ok(ParsedCommand::SaveAs { path, overwrite }) => {
            if let Some(path) = path {
                match save_as_in_pane(editor, other_editor, &path, overwrite) {
                    Ok(()) => {
                        command_bar.close();
                        search_ui.close();
                        outcome.path_changed = true;
                    }
                    Err(error) => command_bar.set_status(error.to_string()),
                }
            } else {
                command_bar.open(if overwrite { ":save-as! " } else { ":save-as " });
                command_bar.set_status("Enter a destination path; quote paths containing spaces");
                search_ui.close();
            }
        }

        Ok(ParsedCommand::Recover { number }) => {
            if let Some(number) = number {
                if editor.dirty {
                    command_bar.show_info("Save the current document before opening recovered work, or switch to an empty pane.");
                } else {
                    let result = piece_table::recovery::root()
                        .and_then(|root| editor.recover_from(&root, number));
                    match result {
                        Ok(incomplete) => {
                            search_ui.close();
                            outcome.document_reloaded = true;
                            outcome.path_changed = true;
                            outcome.cursor_changed = true;
                            command_bar.show_info(if incomplete { "Recovered through the last complete edit; an interrupted journal tail was ignored. Save As chooses the destination. The original file is unchanged." } else { "Recovered into a separate file. Save As chooses the destination; Save keeps this recovery copy. The original file is unchanged." });
                        }
                        Err(error) => command_bar.show_info(&format!(
                            "Recovery could not finish: {error}. The recovery files have been kept."
                        )),
                    }
                }
            } else {
                command_bar.show_info(
                    &piece_table::recovery::describe().unwrap_or_else(|error| {
                        format!("Could not list recovery sessions: {error}")
                    }),
                );
            }
        }

        Ok(ParsedCommand::ExitPane) => {
            if renderer.is_split() {
                command_bar.close();
                search_ui.close();
                outcome.close_pane = true;
            } else {
                command_bar.set_status("No split pane to close. Use :quit to close Pötyi.");
            }
        }

        Ok(ParsedCommand::Quit) => {
            outcome.quit = true;
        }

        Err(error) => {
            command_bar.set_status(error);
        }
    }

    outcome
}

pub(crate) fn run_format_command(
    editor: &mut Editor,
    search_ui: &mut SearchUi,
    command_bar: &mut CommandBar,
    vim: &mut VimController,
    provider: Option<&str>,
) -> bool {
    match editor.format_document(provider) {
        Ok((name, changed)) => {
            if changed && editor.config.keybinding_mode == KeybindingMode::Vim {
                vim.finish_formatting(editor);
            }
            if changed {
                search_ui.close();
            }
            command_bar.set_status(if changed {
                format!("Formatted with {name}; Ctrl+Z undoes the change")
            } else {
                format!("Already formatted ({name})")
            });
            changed
        }
        Err(error) => {
            command_bar.set_status(error.to_string());
            false
        }
    }
}
