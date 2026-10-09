// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! One file state and history owner for every live view of a document.
use super::HistoryEntry;
use std::{
    cell::{Ref, RefCell, RefMut},
    ops::RangeBounds,
    path::PathBuf,
    rc::Rc,
};

#[derive(Default)]
pub(super) struct DocumentState {
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub read_only: bool,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
}

#[derive(Clone)]
pub(crate) struct HistoryStack {
    owner: Rc<RefCell<DocumentState>>,
    redo: bool,
}

impl HistoryStack {
    pub(super) fn new(owner: Rc<RefCell<DocumentState>>, redo: bool) -> Self {
        Self { owner, redo }
    }
    pub(crate) fn entries(&self) -> Ref<'_, Vec<HistoryEntry>> {
        Ref::map(self.owner.borrow(), |s| {
            if self.redo { &s.redo } else { &s.undo }
        })
    }
    fn entries_mut(&self) -> RefMut<'_, Vec<HistoryEntry>> {
        RefMut::map(self.owner.borrow_mut(), |s| {
            if self.redo { &mut s.redo } else { &mut s.undo }
        })
    }
    pub(crate) fn len(&self) -> usize {
        self.entries().len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.entries().is_empty()
    }
    pub(crate) fn push(&mut self, entry: HistoryEntry) {
        self.entries_mut().push(entry);
    }
    pub(crate) fn pop(&mut self) -> Option<HistoryEntry> {
        self.entries_mut().pop()
    }
    pub(crate) fn clear(&mut self) {
        self.entries_mut().clear();
    }
    pub(crate) fn last(&self) -> Option<Ref<'_, HistoryEntry>> {
        Ref::filter_map(self.entries(), |entries| entries.last()).ok()
    }
    pub(crate) fn last_mut(&self) -> Option<RefMut<'_, HistoryEntry>> {
        RefMut::filter_map(self.entries_mut(), |entries| entries.last_mut()).ok()
    }
    pub(crate) fn update<T>(&self, index: usize, action: impl FnOnce(&mut HistoryEntry) -> T) -> T {
        action(&mut self.entries_mut()[index])
    }
    pub(crate) fn drain(
        &mut self,
        range: impl RangeBounds<usize>,
    ) -> std::vec::IntoIter<HistoryEntry> {
        self.entries_mut()
            .drain(range)
            .collect::<Vec<_>>()
            .into_iter()
    }
}
