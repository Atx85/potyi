// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Matched frame export for visual parity review. Run each integration in a
//! fresh headless process with the same directory and font fixture.
use super::*;
use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Original,
    Experimental,
}

impl View {
    fn draw(self, app: &mut Fixture<'_>) {
        match self {
            Self::Original => app.renderer.render_terminal(&mut app.terminal).unwrap(),
            Self::Experimental => app
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
    }

    fn settle(self, app: &mut Fixture<'_>, pump: &mut sdl3::EventPump) {
        let deadline = Instant::now() + Duration::from_secs(45);
        let mut settled = 0;
        let mut frames = terminal::FrameSchedule::default();
        loop {
            match self {
                Self::Original => {
                    app.terminal.poll_background().unwrap();
                }
                Self::Experimental => {
                    crate::app::experimental::poll(
                        &mut app.renderer,
                        &mut app.vim,
                        &mut app.other_vim,
                    );
                }
            }
            for event in pump.poll_iter().take(64) {
                app.send_application(event, &mut frames);
            }
            self.draw(app);
            let pending = match self {
                Self::Original => app.terminal.is_running() || app.terminal.has_pending_work(),
                Self::Experimental => {
                    !app.renderer.experimental_ready()
                        || app.renderer.experimental_running()
                        || app.renderer.experimental_pending()
                        || app.renderer.experimental_output_pending()
                }
            };
            settled = if pending { 0 } else { settled + 1 };
            if settled >= 4 {
                break;
            }
            assert!(Instant::now() < deadline, "visual fixture did not settle");
            std::thread::sleep(Duration::from_millis(4));
        }
    }

    fn clear(self, app: &mut Fixture<'_>, pump: &mut sdl3::EventPump) {
        match self {
            Self::Original => app.terminal.clear().unwrap(),
            Self::Experimental => {
                app.key(Keycode::L, Mod::LCTRLMOD);
            }
        }
        self.settle(app, pump);
    }

    fn command(self, app: &mut Fixture<'_>, pump: &mut sdl3::EventPump, command: &str) {
        match self {
            Self::Original => {
                app.terminal.run_command(command).unwrap();
            }
            Self::Experimental => app.renderer.experimental_send(command).unwrap(),
        }
        self.settle(app, pump);
    }

    fn save(self, app: &mut Fixture<'_>, output: &Path, scene: &str) {
        self.draw(app);
        app.renderer
            .terminal_visual_export(&output.join(format!("{scene}.bmp")))
            .unwrap();
    }

    fn click_prompt(self, app: &mut Fixture<'_>, x: f32) {
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y: 570.0,
            },
            true,
        );
        app.send(
            Event::MouseButtonUp {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y: 570.0,
            },
            true,
        );
    }

    fn save_copy(
        self,
        app: &mut Fixture<'_>,
        pump: &mut sdl3::EventPump,
        output: &Path,
        scene: &str,
    ) {
        app.clipboard
            .set_clipboard_text("POTYI_VISUAL_COPY_SENTINEL")
            .unwrap();
        app.key(Keycode::C, Mod::LCTRLMOD | Mod::LSHIFTMOD);
        self.settle(app, pump);
        let text = app.clipboard.clipboard_text().unwrap();
        assert_ne!(
            text, "POTYI_VISUAL_COPY_SENTINEL",
            "output copy was not published"
        );
        std::fs::write(output.join(format!("{scene}.txt")), text).unwrap();
    }
}

#[cfg(unix)]
#[test]
#[ignore = "Matched frame fixture; use SDL_VIDEODRIVER=dummy in a fresh process"]
fn capture_terminal_visual_frames() {
    let view = match env::var("POTYI_TERM_VISUAL_KIND").as_deref() {
        Ok("old") => View::Original,
        Ok("new") => View::Experimental,
        _ => panic!("set POTYI_TERM_VISUAL_KIND=old/new"),
    };
    let root =
        PathBuf::from(env::var_os("POTYI_TERM_VISUAL_ROOT").expect("shared fixture directory"))
            .canonicalize()
            .unwrap();
    let output =
        PathBuf::from(env::var_os("POTYI_TERM_VISUAL_OUTPUT").expect("frame output directory"));
    std::fs::create_dir_all(&output).unwrap();
    with_fixture(|app| {
        let sdl = sdl3::init().unwrap();
        let mut pump = sdl.event_pump().unwrap();
        match view {
            View::Original => {
                app.terminal = Terminal::new(root.clone()).unwrap();
                app.terminal.set_events(app.event_subsystem.clone());
                app.terminal.open(None);
            }
            View::Experimental => {
                app.renderer
                    .experimental_events(&app.event_subsystem)
                    .unwrap();
                app.renderer.open_experimental(&root, None).unwrap();
            }
        }
        eprintln!(
            "POTYI_VISUAL_SCALAR_WIDTHS {:?}",
            ['a', '́', '\u{200d}', '\u{fe0f}', '界'].map(|character| (
                character,
                app.renderer.terminal_visual_scalar_width(character)
            ))
        );
        view.settle(app, &mut pump);
        view.save(app, &output, "initial");
        view.clear(app, &mut pump);
        view.save(app, &output, "empty");
        view.command(app, &mut pump, "ls");
        view.save(app, &output, "listing");
        app.renderer.window_mut().set_size(420, 600).unwrap();
        app.renderer.update_window_size().unwrap();
        view.settle(app, &mut pump);
        view.save(app, &output, "listing_narrow");
        app.renderer.window_mut().set_size(3200, 600).unwrap();
        app.renderer.update_window_size().unwrap();
        view.settle(app, &mut pump);
        view.clear(app, &mut pump);
        view.command(app, &mut pump, "ls");
        view.save(app, &output, "listing_wide");
        view.clear(app, &mut pump);
        view.command(app, &mut pump, "printf '%0300d\\n' 0");
        view.save(app, &output, "output_wide");

        app.renderer.window_mut().set_size(800, 600).unwrap();
        app.renderer.update_window_size().unwrap();
        view.settle(app, &mut pump);
        app.text("edit \"space name.txt\"");
        view.save(app, &output, "prompt");
        view.click_prompt(app, 110.0);
        view.save(app, &output, "prompt_click");
        app.key(Keycode::End, Mod::NOMOD);
        for _ in 0..256 {
            app.key(Keycode::Backspace, Mod::NOMOD);
        }
        view.clear(app, &mut pump);
        view.command(app, &mut pump, "ls -l");
        view.save(app, &output, "long_listing");
        view.clear(app, &mut pump);
        view.command(app, &mut pump,
            "printf 'ordinary output\\nerror: source.rs:2:3 failure\\nwarning: watch this\\n+added line\\n-removed line\\n'"
        );
        view.save(app, &output, "output");
        app.key(Keycode::Up, Mod::LSHIFTMOD);
        view.settle(app, &mut pump);
        view.save(app, &output, "selection");
        app.key(Keycode::Escape, Mod::NOMOD);
        app.text("a deliberately long command with quoted Unicode \"árvíz 東京.rs\" and enough remaining text to scroll the field safely");
        view.save(app, &output, "long_prompt");
        view.click_prompt(app, 110.0);
        view.save(app, &output, "long_prompt_click");
        app.key(Keycode::End, Mod::NOMOD);
        for _ in 0..256 {
            app.key(Keycode::Backspace, Mod::NOMOD);
        }
        view.clear(app, &mut pump);
        match view {
            View::Original => {
                app.terminal.run_command("sleep 5").unwrap();
            }
            View::Experimental => app.renderer.experimental_send("sleep 5").unwrap(),
        }
        let mut frames = terminal::FrameSchedule::default();
        for _ in 0..10 {
            match view {
                View::Original => {
                    app.terminal.poll_background().unwrap();
                }
                View::Experimental => {
                    app.renderer.poll_experimental();
                }
            }
            for event in pump.poll_iter().take(64) {
                app.send_application(event, &mut frames);
            }
            view.draw(app);
            std::thread::sleep(Duration::from_millis(4));
        }
        view.save(app, &output, "running");
        app.key(Keycode::C, Mod::LCTRLMOD);
        view.settle(app, &mut pump);
        view.save(app, &output, "stopped");
        for (scene, command) in [
            ("ls-copy", "ls"),
            ("ls-long-copy", "ls -l"),
            (
                "plain-copy",
                "printf '\\033[31mred\\033[0m\\nordinary output\\nerror: source.rs:2:3 failure\\nwarning: watch this\\n'",
            ),
            ("raw-copy", "printf '\\ttrail  \\nlong\\rX\\n'"),
            ("unicode-tab-copy", "printf 'á\\tb  \\n東京\\tend\\n'"),
            ("leading-copy", "printf '́leading text\\ń\\n'"),
        ] {
            view.clear(app, &mut pump);
            view.command(app, &mut pump, command);
            view.save(app, &output, scene);
            view.save_copy(app, &mut pump, &output, scene);
        }
        // The last command ends with an accent-only line and an empty EOF row.
        // Exercise the real output navigation and selection paths rather than
        // inferring caret geometry from the unfocused glyph rendering.
        for key in [Keycode::F6, Keycode::Up, Keycode::Home] {
            app.key(key, Mod::NOMOD);
            view.settle(app, &mut pump);
        }
        view.save(app, &output, "accent_home");
        app.key(Keycode::End, Mod::NOMOD);
        view.settle(app, &mut pump);
        view.save(app, &output, "accent_end");
        app.key(Keycode::Home, Mod::NOMOD);
        view.settle(app, &mut pump);
        app.key(Keycode::End, Mod::LSHIFTMOD);
        view.settle(app, &mut pump);
        view.save(app, &output, "accent_selected");
        app.clipboard
            .set_clipboard_text("POTYI_VISUAL_COPY_SENTINEL")
            .unwrap();
        app.key(Keycode::C, Mod::LCTRLMOD);
        view.settle(app, &mut pump);
        let selected = app.clipboard.clipboard_text().unwrap();
        assert_eq!(selected, "́", "the selected accent must copy exactly");
        std::fs::write(output.join("accent-selected-copy.txt"), selected).unwrap();
        app.key(Keycode::Escape, Mod::NOMOD);
        view.settle(app, &mut pump);
        if let Some(git_root) = env::var_os("POTYI_TERM_VISUAL_GIT_ROOT") {
            let git_root = PathBuf::from(git_root).canonicalize().unwrap();
            view.command(app, &mut pump, &format!("cd '{}'", git_root.display()));
            view.clear(app, &mut pump);
            view.command(
                app,
                &mut pump,
                "git -c core.abbrev=7 log --oneline --max-count=2",
            );
            view.save(app, &output, "git_log");
            view.save_copy(app, &mut pump, &output, "git_log");
            // The first hash is the first output row after the command echo.
            // Both integrations handle the actual production mouse activation.
            let y = app.renderer.experimental_row_y(1) as f32;
            for event in [
                Event::MouseButtonDown {
                    timestamp: 0,
                    window_id: 0,
                    which: 0,
                    mouse_btn: MouseButton::Left,
                    clicks: 1,
                    x: 18.0,
                    y,
                },
                Event::MouseButtonUp {
                    timestamp: 0,
                    window_id: 0,
                    which: 0,
                    mouse_btn: MouseButton::Left,
                    clicks: 1,
                    x: 18.0,
                    y,
                },
            ] {
                app.send(event, true);
            }
            view.settle(app, &mut pump);
            assert!(
                match view {
                    View::Original => app.terminal.can_go_back(),
                    View::Experimental => app.renderer.experimental_back_available(),
                },
                "Clicking the actual hash must open Git detail"
            );
            view.save(app, &output, "git_detail");
            view.save_copy(app, &mut pump, &output, "git_detail");
            app.key(Keycode::Left, Mod::LALTMOD);
            view.settle(app, &mut pump);
            view.save(app, &output, "git_back");
            view.save_copy(app, &mut pump, &output, "git_back");
        }
    });
}

#[cfg(unix)]
#[test]
#[ignore = "SDL native filename ink hit regression; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_accent_filename_click_preserves_legacy_hit() {
    let view = match env::var("POTYI_TERM_VISUAL_KIND").as_deref() {
        Ok("old") => View::Original,
        Ok("new") => View::Experimental,
        _ => View::Experimental,
    };
    let supplied = env::var_os("POTYI_TERM_VISUAL_ROOT");
    let temporary = supplied.is_none();
    let root = PathBuf::from(supplied.unwrap_or_else(|| {
        env::temp_dir()
            .join(format!("potyi-accent-hit-{}", std::process::id()))
            .into_os_string()
    }));
    if temporary {
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("́"), "accent ink hit\n").unwrap();
    }
    let root = root.canonicalize().unwrap();
    let output = env::var_os("POTYI_TERM_VISUAL_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("frames"));
    std::fs::create_dir_all(&output).unwrap();
    with_fixture(|app| {
        let sdl = sdl3::init().unwrap();
        let mut pump = sdl.event_pump().unwrap();
        match view {
            View::Original => {
                app.terminal = Terminal::new(root.clone()).unwrap();
                app.terminal.set_events(app.event_subsystem.clone());
                app.terminal.open(None);
            }
            View::Experimental => {
                app.renderer
                    .experimental_events(&app.event_subsystem)
                    .unwrap();
                app.renderer.open_experimental(&root, None).unwrap();
            }
        }
        app.renderer.window_mut().set_size(420, 600).unwrap();
        app.renderer.update_window_size().unwrap();
        view.settle(app, &mut pump);
        view.clear(app, &mut pump);
        view.command(app, &mut pump, "ls -1");
        view.save(app, &output, "accent-filename-before");
        view.save_copy(app, &mut pump, &output, "accent-filename-before");
        let text = app.clipboard.clipboard_text().unwrap();
        let row = text
            .lines()
            .position(|line| line == "́")
            .expect("standalone accent filename visible in ls-1");
        let x = 15.0;
        let y = app.renderer.experimental_row_y(row) as f32 + 2.0;
        for event in [
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
            },
            Event::MouseButtonUp {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
            },
        ] {
            app.send(event, true);
        }
        let actual = app
            .editor
            .path()
            .as_ref()
            .and_then(|path| path.canonicalize().ok());
        eprintln!("POTYI_ACCENT_CLICK row={row} x={x} y={y} actual={actual:?}");
        assert_eq!(
            actual,
            Some(root.join("́")),
            "clicking accent-only filename ink must open its authoritative path"
        );
    });
    if temporary {
        std::fs::remove_dir_all(root).unwrap();
    }
}
