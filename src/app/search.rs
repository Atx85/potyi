// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Search panel actions and keyboard handling.
use super::*;

pub(crate) fn ctrl_pressed(keymod: Mod) -> bool {
    keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)
}

pub(crate) fn shift_pressed(keymod: Mod) -> bool {
    keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD)
}

pub(crate) fn alt_pressed(keymod: Mod) -> bool {
    keymod.intersects(Mod::LALTMOD | Mod::RALTMOD)
}

pub(crate) fn handle_search_key(
    search_ui: &mut SearchUi,
    editor: &mut Editor,
    key: Keycode,
    keymod: Mod,
    repeat: bool,
) -> io::Result<SearchKeyResult> {
    /*
     * The main event loop handles opening/collapsing the panel before this
     * modal key handler runs. Consume the shortcuts here as a safeguard.
     */
    if (key == Keycode::F || key == Keycode::H) && ctrl_pressed(keymod) {
        return Ok(SearchKeyResult::Consumed);
    }

    if search_ui.is_replace_mode() && !repeat {
        let enter = key == Keycode::Return || key == Keycode::KpEnter;

        if (key == Keycode::R && alt_pressed(keymod))
            || (enter && ctrl_pressed(keymod) && !shift_pressed(keymod))
        {
            return Ok(SearchKeyResult::ReplaceCurrent);
        }

        if (key == Keycode::A && alt_pressed(keymod))
            || (enter && ctrl_pressed(keymod) && shift_pressed(keymod))
        {
            return Ok(SearchKeyResult::ReplaceAll);
        }

        if key == Keycode::M && alt_pressed(keymod) {
            search_ui.cycle_mode(&mut editor.document, shift_pressed(keymod))?;

            return Ok(SearchKeyResult::Consumed);
        }
    }

    match key {
        Keycode::Escape => {
            search_ui.close();

            Ok(SearchKeyResult::Closed)
        }

        Keycode::Tab => {
            if !repeat {
                if search_ui.is_replace_mode() {
                    search_ui.focus_next_field(shift_pressed(keymod));
                } else {
                    search_ui.cycle_mode(&mut editor.document, shift_pressed(keymod))?;
                }
            }

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::Return | Keycode::KpEnter => {
            if shift_pressed(keymod) {
                search_ui.previous(&mut editor.document)?;
            } else {
                search_ui.next(&mut editor.document)?;
            }

            // Jump the editor cursor to the found match.
            if let Some(current) = search_ui.current_match() {
                editor.document.move_cursor(current.start)?;
            }

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::Backspace => {
            search_ui.backspace_focused(&mut editor.document)?;

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::Delete => {
            search_ui.delete_focused(&mut editor.document)?;

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::Left => {
            search_ui.move_focused_left();

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::Right => {
            search_ui.move_focused_right();

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::Home => {
            search_ui.move_focused_home();

            Ok(SearchKeyResult::Consumed)
        }

        Keycode::End => {
            search_ui.move_focused_end();

            Ok(SearchKeyResult::Consumed)
        }

        _ => Ok(SearchKeyResult::Ignored),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchKeyResult {
    Consumed,
    Closed,
    Ignored,
    ReplaceCurrent,
    ReplaceAll,
}

pub(crate) fn replacement_requires_cursor_refresh(
    result: SearchKeyResult,
    has_current_match: bool,
) -> bool {
    !has_current_match
        && matches!(
            result,
            SearchKeyResult::ReplaceCurrent | SearchKeyResult::ReplaceAll
        )
}

pub(crate) fn replace_current_match(
    search_ui: &mut SearchUi,
    editor: &mut Editor,
) -> io::Result<usize> {
    let current = match search_ui.current_match() {
        Some(current) => current,
        None => {
            search_ui.set_last_replace_count(Some(0));

            return Ok(0);
        }
    };

    let replacement = search_ui.replacement().to_owned();

    // Calculate progress against the original document before mutation.
    // For a zero-width regex result, advancing one original Unicode scalar
    // prevents repeatedly inserting at the same byte position.
    let original_resume = if current.start == current.end {
        if current.end < editor.document.len() {
            Some(editor.document.next_char_boundary(current.end)?)
        } else {
            None
        }
    } else {
        Some(current.end)
    };

    let changed = editor.replace_range(
        SearchResult {
            start: current.start,
            end: current.end,
        },
        &replacement,
    )?;

    let resume = if current.start == current.end {
        match original_resume {
            Some(position) => Some(position.checked_add(replacement.len()).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "replacement search position overflow",
                )
            })?),
            None => None,
        }
    } else {
        Some(
            current
                .start
                .checked_add(replacement.len())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "replacement search position overflow",
                    )
                })?,
        )
    };

    search_ui.refresh_after_replace(&mut editor.document, resume)?;

    let count = usize::from(changed);

    search_ui.set_last_replace_count(Some(count));

    Ok(count)
}

pub(crate) fn replace_all_matches(
    search_ui: &mut SearchUi,
    editor: &mut Editor,
) -> io::Result<usize> {
    let replacement = search_ui.replacement().to_owned();

    let count = match search_ui.searcher() {
        Some(searcher) => editor.replace_all(searcher, &replacement)?,
        None => 0,
    };

    search_ui.finish_replace_all(count);

    Ok(count)
}

pub(crate) fn search_mode_option(mode: SearchMode) -> &'static str {
    match mode {
        SearchMode::CaseSensitive => "",
        SearchMode::CaseInsensitive => " --ignore-case",
        SearchMode::Regex => " --regex",
    }
}

pub(crate) fn find_command_text(search_ui: &SearchUi) -> String {
    if search_ui.query().is_empty() {
        return ":find ".to_string();
    }

    format!(
        ":find {}{}",
        quote_argument(search_ui.query()),
        search_mode_option(search_ui.mode()),
    )
}

pub(crate) fn replace_command_text(search_ui: &SearchUi) -> String {
    format!(
        ":replace {} {}{}",
        quote_argument(search_ui.query()),
        quote_argument(search_ui.replacement()),
        search_mode_option(search_ui.mode()),
    )
}

pub(crate) fn sync_command_search(
    command_bar: &CommandBar,
    search_ui: &mut SearchUi,
    table: &mut PieceTable,
    origin: Option<usize>,
) -> io::Result<()> {
    match command_bar.parse() {
        Ok(ParsedCommand::Find {
            query,
            mode,
            backward,
        }) => {
            let origin = origin.unwrap_or(table.cursor.position);

            search_ui.configure_command_from(table, &query, None, mode, origin, backward)?;
        }

        Ok(ParsedCommand::Replace {
            query,
            replacement,
            mode,
            ..
        }) => {
            search_ui.configure_command(table, &query, Some(&replacement), mode)?;
        }

        _ => search_ui.close(),
    }

    Ok(())
}
