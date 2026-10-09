// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded text adapters for visible navigation and background clipboard reads.
use super::{
    output_selection::{self, Bounds, RecordText, TextSource},
    session::Session,
    transcript::{RecordData, SharedTranscript},
};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub(super) struct Source<'a> {
    pub(super) store: &'a SharedTranscript,
    pub(super) session: &'a Session,
}
impl TextSource for Source<'_> {
    fn bounds(&self) -> Bounds {
        let store = self.store.lock().unwrap();
        Bounds {
            epoch: store.epoch(),
            first: store.base_id(),
            end: store.base_id() + store.len() as u64 + self.session.live_tail_rows() as u64,
        }
    }
    fn text(&mut self, id: u64) -> io::Result<RecordText> {
        let mut store = self.store.lock().unwrap();
        let saved_end = store.base_id() + store.len() as u64;
        if id < saved_end {
            let record = store.read_id(id)?;
            let (next_matches, completed) = if id + 1 < saved_end {
                let next = store.read_id(id + 1)?;
                (
                    output_selection::joins_next(&record, &next),
                    next.source == record.source && matches!(next.data, RecordData::Result { .. }),
                )
            } else {
                (
                    matches!(record.data, RecordData::Terminal { cols, .. }
                        if (cols == 0) == self.session.plain_tail().is_some())
                        && self.session.live_tail_rows() > 0
                        && self.session.tail_source() == Some(record.source),
                    false,
                )
            };
            drop(store);
            let mut text = output_selection::record_text_at_end(
                &record,
                id + 1 == saved_end && self.session.live_tail_rows() == 0,
            )?;
            text.join_next &= next_matches;
            text.terminated |= completed;
            Ok(text)
        } else {
            drop(store);
            if let Some((text, _)) = self.session.plain_tail() {
                if id != saved_end {
                    return Err(io::Error::other("Output anchor is outside the plain tail"));
                }
                return Ok(plain_text(text));
            }
            let (screen, start, count) = self
                .session
                .tail_screen()
                .ok_or_else(|| io::Error::other("Output tail is no longer available"))?;
            let row = id
                .checked_sub(saved_end)
                .filter(|row| *row < count as u64)
                .ok_or_else(|| io::Error::other("Output anchor is outside the live tail"))?
                as u16;
            let mut text = output_selection::terminal_row_text(screen, start + row);
            text.join_next &= row + 1 < count;
            Ok(text)
        }
    }
}

fn plain_text(text: &str) -> RecordText {
    RecordText {
        text: text.to_owned(),
        join_next: false,
        omitted: false,
        terminated: false,
    }
}

#[derive(Clone)]
enum TailText {
    Grid {
        screen: Arc<vt100::Screen>,
        start: u16,
    },
    Plain(Arc<str>),
}
#[derive(Clone)]
pub(super) struct PinnedTail {
    text: TailText,
    first: u64,
    count: u16,
    source: Option<u64>,
}
impl PinnedTail {
    pub(super) fn covers(&self, id: u64) -> bool {
        id >= self.first && id < self.first + u64::from(self.count)
    }
    pub(super) fn source(&self) -> Option<u64> {
        self.source
    }
    pub(super) fn matches_record(&self, data: &RecordData) -> bool {
        matches!(data, RecordData::Terminal { cols, .. }
            if (*cols == 0) == matches!(self.text, TailText::Plain(_)))
    }
    fn text(&self, row: u16) -> RecordText {
        match &self.text {
            TailText::Plain(text) => plain_text(text),
            TailText::Grid { screen, start } => {
                let mut text = output_selection::terminal_row_text(screen, start + row);
                text.join_next &= row + 1 < self.count;
                text
            }
        }
    }
}
pub(super) struct FrozenSource {
    store: SharedTranscript,
    bounds: Bounds,
    saved_end: u64,
    tail: Option<PinnedTail>,
    cancelled: Arc<AtomicBool>,
}
impl Source<'_> {
    /// Only the bounded transient screen is copied. Retained text stays on disk.
    pub(super) fn pin(
        screen: &vt100::Screen,
        start: u16,
        count: u16,
        source: u64,
        first: u64,
    ) -> PinnedTail {
        PinnedTail {
            text: TailText::Grid {
                screen: Arc::new(screen.normal_screen_clone()),
                start,
            },
            count,
            source: Some(source),
            first,
        }
    }
    pub(super) fn pin_plain(text: Arc<str>, source: u64, first: u64) -> PinnedTail {
        PinnedTail {
            text: TailText::Plain(text),
            first,
            count: 1,
            source: Some(source),
        }
    }
    pub(super) fn freeze(&self, cancelled: Arc<AtomicBool>) -> FrozenSource {
        self.freeze_pinned(cancelled, None)
    }
    pub(super) fn freeze_pinned(
        &self,
        cancelled: Arc<AtomicBool>,
        pinned: Option<&PinnedTail>,
    ) -> FrozenSource {
        let store = self.store.lock().unwrap();
        let saved_end = store.base_id() + store.len() as u64;
        let tail = pinned.cloned().or_else(|| {
            if let Some((text, source)) = self.session.plain_tail() {
                return Some(Self::pin_plain(Arc::from(text), source, saved_end));
            }
            self.session
                .tail_screen()
                .map(|(screen, start, count)| PinnedTail {
                    text: TailText::Grid {
                        screen: Arc::new(if screen.alternate_screen() {
                            screen.clone()
                        } else {
                            screen.normal_screen_clone()
                        }),
                        start,
                    },
                    first: saved_end,
                    count,
                    source: self.session.tail_source(),
                })
        });
        FrozenSource {
            store: self.store.clone(),
            bounds: Bounds {
                epoch: store.epoch(),
                first: store.base_id(),
                end: tail.as_ref().map_or(saved_end, |tail| {
                    saved_end.max(tail.first + u64::from(tail.count))
                }),
            },
            saved_end,
            tail,
            cancelled,
        }
    }
}
impl TextSource for FrozenSource {
    fn bounds(&self) -> Bounds {
        self.bounds
    }
    fn text(&mut self, id: u64) -> io::Result<RecordText> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::other("Copy cancelled"));
        }
        let mut store = self.store.lock().unwrap();
        if store.epoch() != self.bounds.epoch || store.base_id() > self.bounds.first {
            return Err(io::Error::other("History changed while copying; try again"));
        }
        if let Some(tail) = self
            .tail
            .as_ref()
            .filter(|tail| id >= tail.first && id < tail.first + u64::from(tail.count))
        {
            drop(store);
            return Ok(tail.text((id - tail.first) as u16));
        }
        if id < self.saved_end {
            let record = store.read_id(id)?;
            let (next_matches, completed) = if let Some(tail) = self
                .tail
                .as_ref()
                .filter(|tail| id + 1 >= tail.first && id + 1 < tail.first + u64::from(tail.count))
            {
                (
                    tail.matches_record(&record.data) && tail.source == Some(record.source),
                    false,
                )
            } else if id + 1 < self.saved_end {
                let next = store.read_id(id + 1)?;
                (
                    output_selection::joins_next(&record, &next),
                    next.source == record.source && matches!(next.data, RecordData::Result { .. }),
                )
            } else {
                (
                    self.tail.as_ref().is_some_and(|tail| {
                        tail.matches_record(&record.data)
                            && tail.first <= self.saved_end
                            && self.saved_end < tail.first + u64::from(tail.count)
                            && tail.source == Some(record.source)
                    }),
                    false,
                )
            };
            drop(store);
            let mut text =
                output_selection::record_text_at_end(&record, id + 1 == self.bounds.end)?;
            text.join_next &= next_matches;
            text.terminated |= completed;
            Ok(text)
        } else {
            drop(store);
            let tail = self
                .tail
                .as_ref()
                .ok_or_else(|| io::Error::other("No captured output tail"))?;
            let row = id
                .checked_sub(tail.first)
                .filter(|row| *row < tail.count as u64)
                .ok_or_else(|| io::Error::other("Output anchor is outside captured text"))?
                as u16;
            Ok(tail.text(row))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        output_selection::{Range, copy_range},
        transcript::{Anchor, Transcript},
    };
    use super::*;

    fn frozen(text: Arc<str>) -> FrozenSource {
        let store = Transcript::shared().unwrap();
        let cwd = std::env::temp_dir().canonicalize().unwrap();
        store
            .lock()
            .unwrap()
            .append_header(&cwd, 71, "printf")
            .unwrap();
        let saved = store.lock().unwrap();
        let saved_end = saved.base_id() + saved.len() as u64;
        let bounds = Bounds {
            epoch: saved.epoch(),
            first: saved.base_id(),
            end: saved_end + 1,
        };
        drop(saved);
        FrozenSource {
            store,
            bounds,
            saved_end,
            tail: Some(Source::pin_plain(text, 71, saved_end)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    #[test]
    fn frozen_plain_tail_preserves_tabs_unicode_and_trailing_spaces() {
        let text: Arc<str> = Arc::from("\ttrail  á東京  ");
        let mut source = frozen(text.clone());
        let tail = source.tail.as_ref().unwrap();
        assert!(tail.covers(source.saved_end));
        assert!(!tail.covers(source.saved_end + 1));
        assert!(tail.matches_record(&RecordData::Terminal {
            cols: 0,
            wrapped: false,
            formatted: Vec::new(),
        }));
        assert!(!tail.matches_record(&RecordData::Terminal {
            cols: 80,
            wrapped: false,
            formatted: Vec::new(),
        }));
        assert!(!tail.matches_record(&RecordData::Native(Vec::new())));
        let TailText::Plain(pinned) = &tail.text else {
            panic!("Expected plain tail")
        };
        assert!(Arc::ptr_eq(pinned, &text));
        let row = source.text(source.saved_end).unwrap();
        assert_eq!(row.text, &*text);
        assert!(!row.join_next && !row.terminated && !row.omitted);
        let range = Range {
            start: Anchor {
                epoch: source.bounds.epoch,
                record_id: source.bounds.first,
                utf8_byte_offset: 0,
            },
            end: Anchor {
                epoch: source.bounds.epoch,
                record_id: source.saved_end,
                utf8_byte_offset: text.len(),
            },
        };
        assert_eq!(
            copy_range(&mut source, range, 4096).unwrap(),
            format!("$ printf\n{text}")
        );
    }

    #[test]
    fn pinned_plain_tail_survives_sealing_but_clear_and_cancel_abort() {
        let mut source = frozen(Arc::from("displayed  "));
        let cwd = std::env::temp_dir().canonicalize().unwrap();
        {
            let mut store = source.store.lock().unwrap();
            let anchor = store.append_plain(&cwd, 71, "newer", false).unwrap();
            assert_eq!(anchor.record_id, source.saved_end);
            store.append_result(&cwd, 71, 0, "").unwrap();
        }
        assert_eq!(source.text(source.saved_end).unwrap().text, "displayed  ");
        source.cancelled.store(true, Ordering::Release);
        assert!(
            source
                .text(source.saved_end)
                .unwrap_err()
                .to_string()
                .contains("cancelled")
        );
        source.cancelled.store(false, Ordering::Release);
        source.store.lock().unwrap().clear().unwrap();
        assert!(
            source
                .text(source.saved_end)
                .unwrap_err()
                .to_string()
                .contains("History changed")
        );
    }
}
