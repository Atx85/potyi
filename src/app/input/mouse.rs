// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn mouse(
    context: InputContext<'_, '_>,
    event: Event,
    coordinates_converted: bool,
) -> Result<EventFlow, String> {
    let InputContext {
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
        dirty,
        split_mode,
        active_pane,
        vim_enabled,
        ..
    } = context;
    match event {
        Event::MouseButtonDown {
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x,
            y,
            ..
        } => {
            if !coordinates_converted {
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
                    if terminal.is_active() {
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
                                            if handle_terminal_action(
                                                action,
                                                &mut *terminal,
                                                &mut *editor,
                                                &mut *other_editor,
                                                &mut *renderer,
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
                                terminal
                                    .begin_output_drag(offset, x as i32, y as i32, action)
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
                            focus_pane(
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
                            editor.clear_secondary_cursors();
                            editor
                                .document
                                .move_cursor_to_line_column(line, column)
                                .map_err(|error| error.to_string())?;

                            if *vim_enabled {
                                vim.settle_cursor(&mut *editor)?;
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
            if terminal.is_active() && terminal.output_dragging() && coordinates_converted {
                let offset =
                    renderer.terminal_output_offset_at(&mut *terminal, x as i32, y as i32)?;
                terminal
                    .drag_output_to(offset, x as i32, y as i32)
                    .map_err(|error| error.to_string())?;
                if let Some(action) = terminal.finish_output_drag() {
                    terminal.focus_prompt();
                    if handle_terminal_action(
                        action,
                        &mut *terminal,
                        &mut *editor,
                        &mut *other_editor,
                        &mut *renderer,
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
                *dirty = true;
            }
        }

        Event::MouseMotion { x, y, .. } => {
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
            ..
        } => {
            if terminal.is_active() {
                terminal.scroll(y as isize * 3);
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
                    command_bar.scroll_suggestions(if y > 0.0 { -3 } else { 3 });
                    *dirty = true;
                }
                return Ok(EventFlow::Continue);
            }

            /*
             * Mouse wheel still controls the document while search
             * is open. This lets the user inspect nearby matches
             * without closing the search bar.
             */
            renderer.scroll_by(-(y as isize), &mut editor.document);

            renderer.scroll_horizontal(-(x as i32) * 40);

            *dirty = true;
        }
        _ => (),
    }

    Ok(EventFlow::Continue)
}
