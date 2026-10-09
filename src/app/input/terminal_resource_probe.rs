// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Matched old/new terminal resource fixture. Measurements belong to the
//! external sampler, not to timing-budget assertions inside this test.
use super::*;
use std::{env, path::PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Old,
    New,
}
impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::Old => "old",
            Self::New => "new",
        }
    }
}

struct Probe {
    kind: Kind,
    started: Instant,
    frames: terminal::FrameSchedule,
    draws: usize,
    turns: usize,
}
impl Probe {
    fn marker(&self, event: &str, phase: &str) {
        eprintln!(
            "\nPOTYI_TERM_RESOURCE {{\"event\":\"{event}\",\"phase\":\"{phase}\",\"kind\":\"{}\",\"pid\":{},\"elapsed_ms\":{:.3},\"draws\":{},\"turns\":{}}}",
            self.kind.label(),
            std::process::id(),
            self.started.elapsed().as_secs_f64() * 1000.,
            self.draws,
            self.turns
        );
    }
    fn draw(&mut self, app: &mut Fixture<'_>) {
        app.dirty = false;
        match self.kind {
            Kind::Old => app.renderer.render_terminal(&mut app.terminal).unwrap(),
            Kind::New => app
                .renderer
                .render_experimental(
                    &mut app.editor.document,
                    &mut app.other_editor.document,
                    &mut app.terminal,
                    &app.search_ui,
                    &app.command_bar,
                )
                .unwrap(),
        }
        self.draws += 1;
        self.frames.rendered(Instant::now());
    }
    fn running(&self, app: &Fixture<'_>) -> bool {
        match self.kind {
            Kind::Old => app.terminal.is_running(),
            Kind::New => !app.renderer.experimental_ready() || app.renderer.experimental_running(),
        }
    }
    fn pending(&self, app: &Fixture<'_>) -> bool {
        match self.kind {
            Kind::Old => app.terminal.has_pending_work(),
            Kind::New => {
                app.renderer.experimental_pending() || app.renderer.experimental_output_pending()
            }
        }
    }
    fn work_pending(&self, app: &Fixture<'_>) -> bool {
        match self.kind {
            Kind::Old => app.terminal.has_pending_work(),
            Kind::New => app.renderer.experimental_pending(),
        }
    }
    /// Match the app's event-driven poll, frame throttle and bounded event batch.
    /// No forced unchanged redraws or millisecond idle polling are measured.
    fn turn(
        &mut self,
        app: &mut Fixture<'_>,
        pump: &mut sdl3::EventPump,
        max_wait: Duration,
        settling: bool,
    ) {
        self.turns += 1;
        let changed = match self.kind {
            Kind::Old => app.terminal.poll_background().unwrap(),
            Kind::New => {
                crate::app::experimental::poll(&mut app.renderer, &mut app.vim, &mut app.other_vim)
            }
        };
        if changed {
            self.frames.changed();
        }
        // Consuming the last reader chunk leaves one final poll pending. That
        // poll can clear it without changing output; waiting after it would add
        // the app's one-second idle timeout to an already completed operation.
        if settling && !self.running(app) && !self.pending(app) {
            return;
        }
        let timeout = self
            .frames
            .wait(Instant::now(), app.dirty, self.work_pending(app))
            .min(max_wait);
        if let Some(event) =
            pump.wait_event_timeout(crate::app::experimental::wait_timeout(timeout))
        {
            app.send_application(event, &mut self.frames);
        }
        for event in pump.poll_iter().take(64) {
            app.send_application(event, &mut self.frames);
        }
        if app.dirty || self.frames.due(Instant::now()) {
            app.dirty = false;
            self.draw(app);
        }
    }
    fn settle(&mut self, app: &mut Fixture<'_>, pump: &mut sdl3::EventPump) {
        let deadline = Instant::now() + Duration::from_secs(600);
        loop {
            self.turn(app, pump, Duration::from_secs(1), true);
            if !self.running(app) && !self.pending(app) {
                self.draw(app);
                // Drawing can start reflow after a size/grid change. Publish the
                // final viewport, then stop only when its bounded work settles.
                if !self.pending(app) {
                    break;
                }
            }
            assert!(Instant::now() < deadline, "resource fixture did not settle");
        }
    }
    fn hold(
        &mut self,
        app: &mut Fixture<'_>,
        pump: &mut sdl3::EventPump,
        phase: &str,
        seconds: f64,
    ) {
        self.marker("phase", phase);
        let end = Instant::now() + Duration::from_secs_f64(seconds);
        while Instant::now() < end {
            self.turn(
                app,
                pump,
                end.saturating_duration_since(Instant::now()),
                false,
            );
        }
        self.marker("end", phase);
    }
    fn command(
        &mut self,
        app: &mut Fixture<'_>,
        pump: &mut sdl3::EventPump,
        phase: &str,
        command: &str,
    ) {
        self.marker("phase", phase);
        let start = Instant::now();
        match self.kind {
            Kind::Old => {
                app.terminal.run_command(command).unwrap();
            }
            Kind::New => app.renderer.experimental_send(command).unwrap(),
        }
        self.frames.changed();
        self.settle(app, pump);
        eprintln!(
            "\nPOTYI_TERM_RESOURCE {{\"event\":\"operation\",\"phase\":\"{phase}\",\"kind\":\"{}\",\"duration_ms\":{:.3}}}",
            self.kind.label(),
            start.elapsed().as_secs_f64() * 1000.
        );
        self.marker("end", phase);
    }
    fn copy(&mut self, app: &mut Fixture<'_>, pump: &mut sdl3::EventPump, all: bool) {
        let phase = if all { "copy_all" } else { "copy_selected" };
        if !all {
            match self.kind {
                Kind::Old => {
                    let end = app.terminal.output_mut().len();
                    let mut start = end.saturating_sub(4096);
                    // The producer is UTF-8 and the suffix also contains chrome
                    // rows; align the chosen byte range without a full read.
                    let suffix = app
                        .terminal
                        .output_mut()
                        .read_range(start, end - start)
                        .unwrap();
                    while std::str::from_utf8(&suffix[start - end.saturating_sub(4096)..]).is_err()
                    {
                        start += 1;
                    }
                    app.terminal.move_output_cursor(start, false).unwrap();
                    app.terminal.move_output_cursor(end, true).unwrap();
                }
                Kind::New => app
                    .renderer
                    .experimental_resource_select_tail(4096)
                    .unwrap(),
            }
        }
        app.clipboard
            .set_clipboard_text("POTYI_RESOURCE_COPY_SENTINEL")
            .unwrap();
        self.marker("phase", phase);
        let start = Instant::now();
        match self.kind {
            Kind::Old => app
                .terminal
                .copy_output(&app.clipboard, all, false)
                .unwrap(),
            Kind::New => {
                if all {
                    app.key(Keycode::Escape, Mod::NOMOD);
                }
                app.key(Keycode::C, Mod::LGUIMOD);
                self.frames.changed();
                self.settle(app, pump);
            }
        }
        let text = app.clipboard.clipboard_text().unwrap();
        assert_ne!(
            text, "POTYI_RESOURCE_COPY_SENTINEL",
            "copy was not published"
        );
        assert!(!text.is_empty());
        assert!(
            text.len() <= if all { 8 * 1024 * 1024 } else { 4096 },
            "copy exceeds its chosen scope"
        );
        assert!(
            text.contains("RESOURCE_DONE"),
            "copy must include the command's retained tail"
        );
        let length = text.len();
        drop(text);
        eprintln!(
            "\nPOTYI_TERM_RESOURCE {{\"event\":\"operation\",\"phase\":\"{phase}\",\"kind\":\"{}\",\"duration_ms\":{:.3},\"copied_bytes\":{length}}}",
            self.kind.label(),
            start.elapsed().as_secs_f64() * 1000.
        );
        // Keep clipboard storage resident long enough for external RSS samples.
        let end = Instant::now() + Duration::from_millis(750);
        while Instant::now() < end {
            self.turn(
                app,
                pump,
                end.saturating_duration_since(Instant::now()),
                false,
            );
        }
        self.marker("end", phase);
    }
}

#[cfg(unix)]
fn quote(path: &std::path::Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

#[cfg(unix)]
#[test]
#[ignore = "Matched release resource fixture; use tools/qa/terminal-resources.py"]
fn matched_terminal_resource_probe() {
    let kind = match env::var("POTYI_TERM_RESOURCE_KIND").as_deref() {
        Ok("old") => Kind::Old,
        Ok("new") => Kind::New,
        _ => panic!("select old/new with POTYI_TERM_RESOURCE_KIND"),
    };
    let root = PathBuf::from(env::var_os("POTYI_TERM_RESOURCE_ROOT").expect("runner fixture root"))
        .canonicalize()
        .unwrap();
    let python =
        PathBuf::from(env::var_os("POTYI_TERM_RESOURCE_PYTHON").expect("runner Python producer"));
    assert!(python.is_absolute() && root.is_absolute());
    let idle = env::var("POTYI_TERM_RESOURCE_IDLE_SECONDS")
        .unwrap_or_else(|_| "2".into())
        .parse::<f64>()
        .unwrap();
    assert!((0.5..=10.).contains(&idle));
    let large = env::var("POTYI_TERM_RESOURCE_LARGE_BYTES")
        .unwrap_or_else(|_| (12 * 1024 * 1024).to_string())
        .parse::<usize>()
        .unwrap();
    assert!((9 * 1024 * 1024..=64 * 1024 * 1024).contains(&large));
    with_fixture(|app| {
        let sdl = sdl3::init().unwrap();
        let mut pump = sdl.event_pump().unwrap();
        let mut probe = Probe {
            kind,
            started: Instant::now(),
            frames: terminal::FrameSchedule::default(),
            draws: 0,
            turns: 0,
        };
        match kind {
            Kind::Old => {
                app.terminal = Terminal::new(root.clone()).unwrap();
                app.terminal.set_events(app.event_subsystem.clone());
                app.terminal.open(None);
            }
            Kind::New => {
                app.renderer
                    .experimental_events(&app.event_subsystem)
                    .unwrap();
                app.renderer.open_experimental(&root, None).unwrap();
            }
        }
        probe.draw(app);
        probe.settle(app, &mut pump);
        // Remove initial listing output in both integrations, retaining their
        // active shell/session owners and normal caches as a real Clear does.
        match kind {
            Kind::Old => app.terminal.clear().unwrap(),
            Kind::New => {
                app.key(Keycode::L, Mod::LCTRLMOD);
            }
        }
        probe.frames.changed();
        probe.settle(app, &mut pump);
        probe.marker("ready", "ready");
        // Clear preserves monotonic record IDs, so zero is not an eviction
        // baseline. Compare against the actual empty-store base for this epoch.
        let empty_base = if kind == Kind::New {
            let (len, base) = app.renderer.experimental_resource_stats();
            assert_eq!(len, 0);
            base
        } else {
            0
        };
        probe.hold(app, &mut pump, "idle", idle);
        let command = |bytes: usize, pace: usize| {
            format!(
                "{} {} {bytes} {pace}",
                quote(&python),
                quote(&root.join("producer.py"))
            )
        };
        probe.command(app, &mut pump, "small_command", &command(64 * 1024, 0));
        probe.hold(app, &mut pump, "small_idle", idle);
        probe.command(app, &mut pump, "stream_first", &command(large, 5));
        let first_base = if kind == Kind::New {
            let (len, base) = app.renderer.experimental_resource_stats();
            assert!(
                len > 0 && base > empty_base,
                "first large stream must evict fixed-ring history"
            );
            base
        } else {
            assert!(app.terminal.output_mut().len() <= 8 * 1024 * 1024);
            0
        };
        probe.hold(app, &mut pump, "plateau_first", idle);
        probe.command(app, &mut pump, "stream_second", &command(large, 5));
        let second_base = if kind == Kind::New {
            let (len, base) = app.renderer.experimental_resource_stats();
            assert!(
                len > 0 && base > first_base,
                "second stream must continue bounded eviction"
            );
            base
        } else {
            assert!(app.terminal.output_mut().len() <= 8 * 1024 * 1024);
            0
        };
        probe.hold(app, &mut pump, "plateau_second", idle);
        probe.command(app, &mut pump, "stream_third", &command(large, 5));
        if kind == Kind::New {
            let (len, base) = app.renderer.experimental_resource_stats();
            assert!(
                len > 0 && base > second_base,
                "third stream must continue bounded eviction"
            );
        } else {
            assert!(app.terminal.output_mut().len() <= 8 * 1024 * 1024);
        }
        probe.hold(app, &mut pump, "plateau_third", idle);
        probe.marker("phase", "resize");
        let resized = Instant::now();
        for _ in 0..3 {
            for (width, height) in [(420, 480), (1000, 720), (800, 600)] {
                app.renderer.window_mut().set_size(width, height).unwrap();
                app.renderer.update_window_size().unwrap();
                probe.frames.changed();
                probe.settle(app, &mut pump);
            }
        }
        eprintln!(
            "\nPOTYI_TERM_RESOURCE {{\"event\":\"operation\",\"phase\":\"resize\",\"kind\":\"{}\",\"duration_ms\":{:.3},\"resizes\":9}}",
            kind.label(),
            resized.elapsed().as_secs_f64() * 1000.
        );
        probe.marker("end", "resize");
        probe.copy(app, &mut pump, false);
        probe.copy(app, &mut pump, true);
        probe.hold(app, &mut pump, "copied_idle", idle);
        probe.marker("done", "done");
    });
}
