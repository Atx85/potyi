// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded viewport reads and sparse position checkpoints; no retained line text.
use super::{LineInfo, PieceTable};
use std::{collections::VecDeque, io};

const BLOCK: usize = 4096;
const MAX_LINES: usize = 128;
const MAX_POINTS: usize = 32;

#[derive(Clone, Copy, Default)]
struct Point {
    byte: usize,
    column: usize,
    visual: usize,
}

struct Entry {
    start: usize,
    tab: usize,
    stride: usize,
    points: Vec<Point>,
    recent: Point,
    window_start: Point,
}

impl Entry {
    fn remember(&mut self, point: Point) {
        if point.byte >= self.points.last().unwrap().byte.saturating_add(self.stride) {
            if self.points.len() == MAX_POINTS {
                let mut index = 0;
                self.points.retain(|_| {
                    let keep = index % 2 == 0;
                    index += 1;
                    keep
                });
                self.stride = self.stride.saturating_mul(2);
            }
            self.points.push(point);
        }
        self.recent = point;
    }
}

#[derive(Default)]
pub(super) struct LineViewCache {
    revision: u64,
    entries: VecDeque<Entry>,
    #[cfg(test)]
    bytes_read: usize,
}

pub(crate) struct LineWindow {
    pub text: String,
    pub first_byte: usize,
    pub first_column: usize,
    pub first_visual: usize,
}

#[derive(Clone, Copy)]
enum Target {
    Column(usize),
    Visual(usize),
    Byte(usize),
}
impl Target {
    fn before(self, point: Point) -> bool {
        match self {
            Self::Column(n) => point.column <= n,
            Self::Visual(n) => point.visual <= n,
            Self::Byte(n) => point.byte <= n,
        }
    }
}

impl PieceTable {
    fn scan_line_view(
        &self,
        info: LineInfo,
        tab: usize,
        target: Target,
        right: Option<usize>,
    ) -> io::Result<(usize, usize, LineWindow)> {
        let tab = tab.max(1);
        let mut cache = self.line_views.borrow_mut();
        if cache.revision != self.revision() {
            cache.entries.clear();
            cache.revision = self.revision();
        }
        let mut entry = cache
            .entries
            .iter()
            .position(|e| e.start == info.start && e.tab == tab)
            .and_then(|i| cache.entries.remove(i))
            .unwrap_or_else(|| {
                let origin = Point {
                    byte: info.start,
                    ..Point::default()
                };
                Entry {
                    start: info.start,
                    tab,
                    stride: BLOCK,
                    points: vec![origin],
                    recent: origin,
                    window_start: origin,
                }
            });
        let mut point = entry
            .points
            .iter()
            .copied()
            .chain([entry.recent, entry.window_start])
            .filter(|p| target.before(*p) && p.column <= info.length && p.byte <= info.end)
            .max_by_key(|p| p.byte)
            .unwrap();
        let mut first = None;
        let mut text = String::new();
        let result = (|| -> io::Result<()> {
            'read: while point.column < info.length {
                if right.is_none()
                    && (matches!(target, Target::Column(n) if point.column >= n)
                        || matches!(target, Target::Byte(n) if point.byte >= n))
                {
                    break;
                }
                if first.is_some() && right.is_some_and(|r| point.visual >= r) {
                    break;
                }
                let bytes = self.read_range(point.byte, (info.end - point.byte).min(BLOCK))?;
                #[cfg(test)]
                {
                    cache.bytes_read += bytes.len();
                }
                let chunk = match std::str::from_utf8(&bytes) {
                    Ok(s) => s,
                    Err(e) if e.error_len().is_none() && e.valid_up_to() > 0 => {
                        std::str::from_utf8(&bytes[..e.valid_up_to()]).unwrap()
                    }
                    Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                };
                if chunk.is_empty() {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                }
                for ch in chunk.chars() {
                    if point.column == info.length {
                        break 'read;
                    }
                    let advance = if ch == '\t' {
                        tab - point.visual % tab
                    } else {
                        1
                    };
                    let reached = match target {
                        Target::Column(n) => point.column >= n,
                        Target::Byte(n) => point.byte >= n,
                        Target::Visual(n) => point.visual.saturating_add(advance) > n,
                    };
                    if first.is_none() && reached {
                        first = Some(point);
                        if right.is_none() {
                            break 'read;
                        }
                    }
                    if first.is_some() {
                        if right.is_some_and(|r| point.visual >= r) {
                            break 'read;
                        }
                        text.push(ch);
                    }
                    point.byte += ch.len_utf8();
                    point.column += 1;
                    point.visual = point.visual.saturating_add(advance);
                    entry.remember(point);
                }
            }
            Ok(())
        })();
        let first = first.unwrap_or(point);
        if right.is_some() {
            entry.window_start = first;
        }
        entry.remember(point);
        if cache.entries.len() == MAX_LINES {
            cache.entries.pop_front();
        }
        cache.entries.push_back(entry);
        result?;
        Ok((
            point.byte,
            point.visual,
            LineWindow {
                text,
                first_byte: first.byte - info.start,
                first_column: first.column,
                first_visual: first.visual,
            },
        ))
    }

    fn display_line_info(&self, mut info: LineInfo) -> io::Result<LineInfo> {
        // line_text also omits a final CR in a document without a terminating LF.
        if info.end == self.len()
            && info.end > info.start
            && self.byte_at(info.end - 1)? == Some(b'\r')
        {
            info.end -= 1;
            info.length = info.length.saturating_sub(1);
        }
        Ok(info)
    }

    pub(crate) fn line_window(
        &mut self,
        line: usize,
        left: usize,
        right: usize,
        tab: usize,
    ) -> io::Result<LineWindow> {
        self.ensure_line_cached(line)?;
        let info = self.line_cache.get(line).copied().unwrap_or(LineInfo {
            start: self.len(),
            end: self.len(),
            length: 0,
        });
        let info = self.display_line_info(info)?;
        Ok(self
            .scan_line_view(info, tab, Target::Visual(left), Some(right.max(left)))?
            .2)
    }

    pub(crate) fn line_visual_column(
        &mut self,
        line: usize,
        column: usize,
        tab: usize,
    ) -> io::Result<usize> {
        self.ensure_line_cached(line)?;
        let Some(info) = self.line_cache.get(line).copied() else {
            return Ok(0);
        };
        let info = self.display_line_info(info)?;
        Ok(self
            .scan_line_view(info, tab, Target::Column(column.min(info.length)), None)?
            .1)
    }

    pub(crate) fn line_prefix(&mut self, line: usize, max_bytes: usize) -> io::Result<String> {
        self.ensure_line_cached(line)?;
        let Some(info) = self.line_cache.get(line).copied() else {
            return Ok(String::new());
        };
        let bytes = self.read_range(info.start, (info.end - info.start).min(max_bytes))?;
        let text = match std::str::from_utf8(&bytes) {
            Ok(s) => s,
            Err(e) if e.error_len().is_none() => {
                std::str::from_utf8(&bytes[..e.valid_up_to()]).unwrap()
            }
            Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e)),
        };
        Ok(text.strip_suffix('\r').unwrap_or(text).to_owned())
    }

    pub(super) fn cached_column_at_position(
        &self,
        info: LineInfo,
        position: usize,
    ) -> io::Result<usize> {
        let column = self
            .scan_line_view(info, 1, Target::Byte(position), None)?
            .2
            .first_column;
        // Preserve byte-position reporting at the LF of CRLF without caching CR as display text.
        let cr = position == info.end
            && info.end < self.len()
            && position > info.start
            && self.byte_at(position - 1)? == Some(b'\r');
        Ok(column + usize::from(cr))
    }

    pub(super) fn cached_position_at_column(
        &self,
        start: usize,
        column: usize,
    ) -> io::Result<Option<usize>> {
        let Ok(index) = self
            .line_cache
            .binary_search_by_key(&start, |info| info.start)
        else {
            return Ok(None);
        };
        let info = self.line_cache[index];
        Ok(Some(
            self.scan_line_view(info, 1, Target::Column(column.min(info.length)), None)?
                .0,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn table(text: &str) -> PieceTable {
        let mut table = PieceTable::empty().unwrap();
        table.insert(0, text).unwrap();
        table
    }

    #[test]
    fn empty_lines_crlf_and_unterminated_cr_keep_display_and_byte_columns() {
        for content in ["", "\r", "a\r", "a\r\n", "\r\n\n"] {
            let mut table = table(content);
            let count = table.line_count().unwrap();
            for byte in 0..=content.len() {
                table.line_column_at(byte).unwrap();
            }
            for line in 0..count {
                let expected = table.line_text(line).unwrap();
                assert_eq!(table.line_window(line, 0, 80, 1).unwrap().text, expected);
                assert_eq!(
                    table.line_visual_column(line, usize::MAX, 1).unwrap(),
                    expected.chars().count()
                );
            }
        }
        let mut table = table("a\r");
        assert_eq!(table.line_column_at(2).unwrap(), (0, 2));
        let mut crlf = super::tests::table("a\r\n");
        assert_eq!(crlf.line_column_at(2).unwrap(), (0, 2));
    }

    #[test]
    fn viewport_matches_unicode_and_tab_reference_at_every_offset() {
        let text = "aé\t中🙂bc\tZ";
        let mut table = table(&format!("{text}\r\nnext"));
        for tab in [1, 4, 7] {
            let mut visual = 0;
            let reference: Vec<_> = text
                .char_indices()
                .enumerate()
                .map(|(col, (byte, c))| {
                    let start = visual;
                    visual += if c == '\t' { tab - visual % tab } else { 1 };
                    (col, byte, start, visual, c)
                })
                .collect();
            for left in (0..visual + 3).rev() {
                let right = left + 3;
                let view = table.line_window(0, left, right, tab).unwrap();
                let expected: Vec<_> = reference
                    .iter()
                    .filter(|r| r.3 > left && r.2 < right)
                    .collect();
                assert_eq!(view.text, expected.iter().map(|r| r.4).collect::<String>());
                if let Some(first) = expected.first() {
                    assert_eq!(
                        (view.first_column, view.first_byte, view.first_visual),
                        (first.0, first.1, first.2)
                    );
                } else {
                    assert_eq!(view.first_column, text.chars().count());
                }
            }
            for r in &reference {
                assert_eq!(table.line_visual_column(0, r.0, tab).unwrap(), r.2);
                table.move_cursor_to_line_column(0, r.0).unwrap();
                assert_eq!(table.cursor.position, r.1);
                assert_eq!(table.line_column_at(r.1).unwrap(), (0, r.0));
            }
        }
    }

    #[test]
    fn utf8_crossing_scan_blocks_preserves_lines_and_positions() {
        let text = format!("{}é🙂中\r\n{}éz", "a".repeat(65535), "b".repeat(4095));
        let mut table = table(&text);
        assert_eq!(table.line_count().unwrap(), 2);
        assert_eq!(table.line_length(0).unwrap(), 65538);
        for line in 0..2 {
            let expected = text.split("\r\n").nth(line).unwrap();
            for (col, (byte, _)) in expected
                .char_indices()
                .enumerate()
                .skip(expected.chars().count() - 5)
            {
                table.move_cursor_to_line_column(line, col).unwrap();
                let start = table.line_start(line).unwrap();
                assert_eq!(table.cursor.position, start + byte);
                assert_eq!(
                    table.line_column_at(table.cursor.position).unwrap(),
                    (line, col)
                );
            }
            assert_eq!(table.line_text(line).unwrap(), expected);
        }
    }

    #[test]
    fn repeated_far_right_viewport_reads_stay_bounded_and_edits_invalidate_points() {
        let mut table = table(&"ab\té".repeat(250_000));
        let expected = table.line_window(0, 999_900, 999_980, 4).unwrap().text;
        table.line_views.borrow_mut().bytes_read = 0;
        for _ in 0..10 {
            assert_eq!(
                table.line_window(0, 999_900, 999_980, 4).unwrap().text,
                expected
            );
        }
        assert!(
            table.line_views.borrow().bytes_read <= 10 * BLOCK,
            "repeated views should not rescan the million-character prefix"
        );
        table.insert(0, "XYZ").unwrap();
        assert!(
            table
                .line_window(0, 0, 8, 4)
                .unwrap()
                .text
                .starts_with("XYZab")
        );
        table.delete(0, 3).unwrap();
        assert_eq!(
            table.line_window(0, 999_900, 999_980, 4).unwrap().text,
            expected
        );
    }

    #[test]
    fn checkpoint_storage_is_bounded_across_lines_and_tab_settings() {
        let mut table = table(&"x\t🙂\n".repeat(80));
        for tab in 1..8 {
            for line in 0..80 {
                table.line_window(line, 1, 10, tab).unwrap();
            }
        }
        let cache = table.line_views.borrow();
        assert_eq!(cache.entries.len(), MAX_LINES);
        assert!(cache.entries.iter().all(|e| e.points.len() <= MAX_POINTS));
    }

    #[test]
    fn up_down_and_selection_preserve_columns_on_long_lines() {
        let mut table = table(&format!(
            "{}\nshort\n{}",
            "é".repeat(70_000),
            "x".repeat(70_000)
        ));
        table.move_cursor_to_line_column(0, 60_000).unwrap();
        table.cursor_down().unwrap();
        assert_eq!(table.cursor.column, 5);
        table.cursor_down().unwrap();
        assert_eq!(table.cursor.column, 60_000);
        table.cursor_up().unwrap();
        table.cursor_up().unwrap();
        assert_eq!((table.cursor.line, table.cursor.column), (0, 60_000));
        table.select_down().unwrap();
        let anchor = table.cursor.anchor;
        table.select_up().unwrap();
        assert_eq!(table.cursor.anchor, anchor);
        assert_eq!(table.cursor.column, 5);
    }
}
