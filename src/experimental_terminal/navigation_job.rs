// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Logical line/word scans stay off the UI thread, including linewise ranges.
use super::{
    output_selection::{Motion, Range, Selection},
    output_source::{PinnedTail, Source},
    session::Wake,
    transcript::Anchor,
};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

#[derive(Clone, Copy)]
pub(super) enum Work {
    Motion(Motion, bool),
    Apply(Anchor, bool),
    ApplyVisual(Anchor, bool, u16),
    MouseBegin(Anchor, u8, bool),
    MouseDrag(Anchor),
    MouseEnd,
    SelectAll,
    Refresh,
}
pub(super) struct ResultSelection {
    pub(super) selection: Selection,
    pub(super) range: Option<Range>,
}
pub(super) struct NavigationJob {
    receiver: mpsc::Receiver<Result<ResultSelection, String>>,
    cancelled: Arc<AtomicBool>,
}
impl NavigationJob {
    pub(super) fn start(
        source: &Source<'_>,
        selection: &Selection,
        work: Work,
        wake: Wake,
    ) -> io::Result<Self> {
        Self::start_pinned(source, selection, work, wake, None)
    }
    pub(super) fn start_pinned(
        source: &Source<'_>,
        selection: &Selection,
        work: Work,
        wake: Wake,
        pinned: Option<&PinnedTail>,
    ) -> io::Result<Self> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut frozen = source.freeze_pinned(cancelled.clone(), pinned);
        let mut selection = selection.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("potyi-output-navigation".into())
            .spawn(move || {
                let result = (|| -> io::Result<ResultSelection> {
                    match work {
                        Work::Motion(motion, extend) => {
                            selection.move_logical(&mut frozen, motion, extend)?
                        }
                        Work::Apply(target, extend) => {
                            selection.apply(&mut frozen, target, extend)?
                        }
                        Work::ApplyVisual(target, extend, column) => {
                            selection.apply(&mut frozen, target, extend)?;
                            selection.set_desired_x(i32::from(column));
                        }
                        Work::MouseBegin(target, clicks, extend) => {
                            selection.mouse_begin(&mut frozen, target, clicks, extend)?
                        }
                        Work::MouseDrag(target) => selection.mouse_drag(&mut frozen, target)?,
                        Work::MouseEnd => selection.cancel_mouse(),
                        Work::SelectAll => selection.select_all(&mut frozen)?,
                        Work::Refresh => selection.clamp(&mut frozen)?,
                    }
                    let range = selection.range(&mut frozen)?;
                    Ok(ResultSelection { selection, range })
                })()
                .map_err(|error| error.to_string());
                if sender.send(result).is_ok() {
                    wake();
                }
            })?;
        Ok(Self {
            receiver,
            cancelled,
        })
    }
    pub(super) fn poll(&self) -> Option<Result<ResultSelection, String>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Output navigation ended unexpectedly".into()))
            }
        }
    }
}
impl Drop for NavigationJob {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
