// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Visible fragments are projections; durable positions remain transcript anchors.
use super::*;
use output_selection::{Key, Modifiers, Request, TextSource};

pub(super) enum Pending {
    Key(Key, Modifiers, bool),
    Work(Work),
    Activate(bool),
}
pub(super) struct Press {
    x: f32,
    y: f32,
    dragged: bool,
    open: Option<FileLink>,
    commit: Option<super::super::git_detail::Commit>,
    diagnostic: Option<(Anchor, u64, bool)>,
}
impl Pane {
    pub(crate) fn clipboard_copy(&mut self, all: bool) {
        self.enqueue(Pending::Key(
            Key::C,
            Modifiers {
                gui: true,
                shift: all,
                ..Modifiers::default()
            },
            false,
        ));
    }
    pub(crate) fn focus_prompt(&mut self) {
        self.navigation_job = None;
        self.navigation_queue.clear();
        self.visual_navigation = None;
        self.reveal = None;
        self.output_range = None;
        self.pinned_tail = None;
        self.output_press = None;
        self.output_selection.focus_prompt();
        self.selection.clear();
    }
    fn enqueue(&mut self, pending: Pending) {
        // A failed Enter lookup must not cancel newer output interaction.
        if let Some((_, _, keyboard)) = self.diagnostic_job.as_mut() {
            *keyboard = None;
        }
        if matches!(&pending, Pending::Work(Work::MouseDrag(_)))
            && matches!(
                self.navigation_queue.back(),
                Some(Pending::Work(Work::MouseDrag(_)))
            )
        {
            self.navigation_queue.pop_back();
        }
        if self.navigation_queue.len() < 64 {
            self.navigation_queue.push_back(pending);
            (self.wake)();
        }
    }
    fn start_navigation(&mut self, work: Work) -> io::Result<()> {
        let source = Source {
            store: &self.transcript,
            session: &self.session,
        };
        self.navigation_job = Some(NavigationJob::start_pinned(
            &source,
            &self.output_selection,
            work,
            self.wake.clone(),
            self.pinned_tail.as_ref(),
        )?);
        Ok(())
    }
    pub(super) fn poll_navigation(&mut self) -> bool {
        let mut changed = false;
        let bounds = Source {
            store: &self.transcript,
            session: &self.session,
        }
        .bounds();
        let stale_tail = self.pinned_tail.as_ref().is_some_and(|tail| {
            [
                self.output_selection.cursor(),
                self.output_selection.anchor(),
            ]
            .into_iter()
            .flatten()
            .any(|anchor| {
                if !tail.covers(anchor.record_id) || anchor.record_id < bounds.first {
                    return false;
                }
                if anchor.epoch != bounds.epoch || anchor.record_id >= bounds.end {
                    return true;
                }
                let mut store = self.transcript.lock().unwrap();
                let saved_end = store.base_id() + store.len() as u64;
                anchor.record_id < saved_end
                    && store.read_id(anchor.record_id).is_ok_and(|record| {
                        Some(record.source) != tail.source() || !tail.matches_record(&record.data)
                    })
            })
        });
        if stale_tail {
            self.focus_prompt();
            changed = true;
        }
        let validation_store = self.transcript.clone();
        let invalid = |anchor: Anchor| {
            if anchor.epoch != bounds.epoch
                || anchor.record_id < bounds.first
                || anchor.record_id >= bounds.end
            {
                return true;
            }
            let mut store = validation_store.lock().unwrap();
            let saved_end = store.base_id() + store.len() as u64;
            if anchor.record_id >= saved_end {
                return false;
            }
            let Ok(record) = store.read_id(anchor.record_id) else {
                return true;
            };
            if !matches!(record.data, RecordData::Result { .. }) {
                return false;
            }
            output_selection::record_text_at_end(&record, anchor.record_id + 1 == bounds.end)
                .is_ok_and(|text| text.omitted || anchor.utf8_byte_offset > text.text.len())
        };
        if self
            .visual_navigation
            .is_some_and(|(_, anchor, _)| invalid(anchor))
            || self.reveal.is_some_and(invalid)
            || self.output_selection.cursor().is_some_and(invalid)
        {
            self.visual_navigation = None;
            self.reveal = None;
            if self.navigation_job.is_none() {
                if let Err(error) = self.start_navigation(Work::Refresh) {
                    self.set_status(error.to_string());
                }
            }
            changed = true;
        }
        if self.scroll_anchor.is_some_and(invalid) {
            self.scroll_anchor = None;
            changed = true;
        }
        if let Some(result) = self.navigation_job.as_ref().and_then(NavigationJob::poll) {
            self.navigation_job = None;
            changed = true;
            match result {
                Ok(result) => {
                    self.output_selection = result.selection;
                    self.output_range = result.range;
                    self.reveal = self.output_selection.cursor();
                    self.browse_follow = false;
                }
                Err(error) => self.set_status(error),
            }
        }
        if self.navigation_job.is_some() {
            return changed;
        }
        // One outstanding visual query prevents distant row lookups from
        // repeatedly canceling each other before the budgeted scan completes.
        if let Some((request, cursor, column)) = self.visual_navigation {
            let target = match request {
                Request::Rows { delta, .. } | Request::Pages { delta, .. } => {
                    self.layout.neighbour(
                        cursor,
                        delta.clamp(i32::MIN as isize, i32::MAX as isize) as i32,
                        column,
                    )
                }
                Request::VisualEdge { end, .. } => self.layout.row_edge(cursor, end),
                _ => Ok(None),
            };
            match target {
                Ok(Some(target)) => {
                    let extend = match request {
                        Request::Rows { extend, .. }
                        | Request::Pages { extend, .. }
                        | Request::VisualEdge { extend, .. } => extend,
                        _ => false,
                    };
                    self.visual_navigation = None;
                    if let Err(error) =
                        self.start_navigation(Work::ApplyVisual(target, extend, column))
                    {
                        self.set_status(error.to_string());
                    }
                    return true;
                }
                Ok(None) => return changed,
                Err(error) => {
                    self.visual_navigation = None;
                    self.set_status(error.to_string());
                    changed = true;
                }
            }
        }
        // Reveal has priority over a saved viewport anchor, and finishes before
        // the next key starts another row lookup.
        if let Some(anchor) = self.reveal {
            match self.layout.row_at(anchor) {
                Ok(Some(row)) => {
                    let rows = self.output_screen().size().0 as usize;
                    if row < self.browse_offset {
                        self.browse_offset = row;
                    } else if row >= self.browse_offset + rows {
                        self.browse_offset = row + 1 - rows;
                    }
                    self.reveal = None;
                    changed = true;
                }
                Ok(None) => return changed,
                Err(error) => {
                    self.reveal = None;
                    self.set_status(error.to_string());
                }
            }
        }
        if let Some(anchor) = self.scroll_anchor {
            match self.layout.row_at(anchor) {
                Ok(Some(row)) => {
                    self.browse_offset = row;
                    self.scroll_anchor = None;
                    changed = true;
                }
                Ok(None) => return changed,
                Err(error) => {
                    self.scroll_anchor = None;
                    self.set_status(error.to_string());
                }
            }
        }
        if let Some(pending) = self.navigation_queue.pop_front() {
            let result = match pending {
                Pending::Work(work) => self.start_navigation(work),
                Pending::Activate(other) => {
                    if !self.activate_output_cursor(other) {
                        self.focus_prompt();
                    }
                    Ok(())
                }
                Pending::Key(key, mods, vim) => {
                    let request = self.output_selection.key(key, mods, vim);
                    match request {
                        Request::Move { motion, extend } => {
                            self.start_navigation(Work::Motion(motion, extend))
                        }
                        Request::Rows { .. }
                        | Request::Pages { .. }
                        | Request::VisualEdge { .. } => {
                            if let Some(cursor) = self.output_selection.cursor() {
                                let column = self
                                    .output_selection
                                    .desired_x()
                                    .unwrap_or_else(|| {
                                        self.anchor_point(cursor)
                                            .map_or(0, |point| i32::from(point.col))
                                    })
                                    .clamp(0, u16::MAX as i32)
                                    as u16;
                                let request = if let Request::Pages { delta, extend } = request {
                                    Request::Pages {
                                        delta: delta.saturating_mul(20),
                                        extend,
                                    }
                                } else {
                                    request
                                };
                                self.visual_navigation = Some((request, cursor, column));
                                self.reveal = None;
                            }
                            Ok(())
                        }
                        Request::SelectAll => self.start_navigation(Work::SelectAll),
                        Request::Copy { all, yank } => {
                            if yank && self.output_range.is_none_or(|range| range.is_empty()) {
                                Ok(())
                            } else if key == Key::C
                                && mods.control
                                && !mods.gui
                                && !mods.shift
                                && self.output_range.is_none_or(|range| range.is_empty())
                            {
                                if self.running() {
                                    self.stop();
                                }
                                Ok(())
                            } else {
                                self.start_copy(
                                    all || !yank
                                        && self.output_range.is_none_or(|range| range.is_empty()),
                                    yank,
                                )
                                .map_err(io::Error::other)
                            }
                        }
                        Request::FocusPrompt => {
                            self.focus_prompt();
                            Ok(())
                        }
                        Request::None
                            if key == Key::V && vim && !mods.control && !mods.gui && !mods.alt =>
                        {
                            self.start_navigation(Work::Refresh)
                        }
                        Request::None => Ok(()),
                    }
                }
            };
            if let Err(error) = result {
                self.set_status(error.to_string());
            }
            changed = true;
        }
        changed
    }
    pub(super) fn refresh_output(
        &mut self,
        rows: u16,
        cols: u16,
        tab_width: usize,
        font_key: u32,
        measure: &mut dyn FnMut(char) -> i32,
    ) -> io::Result<()> {
        let (epoch, base) = {
            let store = self.transcript.lock().unwrap();
            (store.epoch(), store.base_id())
        };
        if epoch != self.browse_epoch {
            self.fragments = std::sync::Arc::new(Vec::new());
            self.output_range = None;
            self.browse_offset = 0;
            self.browse_follow = true;
        }
        let viewport = self.fragments.first().map(|fragment| fragment.start);
        let configured = self.layout.configure(Metrics {
            columns: cols,
            tab_width,
        }) | self.layout.set_wrap_pixels(
            (self.area.width() as i32 - painting::MARGIN * 2).max(1),
            self.cell.0,
            font_key,
        );
        if !self.browse_follow
            && (configured || base != self.browse_base_id)
            && self.visual_navigation.is_none()
            && self.reveal.is_none()
        {
            self.scroll_anchor = viewport.map(|mut anchor| {
                if anchor.record_id < base {
                    anchor.record_id = base;
                    anchor.utf8_byte_offset = 0;
                }
                anchor
            });
        }
        self.browse_epoch = epoch;
        self.browse_base_id = base;
        if self.browse_follow {
            self.browse_offset = self.layout.len().saturating_sub(rows as usize);
        } else if self.layout.complete() {
            self.browse_offset = self
                .browse_offset
                .min(self.layout.len().saturating_sub(rows as usize));
        }
        self.layout
            .request_visible(self.browse_offset, rows as usize)?;
        self.layout.poll_with_measure(measure)?;
        let fragments = self
            .layout
            .visible_shared(self.browse_offset, rows as usize)?;
        let mut frame_changed = false;
        if !fragments.is_empty() || self.layout.complete() && self.layout.len() == 0 {
            frame_changed = !std::sync::Arc::ptr_eq(&self.fragments, &fragments);
            self.fragments = fragments;
        }
        // Fragments are authoritative for normal output; this compatibility
        // grid stays bounded even when a normal pane is wide or tall.
        let view_rows = rows.min(super::super::session::MAX_ROWS);
        let view_cols = cols.min(super::super::session::MAX_COLS);
        if !frame_changed
            && self
                .view
                .as_ref()
                .is_some_and(|view| view.screen().size() == (view_rows, view_cols))
        {
            // Selection/focus can change without changing the published text.
            // Reuse its compatibility grid, updating only the cursor state.
            let cursor = self
                .output_selection
                .focused()
                .then(|| {
                    self.output_selection
                        .cursor()
                        .and_then(|anchor| self.anchor_point(anchor))
                })
                .flatten();
            let view = self.view.as_mut().unwrap();
            view.process(b"\x1b[?25l");
            if let Some(point) =
                cursor.filter(|point| point.row < view_rows && point.col < view_cols)
            {
                view.process(
                    format!("\x1b[{};{}H\x1b[?25h", point.row + 1, point.col + 1).as_bytes(),
                );
            }
            self.paint_output_selection();
            return Ok(());
        }
        let mut view = vt100::Parser::new(view_rows, view_cols, 0);
        view.process(b"\x1b[?25l");
        for (row, fragment) in self.fragments.iter().take(view_rows as usize).enumerate() {
            for segment in &fragment.segments {
                if segment.column >= view_cols {
                    continue;
                }
                view.process(
                    format!("\x1b[{};{}H", row + 1, usize::from(segment.column) + 1).as_bytes(),
                );
                let style = segment.style;
                let mut sgr = String::from("\x1b[0");
                for (enabled, code) in [
                    (style.bold, 1),
                    (style.dim, 2),
                    (style.italic, 3),
                    (style.underline, 4),
                    (style.inverse, 7),
                ] {
                    if enabled {
                        sgr.push_str(&format!(";{code}"));
                    }
                }
                match segment
                    .color
                    .map(|(r, g, b)| vt100::Color::Rgb(r, g, b))
                    .unwrap_or(style.fg)
                {
                    vt100::Color::Rgb(r, g, b) => sgr.push_str(&format!(";38;2;{r};{g};{b}")),
                    vt100::Color::Idx(n) => sgr.push_str(&format!(";38;5;{n}")),
                    vt100::Color::Default => {}
                }
                match style.bg {
                    vt100::Color::Rgb(r, g, b) => sgr.push_str(&format!(";48;2;{r};{g};{b}")),
                    vt100::Color::Idx(n) => sgr.push_str(&format!(";48;5;{n}")),
                    vt100::Color::Default => {}
                }
                sgr.push('m');
                view.process(sgr.as_bytes());
                view.process(fragment.text[segment.range.clone()].as_bytes());
            }
        }
        if self.output_selection.focused() {
            if let Some(point) = self
                .output_selection
                .cursor()
                .and_then(|anchor| self.anchor_point(anchor))
            {
                if point.row < view_rows && point.col < view_cols {
                    view.process(
                        format!("\x1b[{};{}H\x1b[?25h", point.row + 1, point.col + 1).as_bytes(),
                    );
                }
            }
        }
        self.view = Some(view);
        self.paint_output_selection();
        Ok(())
    }
    pub(super) fn anchor_point(&self, anchor: Anchor) -> Option<Point> {
        for (row, fragment) in self.fragments.iter().enumerate() {
            if fragment.start == anchor && fragment.end == anchor {
                return Some(Point {
                    row: row as u16,
                    col: 0,
                });
            }
            if let Some(segment) = fragment
                .segments
                .iter()
                .find(|segment| segment.start == anchor)
            {
                return Some(Point {
                    row: row as u16,
                    col: segment.column,
                });
            }
        }
        for (row, fragment) in self.fragments.iter().enumerate() {
            for segment in &fragment.segments {
                if anchor >= segment.start && anchor <= segment.end {
                    let text = &fragment.text[segment.range.clone()];
                    let relative = anchor
                        .utf8_byte_offset
                        .saturating_sub(segment.start.utf8_byte_offset)
                        .min(text.len());
                    let column = if segment.end.utf8_byte_offset - segment.start.utf8_byte_offset
                        != text.len()
                    {
                        if anchor == segment.start {
                            0
                        } else {
                            fragment.text_columns(text)
                        }
                    } else {
                        fragment.text_columns(&text[..floor_boundary(text, relative)])
                    };
                    return Some(Point {
                        row: row as u16,
                        col: segment.column.saturating_add(column as u16),
                    });
                }
            }
        }
        None
    }
    fn paint_output_selection(&mut self) {
        self.selection.clear();
        let Some(range) = self.output_range.filter(|range| !range.is_empty()) else {
            return;
        };
        for (row, fragment) in self.fragments.iter().enumerate() {
            for segment in &fragment.segments {
                if segment.end <= range.start || segment.start >= range.end {
                    continue;
                }
                let text = &fragment.text[segment.range.clone()];
                let synthetic =
                    segment.end.utf8_byte_offset - segment.start.utf8_byte_offset != text.len();
                let start = if range.start <= segment.start {
                    0
                } else if synthetic {
                    0
                } else {
                    range
                        .start
                        .utf8_byte_offset
                        .saturating_sub(segment.start.utf8_byte_offset)
                        .min(text.len())
                };
                let end = if range.end >= segment.end {
                    text.len()
                } else if synthetic {
                    text.len()
                } else {
                    range
                        .end
                        .utf8_byte_offset
                        .saturating_sub(segment.start.utf8_byte_offset)
                        .min(text.len())
                };
                let left = segment.column
                    + fragment.text_columns(&text[..floor_boundary(text, start)]) as u16;
                let right = segment.column
                    + fragment.text_columns(&text[..floor_boundary(text, end)]) as u16;
                if right > left {
                    self.selection.spans.push((row as u16, left, right));
                }
            }
            if range.start <= fragment.end && range.end > fragment.end {
                let col = fragment.text_columns(&fragment.text).min(u16::MAX as usize) as u16;
                self.selection
                    .spans
                    .push((row as u16, col, col.saturating_add(1)));
            }
        }
    }
    fn output_hit(&self, x: f32, y: f32) -> Option<(Anchor, Option<(u64, u64, bool)>)> {
        let x = x as i32 - self.area.x() - painting::MARGIN;
        let y = y as i32 - self.area.y() - painting::MARGIN;
        if x < 0 || y < 0 || y >= self.area.height() as i32 - painting::FOOTER - painting::MARGIN {
            return None;
        }
        let row = (y / self.cell.1) as usize;
        let fragment = self.fragments.get(row)?;
        if self.view.is_some() {
            for hit in &self.zero_native_hits {
                if usize::from(hit.row) != row || x < hit.left || x >= hit.left + hit.width as i32 {
                    continue;
                }
                let Some(segment) = fragment.segments.get(hit.segment) else {
                    continue;
                };
                if segment.native
                    && segment.start == hit.start
                    && segment.source == hit.source
                    && fragment.text_columns(&fragment.text[segment.range.clone()]) == 0
                {
                    return Some((
                        segment.start,
                        Some((segment.record_id, segment.source, true)),
                    ));
                }
            }
        }
        let column = (x / self.cell.0) as u16;
        let anchor = fragment.anchor_column(column);
        let id = fragment
            .segments
            .iter()
            .find(|segment| {
                column >= segment.column
                    && usize::from(column - segment.column)
                        < fragment.text_columns(&fragment.text[segment.range.clone()])
            })
            .map(|segment| (segment.record_id, segment.source, segment.native));
        Some((anchor, id))
    }
    fn native_link(
        &self,
        id: u64,
        source: u64,
        epoch: u64,
        other_pane: bool,
        byte: Option<usize>,
    ) -> Option<FileLink> {
        let record = self.transcript.lock().unwrap().read_id(id).ok()?;
        if record.source != source || record.epoch != epoch {
            return None;
        }
        let RecordData::Native(bytes) = record.data else {
            return None;
        };
        if let Some(byte) = byte {
            if !super::super::browser::native_name_range(&bytes)
                .ok()
                .flatten()?
                .contains(&byte)
            {
                return None;
            }
        }
        let row = super::super::browser::decode_native(&bytes).ok()?;
        if !matches!(row.kind, Kind::Text | Kind::Directory) {
            return None;
        }
        Some(FileLink {
            path: row.path?,
            line: None,
            column: None,
            byte_column: false,
            read_only: false,
            other_pane,
        })
    }
    fn activate_output_cursor(&mut self, other: bool) -> bool {
        let Some(anchor) = self.output_selection.cursor() else {
            return false;
        };
        let (epoch, first, end, record) = {
            let mut store = self.transcript.lock().unwrap();
            (
                store.epoch(),
                store.base_id(),
                store.base_id() + store.len() as u64,
                store.read_id(anchor.record_id).ok(),
            )
        };
        if anchor.epoch != epoch || anchor.record_id < first {
            return false;
        }
        if let Some(record) = record {
            if record.epoch != anchor.epoch {
                return false;
            }
            if matches!(&record.data, RecordData::Native(_)) {
                if let Some(link) = self.native_link(
                    record.id,
                    record.source,
                    anchor.epoch,
                    other,
                    Some(anchor.utf8_byte_offset),
                ) {
                    if self.queued.len() < 32 {
                        self.queued.push_back(link);
                    }
                    return true;
                }
                if let Ok(Some(commit)) =
                    super::super::git_detail::GitDetail::commit_at(&record, anchor.utf8_byte_offset)
                {
                    if let Err(error) = self.open_commit(commit) {
                        self.set_status(error.to_string());
                    }
                    return true;
                }
            } else if matches!(&record.data, RecordData::Terminal { .. }) {
                self.start_diagnostic(anchor, record.source, other, true);
                return true;
            }
        } else if anchor.record_id >= end {
            let bounds = Source {
                store: &self.transcript,
                session: &self.session,
            }
            .bounds();
            if anchor.record_id < bounds.end {
                let source = self
                    .pinned_tail
                    .as_ref()
                    .filter(|tail| tail.covers(anchor.record_id))
                    .and_then(PinnedTail::source)
                    .or_else(|| self.session.tail_source());
                if let Some(source) = source {
                    self.start_diagnostic(anchor, source, other, true);
                    return true;
                }
            }
        }
        false
    }
    fn start_diagnostic(&mut self, anchor: Anchor, source: u64, other: bool, keyboard: bool) {
        let record = self
            .transcript
            .lock()
            .unwrap()
            .read_id(anchor.record_id)
            .ok();
        let cwd = record
            .filter(|record| record.epoch == anchor.epoch && record.source == source)
            .map_or_else(|| self.output_directory.clone(), |record| record.cwd);
        let cancellation = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let frozen = Source {
            store: &self.transcript,
            session: &self.session,
        }
        .freeze_pinned(cancellation, self.pinned_tail.as_ref());
        let provenance = super::super::diagnostic_links::Provenance {
            epoch: anchor.epoch,
            source,
            cwd,
            relative_safe: true,
        };
        let fallback = (keyboard && self.navigation_queue.is_empty()).then_some(anchor);
        self.diagnostic_job = Some((
            super::super::diagnostic_links::Job::start(
                anchor,
                self.transcript.clone(),
                Some(frozen),
                provenance,
                self.wake.clone(),
            ),
            other,
            fallback,
        ));
    }
    #[cfg(test)]
    pub(crate) fn resource_stats(&self) -> (usize, u64) {
        let store = self.transcript.lock().unwrap();
        (store.len(), store.base_id())
    }
    #[cfg(test)]
    pub(crate) fn resource_select_tail(&mut self, max_bytes: usize) -> io::Result<()> {
        let mut source = Source {
            store: &self.transcript,
            session: &self.session,
        };
        let bounds = source.bounds();
        if bounds.first == bounds.end {
            return Ok(());
        }
        let mut id = bounds.end - 1;
        let end_text = source.text(id)?;
        let end = Anchor {
            epoch: bounds.epoch,
            record_id: id,
            utf8_byte_offset: end_text.text.len(),
        };
        let mut remaining = max_bytes.min(4096);
        let mut start = end;
        for _ in 0..256 {
            let row = source.text(id)?;
            if remaining <= row.text.len() {
                start = Anchor {
                    epoch: bounds.epoch,
                    record_id: id,
                    utf8_byte_offset: floor_boundary(&row.text, row.text.len() - remaining),
                };
                break;
            }
            start = Anchor {
                epoch: bounds.epoch,
                record_id: id,
                utf8_byte_offset: 0,
            };
            remaining = remaining.saturating_sub(row.text.len() + usize::from(!row.join_next));
            if id == bounds.first {
                break;
            }
            id -= 1;
        }
        self.output_selection.apply(&mut source, start, false)?;
        self.output_selection.apply(&mut source, end, true)?;
        self.output_range = self.output_selection.range(&mut source)?;
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn output_pending(&self) -> bool {
        self.pending()
            || self.navigation_job.is_some()
            || self.copy_job.is_some()
            || self.diagnostic_job.is_some()
    }
    #[cfg(test)]
    pub(crate) fn output_cursor(&self) -> Option<(u16, u16)> {
        let point = self.anchor_point(self.output_selection.cursor()?)?;
        Some((point.row, point.col))
    }
    #[cfg(test)]
    pub(crate) fn text_point(&self, needle: &str) -> Option<(f32, f32)> {
        self.text_point_matching(needle, false)
    }
    #[cfg(test)]
    pub(crate) fn terminal_text_point(&self, needle: &str) -> Option<(f32, f32)> {
        self.text_point_matching(needle, true)
    }
    #[cfg(test)]
    fn text_point_matching(&self, needle: &str, terminal: bool) -> Option<(f32, f32)> {
        for (row, fragment) in self.fragments.iter().enumerate() {
            let Some(byte) = fragment.text.find(needle) else {
                continue;
            };
            let segment = fragment
                .segments
                .iter()
                .find(|segment| segment.range.contains(&byte))?;
            if terminal {
                if segment.native {
                    continue;
                }
                if self
                    .transcript
                    .lock()
                    .unwrap()
                    .read_id(segment.record_id)
                    .is_ok_and(|record| !matches!(record.data, RecordData::Terminal { .. }))
                {
                    continue;
                }
            }
            let column = segment.column
                + fragment.text_columns(&fragment.text[segment.range.start..byte]) as u16;
            return Some((
                (self.area.x() + painting::MARGIN + i32::from(column) * self.cell.0 + 2) as f32,
                (self.area.y() + painting::MARGIN + row as i32 * self.cell.1 + 2) as f32,
            ));
        }
        None
    }
    #[cfg(test)]
    pub(crate) fn path_point(&self, path: &Path) -> Option<(f32, f32)> {
        for (row, fragment) in self.fragments.iter().enumerate() {
            for segment in &fragment.segments {
                if self
                    .native_link(
                        segment.record_id,
                        segment.source,
                        segment.start.epoch,
                        false,
                        None,
                    )
                    .is_some_and(|link| link.path == path)
                {
                    let record = self
                        .transcript
                        .lock()
                        .unwrap()
                        .read_id(segment.record_id)
                        .ok()?;
                    let RecordData::Native(bytes) = record.data else {
                        continue;
                    };
                    let name = super::super::browser::native_name_range(&bytes)
                        .ok()
                        .flatten()?;
                    let byte = name.start.max(segment.start.utf8_byte_offset);
                    if byte >= name.end || byte >= segment.end.utf8_byte_offset {
                        continue;
                    }
                    let relative = byte - segment.start.utf8_byte_offset;
                    let text = &fragment.text[segment.range.clone()];
                    let column = segment.column
                        + fragment
                            .text_columns(&text[..floor_boundary(text, relative.min(text.len()))])
                            as u16;
                    return Some((
                        (self.area.x() + painting::MARGIN + i32::from(column) * self.cell.0 + 2)
                            as f32,
                        (self.area.y() + painting::MARGIN + row as i32 * self.cell.1 + 2) as f32,
                    ));
                }
            }
        }
        None
    }
    pub(super) fn output_event(
        &mut self,
        event: &Event,
        modifiers: sdl3::keyboard::Mod,
        _clipboard: &sdl3::clipboard::ClipboardUtil,
        vim: bool,
    ) -> Option<Option<Action>> {
        if self.view.is_none() {
            return None;
        }
        if let Event::KeyDown {
            keycode: Some(key),
            keymod,
            repeat,
            ..
        } = event
        {
            let enter_output = !self.output_selection.focused()
                && self.navigation_job.is_none()
                && *key == Keycode::Up
                && input::shift(*keymod)
                && !input::control(*keymod)
                && !input::command(*keymod)
                && !input::alt(*keymod);
            if (*key == Keycode::F6 && !repeat) || enter_output {
                if self.output_selection.focused() || self.navigation_job.is_some() {
                    self.focus_prompt();
                } else {
                    self.pinned_tail = self
                        .layout
                        .published_plain_tail()
                        .map(|(text, source, first)| Source::pin_plain(text, source, first))
                        .or_else(|| {
                            self.layout.published_tail().map(
                                |(screen, start, count, source, first)| {
                                    Source::pin(screen, start, count, source, first)
                                },
                            )
                        });
                    self.enqueue(Pending::Work(Work::Motion(
                        output_selection::Motion::End,
                        false,
                    )));
                    if enter_output {
                        self.enqueue(Pending::Key(
                            Key::Up,
                            Modifiers {
                                shift: true,
                                ..Modifiers::default()
                            },
                            vim,
                        ));
                    }
                }
                return Some(Some(Action::Handled));
            }
        }
        match event {
            Event::KeyDown {
                keycode: Some(key),
                keymod,
                ..
            } if self.output_selection.focused()
                || self.navigation_job.is_some()
                || !self.navigation_queue.is_empty() =>
            {
                if (*key == Keycode::Grave || *key == Keycode::P) && input::control(*keymod) {
                    return None;
                }
                if *key == Keycode::Escape {
                    self.focus_prompt();
                    return Some(Some(Action::Handled));
                }
                if *key == Keycode::Return || *key == Keycode::KpEnter {
                    self.enqueue(Pending::Activate(
                        input::command(*keymod) || input::control(*keymod),
                    ));
                    return Some(Some(Action::Handled));
                }
                if *key == Keycode::V
                    && (input::command(*keymod) || input::control(*keymod) && input::shift(*keymod))
                {
                    self.focus_prompt();
                    return None;
                }
                if let Some(key) = output_key(*key, input::shift(*keymod)) {
                    self.enqueue(Pending::Key(
                        key,
                        Modifiers {
                            shift: input::shift(*keymod),
                            control: input::control(*keymod),
                            gui: input::command(*keymod),
                            alt: input::alt(*keymod),
                        },
                        vim,
                    ));
                }
                Some(Some(Action::Handled))
            }
            Event::TextInput { .. }
                if self.output_selection.focused()
                    || self.navigation_job.is_some()
                    || !self.navigation_queue.is_empty() =>
            {
                Some(Some(Action::Handled))
            }
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                clicks,
                ..
            } => {
                if let Some((anchor, id)) = self.output_hit(*x, *y) {
                    let extend = input::shift(modifiers);
                    let other = input::command(modifiers) || input::control(modifiers);
                    let activate = *clicks == 1 && !extend;
                    let open = if activate {
                        id.and_then(|(id, source, native)| {
                            native
                                .then(|| {
                                    self.native_link(
                                        id,
                                        source,
                                        anchor.epoch,
                                        other,
                                        Some(anchor.utf8_byte_offset),
                                    )
                                })
                                .flatten()
                        })
                    } else {
                        None
                    };
                    let commit = if activate {
                        id.filter(|(_, _, native)| *native)
                            .and_then(|(id, source, _)| {
                                let record = self.transcript.lock().unwrap().read_id(id).ok()?;
                                if record.epoch != anchor.epoch || record.source != source {
                                    return None;
                                }
                                super::super::git_detail::GitDetail::commit_at(
                                    &record,
                                    anchor.utf8_byte_offset,
                                )
                                .ok()
                                .flatten()
                            })
                    } else {
                        None
                    };
                    let diagnostic = if activate {
                        id.filter(|(_, _, native)| !native)
                            .map(|(_, source, _)| (anchor, source, other))
                    } else {
                        None
                    };
                    self.pinned_tail = self
                        .layout
                        .published_plain_tail()
                        .map(|(text, source, first)| Source::pin_plain(text, source, first))
                        .or_else(|| {
                            self.layout.published_tail().map(
                                |(screen, start, count, source, first)| {
                                    Source::pin(screen, start, count, source, first)
                                },
                            )
                        });
                    self.output_press = Some(Press {
                        x: *x,
                        y: *y,
                        dragged: false,
                        open,
                        commit,
                        diagnostic,
                    });
                    self.enqueue(Pending::Work(Work::MouseBegin(anchor, *clicks, extend)));
                } else {
                    self.focus_prompt();
                }
                Some(Some(Action::Handled))
            }
            Event::MouseMotion { x, y, .. } if self.output_press.is_some() => {
                if let Some(press) = self.output_press.as_mut() {
                    press.dragged |= (*x - press.x).abs() >= 3.0 || (*y - press.y).abs() >= 3.0;
                }
                if self
                    .output_press
                    .as_ref()
                    .is_some_and(|press| press.dragged)
                {
                    if let Some((anchor, _)) = self.output_hit(*x, *y) {
                        self.enqueue(Pending::Work(Work::MouseDrag(anchor)));
                    }
                }
                Some(Some(Action::Handled))
            }
            Event::MouseButtonUp {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } if self.output_press.is_some() => {
                let press = self.output_press.take().unwrap();
                self.enqueue(Pending::Work(Work::MouseEnd));
                if !press.dragged && (*x - press.x).abs() < 3.0 && (*y - press.y).abs() < 3.0 {
                    if let Some(commit) = press.commit {
                        if let Err(error) = self.open_commit(commit) {
                            self.set_status(error.to_string());
                        }
                    } else if let Some(link) = press.open {
                        return Some(Some(Action::Open(link)));
                    } else if let Some((anchor, source, other)) = press.diagnostic {
                        self.start_diagnostic(anchor, source, other, false);
                    }
                }
                Some(Some(Action::Handled))
            }
            Event::Window {
                win_event: sdl3::event::WindowEvent::FocusLost,
                ..
            } => {
                self.output_press = None;
                self.enqueue(Pending::Work(Work::MouseEnd));
                Some(None)
            }
            _ => None,
        }
    }
}
fn floor_boundary(text: &str, mut byte: usize) -> usize {
    while !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}
fn output_key(key: Keycode, shift: bool) -> Option<Key> {
    Some(match key {
        Keycode::Left => Key::Left,
        Keycode::Right => Key::Right,
        Keycode::Up => Key::Up,
        Keycode::Down => Key::Down,
        Keycode::Home => Key::Home,
        Keycode::End => Key::End,
        Keycode::PageUp => Key::PageUp,
        Keycode::PageDown => Key::PageDown,
        Keycode::A => Key::A,
        Keycode::C => Key::C,
        Keycode::H => Key::H,
        Keycode::J => Key::J,
        Keycode::K => Key::K,
        Keycode::L => Key::L,
        Keycode::W => Key::W,
        Keycode::B => Key::B,
        Keycode::V => Key::V,
        Keycode::Y => Key::Y,
        Keycode::G => Key::G,
        Keycode::_0 => Key::Zero,
        Keycode::_4 | Keycode::Dollar if shift => Key::Dollar,
        _ => return None,
    })
}
