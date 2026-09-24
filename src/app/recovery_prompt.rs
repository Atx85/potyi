// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Scan recovery after the first frame, without delaying the window or input.
use crate::{command_bar::CommandBar, piece_table::recovery::{self, StartupNotice}};
use std::{sync::mpsc::{self, Receiver, TryRecvError}, time::Duration};

#[derive(Default)]
pub(super) struct RecoveryPrompt {
    started: bool,
    receiver: Option<Receiver<Option<StartupNotice>>>,
    pending: Option<StartupNotice>,
    shown: Option<(u64, StartupNotice)>,
}

impl RecoveryPrompt {
    pub fn start(&mut self) {
        if self.started { return; }
        self.started = true;
        let (sender, receiver) = mpsc::channel();
        // No SDL handles on this worker: shutdown never has to wait for a scan.
        if std::thread::Builder::new().name("recovery-scan".into()).spawn(move || {
            let notice = match recovery::root().and_then(|root| recovery::startup_notice(&root)) {
                Ok(notice) => notice,
                Err(error) => {
                    // :recover still reports the error on demand; an inaccessible
                    // directory must not interrupt every launch with a panel.
                    eprintln!("Could not check crash recovery: {error}");
                    None
                }
            };
            let _ = sender.send(notice);
        }).is_ok() {
            self.receiver = Some(receiver);
        }
    }

    pub fn wait_timeout(&self) -> Duration {
        if self.receiver.is_some() { Duration::from_millis(100) }
        else { Duration::from_secs(1) }
    }

    pub fn update(&mut self, bar: &mut CommandBar) -> bool {
        if self.shown.as_ref().is_some_and(|(epoch, _)| !bar.is_active() || bar.epoch() != *epoch) {
            self.acknowledge();
        }
        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok(notice) => { self.pending = notice; self.receiver = None; }
                Err(TryRecvError::Disconnected) => self.receiver = None,
                Err(TryRecvError::Empty) => (),
            }
        }
        if !bar.is_active() && let Some(notice) = self.pending.take() {
            bar.open(":recover");
            bar.show_info(&notice.text);
            self.shown = Some((bar.epoch(), notice));
            return true;
        }
        false
    }

    pub fn acknowledge(&mut self) {
        if let Some((_, notice)) = self.shown.take() { notice.acknowledge(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_offer_waits_for_commands_and_dismissal_keeps_the_document() {
        let root = std::env::temp_dir().join(format!("potyi-prompt-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut table = crate::piece_table::PieceTable::empty().unwrap();
        table.enable_recovery_at(root.clone(), None);
        table.insert(0, "recover me").unwrap();
        drop(table);
        let notice = recovery::startup_notice(&root).unwrap();
        let (sender, receiver) = mpsc::channel();
        let mut prompt = RecoveryPrompt { started: true, receiver: Some(receiver), ..Default::default() };
        let mut bar = CommandBar::new();
        bar.open(":find typing");
        sender.send(notice).unwrap();
        let epoch = bar.epoch();
        assert!(!prompt.update(&mut bar));
        assert_eq!(bar.epoch(), epoch, "background result must not replace an active command");
        assert!(recovery::startup_notice(&root).unwrap().is_some());
        bar.close();
        assert!(prompt.update(&mut bar));
        assert!(bar.is_active());
        bar.close();
        assert!(!prompt.update(&mut bar));
        assert!(recovery::startup_notice(&root).unwrap().is_none());
        assert_eq!(recovery::list(&root).unwrap().len(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }
}
