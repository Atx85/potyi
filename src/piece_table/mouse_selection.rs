// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::PieceTable;
use std::{io, ops::Range};

#[derive(Clone, Copy, Default)]
pub(crate) enum SelectionUnit {
    #[default]
    Character,
    Word,
    Line,
}

impl SelectionUnit {
    pub(crate) fn for_clicks(clicks: u8) -> Self {
        match clicks {
            0 | 1 => Self::Character,
            2 => Self::Word,
            _ => Self::Line,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct MouseSelection {
    unit: SelectionUnit,
    origin: Range<usize>,
}

impl MouseSelection {
    pub(crate) fn begin(
        table: &mut PieceTable,
        position: usize,
        clicks: u8,
        anchor: Option<usize>,
    ) -> io::Result<Self> {
        let unit = SelectionUnit::for_clicks(clicks);
        let origin = if let Some(anchor) = anchor {
            anchor..anchor
        } else {
            table.mouse_range(position, unit)?
        };
        Ok(Self { unit, origin })
    }

    /// Returns (active end, anchor), preserving whole words/lines when reversing a drag.
    pub(crate) fn endpoints(
        &self,
        table: &mut PieceTable,
        position: usize,
    ) -> io::Result<(usize, usize)> {
        let range = table.mouse_range(position, self.unit)?;
        Ok(if range.start < self.origin.start {
            (range.start, self.origin.end)
        } else {
            (range.end.max(self.origin.end), self.origin.start)
        })
    }
}

impl PieceTable {
    fn mouse_range(&mut self, position: usize, unit: SelectionUnit) -> io::Result<Range<usize>> {
        let mut position = position.min(self.len());
        self.ensure_boundary(position)?;
        match unit {
            SelectionUnit::Character => Ok(position..position),
            SelectionUnit::Line => {
                let (line, _) = self.line_column_at(position)?;
                let start = self.line_start(line)?;
                let end = self.next_line_start_from(start)?.unwrap_or(self.len());
                Ok(start..end)
            }
            SelectionUnit::Word => {
                if position == self.len() && position > 0 {
                    let previous = self.previous_char_boundary(position)?;
                    if self.byte_at(previous)? == Some(b'\n') {
                        return Ok(position..position);
                    }
                    position = previous;
                }
                if position == self.len() || self.byte_at(position)? == Some(b'\n') {
                    return Ok(position..position);
                }
                let class = self.word_class_at(position)?;
                let mut start = position;
                let mut end = self.next_char_boundary(position)?;
                while start > 0 {
                    let previous = self.previous_char_boundary(start)?;
                    if self.byte_at(previous)? == Some(b'\n')
                        || self.word_class_at(previous)? != class
                    {
                        break;
                    }
                    start = previous;
                }
                while end < self.len()
                    && self.byte_at(end)? != Some(b'\n')
                    && self.word_class_at(end)? == class
                {
                    end = self.next_char_boundary(end)?;
                }
                Ok(start..end)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_selection_keeps_unicode_words_and_line_endings_intact() {
        let mut table = PieceTable::empty().unwrap();
        table.insert(0, "hello café_東京!\nnext line\n").unwrap();
        let word = MouseSelection::begin(&mut table, 9, 2, None).unwrap();
        assert_eq!(word.endpoints(&mut table, 9).unwrap(), (18, 6));
        assert_eq!(word.endpoints(&mut table, 2).unwrap(), (0, 18));
        let line = MouseSelection::begin(&mut table, 7, 3, None).unwrap();
        assert_eq!(line.endpoints(&mut table, 7).unwrap(), (20, 0));
        assert_eq!(line.endpoints(&mut table, 22).unwrap(), (30, 0));
        let eof = table.len();
        let empty = MouseSelection::begin(&mut table, eof, 2, None).unwrap();
        assert_eq!(empty.endpoints(&mut table, eof).unwrap(), (eof, eof));
    }

    #[test]
    fn mouse_selection_extends_from_original_anchor_and_reverses() {
        let mut table = PieceTable::empty().unwrap();
        table.insert(0, "one two three").unwrap();
        let selection = MouseSelection::begin(&mut table, 8, 1, Some(4)).unwrap();
        assert_eq!(selection.endpoints(&mut table, 8).unwrap(), (8, 4));
        assert_eq!(selection.endpoints(&mut table, 0).unwrap(), (0, 4));
        let word = MouseSelection::begin(&mut table, 5, 2, None).unwrap();
        assert_eq!(word.endpoints(&mut table, 10).unwrap(), (13, 4));
        assert_eq!(word.endpoints(&mut table, 1).unwrap(), (0, 7));
    }
}
