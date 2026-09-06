// Pötyi - Lightweight text editor
// Copyright (C) 2026  Attila Banko
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


use std::io;

use crate::piece_table::PieceTable;
use crate::search::{
    SearchMode,
    SearchResult,
    Searcher,
};

// ==========================================================================
// Search UI
// ==========================================================================
//
// Searcher owns the document-search algorithm.
//
// SearchUi owns temporary search interaction state.
//
// When a match becomes current, SearchUi moves the PieceTable cursor to the
// beginning of that match.
//
// SearchUi does not modify document contents.
//
// SearchMatch stores byte offsets into the document.
//
// The PieceTable cursor is always positioned at SearchMatch.start when a
// match is active.
// ==========================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchPanelMode {
    Find,
    Replace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchField {
    Query,
    Replacement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchMatch {
    pub start: usize,
    pub end: usize,
}

pub struct SearchUi {
    active: bool,

    query: String,
    replacement: String,
    mode: SearchMode,
    searcher: Option<Searcher>,
    pattern_error: Option<String>,

    current_match: Option<SearchMatch>,

    // Byte offset inside the UTF-8 search query.
    query_cursor: usize,
    // Byte offset inside the UTF-8 replacement text.
    replacement_cursor: usize,

    panel_mode: SearchPanelMode,
    focused_field: SearchField,
    last_replace_count: Option<usize>,

    search_failed: bool,
}

impl SearchUi {
    pub fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            replacement: String::new(),
            mode: SearchMode::CaseSensitive,
            searcher: None,
            pattern_error: None,
            current_match: None,
            query_cursor: 0,
            replacement_cursor: 0,
            panel_mode: SearchPanelMode::Find,
            focused_field: SearchField::Query,
            last_replace_count: None,
            search_failed: false,
        }
    }

    // ----------------------------------------------------------------------
    // State
    // ----------------------------------------------------------------------

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn replacement(&self) -> &str {
        &self.replacement
    }

    pub fn is_replace_mode(&self) -> bool {
        self.panel_mode == SearchPanelMode::Replace
    }

    pub fn panel_mode(&self) -> SearchPanelMode {
        self.panel_mode
    }

    pub fn focused_field(&self) -> SearchField {
        self.focused_field
    }

    pub fn mode(&self) -> SearchMode {
        self.mode
    }

    pub fn mode_label(&self) -> &'static str {
        match self.mode() {
            SearchMode::CaseSensitive => "[Aa] Match case",
            SearchMode::CaseInsensitive => "[aa] Ignore case",
            SearchMode::Regex => "[.*] Regex",
        }
    }

    pub fn compact_mode_label(&self) -> &'static str {
        match self.mode() {
            SearchMode::CaseSensitive => "[Aa]",
            SearchMode::CaseInsensitive => "[aa]",
            SearchMode::Regex => "[.*]",
        }
    }

    pub fn pattern_error(&self) -> Option<&str> {
        self.pattern_error.as_deref()
    }

    pub fn current_match(&self) -> Option<SearchMatch> {
        self.current_match
    }

    pub fn searcher(&self) -> Option<&Searcher> {
        self.searcher.as_ref()
    }

    pub fn search_failed(&self) -> bool {
        self.search_failed
    }

    pub fn query_cursor(&self) -> usize {
        self.query_cursor
    }

    pub fn replacement_cursor(&self) -> usize {
        self.replacement_cursor
    }

    pub fn last_replace_count(&self) -> Option<usize> {
        self.last_replace_count
    }

    pub fn set_last_replace_count(&mut self, count: Option<usize>) {
        self.last_replace_count = count;
    }

    /// Change the search mode and immediately re-run the current query.
    pub fn set_mode(
        &mut self,
        table: &mut PieceTable,
        mode: SearchMode,
    ) -> io::Result<()> {
        self.last_replace_count = None;

        if self.mode == mode {
            return Ok(());
        }

        self.mode = mode;
        self.rebuild_searcher();
        self.current_match = None;
        self.search_failed = false;

        if self.query.is_empty() {
            return Ok(());
        }

        let start = table.cursor.position;
        self.find_initial(table, start)
    }

    /// Cycle between case-sensitive, case-insensitive, and regex search.
    pub fn cycle_mode(
        &mut self,
        table: &mut PieceTable,
        backwards: bool,
    ) -> io::Result<()> {
        let mode =
            match (self.mode, backwards) {
                (SearchMode::CaseSensitive, false) =>
                    SearchMode::CaseInsensitive,
                (SearchMode::CaseInsensitive, false) =>
                    SearchMode::Regex,
                (SearchMode::Regex, false) =>
                    SearchMode::CaseSensitive,
                (SearchMode::CaseSensitive, true) =>
                    SearchMode::Regex,
                (SearchMode::CaseInsensitive, true) =>
                    SearchMode::CaseSensitive,
                (SearchMode::Regex, true) =>
                    SearchMode::CaseInsensitive,
            };

        self.set_mode(table, mode)
    }

    // ----------------------------------------------------------------------
    // Opening / closing
    // ----------------------------------------------------------------------

    pub fn open(
        &mut self,
        table: &mut PieceTable,
    ) -> io::Result<()> {
        self.panel_mode = SearchPanelMode::Find;
        self.focused_field = SearchField::Query;
        self.last_replace_count = None;

        if self.active {
            return Ok(());
        }

        self.active = true;
        self.query_cursor = self.query.len();
        self.search_failed = false;

        if !self.query.is_empty() {
            self.rebuild_searcher();

            let start = table.cursor.position;

            self.find_initial(
                table,
                start,
            )?;
        } else {
            self.current_match = None;
        }

        Ok(())
    }

    pub fn open_replace(
        &mut self,
        table: &mut PieceTable,
    ) -> io::Result<()> {
        let was_active = self.active;

        self.panel_mode = SearchPanelMode::Replace;
        self.last_replace_count = None;
        self.focused_field =
            if was_active {
                SearchField::Replacement
            } else {
                SearchField::Query
            };

        if was_active {
            return Ok(());
        }

        self.active = true;
        self.query_cursor = self.query.len();
        self.replacement_cursor = self.replacement.len();
        self.search_failed = false;

        if !self.query.is_empty() {
            self.rebuild_searcher();

            let start = table.cursor.position;

            self.find_initial(
                table,
                start,
            )?;
        } else {
            self.current_match = None;
        }

        Ok(())
    }

    pub fn close(&mut self) {
        self.active = false;
        self.current_match = None;
        self.search_failed = false;
        self.last_replace_count = None;
    }

    /// Configures the search engine from the shared command bar in one pass.
    /// The command bar owns text editing; SearchUi remains the lightweight
    /// search session used for matching and highlighting.
    pub fn configure_command(
        &mut self,
        table: &mut PieceTable,
        query: &str,
        replacement: Option<&str>,
        mode: SearchMode,
    ) -> io::Result<()> {
        let origin = table.cursor.position;

        self.configure_command_from(
            table,
            query,
            replacement,
            mode,
            origin,
            false,
        )
    }

    /// Configures command-bar search from a stable cursor origin.
    ///
    /// Vim-style incremental search uses this so editing the query repeatedly
    /// previews from the position where `/` or `?` was pressed, rather than
    /// using the previous preview as the next search origin.
    pub fn configure_command_from(
        &mut self,
        table: &mut PieceTable,
        query: &str,
        replacement: Option<&str>,
        mode: SearchMode,
        origin: usize,
        backward: bool,
    ) -> io::Result<()> {
        table.move_cursor(origin)?;

        self.active = true;
        self.panel_mode =
            if replacement.is_some() {
                SearchPanelMode::Replace
            } else {
                SearchPanelMode::Find
            };

        self.focused_field =
            SearchField::Query;

        self.query.clear();
        self.query.push_str(query);
        self.query_cursor = self.query.len();

        if let Some(replacement) = replacement {
            self.replacement.clear();
            self.replacement.push_str(
                replacement
            );
            self.replacement_cursor =
                self.replacement.len();
        }

        self.mode = mode;
        self.last_replace_count = None;
        self.current_match = None;
        self.search_failed = false;
        self.rebuild_searcher();

        if self.query.is_empty() {
            return Ok(());
        }

        if backward {
            self.current_match = Some(SearchMatch {
                start: origin,
                end: origin,
            });
            self.previous(table)
        } else {
            self.find_initial(table, origin)
        }
    }

    pub fn focus_next_field(&mut self, _backwards: bool) {
        if !self.active {
            return;
        }

        if self.panel_mode == SearchPanelMode::Find {
            self.focused_field = SearchField::Query;
            return;
        }

        self.focused_field =
            match self.focused_field {
                SearchField::Query => SearchField::Replacement,
                SearchField::Replacement => SearchField::Query,
            };
    }

    // ----------------------------------------------------------------------
    // Query editing
    // ----------------------------------------------------------------------

    pub fn set_query(
        &mut self,
        table: &mut PieceTable,
        query: &str,
    ) -> io::Result<()> {
        self.last_replace_count = None;
        self.query.clear();
        self.query.push_str(query);

        self.query_cursor = self.query.len();

        self.rebuild_searcher();

        self.current_match = None;
        self.search_failed = false;

        if self.query.is_empty() {
            return Ok(());
        }

        let start = table.cursor.position;

        self.find_initial(
            table,
            start,
        )
    }

    pub fn insert_text(
        &mut self,
        table: &mut PieceTable,
        text: &str,
    ) -> io::Result<()> {
        if !self.active || text.is_empty() {
            return Ok(());
        }

        self.last_replace_count = None;

        self.query.insert_str(
            self.query_cursor,
            text,
        );

        self.query_cursor =
            self.query_cursor
                .checked_add(text.len())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "search query length overflow",
                    )
                })?;

        self.rebuild_searcher();

        let start = table.cursor.position;

        self.find_initial(
            table,
            start,
        )
    }

    pub fn backspace(
        &mut self,
        table: &mut PieceTable,
    ) -> io::Result<()> {
        if !self.active || self.query_cursor == 0 {
            return Ok(());
        }

        self.last_replace_count = None;

        let previous =
            previous_char_boundary(
                &self.query,
                self.query_cursor,
            );

        self.query
            .drain(previous..self.query_cursor);

        self.query_cursor = previous;

        self.rebuild_searcher();

        if self.query.is_empty() {
            self.current_match = None;
            self.search_failed = false;
            return Ok(());
        }

        let start = table.cursor.position;

        self.find_initial(
            table,
            start,
        )
    }

    pub fn delete(
        &mut self,
        table: &mut PieceTable,
    ) -> io::Result<()> {
        if !self.active
            || self.query_cursor >= self.query.len()
        {
            return Ok(());
        }

        self.last_replace_count = None;

        let next =
            next_char_boundary(
                &self.query,
                self.query_cursor,
            );

        self.query
            .drain(self.query_cursor..next);

        self.rebuild_searcher();

        if self.query.is_empty() {
            self.current_match = None;
            self.search_failed = false;
            return Ok(());
        }

        let start = table.cursor.position;

        self.find_initial(
            table,
            start,
        )
    }

    // ----------------------------------------------------------------------
    // Query cursor
    // ----------------------------------------------------------------------

    pub fn move_left(&mut self) {
        if !self.active || self.query_cursor == 0 {
            return;
        }

        self.query_cursor =
            previous_char_boundary(
                &self.query,
                self.query_cursor,
            );
    }

    pub fn move_right(&mut self) {
        if !self.active
            || self.query_cursor >= self.query.len()
        {
            return;
        }

        self.query_cursor =
            next_char_boundary(
                &self.query,
                self.query_cursor,
            );
    }

    pub fn move_home(&mut self) {
        if self.active {
            self.query_cursor = 0;
        }
    }

    pub fn move_end(&mut self) {
        if self.active {
            self.query_cursor = self.query.len();
        }
    }

    // ----------------------------------------------------------------------
    // Focused field editing
    // ----------------------------------------------------------------------

    pub fn insert_focused_text(
        &mut self,
        table: &mut PieceTable,
        text: &str,
    ) -> io::Result<()> {
        if !self.active || text.is_empty() {
            return Ok(());
        }

        match self.focused_field {
            SearchField::Query =>
                self.insert_text(table, text),

            SearchField::Replacement => {
                self.last_replace_count = None;

                self.replacement.insert_str(
                    self.replacement_cursor,
                    text,
                );

                self.replacement_cursor =
                    self.replacement_cursor
                        .checked_add(text.len())
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "replacement length overflow",
                            )
                        })?;

                Ok(())
            }
        }
    }

    pub fn backspace_focused(
        &mut self,
        table: &mut PieceTable,
    ) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        match self.focused_field {
            SearchField::Query =>
                self.backspace(table),

            SearchField::Replacement => {
                if self.replacement_cursor == 0 {
                    return Ok(());
                }

                self.last_replace_count = None;

                let previous =
                    previous_char_boundary(
                        &self.replacement,
                        self.replacement_cursor,
                    );

                self.replacement
                    .drain(previous..self.replacement_cursor);

                self.replacement_cursor = previous;

                Ok(())
            }
        }
    }

    pub fn delete_focused(
        &mut self,
        table: &mut PieceTable,
    ) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        match self.focused_field {
            SearchField::Query =>
                self.delete(table),

            SearchField::Replacement => {
                if self.replacement_cursor >= self.replacement.len() {
                    return Ok(());
                }

                self.last_replace_count = None;

                let next =
                    next_char_boundary(
                        &self.replacement,
                        self.replacement_cursor,
                    );

                self.replacement
                    .drain(self.replacement_cursor..next);

                Ok(())
            }
        }
    }

    pub fn move_focused_left(&mut self) {
        if !self.active {
            return;
        }

        match self.focused_field {
            SearchField::Query =>
                self.move_left(),

            SearchField::Replacement => {
                self.replacement_cursor =
                    previous_char_boundary(
                        &self.replacement,
                        self.replacement_cursor,
                    );
            }
        }
    }

    pub fn move_focused_right(&mut self) {
        if !self.active {
            return;
        }

        match self.focused_field {
            SearchField::Query =>
                self.move_right(),

            SearchField::Replacement => {
                self.replacement_cursor =
                    next_char_boundary(
                        &self.replacement,
                        self.replacement_cursor,
                    );
            }
        }
    }

    pub fn move_focused_home(&mut self) {
        if !self.active {
            return;
        }

        match self.focused_field {
            SearchField::Query =>
                self.move_home(),

            SearchField::Replacement => {
                self.replacement_cursor = 0;
            }
        }
    }

    pub fn move_focused_end(&mut self) {
        if !self.active {
            return;
        }

        match self.focused_field {
            SearchField::Query =>
                self.move_end(),

            SearchField::Replacement => {
                self.replacement_cursor = self.replacement.len();
            }
        }
    }

    // ----------------------------------------------------------------------
    // Search navigation
    // ----------------------------------------------------------------------


    
pub fn next(
    &mut self,
    table: &mut PieceTable,
) -> io::Result<()> {
    if !self.active || self.query.is_empty() {
        return Ok(());
    }

    if self.searcher.is_none() {
        self.current_match = None;
        self.search_failed = false;
        return Ok(());
    }

    let start =
        match self.current_match {
            Some(current)
                if current.start == current.end =>
            {
                if current.end < table.len() {
                    Some(
                        table.next_char_boundary(
                            current.end,
                        )?
                    )
                } else {
                    None
                }
            }

            Some(current) => Some(current.end),
            None => Some(table.cursor.position),
        };

    // Search after the current match.
    if let Some(start) = start {
        if let Some(found) =
            self.find_forward_match(
                table,
                start,
            )?
        {
            self.apply_match(table, found)?;
            return Ok(());
        }
    }

    // Nothing after the current match.
    // Wrap around to the beginning.
    if let Some(found) =
        self.find_forward_match(
            table,
            0,
        )?
    {
        self.apply_match(table, found)?;
        return Ok(());
    }

    // No match anywhere.
    self.current_match = None;
    self.search_failed = true;

    Ok(())
}

fn find_forward_match(
    &self,
    table: &mut PieceTable,
    start: usize,
) -> io::Result<Option<SearchResult>> {
    let searcher =
        match self.searcher.as_ref() {
            Some(searcher) => searcher,
            None => return Ok(None),
        };

    searcher.find_forward_match(
        table,
        start,
    )
}


    

pub fn previous(
    &mut self,
    table: &mut PieceTable,
) -> io::Result<()> {
    if !self.active || self.query.is_empty() {
        return Ok(());
    }

    let searcher =
        match self.searcher.as_ref() {
            Some(searcher) => searcher,
            None => {
                self.current_match = None;
                self.search_failed = false;
                return Ok(());
            }
        };

    let start =
        match self.current_match {
            Some(current) => current.start,
            None => table.cursor.position,
        };

    let search_start =
        if self.current_match.is_some() {
            if start > 0 {
                Some(
                    table.previous_char_boundary(
                        start,
                    )?
                )
            } else {
                None
            }
        } else {
            Some(start)
        };

    // Search strictly before the current match, or from the editor cursor
    // when there is no current result.
    if let Some(search_start) = search_start {
        if let Some(found) =
            searcher.find_backward_match(
                table,
                search_start,
            )?
        {
            self.apply_match(table, found)?;
            return Ok(());
        }
    }

    // Nothing before the current match.
    // Wrap around and find the last match.
    let document_len =
        table.len();

    if let Some(found) =
        searcher.find_backward_match(
            table,
            document_len,
        )?
    {
        self.apply_match(table, found)?;
        return Ok(());
    }

    // No match anywhere.
    self.current_match = None;
    self.search_failed = true;

    Ok(())
}

    /// Refresh the current match after replacing text without wrapping.
    ///
    /// `start` is already expressed in the mutated document. `None` is used
    /// when a zero-width match at end-of-file has no safe continuation.
    pub fn refresh_after_replace(
        &mut self,
        table: &mut PieceTable,
        start: Option<usize>,
    ) -> io::Result<()> {
        self.current_match = None;
        self.search_failed = false;

        if !self.active
            || self.query.is_empty()
        {
            return Ok(());
        }

        let start =
            match start {
                Some(start) if start <= table.len() => start,
                _ => return Ok(()),
            };

        let found =
            match self.searcher.as_ref() {
                Some(searcher) =>
                    searcher.find_forward_match(
                        table,
                        start,
                    )?,
                None => return Ok(()),
            };

        if let Some(found) = found {
            self.apply_match(table, found)?;
        }

        Ok(())
    }

    /// Finish a replace-all transaction. Existing byte ranges refer to the
    /// old document, so the next explicit Find command starts a fresh search.
    pub fn finish_replace_all(
        &mut self,
        count: usize,
    ) {
        self.current_match = None;
        self.search_failed = false;
        self.last_replace_count = Some(count);
    }


    // ----------------------------------------------------------------------
    // Internal search
    // ----------------------------------------------------------------------

    fn rebuild_searcher(&mut self) {
        self.pattern_error = None;

        if self.query.is_empty() {
            self.searcher = None;
        } else {
            match Searcher::with_mode(
                &self.query,
                self.mode,
            ) {
                Ok(searcher) => {
                    self.searcher =
                        Some(searcher);
                }

                Err(error) => {
                    self.searcher = None;
                    self.pattern_error =
                        Some(error.to_string());
                }
            }
        }
    }

    /// Apply a newly found search match.
    ///
    /// This is deliberately the single point where a search result changes
    /// the actual editor cursor.
    ///
    /// Every successful search path goes through this function.
fn apply_match(
    &mut self,
    table: &mut PieceTable,
    found: SearchResult,
) -> io::Result<()> {
    let start = found.start;

    // Move the real editor cursor to the beginning of the match.
    table.move_cursor(start)?;

    // `move_cursor()` establishes the byte position, but the renderer
    // also relies on the cached line/column cursor state.
    let (line, column) =
        table.line_column_at(start)?;

    table.cursor.position = start;
    table.cursor.line = line;
    table.cursor.column = column;

    // A search match is a cursor move, not a text selection.
    table.cursor.anchor = start;
    table.cursor.anchor_line = line;
    table.cursor.anchor_column = column;
    table.cursor.desired_column = None;

    self.current_match =
        Some(SearchMatch {
            start,
            end: found.end,
        });

    self.search_failed = false;

    Ok(())
}

    /// Initial search used when the query changes or the search UI opens.
    ///
    /// First searches forward from the current editor cursor.
    ///
    /// If nothing exists after the cursor, searches backward so that starting
    /// a search at the end of a document still finds the nearest match.
    fn find_initial(
        &mut self,
        table: &mut PieceTable,
        start: usize,
    ) -> io::Result<()> {
        let searcher =
            match self.searcher.as_ref() {
                Some(searcher) => searcher,
                None => {
                    self.current_match = None;
                    self.search_failed = false;
                    return Ok(());
                }
            };

        // First search forward.
        let forward =
            searcher.find_forward_match(
                table,
                start,
            )?;

        if let Some(found) = forward {
            self.apply_match(table, found)?;

            return Ok(());
        }

        // Nothing was found at or after the cursor.
        //
        // Search backward from the cursor.
        let backward =
            searcher.find_backward_match(
                table,
                start,
            )?;

        match backward {
            Some(found) => {
                self.apply_match(table, found)?;
            }

            None => {
                self.current_match = None;
                self.search_failed = true;
            }
        }

        Ok(())
    }

}

// ==========================================================================
// UTF-8 helpers
// ==========================================================================

fn previous_char_boundary(
    text: &str,
    position: usize,
) -> usize {
    if position == 0 {
        return 0;
    }

    let mut position =
        position - 1;

    while position > 0
        && !text.is_char_boundary(position)
    {
        position -= 1;
    }

    position
}

fn next_char_boundary(
    text: &str,
    position: usize,
) -> usize {
    if position >= text.len() {
        return text.len();
    }

    let mut position =
        position + 1;

    while position < text.len()
        && !text.is_char_boundary(position)
    {
        position += 1;
    }

    position
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_table(
        name: &str,
        contents: &[u8],
    ) -> PieceTable {
        let path =
            std::env::temp_dir()
                .join(format!(
                    "potyi_search_ui_{name}.txt"
                ));

        fs::write(
            &path,
            contents,
        )
        .expect("failed to create test file");

        PieceTable::open(
            path.to_str()
                .expect(
                    "test path is not valid UTF-8"
                ),
        )
        .expect("failed to open test file")
    }

    #[test]
    fn new_search_ui_is_closed() {
        let search =
            SearchUi::new();

        assert!(!search.is_active());
        assert!(search.query().is_empty());
        assert!(search.replacement().is_empty());
        assert!(search.current_match().is_none());
        assert!(!search.search_failed());
        assert_eq!(
            search.panel_mode(),
            SearchPanelMode::Find
        );
        assert_eq!(
            search.focused_field(),
            SearchField::Query
        );
        assert_eq!(
            search.query_cursor(),
            0
        );
        assert_eq!(
            search.replacement_cursor(),
            0
        );
        assert_eq!(
            search.last_replace_count(),
            None
        );
    }

    #[test]
    fn replace_panel_transitions_set_expected_focus() {
        let mut table =
            test_table(
                "replace_panel_transitions",
                b"hello world",
            );

        let mut search = SearchUi::new();

        search
            .open_replace(&mut table)
            .unwrap();

        assert!(search.is_active());
        assert!(search.is_replace_mode());
        assert_eq!(
            search.focused_field(),
            SearchField::Query
        );

        search.open(&mut table).unwrap();

        assert_eq!(
            search.panel_mode(),
            SearchPanelMode::Find
        );
        assert_eq!(
            search.focused_field(),
            SearchField::Query
        );

        search
            .open_replace(&mut table)
            .unwrap();

        assert_eq!(
            search.panel_mode(),
            SearchPanelMode::Replace
        );
        assert_eq!(
            search.focused_field(),
            SearchField::Replacement
        );
    }

    #[test]
    fn focus_cycles_only_when_replace_panel_is_open() {
        let mut table =
            test_table(
                "replace_focus_cycle",
                b"hello",
            );

        let mut search = SearchUi::new();

        search.focus_next_field(false);
        assert_eq!(
            search.focused_field(),
            SearchField::Query
        );

        search.open(&mut table).unwrap();
        search.focus_next_field(false);
        search.focus_next_field(true);

        assert_eq!(
            search.focused_field(),
            SearchField::Query
        );

        search
            .open_replace(&mut table)
            .unwrap();

        assert_eq!(
            search.focused_field(),
            SearchField::Replacement
        );

        search.focus_next_field(false);
        assert_eq!(
            search.focused_field(),
            SearchField::Query
        );

        search.focus_next_field(true);
        assert_eq!(
            search.focused_field(),
            SearchField::Replacement
        );
    }

    #[test]
    fn replacement_cursor_editing_is_utf8_safe() {
        let mut table =
            test_table(
                "replacement_unicode_cursor",
                b"hello",
            );

        let mut search = SearchUi::new();

        search
            .open_replace(&mut table)
            .unwrap();
        search.focus_next_field(false);

        search
            .insert_focused_text(
                &mut table,
                "aé🙂z",
            )
            .unwrap();

        assert_eq!(search.replacement(), "aé🙂z");
        assert_eq!(
            search.replacement_cursor(),
            "aé🙂z".len()
        );

        search.move_focused_home();
        search.move_focused_right();
        search
            .delete_focused(&mut table)
            .unwrap();

        assert_eq!(search.replacement(), "a🙂z");
        assert_eq!(search.replacement_cursor(), 1);

        search.move_focused_right();
        search
            .backspace_focused(&mut table)
            .unwrap();

        assert_eq!(search.replacement(), "az");
        assert_eq!(search.replacement_cursor(), 1);

        search.move_focused_end();
        search.move_focused_left();
        search
            .insert_focused_text(
                &mut table,
                "ß",
            )
            .unwrap();

        assert_eq!(search.replacement(), "aßz");
        assert_eq!(search.replacement_cursor(), 3);
    }

    #[test]
    fn replacement_edits_do_not_move_match_or_document_cursor() {
        let mut table =
            test_table(
                "replacement_does_not_search",
                b"alpha beta alpha",
            );

        table.move_cursor(6).unwrap();

        let mut search = SearchUi::new();
        search
            .set_query(&mut table, "alpha")
            .unwrap();
        search
            .open_replace(&mut table)
            .unwrap();
        search.focus_next_field(false);

        let original_match = search.current_match();
        let original_position = table.cursor.position;
        let original_text = table.text().unwrap();

        search
            .insert_focused_text(
                &mut table,
                "omega",
            )
            .unwrap();
        search.move_focused_left();
        search
            .backspace_focused(&mut table)
            .unwrap();

        assert_eq!(search.current_match(), original_match);
        assert_eq!(table.cursor.position, original_position);
        assert_eq!(table.text().unwrap(), original_text);
    }

    #[test]
    fn close_retains_query_replacement_and_modes() {
        let mut table =
            test_table(
                "replace_retained_values",
                b"alpha beta",
            );

        let mut search = SearchUi::new();
        search
            .open_replace(&mut table)
            .unwrap();
        search
            .insert_focused_text(
                &mut table,
                "alpha",
            )
            .unwrap();
        search.focus_next_field(false);
        search
            .insert_focused_text(
                &mut table,
                "omega",
            )
            .unwrap();
        search
            .set_mode(
                &mut table,
                SearchMode::Regex,
            )
            .unwrap();

        search.close();

        assert!(!search.is_active());
        assert_eq!(search.query(), "alpha");
        assert_eq!(search.replacement(), "omega");
        assert_eq!(search.mode(), SearchMode::Regex);
        assert_eq!(
            search.panel_mode(),
            SearchPanelMode::Replace
        );

        search
            .open_replace(&mut table)
            .unwrap();

        assert_eq!(
            search.focused_field(),
            SearchField::Query
        );
        assert_eq!(search.query_cursor(), "alpha".len());
        assert_eq!(
            search.replacement_cursor(),
            "omega".len()
        );
    }

    #[test]
    fn field_edits_and_mode_changes_clear_replace_count() {
        let mut table =
            test_table(
                "replace_count_clears",
                b"hello world",
            );

        let mut search = SearchUi::new();
        search
            .open_replace(&mut table)
            .unwrap();
        search.focus_next_field(false);

        search.set_last_replace_count(Some(2));
        search
            .insert_focused_text(
                &mut table,
                "new",
            )
            .unwrap();
        assert_eq!(search.last_replace_count(), None);

        search.set_last_replace_count(Some(2));
        search.focus_next_field(false);
        search
            .insert_focused_text(
                &mut table,
                "h",
            )
            .unwrap();
        assert_eq!(search.last_replace_count(), None);

        search.set_last_replace_count(Some(2));
        search
            .set_mode(
                &mut table,
                SearchMode::CaseSensitive,
            )
            .unwrap();
        assert_eq!(search.last_replace_count(), None);
    }

    #[test]
    fn previous_boundary_handles_ascii() {
        let text = "hello";

        assert_eq!(
            previous_char_boundary(text, 5),
            4
        );

        assert_eq!(
            previous_char_boundary(text, 1),
            0
        );

        assert_eq!(
            previous_char_boundary(text, 0),
            0
        );
    }

    #[test]
    fn next_boundary_handles_ascii() {
        let text = "hello";

        assert_eq!(
            next_char_boundary(text, 0),
            1
        );

        assert_eq!(
            next_char_boundary(text, 4),
            5
        );

        assert_eq!(
            next_char_boundary(text, 5),
            5
        );
    }

    #[test]
    fn utf8_boundaries_are_correct() {
        let text = "héllo";

        let e_start = 1;
        let e_end = 3;

        assert_eq!(
            next_char_boundary(
                text,
                e_start,
            ),
            e_end
        );

        assert_eq!(
            previous_char_boundary(
                text,
                e_end,
            ),
            e_start
        );
    }

    // ----------------------------------------------------------------------
    // Initial search behaviour
    // ----------------------------------------------------------------------

    #[test]
    fn initial_search_finds_match_at_cursor() {
        let mut table =
            test_table(
                "initial_at_cursor",
                b"hello world world",
            );

        table
            .move_cursor(6)
            .expect("failed to move cursor");

        let mut search =
            SearchUi::new();

        search
            .set_query(
                &mut table,
                "world",
            )
            .expect("search failed");

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 6,
                end: 11,
            })
        );

        assert_eq!(
            table.cursor.position,
            6
        );
    }

    #[test]
    fn initial_search_finds_next_match_after_cursor() {
        let mut table =
            test_table(
                "initial_after_cursor",
                b"hello world world",
            );

        table
            .move_cursor(7)
            .expect("failed to move cursor");

        let mut search =
            SearchUi::new();

        search
            .set_query(
                &mut table,
                "world",
            )
            .expect("search failed");

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 12,
                end: 17,
            })
        );

        assert_eq!(
            table.cursor.position,
            12
        );
    }

    #[test]
    fn initial_search_finds_previous_match_when_cursor_is_at_end() {
        let mut table =
            test_table(
                "initial_from_end",
                b"hello world world",
            );

        let end =
            table.len();

        table
            .move_cursor(end)
            .expect("failed to move cursor");

        let mut search =
            SearchUi::new();

        search
            .set_query(
                &mut table,
                "world",
            )
            .expect("search failed");

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 12,
                end: 17,
            })
        );

        assert_eq!(
            table.cursor.position,
            12
        );

        assert!(
            !search.search_failed()
        );
    }

    #[test]
    fn initial_search_from_end_finds_last_of_multiple_matches() {
        let mut table =
            test_table(
                "initial_last_match",
                b"world hello world hello world",
            );

        table
            .move_cursor(table.len())
            .expect("failed to move cursor");

        let mut search =
            SearchUi::new();

        search
            .set_query(
                &mut table,
                "world",
            )
            .expect("search failed");

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 24,
                end: 29,
            })
        );

        assert_eq!(
            table.cursor.position,
            24
        );
    }

    #[test]
    fn initial_search_fails_when_query_does_not_exist() {
        let mut table =
            test_table(
                "initial_not_found",
                b"hello world",
            );

        table
            .move_cursor(table.len())
            .expect("failed to move cursor");

        let original_position =
            table.cursor.position;

        let mut search =
            SearchUi::new();

        search
            .set_query(
                &mut table,
                "banana",
            )
            .expect("search failed");

        assert!(
            search.current_match().is_none()
        );

        assert!(
            search.search_failed()
        );

        // A failed search must not move the editor cursor.
        assert_eq!(
            table.cursor.position,
            original_position
        );
    }

    // ----------------------------------------------------------------------
    // Navigation
    // ----------------------------------------------------------------------

    #[test]
    fn next_moves_cursor_to_next_match() {
        let mut table =
            test_table(
                "next_moves_cursor",
                b"world hello world",
            );

        table
            .move_cursor(0)
            .unwrap();

        let mut search =
            SearchUi::new();

        search
            .set_query(
                &mut table,
                "world",
            )
            .unwrap();

            search
    .open(&mut table)
    .unwrap();

        assert_eq!(
            table.cursor.position,
            0
        );

        search
            .next(&mut table)
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 12,
                end: 17,
            })
        );

        assert_eq!(
            table.cursor.position,
            12
        );
    }

    #[test]
    fn next_after_initial_match_wraps_to_first_match() {
        let mut table =
            test_table(
                "next_after_initial_match_wraps",
                b"world hello world",
            );

        table
            .move_cursor(0)
            .unwrap();

        let mut search =
            SearchUi::new();

        search
            .set_query(
                &mut table,
                "world",
            )
            .unwrap();

        search
            .open(&mut table)
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 0,
                end: 5,
            })
        );

        assert_eq!(
            table.cursor.position,
            0
        );

        search
            .next(&mut table)
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 12,
                end: 17,
            })
        );

        assert_eq!(
            table.cursor.position,
            12
        );

        search
            .next(&mut table)
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 0,
                end: 5,
            })
        );

        assert_eq!(
            table.cursor.position,
            0
        );

        assert!(
            !search.search_failed()
        );
    }

    #[test]
    fn previous_from_first_match_wraps_to_last_match() {
        let mut table =
            test_table(
                "previous_from_first_match_wraps",
                b"world hello world",
            );

        table
            .move_cursor(0)
            .unwrap();

        let mut search =
            SearchUi::new();

        search
            .set_query(
                &mut table,
                "world",
            )
            .unwrap();

        search
            .open(&mut table)
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 0,
                end: 5,
            })
        );

        assert_eq!(
            table.cursor.position,
            0
        );

        search
            .previous(&mut table)
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 12,
                end: 17,
            })
        );

        assert_eq!(
            table.cursor.position,
            12
        );

        assert!(
            !search.search_failed()
        );
    }

    #[test]
    fn previous_moves_cursor_to_previous_match() {
        let mut table =
            test_table(
                "previous_moves_cursor",
                b"world hello world",
            );

        table
            .move_cursor(12)
            .unwrap();

        let mut search =
            SearchUi::new();

        search
            .set_query(
                &mut table,
                "world",
            )
            .unwrap();
search
    .open(&mut table)
    .unwrap();
        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 12,
                end: 17,
            })
        );

        assert_eq!(
            table.cursor.position,
            12
        );

        search
            .previous(&mut table)
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 0,
                end: 5,
            })
        );

        assert_eq!(
            table.cursor.position,
            0
        );
    }

    #[test]
    fn next_does_not_move_cursor_when_query_has_no_match() {
        let mut table =
            test_table(
                "next_no_match",
                b"hello world",
            );

        table
            .move_cursor(3)
            .unwrap();

        let mut search =
            SearchUi::new();

        search
            .set_query(
                &mut table,
                "banana",
            )
            .unwrap();

        assert_eq!(
            table.cursor.position,
            3
        );

        search
            .next(&mut table)
            .unwrap();

        assert_eq!(
            table.cursor.position,
            3
        );

        assert!(
            search.current_match().is_none()
        );

        assert!(
            search.search_failed()
        );
    }

    #[test]
    fn previous_can_find_a_match_after_an_initial_failure() {
        let mut table =
            test_table(
                "previous_after_failure",
                b"start",
            );

        let mut search = SearchUi::new();
        search.open(&mut table).unwrap();
        search
            .set_query(&mut table, "needle")
            .unwrap();

        assert!(search.current_match().is_none());

        table
            .insert(table.len(), " needle")
            .unwrap();

        search.previous(&mut table).unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 6,
                end: 12,
            })
        );
    }

    #[test]
    fn delete_removes_the_next_unicode_query_character() {
        let mut table =
            test_table(
                "delete_unicode_query",
                b"hello",
            );

        let mut search = SearchUi::new();
        search.open(&mut table).unwrap();
        search
            .set_query(&mut table, "h\u{00e9}llo")
            .unwrap();
        search.move_home();
        search.move_right();

        search.delete(&mut table).unwrap();

        assert_eq!(search.query(), "hllo");
        assert_eq!(search.query_cursor(), 1);
    }

    #[test]
    fn search_cursor_moves_to_utf8_match_boundary() {
        let mut table =
            test_table(
                "utf8_match_cursor",
                "hello café café".as_bytes(),
            );

        table
            .move_cursor(0)
            .unwrap();

        let mut search =
            SearchUi::new();

        search
            .set_query(
                &mut table,
                "café",
            )
            .unwrap();
search
    .open(&mut table)
    .unwrap();
        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 6,
                end: 11,
            })
        );

        assert_eq!(
            table.cursor.position,
            6
        );

        search
            .next(&mut table)
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 12,
                end: 17,
            })
        );

        assert_eq!(
            table.cursor.position,
            12
        );
    }

    #[test]
    fn case_insensitive_mode_finds_mixed_case_text() {
        let mut table =
            test_table(
                "case_insensitive_mode",
                b"Hello there",
            );

        let mut search = SearchUi::new();
        search.open(&mut table).unwrap();
        search
            .set_query(&mut table, "hello")
            .unwrap();

        assert!(search.current_match().is_none());
        assert!(search.search_failed());

        search
            .set_mode(
                &mut table,
                SearchMode::CaseInsensitive,
            )
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 0,
                end: 5,
            })
        );
        assert!(!search.search_failed());
    }

    #[test]
    fn regex_mode_uses_the_actual_match_length() {
        let mut table =
            test_table(
                "regex_match_length",
                b"one 12345 two",
            );

        let mut search = SearchUi::new();
        search.open(&mut table).unwrap();
        search
            .set_mode(
                &mut table,
                SearchMode::Regex,
            )
            .unwrap();
        search
            .set_query(&mut table, r"\d+")
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 4,
                end: 9,
            })
        );
    }

    #[test]
    fn invalid_regex_is_recoverable() {
        let mut table =
            test_table(
                "invalid_regex",
                b"Hello",
            );

        let mut search = SearchUi::new();
        search.open(&mut table).unwrap();
        search
            .set_mode(
                &mut table,
                SearchMode::Regex,
            )
            .unwrap();
        search
            .set_query(&mut table, "(")
            .unwrap();

        assert!(search.pattern_error().is_some());
        assert!(search.current_match().is_none());
        assert!(!search.search_failed());

        search.next(&mut table).unwrap();

        assert!(search.pattern_error().is_some());
        assert!(!search.search_failed());

        search
            .set_query(&mut table, "H.+o")
            .unwrap();

        assert!(search.pattern_error().is_none());
        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 0,
                end: 5,
            })
        );
    }

    #[test]
    fn search_modes_cycle_in_both_directions() {
        let mut table =
            test_table(
                "mode_cycle",
                b"text",
            );

        let mut search = SearchUi::new();

        assert_eq!(
            search.mode(),
            SearchMode::CaseSensitive
        );

        search
            .cycle_mode(&mut table, false)
            .unwrap();
        assert_eq!(
            search.mode(),
            SearchMode::CaseInsensitive
        );

        search
            .cycle_mode(&mut table, false)
            .unwrap();
        assert_eq!(search.mode(), SearchMode::Regex);

        search
            .cycle_mode(&mut table, true)
            .unwrap();
        assert_eq!(
            search.mode(),
            SearchMode::CaseInsensitive
        );
    }

    #[test]
    fn command_search_can_preview_backward_from_a_stable_origin() {
        let mut table =
            test_table(
                "command_search_backward",
                b"one two one",
            );
        table.move_cursor(8).unwrap();

        let mut search = SearchUi::new();
        search
            .configure_command_from(
                &mut table,
                "one",
                None,
                SearchMode::CaseSensitive,
                8,
                true,
            )
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 0,
                end: 3,
            })
        );

        search
            .configure_command_from(
                &mut table,
                "two",
                None,
                SearchMode::CaseSensitive,
                8,
                true,
            )
            .unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 4,
                end: 7,
            })
        );
    }

    #[test]
    fn command_search_preview_restarts_from_its_origin() {
        let mut table =
            test_table(
                "command_search_origin",
                b"red blue red green",
            );

        let mut search = SearchUi::new();
        search
            .configure_command_from(
                &mut table,
                "green",
                None,
                SearchMode::CaseSensitive,
                0,
                false,
            )
            .unwrap();
        assert_eq!(table.cursor.position, 13);

        search
            .configure_command_from(
                &mut table,
                "blue",
                None,
                SearchMode::CaseSensitive,
                0,
                false,
            )
            .unwrap();

        assert_eq!(table.cursor.position, 4);
    }

    #[test]
    fn next_advances_past_zero_width_regex_matches() {
        let mut table =
            test_table(
                "zero_width_regex",
                b"a\nb",
            );

        let mut search = SearchUi::new();
        search.open(&mut table).unwrap();
        search
            .set_mode(
                &mut table,
                SearchMode::Regex,
            )
            .unwrap();
        search.set_query(&mut table, "^").unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 0,
                end: 0,
            })
        );

        search.previous(&mut table).unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 2,
                end: 2,
            })
        );

        search.previous(&mut table).unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 0,
                end: 0,
            })
        );

        search.next(&mut table).unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 2,
                end: 2,
            })
        );

        search.next(&mut table).unwrap();

        assert_eq!(
            search.current_match(),
            Some(SearchMatch {
                start: 0,
                end: 0,
            })
        );
    }
}
