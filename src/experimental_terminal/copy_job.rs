// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::{
    output_selection::{Selection, TextSource, Visual},
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

pub(super) struct CopyJob {
    receiver: mpsc::Receiver<Result<Copied, String>>,
    cancelled: Arc<AtomicBool>,
    pub(super) epoch: u64,
    pub(super) all: bool,
    pub(super) yank: bool,
    pub(super) selection: (Option<Anchor>, Option<Anchor>, Visual),
}
pub(super) struct Copied {
    pub(super) text: String,
    pub(super) after_yank: Option<Selection>,
}
impl CopyJob {
    pub(super) fn start(
        source: &Source<'_>,
        selection: &Selection,
        all: bool,
        yank: bool,
        wake: Wake,
    ) -> io::Result<Self> {
        Self::start_pinned(source, selection, all, yank, wake, None)
    }
    pub(super) fn start_pinned(
        source: &Source<'_>,
        selection: &Selection,
        all: bool,
        yank: bool,
        wake: Wake,
        pinned: Option<&PinnedTail>,
    ) -> io::Result<Self> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut frozen = source.freeze_pinned(cancelled.clone(), if all { None } else { pinned });
        let epoch = frozen.bounds().epoch;
        let snapshot = (selection.anchor(), selection.cursor(), selection.visual());
        let mut selection = selection.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("potyi-output-copy".into())
            .spawn(move || {
                let result = (|| -> io::Result<Copied> {
                    let text = selection.copy(&mut frozen, all, false)?;
                    let after_yank = if yank {
                        selection.finish_yank(&mut frozen)?;
                        Some(selection)
                    } else {
                        None
                    };
                    Ok(Copied { text, after_yank })
                })()
                .map_err(|error| error.to_string());
                if sender.send(result).is_ok() {
                    wake();
                }
            })?;
        Ok(Self {
            receiver,
            cancelled,
            epoch,
            all,
            yank,
            selection: snapshot,
        })
    }
    pub(super) fn poll(&self) -> Option<Result<Copied, String>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Output copy ended unexpectedly".into()))
            }
        }
    }
}
impl Drop for CopyJob {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
