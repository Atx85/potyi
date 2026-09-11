// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko

use std::collections::HashMap;
use std::io;
use std::ops::Range;

use crate::piece_table::PieceTable;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WrapMetrics {
    pub width: i32,
    pub cell_width: i32,
    pub tab_width: usize,
    pub font_size: u32,
}

/// Only byte offsets are retained. Appends reflow the final visual row;
/// resizing, font changes, clearing, and cuts inside a line rebuild the layout.
#[derive(Default)]
pub(crate) struct TerminalLayout {
    starts: Vec<usize>,
    length: usize,
    generation: u64,
    metrics: Option<WrapMetrics>,
    character_widths: HashMap<char, i32>,
}

impl TerminalLayout {
    /// Whole-line history trimming preserves the wrapping of retained rows.
    /// Return false if the cutoff lies outside the layout we have measured.
    pub fn discard_prefix(&mut self, bytes: usize, generation: u64) -> bool {
        let Ok(first) = self.starts.binary_search(&bytes) else {
            return false;
        };
        self.starts.drain(..first);
        for start in &mut self.starts {
            *start -= bytes;
        }
        self.length -= bytes;
        self.generation = generation;
        true
    }

    pub fn update(
        &mut self,
        output: &PieceTable,
        generation: u64,
        metrics: WrapMetrics,
        mut measure_character: impl FnMut(char) -> i32,
    ) -> io::Result<usize> {
        let same_layout = self.metrics == Some(metrics) && self.generation == generation;
        if same_layout && self.length == output.len() {
            return Ok(0);
        }
        let append = same_layout && output.len() >= self.length;
        let old_rows = self.starts.len();
        // Completed visual rows cannot change when text is appended. Only the
        // final visual row can gain text or move its last word to a new row.
        let start = if append {
            self.starts.last().copied().unwrap_or(0)
        } else {
            0
        };
        let bytes = output.read_range(start, output.len() - start)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if self.metrics != Some(metrics) {
            self.character_widths.clear();
        }
        if append {
            self.starts
                .truncate(self.starts.partition_point(|offset| *offset < start));
        } else {
            self.starts.clear();
        }
        let mut offset = start;
        let mut starts = Vec::new();
        for line in text.split('\n') {
            wrap_line(line, metrics, &mut starts, |character| {
                *self
                    .character_widths
                    .entry(character)
                    .or_insert_with(|| measure_character(character).max(0))
            });
            self.starts
                .extend(starts.iter().map(|position| offset + position));
            offset += line.len() + 1;
        }
        self.length = output.len();
        self.generation = generation;
        self.metrics = Some(metrics);
        Ok(if append {
            self.starts.len().saturating_sub(old_rows)
        } else {
            0
        })
    }

    pub fn len(&self) -> usize {
        self.starts.len()
    }

    pub fn row_at(&self, offset: usize) -> usize {
        self.starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1)
    }

    pub fn visible_rows(&self, visible: usize, scroll_back: usize) -> Range<usize> {
        let scroll_back = scroll_back.min(self.len().saturating_sub(visible));
        let start = self
            .len()
            .saturating_sub(visible.saturating_add(scroll_back));
        start..start.saturating_add(visible).min(self.len())
    }

    pub fn row_range(&self, row: usize, output: &PieceTable) -> io::Result<Option<Range<usize>>> {
        let Some(&start) = self.starts.get(row) else {
            return Ok(None);
        };
        let mut end = self.starts.get(row + 1).copied().unwrap_or(self.length);
        if end > start && output.byte_at(end - 1)? == Some(b'\n') {
            end -= 1;
        }
        Ok(Some(start..end))
    }
}

fn wrap_line(
    text: &str,
    metrics: WrapMetrics,
    starts: &mut Vec<usize>,
    mut character_width: impl FnMut(char) -> i32,
) {
    starts.clear();
    starts.push(0);
    let mut start = 0;
    while start < text.len() {
        let mut width = 0i32;
        let mut column = 0usize;
        let mut last_break = None;
        let mut next_start = text.len();
        for (relative, character) in text[start..].char_indices() {
            let position = start + relative;
            let advance = if character == '\t' {
                let tab = metrics.tab_width.max(1);
                tab - column % tab
            } else {
                1
            };
            let character_pixels = if character == '\t' {
                advance as i32 * metrics.cell_width.max(1)
            } else {
                character_width(character)
            };
            if position > start && width.saturating_add(character_pixels) > metrics.width.max(1) {
                // Keep separators on the preceding row. Wrapping before a
                // space could otherwise create a row containing only spaces.
                next_start = if character.is_whitespace() {
                    text[position..]
                        .char_indices()
                        .find(|(_, character)| !character.is_whitespace())
                        .map_or(text.len(), |(index, _)| position + index)
                } else {
                    last_break.unwrap_or(position)
                };
                break;
            }
            width = width.saturating_add(character_pixels);
            column += advance;
            if character.is_whitespace() {
                last_break = Some(position + character.len_utf8());
            }
        }
        if next_start >= text.len() {
            break;
        }
        starts.push(next_start);
        start = next_start;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(width: i32) -> WrapMetrics {
        WrapMetrics {
            width,
            cell_width: 1,
            tab_width: 4,
            font_size: 18,
        }
    }

    fn table(text: &str) -> PieceTable {
        let mut table = PieceTable::empty().unwrap();
        table.insert(0, text).unwrap();
        table
    }

    fn rows(layout: &TerminalLayout, table: &PieceTable) -> Vec<String> {
        (0..layout.len())
            .map(|row| {
                let range = layout.row_range(row, table).unwrap().unwrap();
                String::from_utf8(table.read_range(range.start, range.len()).unwrap()).unwrap()
            })
            .collect()
    }

    #[test]
    fn wraps_at_word_boundaries_and_retains_original_bytes() {
        let table = table("../    Cargo.toml    Readme.md");
        let mut layout = TerminalLayout::default();
        layout.update(&table, 0, metrics(16), |_| 1).unwrap();
        let rows = rows(&layout, &table);
        assert_eq!(rows.concat(), "../    Cargo.toml    Readme.md");
        assert!(rows.iter().any(|row| row.contains("Cargo.toml")));
        assert!(rows.iter().any(|row| row.contains("Readme.md")));
        assert!(rows.iter().all(|row| row.trim_end().len() <= 16));
    }

    #[test]
    fn separators_at_the_right_edge_do_not_create_blank_rows() {
        for text in ["one two", "one    two", "one \t two"] {
            let table = table(text);
            let mut layout = TerminalLayout::default();
            layout.update(&table, 0, metrics(3), |_| 1).unwrap();
            let rows = rows(&layout, &table);
            assert_eq!(rows.concat(), text);
            assert_eq!(
                rows.iter().map(|row| row.trim()).collect::<Vec<_>>(),
                ["one", "two"]
            );
        }
    }

    #[test]
    fn wraps_unicode_tabs_and_long_words_without_truncation() {
        let text = format!("東京🙂\tárvíz\n{}", "é".repeat(70_000));
        let table = table(&text);
        let mut layout = TerminalLayout::default();
        layout
            .update(&table, 0, metrics(8), |character| {
                if character == '🙂' { 2 } else { 1 }
            })
            .unwrap();
        let rows = rows(&layout, &table);
        assert_eq!(rows.concat(), text.replace('\n', ""));
        assert!(rows.iter().all(|row| row.chars().count() <= 8));
        assert!(layout.len() > 8_000);
    }

    #[test]
    fn empty_lines_and_trailing_newline_have_visible_rows() {
        for (text, expected) in [("", vec![""]), ("a\n\nb\n", vec!["a", "", "b", ""])] {
            let table = table(text);
            let mut layout = TerminalLayout::default();
            layout.update(&table, 0, metrics(20), |_| 1).unwrap();
            assert_eq!(rows(&layout, &table), expected);
        }
    }

    #[test]
    fn appended_partial_lines_reflow_like_a_fresh_layout() {
        let mut table = table("first\nhello ");
        let mut layout = TerminalLayout::default();
        layout.update(&table, 0, metrics(8), |_| 1).unwrap();
        let old_rows = layout.len();
        table.insert(table.len(), "world longer\nnext").unwrap();
        let added = layout.update(&table, 0, metrics(8), |_| 1).unwrap();
        assert_eq!(added, layout.len() - old_rows);
        let mut fresh = TerminalLayout::default();
        fresh.update(&table, 0, metrics(8), |_| 1).unwrap();
        assert_eq!(rows(&layout, &table), rows(&fresh, &table));
        layout
            .update(&table, 0, metrics(8), |_| {
                panic!("unchanged output must use cached rows")
            })
            .unwrap();
    }

    #[test]
    fn every_small_append_matches_full_reflow() {
        for width in [1, 3, 8, 20] {
            let mut output = table("");
            let mut incremental = TerminalLayout::default();
            let text = "one two 東京🙂\tlongwordwithoutspaces    end\n\nnext".repeat(5);
            for ch in text.chars() {
                output.insert(output.len(), &ch.to_string()).unwrap();
                incremental
                    .update(&output, 0, metrics(width), |_| 1)
                    .unwrap();
                let mut fresh = TerminalLayout::default();
                fresh.update(&output, 0, metrics(width), |_| 1).unwrap();
                assert_eq!(
                    incremental.starts, fresh.starts,
                    "width {width}, ending {ch:?}"
                );
            }
        }
    }

    #[test]
    fn trimming_completed_lines_reuses_retained_rows() {
        let mut output = table("discarded line\nretained words and more\nlast part");
        let mut incremental = TerminalLayout::default();
        incremental.update(&output, 0, metrics(8), |_| 1).unwrap();
        let removed = "discarded line\n".len();
        output.delete(0, removed).unwrap();
        assert!(incremental.discard_prefix(removed, 1));
        output.insert(output.len(), " extended\nnew line").unwrap();
        incremental.update(&output, 1, metrics(8), |_| 1).unwrap();
        let mut fresh = TerminalLayout::default();
        fresh.update(&output, 1, metrics(8), |_| 1).unwrap();
        assert_eq!(incremental.starts, fresh.starts);
        assert!(!incremental.discard_prefix(output.len() + 1, 2));
    }

    #[test]
    fn resize_font_change_and_clear_rebuild_rows() {
        let mut table = table("one two three four");
        let mut layout = TerminalLayout::default();
        layout.update(&table, 0, metrics(30), |_| 1).unwrap();
        assert_eq!(layout.len(), 1);
        layout.update(&table, 0, metrics(8), |_| 1).unwrap();
        assert!(layout.len() > 1);
        let mut larger_font = metrics(8);
        larger_font.font_size = 24;
        let previous_rows = layout.len();
        layout.update(&table, 0, larger_font, |_| 2).unwrap();
        assert!(layout.len() > previous_rows);
        let old_length = table.len();
        table.delete(0, old_length).unwrap();
        table.insert(0, &"x".repeat(old_length)).unwrap();
        layout.update(&table, 1, larger_font, |_| 2).unwrap();
        assert_eq!(rows(&layout, &table).concat(), "x".repeat(old_length));
    }

    #[test]
    fn scrolling_uses_wrapped_rows_and_stays_in_bounds() {
        let table = table("abcdefghijklmnopqrstuvwxyz");
        let mut layout = TerminalLayout::default();
        layout.update(&table, 0, metrics(4), |_| 1).unwrap();
        assert_eq!(layout.len(), 7);
        assert_eq!(layout.visible_rows(3, 0), 4..7);
        assert_eq!(layout.visible_rows(3, 2), 2..5);
        assert_eq!(layout.visible_rows(3, usize::MAX), 0..3);
        assert_eq!(layout.visible_rows(20, 0), 0..7);
    }

    #[test]
    fn tiny_window_and_oversized_glyph_always_make_progress() {
        let table = table("🙂é\t界");
        let mut layout = TerminalLayout::default();
        layout.update(&table, 0, metrics(0), |_| 10).unwrap();
        assert_eq!(rows(&layout, &table), ["🙂", "é\t", "界"]);
    }
}
