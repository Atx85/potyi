// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::{Duration, Instant};
const FRAME: Duration = Duration::from_millis(16);
const IDLE: Duration = Duration::from_secs(1);

/// A frame deadline exists only while output needs displaying. Quiet commands
/// use the editor's idle wait; worker events wake that wait immediately.
#[derive(Default)]
pub(crate) struct FrameSchedule {
    pending: bool,
    next: Option<Instant>,
}

impl FrameSchedule {
    pub fn changed(&mut self) {
        self.pending = true;
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn rendered(&mut self, now: Instant) {
        self.pending = false;
        self.next = Some(now + FRAME);
    }
    pub fn due(&self, now: Instant) -> bool {
        self.pending && self.next.is_none_or(|next| now >= next)
    }
    pub fn wait(&self, now: Instant, input_dirty: bool, work_pending: bool) -> Duration {
        if input_dirty || work_pending {
            return Duration::ZERO;
        }
        if self.pending {
            return self
                .next
                .map_or(Duration::ZERO, |next| next.saturating_duration_since(now));
        }
        IDLE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_has_no_frame_timer_and_input_bypasses_output_throttle() {
        let now = Instant::now();
        let mut schedule = FrameSchedule::default();
        assert_eq!(schedule.wait(now, false, false), IDLE);
        assert!(!schedule.due(now));
        schedule.changed();
        assert!(schedule.due(now));
        schedule.rendered(now);
        assert_eq!(schedule.wait(now, false, false), IDLE);
        schedule.changed();
        assert_eq!(schedule.wait(now, false, false), FRAME);
        assert_eq!(schedule.wait(now, true, false), Duration::ZERO);
        assert_eq!(schedule.wait(now, false, true), Duration::ZERO);
        assert!(schedule.due(now + FRAME));
        schedule.rendered(now + FRAME);
        assert_eq!(schedule.wait(now + FRAME, false, false), IDLE);
    }
}
