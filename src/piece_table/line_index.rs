// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! A bounded detailed window, with progressively sparser restart points.
use super::LineInfo;
use std::{collections::VecDeque, ops::Index};

pub(super) const DETAIL_LIMIT: usize = 4096;
pub(super) const CHECKPOINT_LIMIT: usize = 2048;

#[derive(Clone, Copy, Default)]
struct Point {
    line: usize,
    start: usize,
}

pub(super) struct LineIndex {
    detailed: VecDeque<LineInfo>,
    base: usize,
    points: Vec<Point>,
    stride: usize,
    frontier: Point,
    next: Point,
    local_done: bool,
    count: Option<usize>,
}
impl Default for LineIndex {
    fn default() -> Self {
        Self {
            detailed: VecDeque::new(),
            base: 0,
            points: Vec::new(),
            stride: 64,
            frontier: Point::default(),
            next: Point::default(),
            local_done: false,
            count: None,
        }
    }
}
impl LineIndex {
    pub(super) fn empty_document() -> Self {
        let mut index = Self::default();
        index.push(
            LineInfo {
                start: 0,
                end: 0,
                length: 0,
            },
            true,
        );
        index
    }
    // Extent of the indexed prefix, including evicted details. Renderer scroll
    // estimates must not shrink when the detailed window moves.
    pub(super) fn len(&self) -> usize {
        self.count.unwrap_or(self.frontier.line)
    }
    pub(super) fn is_empty(&self) -> bool {
        self.detailed.is_empty()
    }
    pub(super) fn iter(&self) -> std::collections::vec_deque::Iter<'_, LineInfo> {
        self.detailed.iter()
    }
    pub(super) fn get(&self, line: usize) -> Option<&LineInfo> {
        self.detailed.get(line.checked_sub(self.base)?)
    }
    pub(super) fn partition_point(&self, predicate: impl FnMut(&LineInfo) -> bool) -> usize {
        self.base + self.detailed.partition_point(predicate)
    }
    pub(super) fn binary_search_by_key<K: Ord>(
        &self,
        key: &K,
        f: impl FnMut(&LineInfo) -> K,
    ) -> Result<usize, usize> {
        self.detailed
            .binary_search_by_key(key, f)
            .map(|i| i + self.base)
            .map_err(|i| i + self.base)
    }
    pub(super) fn complete(&self) -> bool {
        self.count.is_some()
    }
    pub(super) fn local_done(&self) -> bool {
        self.local_done
    }
    pub(super) fn next_start(&self) -> usize {
        self.next.start
    }
    pub(super) fn contains_position(&self, position: usize) -> bool {
        self.detailed
            .front()
            .is_some_and(|first| first.start <= position)
            && self
                .detailed
                .back()
                .is_some_and(|last| position <= last.end)
    }
    fn restart(&mut self, point: Point) {
        self.detailed.clear();
        self.base = point.line;
        self.next = point;
        self.local_done = false;
    }
    fn point_before_line(&self, line: usize) -> Point {
        let i = self.points.partition_point(|p| p.line <= line);
        let point = i
            .checked_sub(1)
            .map_or(Point::default(), |i| self.points[i]);
        if self.count.is_none() && self.frontier.line <= line && self.frontier.line > point.line {
            self.frontier
        } else {
            point
        }
    }
    fn point_before_position(&self, position: usize) -> Point {
        let i = self.points.partition_point(|p| p.start <= position);
        let point = i
            .checked_sub(1)
            .map_or(Point::default(), |i| self.points[i]);
        if self.count.is_none()
            && self.frontier.start <= position
            && self.frontier.line > point.line
        {
            self.frontier
        } else {
            point
        }
    }
    pub(super) fn prepare_line(&mut self, line: usize) {
        if self.get(line).is_some() {
            return;
        }
        let target = self
            .count
            .map_or(line, |count| line.min(count.saturating_sub(1)));
        if self.get(target).is_some() {
            return;
        }
        let point = self.point_before_line(target);
        if !self.local_done && target >= self.next.line && point.line <= self.next.line {
            return;
        }
        self.restart(self.point_before_line(target));
    }
    pub(super) fn prepare_position(&mut self, position: usize) {
        if self.contains_position(position) {
            return;
        }
        let point = self.point_before_position(position);
        if !self.local_done && position >= self.next.start && point.line <= self.next.line {
            return;
        }
        self.restart(self.point_before_position(position));
    }
    pub(super) fn prepare_count(&mut self) {
        if self.count.is_none() && self.next.line != self.frontier.line {
            self.restart(self.frontier);
        }
    }
    fn checkpoint(&mut self, point: Point) {
        if point.line == 0 || point.line % self.stride != 0 {
            return;
        }
        let mut i = match self.points.binary_search_by_key(&point.line, |p| p.line) {
            Ok(_) => return,
            Err(i) => i,
        };
        if self.points.len() == CHECKPOINT_LIMIT {
            self.stride = self.stride.saturating_mul(2);
            self.points.retain(|p| p.line % self.stride == 0);
            if point.line % self.stride != 0 {
                return;
            }
            i = self.points.partition_point(|p| p.line < point.line);
        }
        self.points.insert(i, point);
    }
    pub(super) fn push(&mut self, info: LineInfo, terminal: bool) {
        debug_assert_eq!(info.start, self.next.start);
        self.checkpoint(self.next);
        if self.detailed.len() == DETAIL_LIMIT {
            self.detailed.pop_front();
            self.base += 1;
        }
        self.detailed.push_back(info);
        self.next = Point {
            line: self.next.line + 1,
            start: if terminal { info.end } else { info.end + 1 },
        };
        if self.next.line > self.frontier.line {
            self.frontier = self.next;
        }
        if terminal {
            self.count = Some(self.next.line);
            self.local_done = true;
        }
    }
    // Use exact detailed coordinates when present; otherwise discard from a
    // known restart point. No reads or background work occur during invalidation.
    pub(super) fn invalidate(&mut self, position: usize) -> usize {
        let line = self.partition_point(|info| info.end < position);
        let affected = self
            .get(line)
            .filter(|info| info.start <= position)
            .map_or_else(
                || self.point_before_position(position),
                |info| Point {
                    line,
                    start: info.start,
                },
            );
        if affected.line >= self.base && affected.line <= self.base + self.detailed.len() {
            self.detailed.truncate(affected.line - self.base);
            self.next = affected;
            self.local_done = false;
        } else {
            self.restart(affected);
        }
        self.points.retain(|p| p.line <= affected.line);
        self.frontier = affected;
        self.count = None;
        affected.start
    }
    #[cfg(test)]
    pub(super) fn metadata(&self) -> (usize, usize, usize) {
        (
            self.detailed.len(),
            self.points.len(),
            self.detailed.capacity() * std::mem::size_of::<LineInfo>()
                + self.points.capacity() * std::mem::size_of::<Point>(),
        )
    }
}
impl Index<usize> for LineIndex {
    type Output = LineInfo;
    fn index(&self, line: usize) -> &LineInfo {
        self.get(line)
            .expect("line details must be ensured before access")
    }
}
impl<'a> IntoIterator for &'a LineIndex {
    type Item = &'a LineInfo;
    type IntoIter = std::collections::vec_deque::Iter<'a, LineInfo>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
