// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Pane-local cursors borrowing one document on the UI thread.
use super::line_view::LineWindow;
use super::*;
use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub struct PieceTable {
    pub(super) storage: Rc<RefCell<Document>>,
    pub(super) seen: Rc<Cell<u64>>,
    pub(crate) cursor: Cursor,
    pub(crate) secondary_cursors: Vec<Cursor>,
}

struct ViewBorrow<'a> {
    storage: RefMut<'a, Document>,
    cursor: &'a mut Cursor,
    secondary: &'a mut Vec<Cursor>,
}
impl Deref for ViewBorrow<'_> {
    type Target = Document;
    fn deref(&self) -> &Document {
        &self.storage
    }
}
impl DerefMut for ViewBorrow<'_> {
    fn deref_mut(&mut self) -> &mut Document {
        &mut self.storage
    }
}
impl Drop for ViewBorrow<'_> {
    fn drop(&mut self) {
        std::mem::swap(&mut self.storage.cursor, self.cursor);
        std::mem::swap(&mut self.storage.secondary_cursors, self.secondary);
        self.storage.view_history.active = None;
    }
}

impl PieceTable {
    pub fn open(path: &str) -> io::Result<Self> {
        Document::open(path).map(Self::from_storage)
    }
    pub fn empty() -> io::Result<Self> {
        Document::empty().map(Self::from_storage)
    }
    pub(super) fn from_storage(mut storage: Document) -> Self {
        let seen = storage.view_history.register(storage.revision);
        let cursor = std::mem::take(&mut storage.cursor);
        let secondary_cursors = std::mem::take(&mut storage.secondary_cursors);
        Self {
            storage: Rc::new(RefCell::new(storage)),
            seen,
            cursor,
            secondary_cursors,
        }
    }
    pub(super) fn with_storage<T>(&mut self, action: impl FnOnce(&mut Document) -> T) -> T {
        let mut storage = self.storage.borrow_mut();
        std::mem::swap(&mut storage.cursor, &mut self.cursor);
        std::mem::swap(&mut storage.secondary_cursors, &mut self.secondary_cursors);
        storage.view_history.active = Some(self.seen.clone());
        let mut guard = ViewBorrow {
            storage,
            cursor: &mut self.cursor,
            secondary: &mut self.secondary_cursors,
        };
        action(&mut guard)
    }
    pub(crate) fn pieces(&self) -> Ref<'_, [Piece]> {
        Ref::map(self.storage.borrow(), |storage| &storage.pieces[..])
    }
    pub(crate) fn refresh(&mut self) -> io::Result<bool> {
        let mut storage = self.storage.borrow_mut();
        let revision = storage.revision;
        if self.seen.get() == revision {
            return Ok(false);
        }
        let changes = storage
            .view_history
            .changes_since(self.seen.get(), revision)?;
        for cursor in std::iter::once(&mut self.cursor).chain(&mut self.secondary_cursors) {
            for change in &changes {
                cursor.position = change.map(cursor.position);
                cursor.anchor = change.map(cursor.anchor);
            }
            cursor.position = cursor.position.min(storage.len());
            cursor.anchor = cursor.anchor.min(storage.len());
            (cursor.line, cursor.column) = storage.line_column_at(cursor.position)?;
            (cursor.anchor_line, cursor.anchor_column) = storage.line_column_at(cursor.anchor)?;
            cursor.desired_column = None;
        }
        self.seen.set(revision);
        storage.view_history.prune();
        Ok(true)
    }
    pub(crate) fn duplicate_view(&self) -> Self {
        let seen = self
            .storage
            .borrow_mut()
            .view_history
            .register(self.seen.get());
        Self {
            storage: self.storage.clone(),
            seen,
            cursor: self.cursor.clone(),
            secondary_cursors: self.secondary_cursors.clone(),
        }
    }
    pub(crate) fn shares_storage_with(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.storage, &other.storage)
    }
    pub(crate) fn refresh_view_from(&mut self, source: &Self) -> io::Result<()> {
        if !self.shares_storage_with(source) {
            return Err(io::Error::other("Views have different documents"));
        }
        self.refresh().map(|_| ())
    }
    pub fn cursor_line_column(&self) -> io::Result<(usize, usize)> {
        Ok((self.cursor.line, self.cursor.column))
    }
    pub fn selection_start(&self) -> usize {
        self.cursor.position.min(self.cursor.anchor)
    }
    pub fn selection_end(&self) -> usize {
        self.cursor.position.max(self.cursor.anchor)
    }
    pub fn has_selection(&self) -> bool {
        self.cursor.position != self.cursor.anchor
    }
    pub fn debug(&mut self) {
        self.with_storage(|storage| storage.debug());
    }
    pub(super) fn mouse_range(
        &mut self,
        position: usize,
        unit: mouse_selection::SelectionUnit,
    ) -> io::Result<std::ops::Range<usize>> {
        self.refresh()?;
        self.with_storage(|storage| storage.mouse_range(position, unit))
    }
    pub(crate) fn revision(&self) -> u64 {
        self.storage.borrow().revision()
    }

    pub fn len(&self) -> usize {
        self.storage.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.storage.borrow().is_empty()
    }

    #[cfg(test)]
    pub(crate) fn edit_store_len(&self) -> usize {
        self.storage.borrow().edit_store_len()
    }

    #[cfg(test)]
    pub(crate) fn line_index_metadata(&self) -> (usize, usize, usize) {
        self.storage.borrow().line_cache.metadata()
    }

    pub fn cached_line_count(&self) -> usize {
        self.storage.borrow().cached_line_count()
    }

    pub fn move_cursor_to_line_column(&mut self, line: usize, column: usize) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.move_cursor_to_line_column(line, column))
    }

    pub fn write_to(&self, path: &Path) -> io::Result<()> {
        self.storage.borrow().write_to(path)
    }

    pub fn write_to_new(&self, path: &Path) -> io::Result<()> {
        self.storage.borrow().write_to_new(path)
    }

    pub(crate) fn visit_chunks<F>(&self, visit: F) -> io::Result<()>
    where
        F: FnMut(&[u8]) -> io::Result<()>,
    {
        self.storage.borrow().visit_chunks(visit)
    }

    pub fn read_range_into(&self, position: usize, buffer: &mut [u8]) -> io::Result<usize> {
        self.storage.borrow().read_range_into(position, buffer)
    }

    pub fn read_range(&self, position: usize, length: usize) -> io::Result<Vec<u8>> {
        self.storage.borrow().read_range(position, length)
    }

    pub fn byte_at(&self, position: usize) -> io::Result<Option<u8>> {
        self.storage.borrow().byte_at(position)
    }

    pub fn ensure_line_cached(&mut self, line: usize) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.ensure_line_cached(line))
    }

    pub fn line_count(&mut self) -> io::Result<usize> {
        self.refresh()?;
        self.with_storage(|storage| storage.line_count())
    }

    pub fn line_start(&mut self, line: usize) -> io::Result<usize> {
        self.refresh()?;
        self.with_storage(|storage| storage.line_start(line))
    }

    pub fn line_length(&mut self, line: usize) -> io::Result<usize> {
        self.refresh()?;
        self.with_storage(|storage| storage.line_length(line))
    }

    pub fn line_text(&mut self, line: usize) -> io::Result<String> {
        self.refresh()?;
        self.with_storage(|storage| storage.line_text(line))
    }

    pub fn line_text_from_start(&mut self, start: usize) -> io::Result<(String, usize)> {
        self.refresh()?;
        self.with_storage(|storage| storage.line_text_from_start(start))
    }

    pub fn next_line_start_from(&mut self, start: usize) -> io::Result<Option<usize>> {
        self.refresh()?;
        self.with_storage(|storage| storage.next_line_start_from(start))
    }

    pub fn previous_line_start_from(&mut self, start: usize) -> io::Result<usize> {
        self.refresh()?;
        self.with_storage(|storage| storage.previous_line_start_from(start))
    }

    pub fn previous_char_boundary(&self, position: usize) -> io::Result<usize> {
        self.storage.borrow().previous_char_boundary(position)
    }

    pub fn next_char_boundary(&self, position: usize) -> io::Result<usize> {
        self.storage.borrow().next_char_boundary(position)
    }

    pub fn line_column_at(&mut self, position: usize) -> io::Result<(usize, usize)> {
        self.refresh()?;
        self.with_storage(|storage| storage.line_column_at(position))
    }

    pub fn move_cursor(&mut self, position: usize) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.move_cursor(position))
    }

    pub fn select_all(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.select_all())
    }

    pub fn move_word(&mut self, forward: bool, selecting: bool) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.move_word(forward, selecting))
    }

    pub fn move_page(&mut self, forward: bool, lines: usize, selecting: bool) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.move_page(forward, lines, selecting))
    }

    pub fn select_home(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.select_home())
    }

    pub fn select_end(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.select_end())
    }

    pub(crate) fn current_line_start(&mut self) -> io::Result<usize> {
        self.refresh()?;
        self.with_storage(|storage| storage.current_line_start())
    }

    pub fn cursor_left(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.cursor_left())
    }

    pub fn cursor_right(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.cursor_right())
    }

    pub fn cursor_up(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.cursor_up())
    }

    pub fn cursor_down(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.cursor_down())
    }

    pub fn select_left(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.select_left())
    }

    pub fn select_right(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.select_right())
    }

    pub fn select_up(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.select_up())
    }

    pub fn select_down(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.select_down())
    }

    pub fn cursor_home(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.cursor_home())
    }

    pub fn current_line_end(&mut self) -> io::Result<usize> {
        self.refresh()?;
        self.with_storage(|storage| storage.current_line_end())
    }

    pub fn cursor_end(&mut self) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.cursor_end())
    }

    pub(crate) fn replace_runs<I>(
        &mut self,
        runs: I,
        replacement: &str,
    ) -> io::Result<Option<PieceTableSnapshot>>
    where
        I: Iterator<Item = (usize, usize, usize)>,
    {
        self.refresh()?;
        self.with_storage(|storage| storage.replace_runs(runs, replacement))
    }

    pub(crate) fn replace_ranges<I>(
        &mut self,
        ranges: I,
        replacement: &str,
    ) -> io::Result<Option<PieceTableSnapshot>>
    where
        I: Iterator<Item = (usize, usize)>,
    {
        self.refresh()?;
        self.with_storage(|storage| storage.replace_ranges(ranges, replacement))
    }

    pub(crate) fn replace_from_reader(
        &mut self,
        reader: &mut impl Read,
        max_bytes: usize,
    ) -> io::Result<Option<PieceTableSnapshot>> {
        self.refresh()?;
        self.with_storage(|storage| storage.replace_from_reader(reader, max_bytes))
    }

    pub(crate) fn swap_snapshot(&mut self, snapshot: &mut PieceTableSnapshot) {
        self.with_storage(|storage| storage.swap_snapshot(snapshot))
    }

    pub(crate) fn capture_range(&self, position: usize, length: usize) -> io::Result<Vec<Piece>> {
        self.storage.borrow().capture_range(position, length)
    }

    pub(crate) fn range_equals(
        &self,
        position: usize,
        length: usize,
        expected: &[u8],
    ) -> io::Result<bool> {
        self.storage
            .borrow()
            .range_equals(position, length, expected)
    }

    pub(crate) fn insert_pieces(&mut self, position: usize, pieces: &[Piece]) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.insert_pieces(position, pieces))
    }

    pub(crate) fn store_text(&mut self, text: &str) -> io::Result<Option<Piece>> {
        self.refresh()?;
        self.with_storage(|storage| storage.store_text(text))
    }

    pub fn insert(&mut self, position: usize, text: &str) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.insert(position, text))
    }

    pub fn delete(&mut self, position: usize, length: usize) -> io::Result<()> {
        self.refresh()?;
        self.with_storage(|storage| storage.delete(position, length))
    }

    pub fn text(&self) -> io::Result<String> {
        self.storage.borrow().text()
    }

    pub(crate) fn line_window(
        &mut self,
        line: usize,
        left: usize,
        right: usize,
        tab: usize,
    ) -> io::Result<LineWindow> {
        self.refresh()?;
        self.with_storage(|storage| storage.line_window(line, left, right, tab))
    }

    pub(crate) fn line_visual_column(
        &mut self,
        line: usize,
        column: usize,
        tab: usize,
    ) -> io::Result<usize> {
        self.refresh()?;
        self.with_storage(|storage| storage.line_visual_column(line, column, tab))
    }

    pub(crate) fn line_prefix(&mut self, line: usize, max_bytes: usize) -> io::Result<String> {
        self.refresh()?;
        self.with_storage(|storage| storage.line_prefix(line, max_bytes))
    }

    pub(crate) fn enable_recovery(&mut self, source: Option<&Path>) {
        self.with_storage(|storage| storage.enable_recovery(source))
    }

    pub(crate) fn enable_recovery_at(&mut self, root: PathBuf, source: Option<&Path>) {
        self.with_storage(|storage| storage.enable_recovery_at(root, source))
    }

    pub(crate) fn take_recovery_warning(&mut self) -> Option<String> {
        self.with_storage(|storage| storage.take_recovery_warning())
    }

    pub(crate) fn recovered_from(&mut self, directory: PathBuf) {
        self.with_storage(|storage| storage.recovered_from(directory))
    }

    pub(crate) fn recovery_saved(&mut self, path: &Path) {
        self.with_storage(|storage| storage.recovery_saved(path))
    }
}
impl Drop for PieceTable {
    fn drop(&mut self) {
        if let Ok(mut storage) = self.storage.try_borrow_mut() {
            storage.view_history.unregister(&self.seen);
        }
    }
}
