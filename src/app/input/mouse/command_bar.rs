// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(super) fn press(context: MouseContext<'_, '_>, x: f32, y: f32) -> Result<EventFlow, String> {
    let MouseContext {
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
        split_mode,
        active_pane,
        vim_enabled,
        ..
    } = context;
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
            let cursor = renderer.command_bar_cursor_at(&*command_bar, x as i32);

            command_bar.set_cursor(cursor);
            *dirty = true;
        }

        CommandBarHit::Execute => {
            let close_vim_search = *vim_enabled
                && !command_bar.selects_option_on_enter()
                && matches!(command_bar.parse(), Ok(ParsedCommand::Find { .. }));

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

            return Ok(command_outcome::apply(
                CommandOutcomeContext {
                    editor,
                    other_editor,
                    renderer,
                    terminal,
                    vim,
                    other_vim,
                    search_ui,
                    command_bar,
                    dirty,
                    split_mode,
                    active_pane,
                    vim_enabled,
                },
                outcome,
                CommandSource::Mouse,
            ));
        }

        CommandBarHit::Outside => {}
    }
    Ok(EventFlow::Continue)
}
