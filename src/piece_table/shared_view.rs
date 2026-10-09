// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Revision maps reconcile pane-local cursors over one document on the UI thread.
use super::*;
use std::{
    cell::Cell,
    collections::VecDeque,
    rc::{Rc, Weak},
};

/// A replacement in pre-edit coordinates. Zero-length removals are insertions.
#[derive(Clone, Copy, Debug)]
pub(super) struct PositionChange {
    pub start: usize,
    pub removed: usize,
    pub inserted: usize,
}

impl PositionChange {
    pub(super) fn map(self, position: usize) -> usize {
        if position <= self.start {
            position // Keep a cursor at an insertion boundary before the insertion.
        } else if position >= self.start + self.removed {
            position - self.removed + self.inserted
        } else {
            self.start + self.inserted
        }
    }
}

pub(super) fn inverse_changes(changes: &[PositionChange]) -> Arc<[PositionChange]> {
    let (mut old_end, mut new_end) = (0, 0);
    changes
        .iter()
        .map(|change| {
            let start = new_end + (change.start - old_end);
            old_end = change.start + change.removed;
            new_end = start + change.inserted;
            PositionChange {
                start,
                removed: change.inserted,
                inserted: change.removed,
            }
        })
        .collect()
}

#[derive(Clone)]
pub(super) enum PositionChanges {
    Single(PositionChange),
    Multiple(Arc<[PositionChange]>),
}

impl From<[PositionChange; 1]> for PositionChanges {
    fn from(changes: [PositionChange; 1]) -> Self {
        Self::Single(changes[0])
    }
}

impl PositionChanges {
    pub fn inverse(&self) -> Self {
        match self {
            Self::Single(change) => Self::Single(PositionChange {
                start: change.start,
                removed: change.inserted,
                inserted: change.removed,
            }),
            Self::Multiple(changes) => Self::Multiple(inverse_changes(changes)),
        }
    }

    pub(super) fn map(&self, mut position: usize) -> usize {
        match self {
            Self::Single(change) => change.map(position),
            Self::Multiple(changes) => {
                // Batch ranges use pre-edit coordinates. Apply right to left
                // so earlier ranges do not shift the remaining positions.
                for change in changes.iter().rev() {
                    position = change.map(position);
                }
                position
            }
        }
    }
}

struct RevisionChanges {
    before: u64,
    after: u64,
    changes: PositionChanges,
}

/// UI-thread edit maps; the document is borrowed, never locked or duplicated.
#[derive(Default)]
pub(super) struct ViewHistory {
    views: Vec<Weak<Cell<u64>>>,
    changes: VecDeque<RevisionChanges>,
    pub(super) active: Option<Rc<Cell<u64>>>,
}
impl ViewHistory {
    pub fn new(_revision: u64) -> Self {
        Self::default()
    }
    pub(super) fn register(&mut self, revision: u64) -> Rc<Cell<u64>> {
        let seen = Rc::new(Cell::new(revision));
        self.views.push(Rc::downgrade(&seen));
        seen
    }
    pub(super) fn unregister(&mut self, seen: &Rc<Cell<u64>>) {
        self.views
            .retain(|view| view.upgrade().is_some_and(|view| !Rc::ptr_eq(&view, seen)));
        self.prune();
    }
    pub(super) fn prune(&mut self) {
        let mut oldest = u64::MAX;
        self.views.retain(|view| {
            if let Some(view) = view.upgrade() {
                oldest = oldest.min(view.get());
                true
            } else {
                false
            }
        });
        while self
            .changes
            .front()
            .is_some_and(|change| change.after <= oldest)
        {
            self.changes.pop_front();
        }
    }
    pub fn record(&mut self, before: u64, after: u64, changes: PositionChanges) {
        if let Some(view) = &self.active {
            view.set(after);
        }
        if self.views.len() > 1 {
            self.changes.push_back(RevisionChanges {
                before,
                after,
                changes,
            });
        }
        self.prune();
    }
    pub(super) fn changes_since(
        &self,
        mut revision: u64,
        target: u64,
    ) -> io::Result<Vec<PositionChanges>> {
        let mut changes = Vec::new();
        for change in &self.changes {
            if change.before == revision {
                changes.push(change.changes.clone());
                revision = change.after;
                if revision == target {
                    return Ok(changes);
                }
            }
        }
        Err(io::Error::other("Shared view edit history is incomplete"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_history_is_released_after_sync_or_dropping_a_stale_view() {
        let mut a = PieceTable::empty().unwrap();
        a.insert(0, "text").unwrap();
        assert!(a.storage.borrow().view_history.changes.is_empty());
        let mut b = a.duplicate_view();
        a.insert(0, "first ").unwrap();
        assert_eq!(a.storage.borrow().view_history.changes.len(), 1);
        b.refresh_view_from(&a).unwrap();
        assert!(a.storage.borrow().view_history.changes.is_empty());
        a.insert(0, "second ").unwrap();
        drop(b);
        assert!(a.storage.borrow().view_history.changes.is_empty());
        for _ in 0..100 {
            a.insert(0, "x").unwrap();
        }
        assert!(a.storage.borrow().view_history.changes.is_empty());
        assert_eq!(a.storage.borrow().view_history.views.len(), 1);
    }

    #[test]
    fn several_views_keep_edits_until_the_oldest_view_catches_up() {
        let mut a = PieceTable::empty().unwrap();
        a.insert(0, "aaa bbb ccc").unwrap();
        let mut b = a.duplicate_view();
        let mut c = a.duplicate_view();
        b.move_cursor(5).unwrap();
        c.move_cursor(5).unwrap();
        a.insert(0, "X").unwrap();
        b.refresh_view_from(&a).unwrap();
        a.insert(a.len(), "Y").unwrap();
        c.refresh_view_from(&a).unwrap();
        b.refresh_view_from(&a).unwrap();
        assert_eq!(b.cursor.position, 6);
        assert_eq!(c.cursor.position, 6);
        assert!(a.storage.borrow().view_history.changes.is_empty());
    }
}
