// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Dispatch real SDL event values through the same handlers as the application.
//! These headless tests check routing, not OS delivery or physical clipboard UX.
use super::*;

struct Fixture<'font> {
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
