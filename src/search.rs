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

use regex::{Regex, RegexBuilder};

use crate::piece_table::PieceTable;

/// How a search query is interpreted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchMode {
    /// Match the query literally, preserving case.
    #[default]
    CaseSensitive,

    /// Match the query literally, ignoring Unicode case differences.
    CaseInsensitive,

    /// Interpret the query as a regular expression.
    Regex,
}

/// Byte range of a search match in the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub start: usize,
    pub end: usize,
}

/// Text search for the PieceTable.
///
/// Case-sensitive literal search streams directly from the document without
/// creating a complete in-memory copy. Case-insensitive and regex modes use a
/// complete UTF-8 view so Unicode case folding and regex context stay correct.
///
/// Match ranges use the same UTF-8 byte offsets as PieceTable positions, and
/// every match begins and ends on a UTF-8 character boundary.
///
/// The case-sensitive implementation uses Knuth-Morris-Pratt (KMP), which
/// means:
///
///   - searching is O(document length + query length)
///   - the document is never copied into memory
///   - memory usage is O(query length + fixed I/O buffer)
///   - matches can cross PieceTable pieces
///   - matches can cross search-buffer boundaries
///   - arbitrarily large queries are supported
///
/// PieceTable positions are byte offsets, not character offsets.
pub struct Searcher {
    query: Vec<u8>,
    failure: Vec<usize>,
    search_origin: usize,
    regex: Option<Regex>,
}

impl Searcher {
    /// Create a searcher for `query`.
    ///
    /// An empty query never produces a match.
    pub fn new(query: &str) -> Self {
        let query = query.as_bytes().to_vec();
        let failure = build_failure_table(&query);

        Self {
            query,
            failure,
            search_origin: 0,
            regex: None,
        }
    }

    /// Create a searcher using the requested interpretation mode.
    ///
    /// Literal case-sensitive search retains the streaming KMP backend.
    /// Case-insensitive and regular-expression searches use the `regex`
    /// crate and therefore validate/compile their pattern here.
    pub fn with_mode(
        query: &str,
        mode: SearchMode,
    ) -> Result<Self, regex::Error> {
        if mode == SearchMode::CaseSensitive {
            return Ok(Self::new(query));
        }

        let pattern =
            match mode {
                SearchMode::CaseSensitive => unreachable!(),
                SearchMode::CaseInsensitive => regex::escape(query),
                SearchMode::Regex => query.to_owned(),
            };

        let compiled =
            RegexBuilder::new(&pattern)
                .case_insensitive(
                    mode == SearchMode::CaseInsensitive,
                )
                .multi_line(
                    mode == SearchMode::Regex,
                )
                .build()?;

        let query = query.as_bytes().to_vec();
        let failure = build_failure_table(&query);

        Ok(Self {
            query,
            failure,
            search_origin: 0,
            regex: Some(compiled),
        })
    }

    /// Returns true when the search query is empty.
    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    /// Returns the query length in bytes.
    pub fn len(&self) -> usize {
        self.query.len()
    }

    /// Visit every leftmost, non-overlapping match in document order.
    ///
    /// Literal case-sensitive search streams through the PieceTable once
    /// using a fixed-size buffer. Regex-backed modes materialize one UTF-8
    /// view of the document and use the regex iterator's non-overlapping
    /// match semantics, including its handling of zero-width matches.
    ///
    /// Returns the number of matches successfully visited. An empty query
    /// never invokes `visit` and returns zero.
    pub fn visit_non_overlapping_matches<F>(
        &self,
        table: &PieceTable,
        mut visit: F,
    ) -> io::Result<usize>
    where
        F: FnMut(SearchResult) -> io::Result<()>,
    {
        self.visit_non_overlapping_matches_with_bytes(
            table,
            |found, _| visit(found),
        )
    }

    /// Visit only matches whose bytes differ from `replacement`.
    ///
    /// The matched bytes come from the search pass itself, so callers do not
    /// need to read each range back through the piece table. This keeps a
    /// replace-all pass linear even when the document contains many pieces.
    pub fn visit_non_overlapping_replacements<F>(
        &self,
        table: &PieceTable,
        replacement: &str,
        mut visit: F,
    ) -> io::Result<usize>
    where
        F: FnMut(SearchResult) -> io::Result<()>,
    {
        if self.regex.is_none()
            && self.query
                == replacement.as_bytes()
        {
            return Ok(0);
        }

        let mut changed_count = 0usize;

        self.visit_non_overlapping_matches_with_bytes(
            table,
            |found, matched_bytes| {
                if matched_bytes
                    == replacement.as_bytes()
                {
                    return Ok(());
                }

                visit(found)?;

                changed_count =
                    changed_count
                        .checked_add(1)
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "search match count overflow",
                            )
                        })?;

                Ok(())
            },
        )?;

        Ok(changed_count)
    }

    fn visit_non_overlapping_matches_with_bytes<F>(
        &self,
        table: &PieceTable,
        mut visit: F,
    ) -> io::Result<usize>
    where
        F: FnMut(
            SearchResult,
            &[u8],
        ) -> io::Result<()>,
    {
        if self.query.is_empty() {
            return Ok(0);
        }

        if let Some(regex) = &self.regex {
            let text = table.text()?;
            let mut count = 0usize;

            for found in regex.find_iter(&text) {
                let result =
                    SearchResult {
                        start: found.start(),
                        end: found.end(),
                    };

                visit(
                    result,
                    &text.as_bytes()[
                        result.start..result.end
                    ],
                )?;

                count = count
                    .checked_add(1)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "search match count overflow",
                        )
                    })?;
            }

            return Ok(count);
        }

        let document_length = table.len();

        if self.query.len() > document_length {
            return Ok(0);
        }

        let mut matched = 0usize;
        let mut consumed = 0usize;
        let mut count = 0usize;
        let mut pending_match:
            Option<SearchResult> = None;

        table.visit_chunks(
            |chunk| {
                for &byte in chunk {
                    if let Some(found) =
                        pending_match.take()
                    {
                        if (byte & 0xC0) != 0x80 {
                            visit(
                                found,
                                &self.query,
                            )?;

                            count = count
                                .checked_add(1)
                                .ok_or_else(|| {
                                    io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        "search match count overflow",
                                    )
                                })?;

                            matched = 0;
                        } else {
                            matched =
                                self.failure[
                                    self.query.len() - 1
                                ];
                        }
                    }

                    while matched > 0
                        && self.query[matched] != byte
                    {
                        matched = self.failure[matched - 1];
                    }

                    if self.query[matched] == byte {
                        matched += 1;
                    }

                    consumed =
                        consumed
                            .checked_add(1)
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "search position overflow",
                                )
                            })?;

                    if matched == self.query.len() {
                        let start =
                            consumed - self.query.len();

                        let end =
                            start
                                .checked_add(self.query.len())
                                .ok_or_else(|| {
                                    io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        "search position overflow",
                                    )
                                })?;

                        /*
                         * The query starts with a UTF-8 leading byte, so the
                         * start is already a boundary. Defer the result until
                         * the next byte confirms the end boundary; this avoids
                         * a fresh piece-table scan for every match.
                         */
                        pending_match =
                            Some(
                                SearchResult {
                                    start,
                                    end,
                                }
                            );
                    }
                }

                Ok(())
            }
        )?;

        if let Some(found) =
            pending_match
        {
            visit(
                found,
                &self.query,
            )?;

            count = count
                .checked_add(1)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "search match count overflow",
                    )
                })?;
        }

        Ok(count)
    }

    /// Search forward from `start`.
    ///
    /// Returns the first match whose starting byte position is at or after
    /// `start`.
    ///
    /// The search does not wrap around.
    pub fn find_forward(
        &self,
        table: &PieceTable,
        start: usize,
    ) -> io::Result<Option<usize>> {
        Ok(
            self.find_forward_match(table, start)?
                .map(|found| found.start),
        )
    }

    /// Search forward and return the complete byte range of the match.
    ///
    /// Returns the first match whose starting byte position is at or after
    /// `start`. The search does not wrap around.
    pub fn find_forward_match(
        &self,
        table: &PieceTable,
        start: usize,
    ) -> io::Result<Option<SearchResult>> {
        if self.query.is_empty() {
            return Ok(None);
        }

        if let Some(regex) = &self.regex {
            return find_regex_forward(
                regex,
                table,
                start,
            );
        }

        let document_length = table.len();

        if start > document_length {
            return Ok(None);
        }

        if self.query.len() > document_length.saturating_sub(start) {
            return Ok(None);
        }

        /*
         * KMP state:
         *
         * matched = number of query bytes currently matched.
         *
         * Unlike a normal substring search, KMP does not need overlapping
         * document buffers. This is important for very large queries.
         */
        let mut matched = 0usize;

        /*
         * Number of bytes consumed from the document since `start`.
         *
         * This is used to calculate the absolute start position when KMP
         * reports a match.
         */
        let mut consumed = 0usize;

        /*
         * Keep the document I/O buffer fixed for normal searches.
         *
         * The query itself is stored separately, so a 1 MB query does not
         * force us to allocate a 1 MB document buffer.
         */
        let mut buffer = vec![0u8; SEARCH_BUFFER_SIZE];

        let mut position = start;

        while position < document_length {
            let remaining = document_length - position;
            let amount = remaining.min(buffer.len());

            let read =
                table.read_range_into(
                    position,
                    &mut buffer[..amount],
                )?;

            if read == 0 {
                break;
            }

            for &byte in &buffer[..read] {
                /*
                 * KMP fallback.
                 *
                 * This can discard already matched query bytes without
                 * moving backwards through the document.
                 */
                while matched > 0
                    && self.query[matched] != byte
                {
                    matched = self.failure[matched - 1];
                }

                if self.query[matched] == byte {
                    matched += 1;
                }

                consumed += 1;

                if matched == self.query.len() {
                    let candidate =
                        start + consumed - self.query.len();

                    let candidate_end =
                        candidate
                            .checked_add(self.query.len())
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "search position overflow",
                                )
                            })?;

                    /*
                     * The query was created from a Rust &str, so it is valid
                     * UTF-8. A byte sequence equal to it can still occur
                     * inside another UTF-8 character if the query is
                     * manually constructed at the byte level. Require both
                     * ends to be character boundaries.
                     */
                    if is_utf8_boundary(
                        table,
                        candidate,
                    )?
                        && is_utf8_boundary(
                            table,
                            candidate_end,
                        )?
                    {
                        return Ok(
                            Some(SearchResult {
                                start: candidate,
                                end: candidate_end,
                            })
                        );
                    }

                    /*
                     * The byte sequence matched, but it was not a valid
                     * character-aligned text match.
                     *
                     * Continue KMP rather than restarting from scratch.
                     */
                    matched =
                        self.failure[matched - 1];
                }
            }

            position += read;
        }

        Ok(None)
    }

/// Search backward from `start`.
///
/// Returns the last match whose starting byte position is at or before
/// `start`.
///
/// This makes the function convenient for "find previous" in an editor:
///
///     find_backward(cursor_position)
///
/// A match containing the cursor is considered a valid match. This means
/// that when the cursor is at the beginning of a match, that match is still
/// returned.
///
/// The search does not wrap around.
pub fn find_backward(
    &self,
    table: &PieceTable,
    start: usize,
) -> io::Result<Option<usize>> {
    Ok(
        self.find_backward_match(table, start)?
            .map(|found| found.start),
    )
}

/// Search backward and return the complete byte range of the match.
///
/// Returns the last match whose starting byte position is at or before
/// `start`. The search does not wrap around.
pub fn find_backward_match(
    &self,
    table: &PieceTable,
    start: usize,
) -> io::Result<Option<SearchResult>> {
    if self.query.is_empty() {
        return Ok(None);
    }

    if let Some(regex) = &self.regex {
        return find_regex_backward(
            regex,
            table,
            start,
        );
    }

    let document_length = table.len();

    if document_length == 0 {
        return Ok(None);
    }

    let search_end = start.min(document_length);
    
    /*
     * We need to scan far enough to discover a match whose start is
     * exactly at `search_end`.
     *
     * For example:
     *
     *     "hello world world"
     *                 ^
     *                 cursor
     *
     * The second "world" starts exactly at the cursor and must therefore
     * be considered.
     *
     * A match can extend beyond the cursor, so we must scan up to:
     *
     *     search_end + query.len()
     *
     * rather than stopping at `search_end`.
     */
    let scan_end = search_end
        .saturating_add(self.query.len())
        .min(document_length);

    let mut matched = 0usize;
    let mut consumed = 0usize;
    let mut last_match: Option<SearchResult> = None;

    let mut buffer = vec![0u8; SEARCH_BUFFER_SIZE];

    let mut position = 0usize;

    while position < scan_end {
        let remaining = scan_end - position;
        let amount = remaining.min(buffer.len());

        let read =
            table.read_range_into(
                position,
                &mut buffer[..amount],
            )?;

        if read == 0 {
            break;
        }

        for &byte in &buffer[..read] {
            while matched > 0
                && self.query[matched] != byte
            {
                matched =
                    self.failure[matched - 1];
            }

            if self.query[matched] == byte {
                matched += 1;
            }

            consumed += 1;

            if matched == self.query.len() {
                let candidate =
                    consumed - self.query.len();

                let candidate_end =
                    candidate
                        .checked_add(self.query.len())
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "search position overflow",
                            )
                        })?;

                /*
                 * The match is valid when its START is at or before the
                 * cursor.
                 *
                 * It is intentionally allowed to extend beyond the cursor.
                 *
                 * This gives us:
                 *
                 *     hello wor|ld world
                 *              ^
                 *
                 *     -> first "world"
                 *
                 * and:
                 *
                 *     hello world |world
                 *                ^
                 *
                 *     -> second "world"
                 */
                if candidate <= search_end
                    && candidate_end <= document_length
                    && is_utf8_boundary(
                        table,
                        candidate,
                    )?
                    && is_utf8_boundary(
                        table,
                        candidate_end,
                    )?
                {
                    last_match =
                        Some(SearchResult {
                            start: candidate,
                            end: candidate_end,
                        });
                }

                /*
                 * Continue KMP so that we can find a later match.
                 */
                matched =
                    self.failure[matched - 1];
            }
        }

        position += read;
    }

    Ok(last_match)
}

}

/// Run a compiled regex against the complete document while beginning the
/// search at `start`.
///
/// Passing the complete haystack to `find_at` is important: slicing at
/// `start` would make anchors and word boundaries observe an artificial
/// beginning of text.
fn find_regex_forward(
    regex: &Regex,
    table: &PieceTable,
    start: usize,
) -> io::Result<Option<SearchResult>> {
    let text = table.text()?;

    if start > text.len() {
        return Ok(None);
    }

    let start = next_utf8_boundary(&text, start);

    Ok(
        regex.find_at(&text, start)
            .map(|found| SearchResult {
                start: found.start(),
                end: found.end(),
            })
    )
}

/// Find the last regex match whose start is at or before `start`.
///
/// Searching against the complete document preserves the context needed by
/// anchors and word boundaries. Advancing from each match's start preserves
/// overlapping results, matching the literal-search behaviour.
fn find_regex_backward(
    regex: &Regex,
    table: &PieceTable,
    start: usize,
) -> io::Result<Option<SearchResult>> {
    let text = table.text()?;
    let search_end = start.min(text.len());
    let mut candidate_start = 0usize;
    let mut last_match = None;

    while candidate_start <= search_end {
        let found =
            match regex.find_at(
                &text,
                candidate_start,
            ) {
                Some(found)
                    if found.start() <= search_end => found,
                _ => break,
            };

        last_match =
            Some(SearchResult {
                start: found.start(),
                end: found.end(),
            });

        /*
         * Move one Unicode scalar past the match start, rather than past the
         * match end. This preserves overlapping matches and also guarantees
         * progress for zero-width expressions.
         */
        if found.start() == text.len() {
            break;
        }

        candidate_start =
            next_utf8_boundary(
                &text,
                found.start() + 1,
            );
    }

    Ok(last_match)
}

/// Return the first UTF-8 boundary at or after `position`.
fn next_utf8_boundary(
    text: &str,
    position: usize,
) -> usize {
    let mut position = position.min(text.len());

    while position < text.len()
        && !text.is_char_boundary(position)
    {
        position += 1;
    }

    position
}

// ==========================================================================
// Internal search helpers
// ==========================================================================

/// Size of the fixed document I/O buffer.
///
/// This is deliberately independent of the query size. A 10 MB query does
/// not require a 10 MB document buffer.
const SEARCH_BUFFER_SIZE: usize = 64 * 1024;

/// Build the KMP failure table.
///
/// failure[i] contains the length of the longest proper prefix of the query
/// that is also a suffix ending at `i`.
///
/// Memory: O(query length).
fn build_failure_table(
    query: &[u8],
) -> Vec<usize> {
    let mut failure =
        vec![0usize; query.len()];

    let mut matched = 0usize;
    let mut index = 1usize;

    while index < query.len() {
        while matched > 0
            && query[index] != query[matched]
        {
            matched =
                failure[matched - 1];
        }

        if query[index] == query[matched] {
            matched += 1;
        }

        failure[index] = matched;
        index += 1;
    }

    failure
}

/// Check whether `position` is a UTF-8 character boundary.
///
/// PieceTable positions are byte offsets, so this prevents a search result
/// from beginning or ending in the middle of a UTF-8 character.
fn is_utf8_boundary(
    table: &PieceTable,
    position: usize,
) -> io::Result<bool> {
    if position == 0
        || position == table.len()
    {
        return Ok(true);
    }

    match table.byte_at(position)? {
        Some(byte) => Ok(
            (byte & 0xC0) != 0x80
        ),
        None => Ok(false),
    }
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::{self, File};
    use std::io::Write;

    fn make_table(
        name: &str,
        text: &str,
    ) -> PieceTable {
        let mut path =
            std::env::temp_dir();

        path.push(format!(
            "potyi_search_{name}.txt"
        ));

        {
            let mut file =
                File::create(&path)
                    .unwrap();

            file.write_all(
                text.as_bytes()
            )
            .unwrap();
        }

        let path_string =
            path.to_string_lossy()
                .to_string();

        let table =
            PieceTable::open(
                &path_string
            )
            .unwrap();

        fs::remove_file(path)
            .ok();

        table
    }

    // ----------------------------------------------------------------------
    // Basic search
    // ----------------------------------------------------------------------

    #[test]
    fn finds_forward() {
        let table =
            make_table(
                "forward",
                "hello world hello",
            );

        let searcher =
            Searcher::new("hello");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(0)
        );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    1,
                )
                .unwrap(),
            Some(12)
        );
    }

    #[test]
    fn finds_backward() {
        let table =
            make_table(
                "backward",
                "hello world hello",
            );

        let searcher =
            Searcher::new("hello");

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            Some(12)
        );

assert_eq!(
    searcher
        .find_backward(
            &table,
            12,
        )
        .unwrap(),
    Some(12)
);
    }

    #[test]
    fn returns_none_when_not_found() {
        let table =
            make_table(
                "not_found",
                "hello world",
            );

        let searcher =
            Searcher::new("xyz");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            None
        );

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            None
        );
    }

    #[test]
    fn empty_query_never_matches() {
        let table =
            make_table(
                "empty",
                "hello world",
            );

        let searcher =
            Searcher::new("");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            None
        );

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            None
        );
    }

    // ----------------------------------------------------------------------
    // UTF-8
    // ----------------------------------------------------------------------

    #[test]
    fn unicode_search_is_safe() {
        let table =
            make_table(
                "unicode",
                "hello café world café",
            );

        let searcher =
            Searcher::new("café");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(6)
        );

        /*
         * "hello café world café"
         *
         * The second "café" starts at byte 18, not 17.
         *
         * "world" occupies bytes 12..17 and the space is byte 17.
         */
        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    7,
                )
                .unwrap(),
            Some(18)
        );

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            Some(18)
        );
    }

    #[test]
    fn does_not_match_inside_utf8_character() {
        let table =
            make_table(
                "utf8_boundary",
                "é",
            );

        /*
         * 0xA9 is the second byte of é. Searching for that byte alone must
         * not produce a text match.
         */
        let searcher =
            Searcher {
                query: vec![0xA9],
                failure: vec![0],
                search_origin: 0,
                regex: None,
            };

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            None
        );

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            None
        );
    }


    #[test]
fn finds_world_backward_from_end() {
    let table =
        make_table(
            "world_from_end",
            "hello world world",
        );

    let searcher =
        Searcher::new("world");

    assert_eq!(
        searcher
            .find_backward(
                &table,
                table.len(),
            )
            .unwrap(),
        Some(12)
    );
}

    #[test]
    fn unicode_search_finds_multibyte_query() {
        let table =
            make_table(
                "unicode_multibyte",
                "one 😀 two 😀 three",
            );

        let searcher =
            Searcher::new("😀");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(4)
        );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    5,
                )
                .unwrap(),
            Some(13)
        );

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            Some(13)
        );
    }

    #[test]
    fn unicode_ascii_mixed_search() {
        let table =
            make_table(
                "unicode_mixed",
                "abc é xyz é abc",
            );

        let searcher =
            Searcher::new("é xyz");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(4)
        );
    }

    // ----------------------------------------------------------------------
    // Lines
    // ----------------------------------------------------------------------

    #[test]
    fn search_can_find_across_lines() {
        let table =
            make_table(
                "lines",
                "hello\nworld\nhello world",
            );

        let searcher =
            Searcher::new(
                "world\nhello",
            );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(6)
        );
    }

    #[test]
    fn search_can_find_across_multiple_lines() {
        let table =
            make_table(
                "multiple_lines",
                "aaa\nbbb\nccc\nddd",
            );

        let searcher =
            Searcher::new(
                "bbb\nccc\nddd",
            );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(4)
        );
    }

    // ----------------------------------------------------------------------
    // Start positions
    // ----------------------------------------------------------------------

    #[test]
    fn search_after_start_position() {
        let table =
            make_table(
                "after",
                "abc abc abc",
            );

        let searcher =
            Searcher::new("abc");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    3,
                )
                .unwrap(),
            Some(4)
        );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    5,
                )
                .unwrap(),
            Some(8)
        );
    }

    #[test]
    fn forward_search_at_exact_match_position() {
        let table =
            make_table(
                "exact_position",
                "abc abc",
            );

        let searcher =
            Searcher::new("abc");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    4,
                )
                .unwrap(),
            Some(4)
        );
    }

    #[test]
    fn forward_search_at_end_returns_none() {
        let table =
            make_table(
                "forward_end",
                "abc",
            );

        let searcher =
            Searcher::new("abc");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            None
        );
    }

    #[test]
    fn forward_search_beyond_end_returns_none() {
        let table =
            make_table(
                "forward_beyond_end",
                "abc",
            );

        let searcher =
            Searcher::new("abc");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    table.len() + 100,
                )
                .unwrap(),
            None
        );
    }

#[test]
fn backward_search_includes_match_at_start() {
    let table =
        make_table(
            "backward_includes_start",
            "abc abc abc",
        );

    let searcher =
        Searcher::new("abc");

    assert_eq!(
        searcher
            .find_backward(
                &table,
                8,
            )
            .unwrap(),
        Some(8)
    );
}

    #[test]
fn backward_search_from_exact_match_includes_match() {
    let table =
        make_table(
            "backward_exact_inclusive",
            "abc abc",
        );

    let searcher =
        Searcher::new("abc");

    assert_eq!(
        searcher
            .find_backward(
                &table,
                4,
            )
            .unwrap(),
        Some(4)
    );
}

#[test]
fn backward_search_from_end_finds_last_match() {
    let table =
        make_table(
            "backward_from_end",
            "hello world world",
        );

    let searcher =
        Searcher::new("world");

    assert_eq!(
        searcher
            .find_backward(
                &table,
                table.len(),
            )
            .unwrap(),
        Some(12)
    );
}

#[test]
fn backward_search_cursor_at_match_start() {
    let table =
        make_table(
            "backward_cursor_at_start",
            "hello world world",
        );

    let searcher =
        Searcher::new("world");

    // Second "world" begins at byte 12.
    assert_eq!(
        searcher
            .find_backward(
                &table,
                12,
            )
            .unwrap(),
        Some(12)
    );
}
    
#[test]
fn backward_search_at_zero_finds_match() {
    let table =
        make_table(
            "backward_zero",
            "abc",
        );

    let searcher =
        Searcher::new("abc");

    assert_eq!(
        searcher
            .find_backward(
                &table,
                0,
            )
            .unwrap(),
        Some(0)
    );
}
    // ----------------------------------------------------------------------
    // Empty / short documents
    // ----------------------------------------------------------------------

    #[test]
    fn handles_query_larger_than_document() {
        let table =
            make_table(
                "large_query",
                "abc",
            );

        let searcher =
            Searcher::new("abcdef");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            None
        );

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            None
        );
    }

    #[test]
    fn handles_empty_document() {
        let table =
            make_table(
                "empty_document",
                "",
            );

        let searcher =
            Searcher::new("hello");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            None
        );

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    0,
                )
                .unwrap(),
            None
        );
    }

    #[test]
    fn query_equal_to_document_matches() {
        let table =
            make_table(
                "equal_document",
                "hello world",
            );

        let searcher =
            Searcher::new("hello world");

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(0)
        );

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            Some(0)
        );
    }

    // ----------------------------------------------------------------------
    // Repeated patterns / KMP-specific cases
    // ----------------------------------------------------------------------

    #[test]
    fn handles_repeated_pattern() {
        let table =
            make_table(
                "repeated_pattern",
                "aaaaaaaaaaaaaaaaab",
            );

        let searcher =
            Searcher::new(
                "aaaaaaaab",
            );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(9)
        );
    }

    #[test]
    fn handles_highly_repetitive_query() {
        let text =
            format!(
                "{}b",
                "a".repeat(100_000),
            );

        let table =
            make_table(
                "repetitive_large",
                &text,
            );

        let searcher =
            Searcher::new(
                &format!(
                    "{}b",
                    "a".repeat(50_000),
                ),
            );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(50_000)
        );
    }

    #[test]
    fn finds_later_repeated_match_backward() {
        let table =
            make_table(
                "repeated_backward",
                "aaaa abc aaaa abc aaaa",
            );

        let searcher =
            Searcher::new("aaaa");

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            Some(18)
        );
    }

    // ----------------------------------------------------------------------
    // Piece boundaries
    // ----------------------------------------------------------------------

    #[test]
    fn search_finds_text_across_pieces() {
        let mut table =
            make_table(
                "piece_boundary",
                "hello world",
            );

        table
            .insert(
                6,
                "beautiful ",
            )
            .unwrap();

        let searcher =
            Searcher::new(
                "world",
            );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(16)
        );
    }

    #[test]
    fn search_finds_match_created_by_edit() {
        let mut table =
            make_table(
                "piece_edit",
                "hello world",
            );

        table
            .insert(
                5,
                " amazing",
            )
            .unwrap();

        let searcher =
            Searcher::new(
                "hello amazing",
            );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(0)
        );
    }

    #[test]
    fn search_finds_match_after_delete() {
        let mut table =
            make_table(
                "piece_delete",
                "hello cruel world",
            );

        table
            .delete(
                6,
                6,
            )
            .unwrap();

        let searcher =
            Searcher::new(
                "hello world",
            );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(0)
        );
    }

    // ----------------------------------------------------------------------
    // Search buffer boundaries
    // ----------------------------------------------------------------------

    #[test]
    fn finds_match_across_search_buffer_boundary() {
        let prefix =
            "a".repeat(
                SEARCH_BUFFER_SIZE - 5
            );

        let text =
            format!(
                "{}HELLO WORLD",
                prefix
            );

        let table =
            make_table(
                "buffer_boundary",
                &text,
            );

        let searcher =
            Searcher::new(
                "HELLO WORLD",
            );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(
                SEARCH_BUFFER_SIZE - 5
            )
        );
    }

    #[test]
    fn finds_match_starting_before_buffer_boundary() {
        let prefix =
            "a".repeat(
                SEARCH_BUFFER_SIZE - 3
            );

        let text =
            format!(
                "{}abcdef",
                prefix
            );

        let table =
            make_table(
                "buffer_boundary_start",
                &text,
            );

        let searcher =
            Searcher::new(
                "abcdef",
            );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(
                SEARCH_BUFFER_SIZE - 3
            )
        );
    }

    #[test]
    fn backward_finds_match_across_buffer_boundary() {
        let prefix =
            "a".repeat(
                SEARCH_BUFFER_SIZE - 3
            );

        let text =
            format!(
                "{}abcdef",
                prefix
            );

        let table =
            make_table(
                "backward_buffer_boundary",
                &text,
            );

        let searcher =
            Searcher::new(
                "abcdef",
            );

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            Some(
                SEARCH_BUFFER_SIZE - 3
            )
        );
    }

    // ----------------------------------------------------------------------
    // Large queries
    // ----------------------------------------------------------------------

#[test]
fn finds_large_query_without_large_memory_usage() {
    // The document begins with 70,000 'b' bytes so the query cannot
    // accidentally match at position 0.
    let prefix = "b".repeat(70_000);

    // The query itself is larger than SEARCH_CHUNK_SIZE.
    let query_prefix = "a".repeat(70_000);

    let text = format!(
        "{}{}TARGETTAIL",
        prefix,
        query_prefix,
    );

    let query = format!(
        "{}TARGET",
        query_prefix,
    );

    let table = make_table(
        "large_query_streaming",
        &text,
    );

    let searcher = Searcher::new(&query);

    assert_eq!(
        searcher
            .find_forward(&table, 0)
            .unwrap(),
        Some(70_000)
    );
}
    #[test]
    fn finds_query_much_larger_than_search_buffer() {
        let query =
            "x".repeat(
                SEARCH_BUFFER_SIZE * 2 + 123
            );

        let prefix =
            "a".repeat(10_000);

        let text =
            format!(
                "{}{}TAIL",
                prefix,
                query,
            );

        let table =
            make_table(
                "very_large_query",
                &text,
            );

        let searcher =
            Searcher::new(
                &query,
            );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(10_000)
        );

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            Some(10_000)
        );
    }

    #[test]
    fn large_query_not_found() {
        let text =
            "a".repeat(
                SEARCH_BUFFER_SIZE * 2
            );

        let query =
            "b".repeat(
                SEARCH_BUFFER_SIZE + 100
            );

        let table =
            make_table(
                "large_query_not_found",
                &text,
            );

        let searcher =
            Searcher::new(
                &query,
            );

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            None
        );

        assert_eq!(
            searcher
                .find_backward(
                    &table,
                    table.len(),
                )
                .unwrap(),
            None
        );
    }

    // ----------------------------------------------------------------------
    // Search modes and match ranges
    // ----------------------------------------------------------------------

    #[test]
    fn case_sensitive_mode_preserves_literal_search() {
        let table =
            make_table(
                "case_sensitive_mode",
                "Hello hello",
            );

        let searcher =
            Searcher::with_mode(
                "hello",
                SearchMode::CaseSensitive,
            )
            .unwrap();

        assert_eq!(
            searcher
                .find_forward_match(
                    &table,
                    0,
                )
                .unwrap(),
            Some(SearchResult {
                start: 6,
                end: 11,
            })
        );
    }

    #[test]
    fn case_insensitive_mode_matches_unicode_case() {
        let table =
            make_table(
                "case_insensitive_unicode",
                "CAFÉ café",
            );

        let searcher =
            Searcher::with_mode(
                "café",
                SearchMode::CaseInsensitive,
            )
            .unwrap();

        assert_eq!(
            searcher
                .find_forward_match(
                    &table,
                    0,
                )
                .unwrap(),
            Some(SearchResult {
                start: 0,
                end: 5,
            })
        );

        assert_eq!(
            searcher
                .find_forward_match(
                    &table,
                    1,
                )
                .unwrap(),
            Some(SearchResult {
                start: 6,
                end: 11,
            })
        );
    }

    #[test]
    fn case_insensitive_mode_treats_regex_syntax_literally() {
        let table =
            make_table(
                "case_insensitive_literal",
                "axb A.B",
            );

        let searcher =
            Searcher::with_mode(
                "a.b",
                SearchMode::CaseInsensitive,
            )
            .unwrap();

        assert_eq!(
            searcher
                .find_forward_match(
                    &table,
                    0,
                )
                .unwrap(),
            Some(SearchResult {
                start: 4,
                end: 7,
            })
        );
    }

    #[test]
    fn case_insensitive_backward_search_finds_overlapping_match() {
        let table =
            make_table(
                "case_insensitive_overlap",
                "AbAbA",
            );

        let searcher =
            Searcher::with_mode(
                "aba",
                SearchMode::CaseInsensitive,
            )
            .unwrap();

        assert_eq!(
            searcher
                .find_backward_match(
                    &table,
                    table.len(),
                )
                .unwrap(),
            Some(SearchResult {
                start: 2,
                end: 5,
            })
        );
    }

    #[test]
    fn regex_mode_returns_variable_length_match_ranges() {
        let table =
            make_table(
                "regex_variable_length",
                "id=7 id=12345",
            );

        let searcher =
            Searcher::with_mode(
                r"id=\d+",
                SearchMode::Regex,
            )
            .unwrap();

        assert_eq!(
            searcher
                .find_forward_match(
                    &table,
                    0,
                )
                .unwrap(),
            Some(SearchResult {
                start: 0,
                end: 4,
            })
        );

        assert_eq!(
            searcher
                .find_forward_match(
                    &table,
                    4,
                )
                .unwrap(),
            Some(SearchResult {
                start: 5,
                end: 13,
            })
        );

        assert_eq!(
            searcher
                .find_backward_match(
                    &table,
                    table.len(),
                )
                .unwrap(),
            Some(SearchResult {
                start: 5,
                end: 13,
            })
        );
    }

    #[test]
    fn regex_backward_search_finds_overlapping_match() {
        let table =
            make_table(
                "regex_overlap",
                "ababa",
            );

        let searcher =
            Searcher::with_mode(
                "aba",
                SearchMode::Regex,
            )
            .unwrap();

        assert_eq!(
            searcher
                .find_backward_match(
                    &table,
                    table.len(),
                )
                .unwrap(),
            Some(SearchResult {
                start: 2,
                end: 5,
            })
        );
    }

    #[test]
    fn regex_search_keeps_context_before_start() {
        let table =
            make_table(
                "regex_full_context",
                "xfoo foo",
            );

        let searcher =
            Searcher::with_mode(
                r"\bfoo",
                SearchMode::Regex,
            )
            .unwrap();

        assert_eq!(
            searcher
                .find_forward_match(
                    &table,
                    1,
                )
                .unwrap(),
            Some(SearchResult {
                start: 5,
                end: 8,
            })
        );
    }

    #[test]
    fn regex_mode_reports_invalid_patterns() {
        assert!(
            Searcher::with_mode(
                "[",
                SearchMode::Regex,
            )
            .is_err()
        );
    }

    #[test]
    fn regex_zero_width_matches_have_exact_ranges() {
        let table =
            make_table(
                "regex_zero_width",
                "abc",
            );

        let searcher =
            Searcher::with_mode(
                r"^|$",
                SearchMode::Regex,
            )
            .unwrap();

        assert_eq!(
            searcher
                .find_forward_match(
                    &table,
                    0,
                )
                .unwrap(),
            Some(SearchResult {
                start: 0,
                end: 0,
            })
        );

        assert_eq!(
            searcher
                .find_forward_match(
                    &table,
                    1,
                )
                .unwrap(),
            Some(SearchResult {
                start: 3,
                end: 3,
            })
        );

        assert_eq!(
            searcher
                .find_backward_match(
                    &table,
                    table.len(),
                )
                .unwrap(),
            Some(SearchResult {
                start: 3,
                end: 3,
            })
        );

        assert_eq!(
            searcher
                .find_backward_match(
                    &table,
                    2,
                )
                .unwrap(),
            Some(SearchResult {
                start: 0,
                end: 0,
            })
        );
    }

    #[test]
    fn legacy_start_only_api_honours_selected_mode() {
        let table =
            make_table(
                "legacy_mode_api",
                "Hello",
            );

        let searcher =
            Searcher::with_mode(
                "hello",
                SearchMode::CaseInsensitive,
            )
            .unwrap();

        assert_eq!(
            searcher
                .find_forward(
                    &table,
                    0,
                )
                .unwrap(),
            Some(0)
        );
    }

    // ----------------------------------------------------------------------
    // Non-overlapping match visitation
    // ----------------------------------------------------------------------

    #[test]
    fn visitor_reports_leftmost_non_overlapping_literal_matches() {
        let table =
            make_table(
                "visitor_non_overlapping",
                "aaaa",
            );

        let searcher =
            Searcher::new("aa");

        let mut matches = Vec::new();

        let count =
            searcher
                .visit_non_overlapping_matches(
                    &table,
                    |found| {
                        matches.push(found);
                        Ok(())
                    },
                )
                .unwrap();

        assert_eq!(count, 2);
        assert_eq!(
            matches,
            vec![
                SearchResult {
                    start: 0,
                    end: 2,
                },
                SearchResult {
                    start: 2,
                    end: 4,
                },
            ]
        );
    }

    #[test]
    fn visitor_finds_literal_match_across_piece_boundaries() {
        let mut table =
            make_table(
                "visitor_piece_boundary",
                "abef--abef",
            );

        table
            .insert(2, "cd")
            .unwrap();

        let searcher =
            Searcher::new("cdef");

        let mut matches = Vec::new();

        let count =
            searcher
                .visit_non_overlapping_matches(
                    &table,
                    |found| {
                        matches.push(found);
                        Ok(())
                    },
                )
                .unwrap();

        assert_eq!(count, 1);
        assert_eq!(
            matches,
            vec![
                SearchResult {
                    start: 2,
                    end: 6,
                },
            ]
        );
    }

    #[test]
    fn visitor_keeps_kmp_state_across_search_buffer_boundary() {
        let prefix =
            "x".repeat(
                SEARCH_BUFFER_SIZE - 1
            );

        let text =
            format!("{prefix}abab");

        let table =
            make_table(
                "visitor_buffer_boundary",
                &text,
            );

        let searcher =
            Searcher::new("ab");

        let mut matches = Vec::new();

        let count =
            searcher
                .visit_non_overlapping_matches(
                    &table,
                    |found| {
                        matches.push(found);
                        Ok(())
                    },
                )
                .unwrap();

        assert_eq!(count, 2);
        assert_eq!(
            matches,
            vec![
                SearchResult {
                    start: SEARCH_BUFFER_SIZE - 1,
                    end: SEARCH_BUFFER_SIZE + 1,
                },
                SearchResult {
                    start: SEARCH_BUFFER_SIZE + 1,
                    end: SEARCH_BUFFER_SIZE + 3,
                },
            ]
        );
    }

    #[test]
    fn visitor_reports_unicode_byte_ranges() {
        let table =
            make_table(
                "visitor_unicode_ranges",
                "é😀é😀",
            );

        let searcher =
            Searcher::new("é😀");

        let mut matches = Vec::new();

        let count =
            searcher
                .visit_non_overlapping_matches(
                    &table,
                    |found| {
                        matches.push(found);
                        Ok(())
                    },
                )
                .unwrap();

        assert_eq!(count, 2);
        assert_eq!(
            matches,
            vec![
                SearchResult {
                    start: 0,
                    end: 6,
                },
                SearchResult {
                    start: 6,
                    end: 12,
                },
            ]
        );
    }

    #[test]
    fn visitor_iterates_case_insensitive_matches() {
        let table =
            make_table(
                "visitor_case_insensitive",
                "Aa aA",
            );

        let searcher =
            Searcher::with_mode(
                "aa",
                SearchMode::CaseInsensitive,
            )
            .unwrap();

        let mut matches = Vec::new();

        let count =
            searcher
                .visit_non_overlapping_matches(
                    &table,
                    |found| {
                        matches.push(found);
                        Ok(())
                    },
                )
                .unwrap();

        assert_eq!(count, 2);
        assert_eq!(
            matches,
            vec![
                SearchResult {
                    start: 0,
                    end: 2,
                },
                SearchResult {
                    start: 3,
                    end: 5,
                },
            ]
        );
    }

    #[test]
    fn replacement_visitor_skips_byte_identical_matches() {
        let table =
            make_table(
                "replacement_visitor_identical",
                "foo FOO Foo",
            );

        let searcher =
            Searcher::with_mode(
                "foo",
                SearchMode::CaseInsensitive,
            )
            .unwrap();

        let mut matches = Vec::new();

        let count =
            searcher
                .visit_non_overlapping_replacements(
                    &table,
                    "foo",
                    |found| {
                        matches.push(found);
                        Ok(())
                    },
                )
                .unwrap();

        assert_eq!(count, 2);
        assert_eq!(
            matches,
            vec![
                SearchResult {
                    start: 4,
                    end: 7,
                },
                SearchResult {
                    start: 8,
                    end: 11,
                },
            ]
        );
    }

    #[test]
    fn literal_replacement_visitor_short_circuits_identical_query() {
        let table =
            make_table(
                "replacement_visitor_literal_noop",
                "aaaa",
            );

        let searcher =
            Searcher::new("a");

        let mut visited = false;

        let count =
            searcher
                .visit_non_overlapping_replacements(
                    &table,
                    "a",
                    |_| {
                        visited = true;
                        Ok(())
                    },
                )
                .unwrap();

        assert_eq!(count, 0);
        assert!(!visited);
    }

    #[test]
    fn visitor_iterates_multiline_zero_width_regex_matches() {
        let table =
            make_table(
                "visitor_regex_zero_width",
                "a\nβ",
            );

        let searcher =
            Searcher::with_mode(
                r"^|$",
                SearchMode::Regex,
            )
            .unwrap();

        let mut matches = Vec::new();

        let count =
            searcher
                .visit_non_overlapping_matches(
                    &table,
                    |found| {
                        matches.push(found);
                        Ok(())
                    },
                )
                .unwrap();

        assert_eq!(count, 4);
        assert_eq!(
            matches,
            vec![
                SearchResult {
                    start: 0,
                    end: 0,
                },
                SearchResult {
                    start: 1,
                    end: 1,
                },
                SearchResult {
                    start: 2,
                    end: 2,
                },
                SearchResult {
                    start: 4,
                    end: 4,
                },
            ]
        );
    }

    #[test]
    fn visitor_does_not_call_callback_for_empty_query() {
        let table =
            make_table(
                "visitor_empty_query",
                "anything",
            );

        let searcher =
            Searcher::new("");

        let mut visited = false;

        let count =
            searcher
                .visit_non_overlapping_matches(
                    &table,
                    |_| {
                        visited = true;
                        Ok(())
                    },
                )
                .unwrap();

        assert_eq!(count, 0);
        assert!(!visited);
    }

    // ----------------------------------------------------------------------
    // Query API
    // ----------------------------------------------------------------------

    #[test]
    fn reports_query_length_in_bytes() {
        let searcher =
            Searcher::new("café");

        assert_eq!(
            searcher.len(),
            5
        );
    }

    #[test]
    fn reports_empty_query() {
        let searcher =
            Searcher::new("");

        assert!(
            searcher.is_empty()
        );

        assert_eq!(
            searcher.len(),
            0
        );
    }

    #[test]
    fn reports_non_empty_query() {
        let searcher =
            Searcher::new("hello");

        assert!(
            !searcher.is_empty()
        );

        assert_eq!(
            searcher.len(),
            5
        );
    }

    // ----------------------------------------------------------------------
    // Failure table
    // ----------------------------------------------------------------------

    #[test]
    fn failure_table_for_simple_query() {
        assert_eq!(
            build_failure_table(
                b"abcde"
            ),
            vec![
                0,
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn failure_table_for_repeated_query() {
        assert_eq!(
            build_failure_table(
                b"aaaa"
            ),
            vec![
                0,
                1,
                2,
                3,
            ]
        );
    }

    #[test]
    fn failure_table_for_mixed_query() {
        assert_eq!(
            build_failure_table(
                b"ababaca"
            ),
            vec![
                0,
                0,
                1,
                2,
                3,
                0,
                1,
            ]
        );
    }

    #[test]
    fn failure_table_for_empty_query() {
        assert!(
            build_failure_table(
                b""
            )
            .is_empty()
        );
    }
}
