// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in startup stages and repeated commands, separate from output resources.
//! Durations are evidence, never performance-threshold test assertions.
use super::*;
use std::{env, path::PathBuf};

const PREFIX: &str = "POTYI_TERM_STARTUP ";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Old,
    New,
}
impl Kind {
    fn label(self) -> &'static str {
        if self == Self::Old { "old" } else { "new" }
    }
}

struct Probe {
    kind: Kind,
    started: Instant,
    frames: terminal::FrameSchedule,
    draws: usize,
    turns: usize,
    ready_seen: bool,
    visible: bool,
}

impl Probe {
    fn event(&self, event: &str, phase: &str, duration_ms: Option<f64>, sequence: Option<usize>) {
        let data = serde_json::json!({
            "event": event, "phase": phase, "kind": self.kind.label(),
            "pid": std::process::id(), "elapsed_ms": self.started.elapsed().as_secs_f64() * 1000.,
            "duration_ms": duration_ms, "command_sequence": sequence,
            "draws": self.draws, "turns": self.turns, "visible": self.visible,
        });
        eprintln!("\n{PREFIX}{data}");
    }

    fn observe_ready(&mut self, app: &Fixture<'_>) {
        if !self.ready_seen && (self.kind == Kind::Old || app.renderer.experimental_ready()) {
            self.ready_seen = true;
            // The legacy panel has no persistent Session to authenticate. Its
            // synchronous-open readiness is deliberately a different event.
            self.event(
                if self.kind == Kind::New {
                    "session_ready"
                } else {
                    "legacy_open_ready"
                },
                "pane_create",
                None,
                None,
            );
        }
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
        self.observe_ready(app);
        if changed {
            self.frames.changed();
        }
        // Preserve the production schedule while avoiding a full idle wait
        // after the last reader/layout poll has already finished an operation.
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
            assert_eq!(
                app.send_application(event, &mut self.frames),
                EventFlow::Continue,
                "Startup probe interrupted by a window close"
            );
        }
        for event in pump.poll_iter().take(64) {
            assert_eq!(
                app.send_application(event, &mut self.frames),
                EventFlow::Continue,
                "Startup probe interrupted by a window close"
            );
        }
        if app.dirty || self.frames.due(Instant::now()) {
            self.draw(app);
        }
    }

    fn settle(&mut self, app: &mut Fixture<'_>, pump: &mut sdl3::EventPump) {
        self.settle_with_completion(app, pump, None);
    }

    fn settle_with_completion(
        &mut self,
        app: &mut Fixture<'_>,
        pump: &mut sdl3::EventPump,
        command: Option<(&str, usize, Instant)>,
    ) {
        let mut completed = false;
        // An operational hang guard, matching the resource fixture. There is
        // intentionally no latency-budget assertion on a successful run.
        let deadline = Instant::now() + Duration::from_secs(600);
        loop {
            self.turn(app, pump, Duration::from_secs(1), true);
            if !completed && !self.running(app) {
                if let Some((phase, sequence, started)) = command {
                    self.event(
                        "command_complete",
                        phase,
                        Some(started.elapsed().as_secs_f64() * 1000.),
                        Some(sequence),
                    );
                }
                completed = true;
            }
            if !self.running(app) && !self.pending(app) {
                self.draw(app);
                if !self.pending(app) {
                    break;
                }
            }
            assert!(Instant::now() < deadline, "Startup probe did not settle");
        }
    }

    fn command(
        &mut self,
        app: &mut Fixture<'_>,
        pump: &mut sdl3::EventPump,
        command: &str,
        sequence: usize,
    ) {
        // Distinct phase names prevent repeated intervals being conflated by
        // phase aggregators or whole-interval process-sample validation.
        let phase = if sequence == 0 {
            "first_command".to_owned()
        } else {
            format!("warm_command_{sequence:02}")
        };
        self.event("phase", &phase, None, Some(sequence));
        let started = Instant::now();
        match self.kind {
            Kind::Old => {
                app.terminal.run_command(command).unwrap();
            }
            Kind::New => app.renderer.experimental_send(command).unwrap(),
        }
        self.event(
            "submitted",
            &phase,
            Some(started.elapsed().as_secs_f64() * 1000.),
            Some(sequence),
        );
        self.frames.changed();
        self.settle_with_completion(app, pump, Some((&phase, sequence, started)));
        self.event(
            "operation",
            &phase,
            Some(started.elapsed().as_secs_f64() * 1000.),
            Some(sequence),
        );
        self.event("end", &phase, None, Some(sequence));
    }
}

#[test]
#[ignore = "Opt-in startup timing; select kind/root and run alone with --ignored --nocapture --test-threads=1"]
fn matched_terminal_startup_probe() {
    let kind = match env::var("POTYI_TERM_STARTUP_KIND").as_deref() {
        Ok("old") => Kind::Old,
        Ok("new") => Kind::New,
        _ => panic!("Select old/new with POTYI_TERM_STARTUP_KIND"),
    };
    let root = PathBuf::from(
        env::var_os("POTYI_TERM_STARTUP_ROOT").expect("Private startup fixture root"),
    )
    .canonicalize()
    .unwrap();
    assert!(root.is_dir() && root.is_absolute());
    let visible = env::var("POTYI_TERM_STARTUP_VISIBLE").as_deref() == Ok("1");
    #[cfg(windows)]
    let default_command = "echo POTYI_STARTUP_DONE";
    #[cfg(not(windows))]
    let default_command = "printf '%s\\n' POTYI_STARTUP_DONE";
    let custom_command = env::var("POTYI_TERM_STARTUP_COMMAND").ok();
    let command = custom_command
        .clone()
        .unwrap_or_else(|| default_command.into());
    let expected_line = env::var("POTYI_TERM_STARTUP_EXPECT_LINE").ok().or_else(|| {
        custom_command
            .is_none()
            .then(|| "POTYI_STARTUP_DONE".into())
    });
    if let Some(line) = &expected_line {
        assert!(!line.is_empty() && !line.contains(['\r', '\n']));
    }
    assert!(!command.is_empty() && command.len() <= 64 * 1024);
    let hold_ms = env::var("POTYI_TERM_STARTUP_HOLD_MS")
        .ok()
        .map(|value| value.parse::<u64>().unwrap())
        .unwrap_or(if visible { 2000 } else { 0 });
    assert!(hold_ms <= 10_000);

    with_fixture(|app| {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let driver = video.current_video_driver();
        if visible {
            assert_ne!(
                driver, "dummy",
                "Visible verification needs the native SDL video driver"
            );
            let window = app.renderer.window_mut();
            window
                .set_title(&format!("Pötyi terminal startup probe ({})", kind.label()))
                .unwrap();
            assert!(window.set_bordered(false), "{}", sdl3::get_error());
            assert!(window.show(), "{}", sdl3::get_error());
        }
        let mut pump = sdl.event_pump().unwrap();
        let mut probe = Probe {
            kind,
            started: Instant::now(),
            frames: terminal::FrameSchedule::default(),
            draws: 0,
            turns: 0,
            ready_seen: false,
            visible,
        };
        eprintln!(
            "\n{PREFIX}{}",
            serde_json::json!({
                "event": "context", "kind": kind.label(), "pid": std::process::id(),
                "video_driver": driver, "visible": visible, "fixture_setup_measured": false,
            "helper_trace_included": false, "warm_commands": 5,
            "command_source": if custom_command.is_some() { "environment override" } else { "built-in small command" },
            })
        );
        probe.event("phase", "pane_create", None, None);
        let opened = Instant::now();
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
        probe.event(
            "open_return",
            "pane_create",
            Some(opened.elapsed().as_secs_f64() * 1000.),
            None,
        );
        probe.observe_ready(app);
        probe.draw(app);
        probe.settle(app, &mut pump);
        probe.event(
            "viewport_settled",
            "pane_create",
            Some(opened.elapsed().as_secs_f64() * 1000.),
            None,
        );
        probe.event("end", "pane_create", None, None);

        probe.event("phase", "clear", None, None);
        let cleared = Instant::now();
        match kind {
            Kind::Old => app.terminal.clear().unwrap(),
            Kind::New => {
                app.key(Keycode::L, Mod::LCTRLMOD);
            }
        }
        probe.frames.changed();
        probe.settle(app, &mut pump);
        probe.event(
            "operation",
            "clear",
            Some(cleared.elapsed().as_secs_f64() * 1000.),
            None,
        );
        probe.event("end", "clear", None, None);
        for sequence in 0..=5 {
            probe.command(app, &mut pump, &command, sequence);
            // Outside the measured operation: default commands must really
            // publish one new output line, rather than merely stop running.
            if let Some(expected) = &expected_line {
                let text = match kind {
                    Kind::Old => app.terminal.output_mut().text().unwrap(),
                    Kind::New => app.renderer.experimental_contents(),
                };
                assert!(
                    text.lines()
                        .filter(|line| line.trim_end_matches('\r') == expected)
                        .count()
                        >= sequence + 1,
                    "Command did not publish its expected output line"
                );
            }
        }
        probe.event("done", "done", None, None);

        // Keep a real window available for observation after measured stages.
        // This hold is explicitly outside the command durations.
        let until = Instant::now() + Duration::from_millis(hold_ms);
        while Instant::now() < until {
            probe.turn(
                app,
                &mut pump,
                until.saturating_duration_since(Instant::now()),
                false,
            );
        }
    });
}
