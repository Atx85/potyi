// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Real keyboard events activate the same stored links as pointer events.
use super::*;

fn directory(label: &str) -> std::path::PathBuf {
    let mut random = [0; 8];
    getrandom::getrandom(&mut random).unwrap();
    let root = std::env::temp_dir().join(format!(
        "potyi-keyboard-{label}-{:x}",
        u64::from_le_bytes(random)
    ));
    std::fs::create_dir(&root).unwrap();
    root.canonicalize().unwrap()
}

fn last_name(app: &mut Fixture<'_>) {
    // Move past the empty EOF rows and onto the name's final character.
    // Deliberately do not wait between navigation and Enter in the callers.
    app.key(Keycode::F6, Mod::NOMOD);
    app.key(Keycode::Left, Mod::NOMOD);
    app.key(Keycode::Left, Mod::NOMOD);
    app.key(Keycode::Left, Mod::NOMOD);
}

#[test]
#[ignore = "SDL keyboard file links; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_enter_opens_file_after_queued_navigation() {
    for mode in [KeybindingMode::Conventional, KeybindingMode::Vim] {
        with_fixture(|app| {
            app.mode(mode);
            let root = directory("file");
            let file = root.join("keyboard 界.txt");
            std::fs::write(&file, "opened with Enter\n").unwrap();
            app.renderer
                .experimental_events(&app.event_subsystem)
                .unwrap();
            app.renderer
                .open_experimental(&root, Some("ls -1 'keyboard 界.txt'"))
                .unwrap();
            experimental_wait_idle(app);
            experimental_wait_output(app);
            app.text("unsubmitted draft");
            last_name(app);
            app.key(Keycode::Return, Mod::NOMOD);
            experimental_wait_output(app);
            experimental_open_requests(app);
            assert_eq!(app.editor.path().as_ref(), Some(&file));
            assert_eq!(app.editor.document.text().unwrap(), "opened with Enter\n");
            assert!(!app.renderer.experimental_visible());
            assert_eq!(app.renderer.experimental_prompt(), "unsubmitted draft");
            std::fs::remove_dir_all(root).unwrap();
        });
    }
}

#[test]
#[ignore = "SDL keyboard folder links; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_enter_browses_directory() {
    with_fixture(|app| {
        let root = directory("folder");
        let child = root.join("child");
        std::fs::create_dir(&child).unwrap();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer
            .open_experimental(&root, Some("ls -1"))
            .unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        last_name(app);
        app.key(Keycode::KpEnter, Mod::NOMOD);
        experimental_wait_output(app);
        experimental_open_requests(app);
        experimental_wait_idle(app);
        experimental_wait_output(app);
        assert_eq!(app.renderer.experimental_directory(), Some(child.as_path()));
        assert!(app.renderer.experimental_visible());
        assert!(app.editor.path().is_none());
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[test]
#[ignore = "SDL modified Enter, repeat and Clear; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_enter_other_pane_and_cancelled_activation() {
    for modifiers in [Mod::LGUIMOD, Mod::LCTRLMOD] {
        with_fixture(|app| {
            let root = directory("other");
            let file = root.join("target.txt");
            std::fs::write(&file, "other pane\n").unwrap();
            app.split_mode = true;
            app.renderer.set_split_mode(true);
            app.renderer
                .experimental_events(&app.event_subsystem)
                .unwrap();
            app.renderer
                .open_experimental(&root, Some("ls -1 target.txt"))
                .unwrap();
            experimental_wait_idle(app);
            experimental_wait_output(app);
            last_name(app);
            app.key_repeat(Keycode::Return, modifiers, true);
            experimental_wait_output(app);
            experimental_open_requests(app);
            assert!(app.editor.path().is_none() && app.other_editor.path().is_none());
            app.key(Keycode::KpEnter, modifiers);
            experimental_wait_output(app);
            experimental_open_requests(app);
            assert_eq!(app.editor.path().as_ref(), Some(&file));
            assert_eq!(app.active_pane, 1);
            assert!(app.renderer.experimental_visible());
            app.key(Keycode::Grave, Mod::LCTRLMOD);
            app.renderer.experimental_send("ls -1 target.txt").unwrap();
            experimental_wait_idle(app);
            experimental_wait_output(app);
            last_name(app);
            app.key(Keycode::Return, Mod::NOMOD);
            app.key(Keycode::L, Mod::LCTRLMOD);
            experimental_wait_output(app);
            experimental_open_requests(app);
            assert!(app.renderer.experimental_visible());
            assert!(app.other_editor.path().is_none());
            std::fs::remove_dir_all(root).unwrap();
        });
    }
}

#[cfg(unix)]
#[test]
#[ignore = "SDL keyboard wrapped diagnostics; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_enter_opens_wrapped_diagnostic_location() {
    with_fixture(|app| {
        let root = directory("diagnostic");
        let name = "a long source filename with Unicode 界 and spaces that crosses terminal rows and retains its original path.rs";
        let file = root.join(name);
        std::fs::write(&file, "first\nsecond\nthird\n").unwrap();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer.open_experimental(&root, None).unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        let line = format!("error: \"{name}:2:3\" failed");
        app.renderer
            .experimental_send(&format!("printf '{line}\\n'"))
            .unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        app.key(Keycode::F6, Mod::NOMOD);
        experimental_wait_output(app);
        let suffix = line[line.find("retains").unwrap()..].chars().count();
        for index in 0..=suffix {
            app.key(Keycode::Left, Mod::NOMOD);
            if index % 16 == 15 {
                experimental_wait_output(app);
            }
        }
        app.key(Keycode::Return, Mod::NOMOD);
        experimental_wait_output(app);
        experimental_open_requests(app);
        assert_eq!(app.editor.path().as_ref(), Some(&file));
        assert_eq!(app.editor.document.cursor_line_column().unwrap().0, 1);
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[test]
#[ignore = "SDL keyboard commit activation; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_enter_opens_git_commit_and_back_restores_log() {
    with_fixture(|app| {
        let (root, hash) = experimental_git_fixture();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer
            .open_experimental(&root, Some("git log -1 --oneline"))
            .unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        let before = app.renderer.experimental_contents();
        app.key(Keycode::F6, Mod::NOMOD);
        app.key(Keycode::Left, Mod::NOMOD);
        app.key(Keycode::Home, Mod::NOMOD);
        app.key(Keycode::Return, Mod::NOMOD);
        experimental_wait_output(app);
        experimental_wait_idle(app);
        assert!(app.renderer.experimental_back_available());
        assert!(app.renderer.experimental_contents().contains(&hash[..7]));
        app.key(Keycode::Left, Mod::LALTMOD);
        experimental_wait_output(app);
        assert!(!app.renderer.experimental_back_available());
        assert_eq!(app.renderer.experimental_contents(), before);
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[cfg(unix)]
#[test]
#[ignore = "SDL unlinked Enter fallback preserves newer navigation; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_enter_on_plain_output_returns_to_prompt_without_losing_newer_keys() {
    with_fixture(|app| {
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.execute_command(":term-new", false);
        experimental_wait_idle(app);
        app.renderer
            .experimental_send("printf 'plain output without a link\\n'")
            .unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        app.text("draft");
        last_name(app);
        app.key(Keycode::Return, Mod::NOMOD);
        experimental_wait_output(app);
        assert!(app.renderer.experimental_output_cursor().is_none());
        assert_eq!(app.renderer.experimental_prompt(), "draft");
        last_name(app);
        app.key(Keycode::Return, Mod::NOMOD);
        app.key(Keycode::Home, Mod::NOMOD);
        experimental_wait_output(app);
        assert!(app.renderer.experimental_output_cursor().is_some());
        assert_eq!(app.renderer.experimental_prompt(), "draft");
    });
}
