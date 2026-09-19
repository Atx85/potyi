// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Pane-local cursors over shared, file-backed text storage. The UI serializes
//! edits and reconciles the piece layouts before another view can edit.
use super::*;

impl PieceTable {
    pub(crate) fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.add, &other.add)
    }

    pub(crate) fn duplicate_view(&self) -> Self {
        Self {
            revision: self.revision,
            line_views: Default::default(),
            original: self.original.clone(),
            original_length: self.original_length,
            add: self.add.clone(),
            pieces: self.pieces.clone(),
            length: self.length,
            line_cache: self.line_cache.clone(),
            all_lines_cached: self.all_lines_cached,
            cursor: self.cursor.clone(),
            secondary_cursors: self.secondary_cursors.clone(),
            recovery: self.recovery.clone(),
        }
    }

    pub(crate) fn refresh_view_from(&mut self, source: &Self) -> io::Result<()> {
        if self.revision == source.revision {
            return Ok(());
        }
        let prefix = common_edge(&self.pieces, &source.pieces, false);
        let suffix = common_edge(&self.pieces, &source.pieces, true)
            .min(self.len().min(source.len()) - prefix);
        let old_end = self.len() - suffix;
        let new_end = source.len() - suffix;
        let map = |position: usize| {
            if position <= prefix {
                position
            } else if position >= old_end {
                new_end + position - old_end
            } else {
                new_end
            }
        };
        let mut cursors = Vec::with_capacity(1 + self.secondary_cursors.len());
        cursors.push(self.cursor.clone());
        cursors.extend(self.secondary_cursors.iter().cloned());
        let mut updated = source.duplicate_view();
        for cursor in &mut cursors {
            cursor.position = map(cursor.position).min(updated.len());
            cursor.anchor = map(cursor.anchor).min(updated.len());
            (cursor.line, cursor.column) = updated.line_column_at(cursor.position)?;
            (cursor.anchor_line, cursor.anchor_column) = updated.line_column_at(cursor.anchor)?;
            cursor.desired_column = None;
        }
        updated.cursor = cursors.remove(0);
        updated.secondary_cursors = cursors;
        *self = updated;
        Ok(())
    }
}

// Compare piece references, without scanning or copying the file's contents.
fn common_edge(left: &[Piece], right: &[Piece], reverse: bool) -> usize {
    let mut a: Box<dyn Iterator<Item = &Piece>> = if reverse {
        Box::new(left.iter().rev())
    } else {
        Box::new(left.iter())
    };
    let mut b: Box<dyn Iterator<Item = &Piece>> = if reverse {
        Box::new(right.iter().rev())
    } else {
        Box::new(right.iter())
    };
    let (mut x, mut y) = (a.next(), b.next());
    let (mut used_x, mut used_y, mut total) = (0, 0, 0);
    while let (Some(px), Some(py)) = (x, y) {
        let offset_x = if reverse {
            px.start + px.length - used_x
        } else {
            px.start + used_x
        };
        let offset_y = if reverse {
            py.start + py.length - used_y
        } else {
            py.start + used_y
        };
        if px.original != py.original || offset_x != offset_y {
            break;
        }
        let count = (px.length - used_x).min(py.length - used_y);
        total += count;
        used_x += count;
        used_y += count;
        if used_x == px.length {
            x = a.next();
            used_x = 0;
        }
        if used_y == py.length {
            y = b.next();
            used_y = 0;
        }
    }
    total
}
