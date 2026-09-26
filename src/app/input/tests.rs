// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Dispatch real SDL event values through the same handlers as the application.
//! These headless tests check routing, not OS delivery or physical clipboard UX.
use super::*;

struct Fixture<'font> {
    pane_keys: PaneKeys,
    mouse_state: MouseState,
    editor: Editor,
    other_editor: Editor,
    renderer: Renderer<'font>,
    terminal: Terminal,
    vim: VimController,
    other_vim: VimController,
    emacs: emacs::Controller,
    lsp_ui: lsp_ui::LspUi,
    search_ui: SearchUi,
    command_bar: CommandBar,
    key_bindings: KeyBindings,
    keyboard: sdl3::keyboard::KeyboardUtil,
    clipboard: sdl3::clipboard::ClipboardUtil,
    event_subsystem: sdl3::EventSubsystem,
    dirty: bool,
    split_mode: bool,
    active_pane: usize,
    vim_enabled: bool,
}

impl Fixture<'_> {
    fn send(&mut self, event: Event, coordinates: bool) -> EventFlow {
        dispatch(
            InputContext {
                pane_keys: &mut self.pane_keys,
                mouse_state: &mut self.mouse_state,
                editor: &mut self.editor,
                other_editor: &mut self.other_editor,
                renderer: &mut self.renderer,
                terminal: &mut self.terminal,
                vim: &mut self.vim,
                other_vim: &mut self.other_vim,
                emacs: &mut self.emacs,
                lsp_ui: &mut self.lsp_ui,
                search_ui: &mut self.search_ui,
                command_bar: &mut self.command_bar,
                key_bindings: &self.key_bindings,
                keyboard: &self.keyboard,
                clipboard: &self.clipboard,
                event_subsystem: &self.event_subsystem,
                dirty: &mut self.dirty,
                split_mode: &mut self.split_mode,
                active_pane: &mut self.active_pane,
                vim_enabled: &mut self.vim_enabled,
            },
            event,
            coordinates,
        )
        .unwrap()
    }

    fn key(&mut self, key: Keycode, keymod: Mod) -> EventFlow {
        self.key_repeat(key, keymod, false)
    }

    fn key_repeat(&mut self, key: Keycode, keymod: Mod, repeat: bool) -> EventFlow {
        self.send(
            Event::KeyDown {
                timestamp: 0,
                window_id: 0,
                keycode: Some(key),
                scancode: None,
                keymod,
                repeat,
                which: 0,
                raw: 0,
            },
            true,
        )
    }

    fn text(&mut self, text: &str) {
        assert_eq!(
            self.send(
                Event::TextInput {
                    timestamp: 0,
                    window_id: 0,
                    text: text.into(),
                },
                true
            ),
            EventFlow::Continue
        );
    }

    fn mode(&mut self, mode: KeybindingMode) {
        self.editor.config.keybinding_mode = mode;
        self.other_editor.config.keybinding_mode = mode;
        apply_keybinding_mode(
            mode,
            &mut self.vim_enabled,
            &mut self.editor,
            &mut self.other_editor,
            &mut self.vim,
            &mut self.other_vim,
            &mut self.renderer,
        );
    }
}

fn with_fixture(test: impl FnOnce(&mut Fixture<'_>)) {
    let sdl = sdl3::init().unwrap();
    let video = sdl.video().unwrap();
    let window = video
        .window("Input routing test", 800, 600)
        .hidden()
        .build()
        .unwrap();
    let ttf = sdl3::ttf::init().unwrap();
    let font = || {
        ttf.load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(FONT_DATA).unwrap(),
            18.0,
        )
        .unwrap()
    };
    let canvas = window.into_canvas();
    let texture_creator = canvas.texture_creator();
    let renderer = Renderer::new(
        canvas,
        &texture_creator,
        font(),
        font(),
        18.0,
        (font(), font()),
        window::WindowHitTestState::new(800, 1.0),
    )
    .unwrap();
    let events = sdl.event().unwrap();
    terminal::register_test_events(&events);
    let config = EditorConfig {
        keybinding_mode: KeybindingMode::Conventional,
        ..EditorConfig::default()
    };
    let mut fixture = Fixture {
        pane_keys: PaneKeys::default(),
        mouse_state: MouseState::default(),
        editor: Editor::new(config.clone()).unwrap(),
        other_editor: Editor::new(config).unwrap(),
        renderer,
        terminal: Terminal::new(std::env::temp_dir()).unwrap(),
        vim: VimController::new(),
        other_vim: VimController::new(),
        emacs: emacs::Controller::default(),
        lsp_ui: lsp_ui::LspUi::new(events.clone()),
        search_ui: SearchUi::new(),
        command_bar: CommandBar::new(),
        key_bindings: KeyBindings::default(),
        keyboard: sdl.keyboard(),
        clipboard: video.clipboard(),
        event_subsystem: events,
        dirty: false,
        split_mode: false,
        active_pane: 0,
        vim_enabled: false,
    };
    test(&mut fixture);
}

#[test]
#[ignore = "Headless SDL event routing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn conventional_events_keep_command_and_terminal_input_out_of_the_document() {
    with_fixture(|app| {
        app.text("hello");
        app.key(Keycode::Z, Mod::LCTRLMOD);
        assert_eq!(app.editor.document.text().unwrap(), "");
        app.key(Keycode::Y, Mod::LCTRLMOD);
        assert_eq!(app.editor.document.text().unwrap(), "hello");

        app.command_bar.open(":find ");
        app.text("hello");
        assert_eq!(app.command_bar.input(), ":find hello");
        assert_eq!(app.editor.document.text().unwrap(), "hello");
        app.terminal.open(None);
        app.text("ls");
        app.key(Keycode::Backspace, Mod::NOMOD);
        assert_eq!(app.terminal.input(), "l");
        assert_eq!(app.command_bar.input(), ":find hello");
        assert_eq!(app.editor.document.text().unwrap(), "hello");

        app.terminal.close_to_editor();
        app.command_bar.open(":quit");
        assert_eq!(app.key(Keycode::Return, Mod::NOMOD), EventFlow::Quit);
        assert_eq!(
            app.send(Event::Quit { timestamp: 0 }, true),
            EventFlow::Quit
        );
    });
}

#[test]
#[ignore = "Headless SDL event routing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn keyboard_priorities_keep_terminal_toggle_and_search_modes_consistent() {
    with_fixture(|app| {
        app.text("alpha\nbeta\ngamma\n");
        app.command_bar.open(":find ");
        app.key(Keycode::Grave, Mod::LCTRLMOD);
        assert!(app.terminal.is_active());
        assert!(!app.command_bar.is_active());
        app.key_repeat(Keycode::Grave, Mod::LCTRLMOD, true);
        assert!(
            app.terminal.is_active(),
            "holding the toggle must not close the terminal"
        );
        app.key(Keycode::F, Mod::LCTRLMOD);
        assert!(
            !app.command_bar.is_active(),
            "terminal keys must not open editor search"
        );
        app.key(Keycode::Grave, Mod::LCTRLMOD);
        assert!(!app.terminal.is_active());

        app.key(Keycode::F, Mod::LCTRLMOD);
        assert!(app.command_bar.is_active());
        assert_eq!(app.command_bar.input(), ":find ");
        app.key(Keycode::Escape, Mod::NOMOD);
        assert!(!app.command_bar.is_active());

        app.mode(KeybindingMode::Vim);
        app.editor.document.move_cursor(0).unwrap();
        app.key(Keycode::F, Mod::LCTRLMOD);
        assert!(
            !app.command_bar.is_active(),
            "Vim's page motion has priority over Ctrl+F search"
        );

        app.mode(KeybindingMode::Emacs);
        app.editor.document.move_cursor(0).unwrap();
        app.key(Keycode::F, Mod::LCTRLMOD);
        assert_eq!(app.editor.document.cursor.position, 1);
        assert!(!app.command_bar.is_active());
        assert_eq!(app.editor.document.text().unwrap(), "alpha\nbeta\ngamma\n");
    });
}

#[test]
#[ignore = "Headless SDL event routing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn vim_events_preserve_insert_suppression_and_undo_groups() {
    with_fixture(|app| {
        app.mode(KeybindingMode::Vim);
        app.text("ignored in normal mode");
        assert_eq!(app.editor.document.len(), 0);
        app.key(Keycode::I, Mod::NOMOD);
        app.text("i"); // SDL text event for the key that entered Insert mode.
        assert_eq!(app.editor.document.len(), 0);
        app.key(Keycode::A, Mod::NOMOD);
        app.text("abc");
        app.key(Keycode::Escape, Mod::NOMOD);
        assert_eq!(app.editor.document.text().unwrap(), "abc");
        assert_eq!(app.vim.mode(), vim::VimMode::Normal);
        app.key(Keycode::U, Mod::NOMOD);
        app.text("u");
        assert_eq!(app.editor.document.len(), 0);
        app.key(Keycode::R, Mod::LCTRLMOD);
        assert_eq!(app.editor.document.text().unwrap(), "abc");
    });
}

#[test]
#[ignore = "Headless SDL text-object routing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn vim_text_objects_handle_shift_text_events_and_undo() {
    with_fixture(|app| {
        app.editor.insert("before {old} after").unwrap();
        app.editor.undo_stack.clear();
        app.editor.document.move_cursor(9).unwrap();
        app.mode(KeybindingMode::Vim);
        app.key(Keycode::C, Mod::NOMOD);
        app.text("c");
        app.key(Keycode::I, Mod::NOMOD);
        app.text("i");
        app.key(Keycode::LShift, Mod::LSHIFTMOD);
        app.key(Keycode::LeftBracket, Mod::LSHIFTMOD);
        app.text("{");
        assert_eq!(app.vim.mode(), vim::VimMode::Insert);
        assert_eq!(app.editor.document.text().unwrap(), "before {} after");
        app.key(Keycode::N, Mod::NOMOD);
        app.text("new");
        app.key(Keycode::Escape, Mod::NOMOD);
        assert_eq!(app.editor.document.text().unwrap(), "before {new} after");
        assert_eq!(app.editor.undo_stack.len(), 1);
        app.key(Keycode::U, Mod::NOMOD);
        app.text("u");
        assert_eq!(app.editor.document.text().unwrap(), "before {old} after");
        app.key(Keycode::R, Mod::LCTRLMOD);
        assert_eq!(app.editor.document.text().unwrap(), "before {new} after");
        app.editor.document.move_cursor(9).unwrap();
        app.key(Keycode::V, Mod::NOMOD);
        app.text("v");
        app.key(Keycode::I, Mod::NOMOD);
        app.text("i");
        app.key(Keycode::W, Mod::NOMOD);
        app.text("w");
        app.key(Keycode::Y, Mod::NOMOD);
        app.text("y");
        assert_eq!(app.clipboard.clipboard_text().unwrap(), "new");
        assert_eq!(app.editor.document.text().unwrap(), "before {new} after");
    });
}

#[test]
#[ignore = "Headless SDL event routing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn emacs_events_preserve_prefixes_text_suppression_and_cancellation() {
    with_fixture(|app| {
        app.mode(KeybindingMode::Emacs);
        app.text("hello");
        app.key(Keycode::A, Mod::LCTRLMOD);
        assert_eq!(app.editor.document.cursor.position, 0);
        app.key(Keycode::X, Mod::LCTRLMOD);
        app.key(Keycode::F, Mod::LCTRLMOD);
        assert_eq!(app.command_bar.input(), ":open ");
        app.text("f"); // Suppress the text paired with the prefix command.
        assert_eq!(app.command_bar.input(), ":open ");
        app.key(Keycode::A, Mod::NOMOD);
        app.text("another.txt");
        assert_eq!(app.command_bar.input(), ":open another.txt");
        app.key(Keycode::G, Mod::LCTRLMOD);
        assert!(!app.command_bar.is_active());
        assert_eq!(app.editor.document.text().unwrap(), "hello");
    });
}

#[test]
#[ignore = "Headless SDL event routing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn pane_keyboard_vim_switches_direction_and_preserves_insert_text() {
    with_fixture(|app| {
        app.editor.insert_text("left").unwrap();
        app.other_editor.insert_text("right").unwrap();
        app.split_mode = true;
        app.renderer.set_split_mode(true);
        app.mode(KeybindingMode::Vim);
        app.key(Keycode::W, Mod::LCTRLMOD);
        app.key_repeat(Keycode::W, Mod::LCTRLMOD, true);
        app.key(Keycode::L, Mod::NOMOD);
        app.text("l");
        assert_eq!(app.active_pane, 1);
        assert_eq!(app.editor.document.text().unwrap(), "right");
        app.key(Keycode::W, Mod::LCTRLMOD);
        app.key(Keycode::L, Mod::LCTRLMOD);
        assert_eq!(app.active_pane, 1, "right at the edge stays in place");
        app.key(Keycode::I, Mod::NOMOD);
        app.text("i");
        assert_eq!(app.vim.mode(), vim::VimMode::Insert);
        // Switch away with the same pointer path users can use while inserting.
        focus_pane(0, &mut app.active_pane, &mut app.editor, &mut app.other_editor,
            &mut app.vim, &mut app.other_vim, &mut app.renderer);
        app.key(Keycode::W, Mod::LCTRLMOD);
        app.key(Keycode::W, Mod::NOMOD);
        app.text("w");
        app.key_repeat(Keycode::W, Mod::NOMOD, true);
        app.text("w");
        assert_eq!(app.active_pane, 1);
        assert_eq!(app.editor.document.text().unwrap(), "right", "shortcut text cannot leak into Insert mode");
        app.key(Keycode::A, Mod::NOMOD);
        app.text("a");
        assert!(app.editor.document.text().unwrap().contains('a'), "the next ordinary key must not be swallowed");
        app.key(Keycode::Escape, Mod::NOMOD);
        app.key(Keycode::W, Mod::LCTRLMOD);
        app.key(Keycode::LCtrl, Mod::LCTRLMOD);
        app.key(Keycode::H, Mod::LCTRLMOD);
        assert_eq!(app.active_pane, 0);
        assert_eq!(app.editor.document.text().unwrap(), "left");
        app.key(Keycode::W, Mod::LCTRLMOD);
        app.key(Keycode::Escape, Mod::NOMOD);
        app.key(Keycode::L, Mod::NOMOD);
        assert_eq!(app.active_pane, 0, "Escape cancels the prefix");
        app.split_mode = false;
        app.renderer.set_split_mode(false);
        app.key(Keycode::W, Mod::LCTRLMOD);
        app.key(Keycode::W, Mod::LCTRLMOD);
        assert_eq!(app.active_pane, 0, "single-pane mode must not expose a hidden document");
    });
}

#[test]
#[ignore = "Headless SDL event routing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn pane_keyboard_switches_between_terminal_and_editor_in_vim_and_emacs() {
    with_fixture(|app| {
        app.split_mode = true;
        app.renderer.set_split_mode(true);
        app.editor.insert_text("left").unwrap();
        app.other_editor.insert_text("right").unwrap();
        app.renderer.place_terminal_in_active_pane();
        app.terminal.open(None);
        app.terminal.insert_text("draft command");
        for (mode, prefix, second, text) in [
            (KeybindingMode::Vim, Keycode::W, Keycode::W, "w"),
            (KeybindingMode::Emacs, Keycode::X, Keycode::O, "o"),
        ] {
            app.mode(mode);
            for target in [1, 0, 1, 0] {
                app.key(prefix, Mod::LCTRLMOD);
                app.key(second, Mod::NOMOD);
                app.text(text);
                assert_eq!(app.active_pane, target);
                assert_eq!(app.renderer.terminal_focused(&app.terminal), target == 0);
                assert_eq!(app.terminal.input(), "draft command");
                assert_eq!(app.editor.document.text().unwrap(), if target == 0 { "left" } else { "right" });
            }
            app.key(prefix, Mod::LCTRLMOD);
            app.key(Keycode::Escape, Mod::NOMOD);
            assert!(app.renderer.terminal_focused(&app.terminal), "prefix cancellation must not close the terminal");
        }
        app.terminal.close_to_editor();
        app.key(Keycode::X, Mod::LCTRLMOD);
        app.key(Keycode::O, Mod::NOMOD);
        app.text("o");
        assert_eq!(app.active_pane, 1, "existing Emacs editor-to-editor navigation still works");
        assert_eq!(app.editor.document.text().unwrap(), "right");
    });
}

#[test]
#[ignore = "Headless SDL event routing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn mouse_and_drop_events_preserve_panes_and_report_open_failures() {
    with_fixture(|app| {
        app.editor.insert_text("left unsaved").unwrap();
        app.other_editor.insert_text("right unsaved").unwrap();
        app.other_editor.path = Some(std::env::temp_dir().join("potyi-input-routing-other.txt"));
        app.split_mode = true;
        app.renderer.set_split_mode(true);
        app.renderer
            .render_split(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
        let click = |x, y| Event::MouseButtonDown {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x,
            y,
        };
        app.send(click(600.0, 150.0), false);
        assert_eq!(app.active_pane, 0);
        app.send(click(600.0, 150.0), true);
        assert_eq!(app.active_pane, 1);
        assert_eq!(app.editor.document.text().unwrap(), "right unsaved");
        app.send(click(100.0, 150.0), true);
        assert_eq!(app.active_pane, 0);
        let filename = app
            .other_editor
            .path
            .as_ref()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        app.send(
            Event::DropFile {
                timestamp: 0,
                window_id: 0,
                filename,
            },
            true,
        );
        assert_eq!(app.active_pane, 1);
        assert_eq!(app.other_editor.document.text().unwrap(), "left unsaved");
        assert!(app.other_editor.dirty);

        let missing = std::env::temp_dir().join(format!(
            "potyi-input-missing-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        app.send(
            Event::DropFile {
                timestamp: 0,
                window_id: 0,
                filename: missing.to_string_lossy().into_owned(),
            },
            true,
        );
        assert!(app.command_bar.is_info());
        assert_eq!(app.editor.document.text().unwrap(), "right unsaved");
        assert!(app.editor.dirty);
        assert!(matches!(
            app.renderer.window_control_at(780, 15),
            WindowControl::Close
        ));
        assert_eq!(app.send(click(780.0, 15.0), true), EventFlow::Quit);
    });
}

#[test]
#[ignore = "Headless SDL event routing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn split_terminal_keeps_focus_input_and_documents_in_their_panes() {
    with_fixture(|app| {
        let click = |x, y| Event::MouseButtonDown {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x,
            y,
        };
        let render = |app: &mut Fixture<'_>| {
            app.renderer
                .render_split_terminal(
                    &mut app.editor.document,
                    &mut app.other_editor.document,
                    &mut app.terminal,
                    &app.search_ui,
                    &app.command_bar,
                )
                .unwrap();
        };
        app.text("left unsaved");
        app.other_editor.insert_text("right unsaved").unwrap();
        app.command_bar.open(":split");
        app.key(Keycode::Return, Mod::NOMOD);
        assert!(app.split_mode);
        for pane in [0, 1] {
            let x = pane as f32 * 400.0 + 100.0;
            let other_x = (1 - pane) as f32 * 400.0 + 100.0;
            app.send(click(x, 100.0), true);
            app.command_bar.open(":term");
            app.key(Keycode::Return, Mod::NOMOD);
            assert_eq!(app.renderer.terminal_pane(), pane);
            assert!(app.renderer.terminal_focused(&app.terminal));
            render(app);
            app.text("echo hello");
            let input = app.terminal.input().to_owned();
            let hidden_document = app.editor.document.text().unwrap();
            app.send(click(other_x, 100.0), true);
            assert_eq!(app.active_pane, 1 - pane);
            assert!(!app.renderer.terminal_focused(&app.terminal));
            assert!(app.renderer.terminal_visible(&app.terminal));
            app.text("!");
            let wheel = |mouse_x| Event::MouseWheel {
                timestamp: 0,
                window_id: 0,
                which: 0,
                x: 0.0,
                y: 1.0,
                direction: sdl3::mouse::MouseWheelDirection::Normal,
                mouse_x,
                mouse_y: 100.0,
                integer_x: 0,
                integer_y: 1,
            };
            app.terminal.set_scroll_back(0);
            app.send(wheel(x), true);
            assert_eq!(app.terminal.scroll_back(), 3);
            assert_eq!(
                app.active_pane,
                1 - pane,
                "scrolling terminal output keeps editor focus"
            );
            app.send(wheel(other_x), true);
            assert_eq!(
                app.terminal.scroll_back(),
                3,
                "document scrolling must not scroll the terminal"
            );
            assert_eq!(app.terminal.input(), input);
            assert_eq!(app.other_editor.document.text().unwrap(), hidden_document);
            app.command_bar.open(":find unsaved");
            render(app);
            app.send(click(x, 100.0), true);
            assert_eq!(app.active_pane, pane);
            assert!(!app.command_bar.is_active());
            app.terminal.focus_prompt();
            app.key(Keycode::Escape, Mod::NOMOD);
            assert!(!app.terminal.is_active());
            assert_eq!(app.editor.document.text().unwrap(), hidden_document);
        }
        // The shortcut can move the same terminal, preserving its session.
        app.key(Keycode::Grave, Mod::LCTRLMOD);
        render(app);
        let input = app.terminal.input().to_owned();
        app.send(click(100.0, 100.0), true);
        app.key(Keycode::Grave, Mod::LCTRLMOD);
        assert_eq!(app.renderer.terminal_pane(), 0);
        assert_eq!(app.terminal.input(), input);
        render(app);
        app.send(click(500.0, 100.0), true);
        app.command_bar.open(":split");
        app.key(Keycode::Return, Mod::NOMOD);
        assert!(!app.split_mode);
        assert!(!app.renderer.terminal_visible(&app.terminal));
        app.text("still editing");
        assert_eq!(app.terminal.input(), input);
        app.command_bar.open(":split");
        app.key(Keycode::Return, Mod::NOMOD);
        assert!(app.renderer.terminal_visible(&app.terminal));
    });
}

struct FolderFixture(std::path::PathBuf);

impl FolderFixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "potyi-folder-split-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(root.join("nested folder")).unwrap();
        std::fs::write(root.join("sample.txt"), "project file\n").unwrap();
        Self(root.canonicalize().unwrap())
    }
}

impl Drop for FolderFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn finish_folder_listing(app: &mut Fixture<'_>) {
    let start = std::time::Instant::now();
    while app.terminal.is_running() {
        app.terminal.poll_background().unwrap();
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
#[ignore = "Headless SDL folder workspace; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn folder_launch_file_clicks_choose_the_requested_pane() {
    let folder = FolderFixture::new();
    with_fixture(|app| {
        let launch = startup::LaunchTarget::resolve(Some("."), &folder.0).unwrap();
        open_folder_workspace(
            launch.workspace_root().unwrap(),
            &mut app.split_mode,
            &mut app.active_pane,
            &mut app.editor,
            &mut app.other_editor,
            &mut app.vim,
            &mut app.other_vim,
            &mut app.renderer,
            &mut app.terminal,
        )
        .unwrap();
        finish_folder_listing(app);
        assert!(app.split_mode);
        assert_eq!(app.active_pane, 1);
        assert_eq!(app.renderer.terminal_pane(), 0);
        assert!(app.renderer.terminal_visible(&app.terminal));
        assert!(!app.renderer.terminal_focused(&app.terminal));
        assert!(app.editor.document.is_empty());
        assert!(app.editor.path.is_none());
        let output = app.terminal.output_text().unwrap();
        assert!(output.contains("sample.txt"));
        assert!(output.contains("nested folder/"));
        assert!(!output.contains("Pötyi terminal"));
        app.renderer
            .render_split_terminal(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &mut app.terminal,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();

        let keyboard = sdl3::init().unwrap().keyboard();
        for modifiers in [Mod::NOMOD, Mod::LCTRLMOD, Mod::RCTRLMOD, Mod::LGUIMOD, Mod::RGUIMOD] {
            app.editor = Editor::new(app.editor.config.clone()).unwrap();
            app.other_editor = Editor::new(app.other_editor.config.clone()).unwrap();
            focus_pane(0, &mut app.active_pane, &mut app.editor, &mut app.other_editor,
                &mut app.vim, &mut app.other_vim, &mut app.renderer);
            app.renderer.place_terminal_in_active_pane();
            app.terminal.open(None);
            focus_pane(1, &mut app.active_pane, &mut app.editor, &mut app.other_editor,
                &mut app.vim, &mut app.other_vim, &mut app.renderer);
            app.renderer.render_split_terminal(
                &mut app.editor.document, &mut app.other_editor.document,
                &mut app.terminal, &app.search_ui, &app.command_bar,
            ).unwrap();
            // Find the file's rendered link, so these exercise the actual mouse
            // hit testing, focus changes and release handling together.
            let mut target = None;
            'find_link: for y in (44..450).step_by(8) {
                for x in (12..390).step_by(8) {
                    if let Some(TerminalAction::ListedFile(path)) = app.renderer
                        .terminal_action_at(&mut app.terminal, x, y).unwrap()
                        && path == folder.0.join("sample.txt")
                    {
                        target = Some((x as f32, y as f32));
                        break 'find_link;
                    }
                }
            }
            let (x, y) = target.expect("the folder listing must contain a clickable file");
            keyboard.set_mod_state(modifiers);
            app.send(Event::MouseButtonDown {
                timestamp: 0, window_id: 0, which: 0,
                mouse_btn: MouseButton::Left, clicks: 1, x, y,
            }, true);
            // The modifier at press time determines the destination, even
            // if it is released before the mouse button.
            keyboard.set_mod_state(Mod::NOMOD);
            app.send(Event::MouseButtonUp {
                timestamp: 0, window_id: 0, which: 0,
                mouse_btn: MouseButton::Left, clicks: 1, x, y,
            }, true);
            keyboard.set_mod_state(Mod::NOMOD);
            assert_eq!(app.active_pane, usize::from(modifiers != Mod::NOMOD),
                "click modifiers: {modifiers:?}");
            assert_eq!(app.editor.document.text().unwrap(), "project file\n");
            assert_eq!(app.renderer.terminal_visible(&app.terminal), modifiers != Mod::NOMOD);
        }
        assert_eq!(app.editor.document.text().unwrap(), "project file\n");
        assert_eq!(
            app.editor.path.as_deref(),
            Some(folder.0.join("sample.txt").as_path())
        );
        assert!(app.renderer.terminal_visible(&app.terminal));
        assert_eq!(app.terminal.output_text().unwrap(), output);
        app.text("edited ");
        assert!(app.editor.dirty);
        assert!(app.other_editor.document.is_empty());
    });
}

#[test]
#[ignore = "Headless SDL folder workspace; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn folder_drop_preserves_unsaved_panes_and_refuses_to_interrupt_terminal_work() {
    let folder = FolderFixture::new();
    let next = FolderFixture::new();
    with_fixture(|app| {
        let drop_folder = |path: &std::path::Path| Event::DropFile {
            timestamp: 0,
            window_id: 0,
            filename: path.to_string_lossy().into_owned(),
        };
        let cwd = std::env::current_dir().unwrap();
        app.editor.insert_text("left unsaved").unwrap();
        app.other_editor
            .open(folder.0.join("sample.txt").to_str().unwrap())
            .unwrap();
        // The right pane is clean, so a folder drop starts a fresh empty editor.
        app.send(drop_folder(&folder.0), true);
        assert_eq!(app.active_pane, 1);
        assert!(app.editor.document.is_empty());
        assert!(app.editor.path.is_none());
        assert_eq!(app.other_editor.document.text().unwrap(), "left unsaved");
        assert!(app.other_editor.dirty);
        assert!(app.terminal.is_running());
        // Listing work is deliberately not polled yet, making this busy case deterministic.
        app.send(drop_folder(&next.0), true);
        assert!(app.command_bar.is_info());
        finish_folder_listing(app);
        assert_eq!(
            app.terminal.resolve_path("sample.txt").unwrap(),
            folder.0.join("sample.txt")
        );
        app.command_bar.close();
        app.text("right unsaved");
        let history = app.editor.undo_stack.len();
        for pane in [0, 1] {
            focus_pane(
                pane,
                &mut app.active_pane,
                &mut app.editor,
                &mut app.other_editor,
                &mut app.vim,
                &mut app.other_vim,
                &mut app.renderer,
            );
            app.send(drop_folder(&next.0), true);
            finish_folder_listing(app);
            assert_eq!(app.active_pane, 1);
            assert_eq!(app.editor.document.text().unwrap(), "right unsaved");
            assert_eq!(app.editor.undo_stack.len(), history);
            assert_eq!(app.other_editor.document.text().unwrap(), "left unsaved");
            assert!(app.editor.dirty && app.other_editor.dirty);
            assert_eq!(
                app.terminal.resolve_path("sample.txt").unwrap(),
                next.0.join("sample.txt")
            );
            assert_eq!(std::env::current_dir().unwrap(), cwd);
        }
    });
}

#[cfg(unix)]
#[test]
#[ignore = "Headless SDL folder workspace; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn folder_drop_handles_spaces_trailing_slashes_and_symlinks() {
    let folder = FolderFixture::new();
    let nested = folder.0.join("nested folder");
    std::fs::write(nested.join("child.txt"), "nested file\n").unwrap();
    let link = folder.0.join("linked folder");
    std::os::unix::fs::symlink(&nested, &link).unwrap();
    with_fixture(|app| {
        for path in [nested.clone(), link] {
            app.send(Event::DropFile {
                timestamp: 0,
                window_id: 0,
                filename: format!("{}/", path.display()),
            }, true);
            finish_folder_listing(app);
            assert!(app.split_mode);
            assert_eq!(app.active_pane, 1);
            assert!(app.editor.path.is_none());
            assert!(app.editor.document.is_empty());
            assert!(!app.command_bar.is_info());
            assert_eq!(app.terminal.resolve_path("child.txt").unwrap(), nested.join("child.txt"));
            assert!(app.terminal.output_text().unwrap().contains("child.txt"));
            // Rendering forces document reads, where a directory mistakenly
            // opened as a Unix file would otherwise fail with EISDIR.
            render_terminal_fixture(app);
        }
    });
}

#[test]
#[ignore = "Headless SDL pane editing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn editable_panes_keep_independent_scrolling_and_edit_beyond_the_initial_view() {
    with_fixture(|app| {
        let text = format!("{}\n", "long code ".repeat(100)).repeat(100);
        for editor in [&mut app.editor, &mut app.other_editor] {
            editor.insert_text(&text).unwrap();
            editor.document.move_cursor_to_line_column(0, 0).unwrap();
        }
        app.split_mode = true;
        app.renderer.set_split_mode(true);
        app.renderer.update_cursor(&app.editor.document);
        app.renderer.render_split(&mut app.editor.document, &mut app.other_editor.document,
            &app.search_ui, &app.command_bar).unwrap();
        let wheel = |pane, x, y| Event::MouseWheel {
            timestamp: 0, window_id: 0, which: 0, x, y,
            direction: sdl3::mouse::MouseWheelDirection::Normal,
            mouse_x: pane as f32 * 400.0 + 200.0, mouse_y: 100.0,
            integer_x: x as i32, integer_y: y as i32,
        };
        let target = |app: &mut Fixture<'_>, pane| {
            app.renderer.cursor_target_at(&mut app.editor.document, pane * 400 + 200, 100)
                .unwrap().unwrap()
        };
        let initial = target(app, 0);
        app.send(wheel(0, -3.0, -10.0), true);
        let left = target(app, 0);
        assert!(left.0 > initial.0 && left.1 > initial.1);
        app.send(wheel(1, -5.0, -20.0), true);
        let right = target(app, 1);
        assert!(right.0 > left.0 && right.1 > left.1);
        app.send(wheel(0, 0.0, -1.0), true);
        assert_eq!(target(app, 0), (left.0 + 1, left.1));
        app.send(wheel(1, 0.0, -1.0), true);
        assert_eq!(target(app, 1), (right.0 + 1, right.1));

        for pane in [0, 1] {
            app.send(wheel(pane, 0.0, 0.0), true);
            let other = app.other_editor.document.text().unwrap();
            app.key(Keycode::End, Mod::NOMOD);
            assert_eq!(app.editor.document.cursor.column, 1000);
            app.text("TAIL");
            assert!(app.editor.document.line_text(0).unwrap().ends_with("TAIL"));
            assert!(target(app, pane).1 > 950, "typing must reveal the end of long lines");
            app.key(Keycode::Home, Mod::NOMOD);
            assert_eq!(app.editor.document.cursor.column, 0);
            app.key(Keycode::PageDown, Mod::NOMOD);
            let line = app.editor.document.cursor.line;
            assert!(line > 0);
            app.text("EDIT");
            assert!(app.editor.document.line_text(line).unwrap().starts_with("EDIT"));
            assert_eq!(app.other_editor.document.text().unwrap(), other);
        }
    });
}

#[test]
#[ignore = "Headless SDL pane exit; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn exit_closes_only_the_focused_editor_pane_and_preserves_unsaved_work() {
    with_fixture(|app| {
        app.editor.insert_text("left unsaved").unwrap();
        app.other_editor.insert_text("right unsaved").unwrap();
        for mode in [KeybindingMode::Conventional, KeybindingMode::Vim, KeybindingMode::Emacs] {
            app.mode(mode);
            for pane in [0, 1] {
                app.split_mode = true;
                app.renderer.set_split_mode(true);
                focus_pane(pane, &mut app.active_pane, &mut app.editor, &mut app.other_editor,
                    &mut app.vim, &mut app.other_vim, &mut app.renderer);
                let text = app.editor.document.text().unwrap();
                let history = app.editor.undo_stack.len();
                app.command_bar.open(":exit");
                assert_eq!(app.key(Keycode::Return, Mod::NOMOD), EventFlow::Continue);
                assert!(!app.split_mode && !app.renderer.is_split());
                assert_eq!(app.active_pane, 1 - pane);
                assert_eq!(app.other_editor.document.text().unwrap(), text);
                assert_eq!(app.other_editor.undo_stack.len(), history);
                assert!(app.editor.dirty && app.other_editor.dirty);
                app.command_bar.open(":exit");
                assert_eq!(app.key(Keycode::Return, Mod::NOMOD), EventFlow::Continue);
                assert_eq!(app.active_pane, 1 - pane);
                assert!(app.command_bar.status().unwrap().contains("No split pane"));
                app.command_bar.open(":split");
                app.key(Keycode::Return, Mod::NOMOD);
                assert!(app.split_mode);
                assert_eq!(app.other_editor.document.text().unwrap(), text);
            }
        }
        app.mode(KeybindingMode::Conventional);
        app.command_bar.open(":quit");
        assert_eq!(app.key(Keycode::Return, Mod::NOMOD), EventFlow::Quit);
    });
}

#[test]
#[ignore = "Headless SDL pane exit; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn exit_works_in_terminal_panes_and_keeps_the_remaining_terminal_visible() {
    with_fixture(|app| {
        app.editor.insert_text("left unsaved").unwrap();
        app.other_editor.insert_text("right unsaved").unwrap();
        for pane in [0, 1] {
            app.split_mode = true;
            app.renderer.set_split_mode(true);
            focus_pane(pane, &mut app.active_pane, &mut app.editor, &mut app.other_editor,
                &mut app.vim, &mut app.other_vim, &mut app.renderer);
            let text = app.editor.document.text().unwrap();
            app.renderer.place_terminal_in_active_pane();
            app.terminal.open(None);
            let output = app.terminal.output_text().unwrap();
            app.text(":exit");
            assert_eq!(app.key(Keycode::Return, Mod::NOMOD), EventFlow::Continue);
            assert!(!app.split_mode && !app.terminal.is_active());
            assert_eq!(app.active_pane, 1 - pane);
            assert_eq!(app.other_editor.document.text().unwrap(), text);
            assert!(app.terminal.input().is_empty());
            assert_eq!(app.terminal.output_text().unwrap(), output);
        }
        app.split_mode = true;
        app.renderer.set_split_mode(true);
        app.renderer.place_terminal_in_active_pane();
        app.terminal.open(None);
        let terminal_pane = app.active_pane;
        focus_pane(1 - terminal_pane, &mut app.active_pane, &mut app.editor,
            &mut app.other_editor, &mut app.vim, &mut app.other_vim, &mut app.renderer);
        app.command_bar.open(":exit");
        app.key(Keycode::Return, Mod::NOMOD);
        assert!(!app.split_mode);
        assert_eq!(app.active_pane, terminal_pane);
        assert!(app.renderer.terminal_focused(&app.terminal));
        app.text(":exit");
        assert_eq!(app.key(Keycode::Return, Mod::NOMOD), EventFlow::Continue);
        assert!(app.terminal.is_active());
        assert!(app.terminal.status().unwrap().contains("No split pane"));
    });
}

#[test]
#[ignore = "Headless SDL pane exit; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn exit_command_can_be_run_with_the_mouse() {
    with_fixture(|app| {
        app.split_mode = true;
        app.renderer.set_split_mode(true);
        app.command_bar.open(":exit");
        app.renderer.render_split(&mut app.editor.document, &mut app.other_editor.document,
            &app.search_ui, &app.command_bar).unwrap();
        let mut target = None;
        'find_button: for y in (600 - app.command_bar.reserved_height()..600).step_by(4) {
            for x in (700..800).step_by(4) {
                if app.renderer.command_bar_hit_at(&app.command_bar, x, y) == CommandBarHit::Execute {
                    target = Some((x as f32, y as f32));
                    break 'find_button;
                }
            }
        }
        let (x, y) = target.expect("command execute button");
        assert_eq!(app.send(Event::MouseButtonDown {
            timestamp: 0, window_id: 0, which: 0, mouse_btn: MouseButton::Left,
            clicks: 1, x, y,
        }, true), EventFlow::Continue);
        assert!(!app.split_mode);
        assert_eq!(app.active_pane, 1);
    });
}

fn terminal_link_position(app: &mut Fixture<'_>, action: &TerminalAction) -> (f32, f32) {
    let left = if app.split_mode { app.renderer.terminal_pane() as i32 * 400 } else { 0 };
    let width = if app.split_mode { 400 } else { 800 };
    for y in (44..450).step_by(8) {
        for x in (left + 12..left + width - 10).step_by(8) {
            if app.renderer.terminal_action_at(&mut app.terminal, x, y).unwrap().as_ref() == Some(action) {
                return (x as f32, y as f32);
            }
        }
    }
    panic!("expected a visible link for {action:?}");
}

fn render_terminal_fixture(app: &mut Fixture<'_>) {
    if app.split_mode {
        app.renderer.render_split_terminal(&mut app.editor.document, &mut app.other_editor.document,
            &mut app.terminal, &app.search_ui, &app.command_bar).unwrap();
    } else {
        app.renderer.render_terminal(&mut app.terminal).unwrap();
    }
}

fn click_terminal_action(app: &mut Fixture<'_>, action: &TerminalAction, modifiers: Mod, drag: bool) {
    render_terminal_fixture(app);
    let (x, y) = terminal_link_position(app, action);
    app.keyboard.set_mod_state(modifiers);
    app.send(Event::MouseButtonDown {
        timestamp: 0, window_id: 0, which: 0, mouse_btn: MouseButton::Left,
        clicks: 1, x, y,
    }, true);
    app.keyboard.set_mod_state(Mod::NOMOD);
    app.send(Event::MouseButtonUp {
        timestamp: 0, window_id: 0, which: 0, mouse_btn: MouseButton::Left,
        clicks: 1, x: x + if drag { 20.0 } else { 0.0 }, y,
    }, true);
}

fn open_folder_fixture(app: &mut Fixture<'_>, folder: &FolderFixture) {
    open_folder_workspace(&folder.0, &mut app.split_mode, &mut app.active_pane,
        &mut app.editor, &mut app.other_editor, &mut app.vim, &mut app.other_vim,
        &mut app.renderer, &mut app.terminal).unwrap();
    finish_folder_listing(app);
}

#[test]
#[ignore = "Headless SDL terminal clicks; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn terminal_clicks_choose_both_panes_and_create_splits_only_for_file_opens() {
    let folder = FolderFixture::new();
    let file = TerminalAction::ListedFile(folder.0.join("sample.txt"));
    with_fixture(|app| {
        for split in [false, true] {
            for pane in [0, 1] {
                for modifiers in [Mod::NOMOD, Mod::LCTRLMOD, Mod::LGUIMOD] {
                    app.editor = Editor::new(app.editor.config.clone()).unwrap();
                    app.other_editor = Editor::new(app.other_editor.config.clone()).unwrap();
                    open_folder_fixture(app, &folder);
                    focus_pane(pane, &mut app.active_pane, &mut app.editor, &mut app.other_editor,
                        &mut app.vim, &mut app.other_vim, &mut app.renderer);
                    app.renderer.place_terminal_in_active_pane();
                    app.split_mode = split;
                    app.renderer.set_split_mode(split);
                    click_terminal_action(app, &file, modifiers, true);
                    assert_eq!(app.split_mode, split, "dragging must not open a split");
                    assert!(app.editor.path.is_none() && app.other_editor.path.is_none());
                    assert!(app.terminal.is_active());
                    click_terminal_action(app, &file, modifiers, false);
                    let other = modifiers != Mod::NOMOD;
                    assert_eq!(app.active_pane, if other { 1 - pane } else { pane });
                    assert_eq!(app.split_mode, split || other);
                    assert_eq!(app.terminal.is_active(), other);
                    assert_eq!(app.editor.document.text().unwrap(), "project file\n");
                    assert!(app.other_editor.document.is_empty());
                }
            }
        }
        for modifiers in [Mod::NOMOD, Mod::LCTRLMOD, Mod::LGUIMOD] {
            open_folder_fixture(app, &folder);
            focus_pane(0, &mut app.active_pane, &mut app.editor, &mut app.other_editor,
                &mut app.vim, &mut app.other_vim, &mut app.renderer);
            app.split_mode = false;
            app.renderer.set_split_mode(false);
            click_terminal_action(app, &TerminalAction::EnterDirectory(folder.0.join("nested folder")), modifiers, false);
            finish_folder_listing(app);
            assert!(!app.split_mode);
            assert!(app.terminal.is_active());
            assert_eq!(app.terminal.resolve_path("new.txt").unwrap(), folder.0.join("nested folder/new.txt"));
        }
    });
}

#[test]
#[ignore = "Headless SDL terminal clicks; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn terminal_clicks_protect_the_selected_destination_and_reuse_open_files() {
    let folder = FolderFixture::new();
    let file = TerminalAction::ListedFile(folder.0.join("sample.txt"));
    with_fixture(|app| {
        open_folder_fixture(app, &folder);
        // Plain clicks must not fall back to the other pane when this one is dirty.
        app.other_editor.insert_text("hidden unsaved").unwrap();
        click_terminal_action(app, &file, Mod::NOMOD, false);
        assert_eq!(app.active_pane, 0);
        assert!(app.terminal.is_active());
        assert!(app.terminal.status().unwrap().contains("selected pane has unsaved"));
        assert_eq!(app.editor.document.text().unwrap(), "hidden unsaved");
        assert!(app.other_editor.document.is_empty());
        // Modified clicks must not fall back into the terminal's clean document either.
        app.editor = Editor::new(app.editor.config.clone()).unwrap();
        app.other_editor.insert_text("destination unsaved").unwrap();
        click_terminal_action(app, &file, Mod::LGUIMOD, false);
        assert_eq!(app.active_pane, 0);
        assert!(app.terminal.is_active());
        assert!(app.terminal.status().unwrap().contains("selected pane has unsaved"));
        assert!(app.editor.document.is_empty());
        assert_eq!(app.other_editor.document.text().unwrap(), "destination unsaved");
        // An already-open file keeps its in-memory changes and undo history.
        app.other_editor.open(folder.0.join("sample.txt").to_str().unwrap()).unwrap();
        app.other_editor.insert_text("unsaved ").unwrap();
        let text = app.other_editor.document.text().unwrap();
        let history = app.other_editor.undo_stack.len();
        click_terminal_action(app, &file, Mod::LCTRLMOD, false);
        assert_eq!(app.active_pane, 1);
        assert_eq!(app.editor.document.text().unwrap(), text);
        assert_eq!(app.editor.undo_stack.len(), history);
        assert!(app.terminal.is_active());
        assert!(app.other_editor.path.is_none());
    });
}

#[test]
#[ignore = "Headless SDL shared pane editing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn terminal_clicks_open_shared_views_and_edit_undo_save_from_either_pane() {
    let folder = FolderFixture::new();
    let path = folder.0.join("sample.txt");
    let file = TerminalAction::ListedFile(path.clone());
    with_fixture(|app| {
        open_folder_fixture(app, &folder);
        click_terminal_action(app, &file, Mod::LGUIMOD, false);
        assert_eq!(app.active_pane, 1);
        app.text("unsaved ");
        let expected = app.editor.document.text().unwrap();
        // Plain click from the listing opens a second live view on the left.
        click_terminal_action(app, &file, Mod::NOMOD, false);
        assert_eq!(app.active_pane, 0);
        assert!(!app.terminal.is_active());
        assert!(app.editor.shares_document_with(&app.other_editor));
        assert_eq!(app.editor.document.text().unwrap(), expected);
        assert_eq!(app.editor.path, app.other_editor.path);
        app.editor.document.move_cursor(0).unwrap();
        app.text("left ");
        assert_eq!(app.editor.document.text().unwrap(), app.other_editor.document.text().unwrap());
        focus_pane(1, &mut app.active_pane, &mut app.editor, &mut app.other_editor,
            &mut app.vim, &mut app.other_vim, &mut app.renderer);
        app.key(Keycode::Z, Mod::LCTRLMOD);
        assert_eq!(app.editor.document.text().unwrap(), expected);
        assert_eq!(app.editor.document.text().unwrap(), app.other_editor.document.text().unwrap());
        app.key(Keycode::S, Mod::LCTRLMOD);
        assert!(!app.editor.dirty && !app.other_editor.dirty);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);
        // Reopen terminal over this view and Ctrl-click the same file into a clean empty pane.
        app.other_editor = Editor::new(app.other_editor.config.clone()).unwrap();
        app.renderer.place_terminal_in_active_pane();
        app.terminal.open(None);
        click_terminal_action(app, &file, Mod::LCTRLMOD, false);
        assert_eq!(app.active_pane, 0);
        assert!(app.editor.shares_document_with(&app.other_editor));
        app.text("right ");
        assert_eq!(app.editor.document.text().unwrap(), app.other_editor.document.text().unwrap());
        // Closing a view leaves the live document and its history usable.
        app.terminal.close_to_editor();
        assert!(close_focused_pane(&mut app.split_mode, &mut app.active_pane,
            &mut app.editor, &mut app.other_editor, &mut app.vim, &mut app.other_vim,
            &mut app.renderer, &mut app.terminal));
        app.key(Keycode::Z, Mod::LCTRLMOD);
        assert_eq!(app.editor.document.text().unwrap(), expected);
    });
}

#[test]
#[ignore = "Headless SDL shared Vim history; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn shared_views_keep_vim_insert_groups_separate_when_switching_panes() {
    with_fixture(|app| {
        app.other_editor = app.editor.duplicate_view();
        app.split_mode = true;
        app.renderer.set_split_mode(true);
        app.mode(KeybindingMode::Vim);
        app.key(Keycode::I, Mod::NOMOD);
        app.text("i");
        app.key(Keycode::A, Mod::NOMOD);
        app.text("aa");
        app.text("bb");
        focus_pane(1, &mut app.active_pane, &mut app.editor, &mut app.other_editor,
            &mut app.vim, &mut app.other_vim, &mut app.renderer);
        app.key(Keycode::I, Mod::NOMOD);
        app.text("i");
        app.key(Keycode::C, Mod::NOMOD);
        app.text("cc");
        app.key(Keycode::Escape, Mod::NOMOD);
        focus_pane(0, &mut app.active_pane, &mut app.editor, &mut app.other_editor,
            &mut app.vim, &mut app.other_vim, &mut app.renderer);
        assert_eq!(app.vim.mode(), vim::VimMode::Insert);
        app.key(Keycode::D, Mod::NOMOD);
        app.text("dd");
        app.key(Keycode::Escape, Mod::NOMOD);
        app.key(Keycode::U, Mod::NOMOD);
        app.text("u");
        assert_eq!(app.editor.document.len(), 6);
        app.key(Keycode::U, Mod::NOMOD);
        app.text("u");
        assert_eq!(app.editor.document.text().unwrap(), "aabb");
        app.key(Keycode::U, Mod::NOMOD);
        app.text("u");
        assert!(app.editor.document.is_empty());
        assert!(app.other_editor.document.is_empty());
    });
}

fn document_mouse_point(app: &mut Fixture<'_>, line: usize, column: usize) -> (f32, f32) {
    for y in (40..590).step_by(2) {
        let left = if app.active_pane == 1 && app.split_mode { app.renderer.split_divider() } else { 0 };
        for x in (left..800).step_by(2) {
            if app.renderer.cursor_target_at(&mut app.editor.document, x, y).unwrap() == Some((line, column)) {
                return (x as f32, y as f32);
            }
        }
    }
    panic!("no mouse target for {line}:{column}");
}

fn mouse_press(app: &mut Fixture<'_>, point: (f32, f32), clicks: u8, modifiers: Mod) {
    app.keyboard.set_mod_state(modifiers);
    app.send(Event::MouseButtonDown { timestamp: 0, window_id: 0, which: 0,
        mouse_btn: MouseButton::Left, clicks, x: point.0, y: point.1 }, true);
}

fn mouse_move(app: &mut Fixture<'_>, point: (f32, f32)) {
    app.send(Event::MouseMotion { timestamp: 0, window_id: 0, which: 0,
        mousestate: sdl3::mouse::MouseState::from_sdl_state(1),
        x: point.0, y: point.1, xrel: 0.0, yrel: 0.0 }, true);
}

fn mouse_release(app: &mut Fixture<'_>, point: (f32, f32)) {
    app.send(Event::MouseButtonUp { timestamp: 0, window_id: 0, which: 0,
        mouse_btn: MouseButton::Left, clicks: 1, x: point.0, y: point.1 }, true);
    app.keyboard.set_mod_state(Mod::NOMOD);
}

fn selected_document_text(app: &Fixture<'_>) -> String {
    let doc = &app.editor.document;
    String::from_utf8(doc.read_range(doc.selection_start(), doc.selection_end() - doc.selection_start()).unwrap()).unwrap()
}

#[test]
#[ignore = "Headless SDL mouse selection; run with SDL_VIDEODRIVER=dummy"]
fn mouse_interactions_select_edit_extend_words_and_lines() {
    with_fixture(|app| {
        app.editor.insert_text("hello café_東京!\nsecond line\nthird").unwrap();
        app.editor.document.move_cursor(0).unwrap();
        app.renderer.render(&mut app.editor.document, &app.search_ui, &app.command_bar).unwrap();
        let start = document_mouse_point(app, 0, 0);
        let end = document_mouse_point(app, 0, 5);
        mouse_press(app, start, 1, Mod::NOMOD);
        mouse_move(app, end);
        mouse_release(app, end);
        assert_eq!(selected_document_text(app), "hello");
        let word = document_mouse_point(app, 0, 9);
        mouse_press(app, word, 2, Mod::NOMOD);
        mouse_release(app, word);
        assert_eq!(selected_document_text(app), "café_東京");
        mouse_press(app, start, 1, Mod::LSHIFTMOD);
        mouse_release(app, start);
        assert_eq!(selected_document_text(app), "hello ");
        let second = document_mouse_point(app, 1, 4);
        mouse_press(app, second, 3, Mod::NOMOD);
        mouse_release(app, second);
        assert_eq!(selected_document_text(app), "second line\n");
        app.text("replacement\n");
        assert_eq!(app.editor.document.text().unwrap(), "hello café_東京!\nreplacement\nthird");
        assert_eq!(app.editor.undo_stack.len(), 2);
    });
}

#[test]
#[ignore = "Headless SDL split mouse selection; run with SDL_VIDEODRIVER=dummy"]
fn mouse_interactions_resize_split_and_keep_drag_in_its_pane() {
    with_fixture(|app| {
        for editor in [&mut app.editor, &mut app.other_editor] {
            editor.insert_text("one two three\nnext line").unwrap();
            editor.document.move_cursor(0).unwrap();
        }
        app.split_mode = true;
        app.renderer.set_split_mode(true);
        app.renderer.render_split(&mut app.editor.document, &mut app.other_editor.document,
            &app.search_ui, &app.command_bar).unwrap();
        mouse_press(app, (400.0, 150.0), 1, Mod::NOMOD);
        mouse_move(app, (280.0, 150.0));
        mouse_release(app, (280.0, 150.0));
        assert_eq!(app.renderer.split_divider(), 280);
        assert_eq!(app.renderer.pane_at(300), 1);
        app.renderer.render_split(&mut app.editor.document, &mut app.other_editor.document,
            &app.search_ui, &app.command_bar).unwrap();
        let start = document_mouse_point(app, 0, 4);
        mouse_press(app, start, 2, Mod::NOMOD);
        mouse_move(app, (600.0, start.1));
        mouse_release(app, (600.0, start.1));
        assert_eq!(app.active_pane, 0);
        assert!(selected_document_text(app).starts_with("two three"));
        assert!(!app.other_editor.document.has_selection());
        mouse_press(app, (380.0, start.1), 1, Mod::NOMOD);
        mouse_release(app, (380.0, start.1));
        assert_eq!(app.active_pane, 1);
        assert!(!app.editor.document.has_selection());
        assert!(app.other_editor.document.has_selection());
        app.renderer.window_mut().set_size(1000, 600).unwrap();
        app.renderer.update_window_size().unwrap();
        assert_eq!(app.renderer.split_divider(), 350);
        mouse_press(app, (350.0, 120.0), 1, Mod::NOMOD);
        mouse_move(app, (-100.0, 120.0));
        mouse_release(app, (-100.0, 120.0));
        assert_eq!(app.renderer.split_divider(), 120);
    });
}

#[test]
#[ignore = "Headless SDL mouse scrolling; run with SDL_VIDEODRIVER=dummy"]
fn mouse_interactions_accumulate_fractional_scroll_separately_per_pane() {
    with_fixture(|app| {
        for editor in [&mut app.editor, &mut app.other_editor] {
            editor.insert_text(&format!("{}\n", "long ".repeat(100)).repeat(80)).unwrap();
            editor.document.move_cursor(0).unwrap();
        }
        app.split_mode = true;
        app.renderer.set_split_mode(true);
        app.renderer.render_split(&mut app.editor.document, &mut app.other_editor.document,
            &app.search_ui, &app.command_bar).unwrap();
        let wheel = |app: &mut Fixture<'_>, pane, x, y| {
            app.send(Event::MouseWheel { timestamp: 0, window_id: 0, which: 0, x, y,
                direction: sdl3::mouse::MouseWheelDirection::Normal,
                mouse_x: pane as f32 * 400.0 + 200.0, mouse_y: 60.0, integer_x: 0, integer_y: 0 }, true);
        };
        for _ in 0..3 { wheel(app, 0, 0.0, -0.25); }
        wheel(app, 1, 0.0, -0.25);
        assert_eq!(app.renderer.cursor_target_at(&mut app.editor.document, 600, 50).unwrap().unwrap().0, 0);
        wheel(app, 0, -0.25, -0.25);
        assert_eq!(app.renderer.cursor_target_at(&mut app.editor.document, 200, 50).unwrap().unwrap().0, 1);
        let before = app.renderer.cursor_target_at(&mut app.editor.document, 200, 50).unwrap().unwrap().1;
        wheel(app, 0, -0.25, 0.0);
        let after = app.renderer.cursor_target_at(&mut app.editor.document, 200, 50).unwrap().unwrap().1;
        assert!(after > before, "fractional horizontal wheel movement must not disappear");
    });
}

#[test]
#[ignore = "Headless SDL Vim mouse selection; run with SDL_VIDEODRIVER=dummy"]
fn mouse_interactions_vim_can_yank_and_delete_mouse_selection() {
    with_fixture(|app| {
        app.editor.insert_text("one two three").unwrap();
        app.mode(KeybindingMode::Vim);
        app.renderer.render(&mut app.editor.document, &app.search_ui, &app.command_bar).unwrap();
        let word = document_mouse_point(app, 0, 5);
        mouse_press(app, word, 2, Mod::NOMOD);
        mouse_release(app, word);
        assert_eq!(selected_document_text(app), "two");
        app.key(Keycode::Y, Mod::NOMOD);
        assert_eq!(app.clipboard.clipboard_text().unwrap(), "two");
        mouse_press(app, word, 2, Mod::NOMOD);
        mouse_release(app, word);
        app.key(Keycode::D, Mod::NOMOD);
        assert_eq!(app.editor.document.text().unwrap(), "one  three");
    });
}

#[test]
#[ignore = "Headless SDL mouse autoscroll; run with SDL_VIDEODRIVER=dummy"]
fn mouse_interactions_autoscroll_stops_on_release_and_focus_loss() {
    with_fixture(|app| {
        app.editor.insert_text(&"line of code\n".repeat(150)).unwrap();
        app.editor.document.move_cursor(0).unwrap();
        app.renderer.render(&mut app.editor.document, &app.search_ui, &app.command_bar).unwrap();
        let start = document_mouse_point(app, 0, 0);
        mouse_press(app, start, 1, Mod::NOMOD);
        mouse_move(app, (start.0, 620.0));
        let before = app.editor.document.cursor.line;
        assert!(app.mouse_state.wait_timeout() <= std::time::Duration::from_millis(40));
        std::thread::sleep(std::time::Duration::from_millis(45));
        assert!(app.mouse_state.tick(&mut app.editor, &mut app.renderer, &mut app.vim, false, 0).unwrap());
        assert!(app.editor.document.cursor.line > before);
        mouse_release(app, (start.0, 620.0));
        assert_eq!(app.mouse_state.wait_timeout(), std::time::Duration::MAX);
        mouse_press(app, start, 1, Mod::NOMOD);
        mouse_move(app, (start.0, 620.0));
        app.send(Event::Window { timestamp: 0, window_id: 0,
            win_event: WindowEvent::FocusLost }, true);
        assert_eq!(app.mouse_state.wait_timeout(), std::time::Duration::MAX);
        assert!(!app.mouse_state.tick(&mut app.editor, &mut app.renderer, &mut app.vim, false, 0).unwrap());
        let position = app.editor.document.cursor.position;
        mouse_move(app, start);
        assert_eq!(app.editor.document.cursor.position, position);
    });
}

#[test]
#[ignore = "Headless SDL terminal divider; run with SDL_VIDEODRIVER=dummy"]
fn mouse_interactions_resize_terminal_pane_and_preserve_editor_hit_testing() {
    let folder = FolderFixture::new();
    with_fixture(|app| {
        open_folder_fixture(app, &folder);
        finish_folder_listing(app);
        app.editor.insert_text("right pane code").unwrap();
        app.editor.document.move_cursor(0).unwrap();
        render_terminal_fixture(app);
        let old_columns = app.renderer.terminal_columns();
        mouse_press(app, (400.0, 120.0), 1, Mod::NOMOD);
        mouse_move(app, (240.0, 120.0));
        mouse_release(app, (240.0, 120.0));
        render_terminal_fixture(app);
        assert_eq!(app.renderer.split_divider(), 240);
        assert!(app.renderer.terminal_columns() < old_columns);
        let point = document_mouse_point(app, 0, 8);
        mouse_press(app, point, 2, Mod::NOMOD);
        mouse_release(app, point);
        assert_eq!(app.active_pane, 1);
        assert_eq!(selected_document_text(app), "pane");
        assert!(app.renderer.terminal_visible(&app.terminal));
    });
}
