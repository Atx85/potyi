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

impl<'font> Fixture<'font> {
    fn context(&mut self) -> InputContext<'_, 'font> {
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
        }
    }

    fn send(&mut self, event: Event, coordinates: bool) -> EventFlow {
        dispatch(self.context(), event, coordinates).unwrap()
    }

    fn send_application(
        &mut self,
        event: Event,
        frames: &mut terminal::FrameSchedule,
    ) -> EventFlow {
        super::super::events::dispatch(self.context(), event, frames).unwrap()
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

    fn execute_command(&mut self, input: &str, mouse: bool) -> EventFlow {
        self.command_bar.open(input);
        if !mouse {
            return self.key(Keycode::Return, Mod::NOMOD);
        }
        if self.split_mode {
            self.renderer
                .render_split(
                    &mut self.editor.document,
                    &mut self.other_editor.document,
                    &self.search_ui,
                    &self.command_bar,
                )
                .unwrap();
        } else {
            self.renderer
                .render(
                    &mut self.editor.document,
                    &self.search_ui,
                    &self.command_bar,
                )
                .unwrap();
        }
        for y in (600 - self.command_bar.reserved_height()..600).step_by(4) {
            for x in (0..800).step_by(4) {
                if self.renderer.command_bar_hit_at(&self.command_bar, x, y)
                    == CommandBarHit::Execute
                {
                    return self.send(
                        Event::MouseButtonDown {
                            timestamp: 0,
                            window_id: 0,
                            which: 0,
                            mouse_btn: MouseButton::Left,
                            clicks: 1,
                            x: x as f32,
                            y: y as f32,
                        },
                        true,
                    );
                }
            }
        }
        panic!("command execute button");
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
        focus_pane(
            0,
            &mut app.active_pane,
            &mut app.editor,
            &mut app.other_editor,
            &mut app.vim,
            &mut app.other_vim,
            &mut app.renderer,
        );
        app.key(Keycode::W, Mod::LCTRLMOD);
        app.key(Keycode::W, Mod::NOMOD);
        app.text("w");
        app.key_repeat(Keycode::W, Mod::NOMOD, true);
        app.text("w");
        assert_eq!(app.active_pane, 1);
        assert_eq!(
            app.editor.document.text().unwrap(),
            "right",
            "shortcut text cannot leak into Insert mode"
        );
        app.key(Keycode::A, Mod::NOMOD);
        app.text("a");
        assert!(
            app.editor.document.text().unwrap().contains('a'),
            "the next ordinary key must not be swallowed"
        );
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
        assert_eq!(
            app.active_pane, 0,
            "single-pane mode must not expose a hidden document"
        );
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
                assert_eq!(
                    app.editor.document.text().unwrap(),
                    if target == 0 { "left" } else { "right" }
                );
            }
            app.key(prefix, Mod::LCTRLMOD);
            app.key(Keycode::Escape, Mod::NOMOD);
            assert!(
                app.renderer.terminal_focused(&app.terminal),
                "prefix cancellation must not close the terminal"
            );
        }
        app.terminal.close_to_editor();
        app.key(Keycode::X, Mod::LCTRLMOD);
        app.key(Keycode::O, Mod::NOMOD);
        app.text("o");
        assert_eq!(
            app.active_pane, 1,
            "existing Emacs editor-to-editor navigation still works"
        );
        assert_eq!(app.editor.document.text().unwrap(), "right");
    });
}

#[test]
#[ignore = "Headless SDL event routing; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn mouse_and_drop_events_preserve_panes_and_report_open_failures() {
    with_fixture(|app| {
        app.editor.insert_text("left unsaved").unwrap();
        app.other_editor.insert_text("right unsaved").unwrap();
        app.other_editor.set_path(Some(
            std::env::temp_dir().join("potyi-input-routing-other.txt"),
        ));
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
            .path()
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
        assert!(app.other_editor.is_dirty());

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
        assert!(app.editor.is_dirty());
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
        assert!(app.editor.path().is_none());
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
        for modifiers in [
            Mod::NOMOD,
            Mod::LCTRLMOD,
            Mod::RCTRLMOD,
            Mod::LGUIMOD,
            Mod::RGUIMOD,
        ] {
            app.editor = Editor::new(app.editor.config.clone()).unwrap();
            app.other_editor = Editor::new(app.other_editor.config.clone()).unwrap();
            focus_pane(
                0,
                &mut app.active_pane,
                &mut app.editor,
                &mut app.other_editor,
                &mut app.vim,
                &mut app.other_vim,
                &mut app.renderer,
            );
            app.renderer.place_terminal_in_active_pane();
            app.terminal.open(None);
            focus_pane(
                1,
                &mut app.active_pane,
                &mut app.editor,
                &mut app.other_editor,
                &mut app.vim,
                &mut app.other_vim,
                &mut app.renderer,
            );
            app.renderer
                .render_split_terminal(
                    &mut app.editor.document,
                    &mut app.other_editor.document,
                    &mut app.terminal,
                    &app.search_ui,
                    &app.command_bar,
                )
                .unwrap();
            // Find the file's rendered link, so these exercise the actual mouse
            // hit testing, focus changes and release handling together.
            let mut target = None;
            'find_link: for y in (44..450).step_by(8) {
                for x in (12..390).step_by(8) {
                    if let Some(TerminalAction::ListedFile(path)) = app
                        .renderer
                        .terminal_action_at(&mut app.terminal, x, y)
                        .unwrap()
                        && path == folder.0.join("sample.txt")
                    {
                        target = Some((x as f32, y as f32));
                        break 'find_link;
                    }
                }
            }
            let (x, y) = target.expect("the folder listing must contain a clickable file");
            keyboard.set_mod_state(modifiers);
            app.send(
                Event::MouseButtonDown {
                    timestamp: 0,
                    window_id: 0,
                    which: 0,
                    mouse_btn: MouseButton::Left,
                    clicks: 1,
                    x,
                    y,
                },
                true,
            );
            // The modifier at press time determines the destination, even
            // if it is released before the mouse button.
            keyboard.set_mod_state(Mod::NOMOD);
            app.send(
                Event::MouseButtonUp {
                    timestamp: 0,
                    window_id: 0,
                    which: 0,
                    mouse_btn: MouseButton::Left,
                    clicks: 1,
                    x,
                    y,
                },
                true,
            );
            keyboard.set_mod_state(Mod::NOMOD);
            assert_eq!(
                app.active_pane,
                usize::from(modifiers != Mod::NOMOD),
                "click modifiers: {modifiers:?}"
            );
            assert_eq!(app.editor.document.text().unwrap(), "project file\n");
            assert_eq!(
                app.renderer.terminal_visible(&app.terminal),
                modifiers != Mod::NOMOD
            );
        }
        assert_eq!(app.editor.document.text().unwrap(), "project file\n");
        assert_eq!(
            app.editor.path().as_deref(),
            Some(folder.0.join("sample.txt").as_path())
        );
        assert!(app.renderer.terminal_visible(&app.terminal));
        assert_eq!(app.terminal.output_text().unwrap(), output);
        app.text("edited ");
        assert!(app.editor.is_dirty());
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
        assert!(app.editor.path().is_none());
        assert_eq!(app.other_editor.document.text().unwrap(), "left unsaved");
        assert!(app.other_editor.is_dirty());
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
            assert!(app.editor.is_dirty() && app.other_editor.is_dirty());
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
            app.send(
                Event::DropFile {
                    timestamp: 0,
                    window_id: 0,
                    filename: format!("{}/", path.display()),
                },
                true,
            );
            finish_folder_listing(app);
            assert!(app.split_mode);
            assert_eq!(app.active_pane, 1);
            assert!(app.editor.path().is_none());
            assert!(app.editor.document.is_empty());
            assert!(!app.command_bar.is_info());
            assert_eq!(
                app.terminal.resolve_path("child.txt").unwrap(),
                nested.join("child.txt")
            );
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
        app.renderer
            .render_split(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
        let wheel = |pane, x, y| Event::MouseWheel {
            timestamp: 0,
            window_id: 0,
            which: 0,
            x,
            y,
            direction: sdl3::mouse::MouseWheelDirection::Normal,
            mouse_x: pane as f32 * 400.0 + 200.0,
            mouse_y: 100.0,
            integer_x: x as i32,
            integer_y: y as i32,
        };
        let target = |app: &mut Fixture<'_>, pane| {
            app.renderer
                .cursor_target_at(&mut app.editor.document, pane * 400 + 200, 100)
                .unwrap()
                .unwrap()
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
            assert!(
                target(app, pane).1 > 950,
                "typing must reveal the end of long lines"
            );
            app.key(Keycode::Home, Mod::NOMOD);
            assert_eq!(app.editor.document.cursor.column, 0);
            app.key(Keycode::PageDown, Mod::NOMOD);
            let line = app.editor.document.cursor.line;
            assert!(line > 0);
            app.text("EDIT");
            assert!(
                app.editor
                    .document
                    .line_text(line)
                    .unwrap()
                    .starts_with("EDIT")
            );
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
        for mode in [
            KeybindingMode::Conventional,
            KeybindingMode::Vim,
            KeybindingMode::Emacs,
        ] {
            app.mode(mode);
            for pane in [0, 1] {
                app.split_mode = true;
                app.renderer.set_split_mode(true);
                focus_pane(
                    pane,
                    &mut app.active_pane,
                    &mut app.editor,
                    &mut app.other_editor,
                    &mut app.vim,
                    &mut app.other_vim,
                    &mut app.renderer,
                );
                let text = app.editor.document.text().unwrap();
                let history = app.editor.undo_stack.len();
                app.command_bar.open(":exit");
                assert_eq!(app.key(Keycode::Return, Mod::NOMOD), EventFlow::Continue);
                assert!(!app.split_mode && !app.renderer.is_split());
                assert_eq!(app.active_pane, 1 - pane);
                assert_eq!(app.other_editor.document.text().unwrap(), text);
                assert_eq!(app.other_editor.undo_stack.len(), history);
                assert!(app.editor.is_dirty() && app.other_editor.is_dirty());
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
            focus_pane(
                pane,
                &mut app.active_pane,
                &mut app.editor,
                &mut app.other_editor,
                &mut app.vim,
                &mut app.other_vim,
                &mut app.renderer,
            );
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
        focus_pane(
            1 - terminal_pane,
            &mut app.active_pane,
            &mut app.editor,
            &mut app.other_editor,
            &mut app.vim,
            &mut app.other_vim,
            &mut app.renderer,
        );
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
        app.renderer
            .render_split(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
        let mut target = None;
        'find_button: for y in (600 - app.command_bar.reserved_height()..600).step_by(4) {
            for x in (700..800).step_by(4) {
                if app.renderer.command_bar_hit_at(&app.command_bar, x, y) == CommandBarHit::Execute
                {
                    target = Some((x as f32, y as f32));
                    break 'find_button;
                }
            }
        }
        let (x, y) = target.expect("command execute button");
        assert_eq!(
            app.send(
                Event::MouseButtonDown {
                    timestamp: 0,
                    window_id: 0,
                    which: 0,
                    mouse_btn: MouseButton::Left,
                    clicks: 1,
                    x,
                    y,
                },
                true
            ),
            EventFlow::Continue
        );
        assert!(!app.split_mode);
        assert_eq!(app.active_pane, 1);
    });
}

#[test]
#[ignore = "Headless SDL command outcomes; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn command_execution_paths_preserve_pane_mode_and_document_outcomes() {
    with_fixture(|app| {
        for mouse in [false, true] {
            app.mode(KeybindingMode::Conventional);
            app.editor = Editor::new(app.editor.config.clone()).unwrap();
            app.other_editor = Editor::new(app.other_editor.config.clone()).unwrap();
            app.editor.insert("one\ntwo\nthree").unwrap();
            app.editor.set_dirty(false);
            app.other_editor.insert("other pane").unwrap();
            app.split_mode = false;
            app.renderer.set_split_mode(false);

            assert_eq!(
                app.execute_command(":goto 2 --abs", mouse),
                EventFlow::Continue
            );
            assert_eq!(app.editor.document.cursor.line, 1);
            assert!(!app.command_bar.is_active());
            assert_eq!(
                app.execute_command(":set keybindings vim", mouse),
                EventFlow::Continue
            );
            assert!(app.vim_enabled);
            assert_eq!(app.vim.mode(), vim::VimMode::Normal);
            assert_eq!(app.other_editor.config.keybinding_mode, KeybindingMode::Vim);
            assert_eq!(app.execute_command(":split", mouse), EventFlow::Continue);
            assert!(app.split_mode);
            assert_eq!(app.execute_command(":new", mouse), EventFlow::Continue);
            assert_eq!(app.editor.document.len(), 0);
            assert_eq!(app.editor.document.cursor.position, 0);
            assert_eq!(app.vim.mode(), vim::VimMode::Normal);
            assert_eq!(app.other_editor.document.text().unwrap(), "other pane");
            assert!(app.dirty);
        }
    });
}

#[test]
#[ignore = "Headless SDL application events; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn application_dispatch_keeps_background_scheduling_and_input_order() {
    with_fixture(|app| {
        let sdl = sdl3::init().unwrap();
        let mut pump = sdl.event_pump().unwrap();
        for _ in pump.poll_iter() {}
        app.event_subsystem
            .register_custom_event::<lsp::Event>()
            .unwrap();
        let mut frames = terminal::FrameSchedule::default();

        app.renderer.place_terminal_in_active_pane();
        app.terminal.open(None);
        app.dirty = false;
        app.event_subsystem
            .push_custom_event(TerminalEvent::ReaderError("reader failed".into()))
            .unwrap();
        assert_eq!(
            app.send_application(
                pump.wait_event_timeout(Duration::from_secs(1))
                    .expect("queued background event"),
                &mut frames
            ),
            EventFlow::Continue
        );
        assert!(
            app.terminal
                .output_text()
                .unwrap()
                .contains("reader failed")
        );
        assert!(frames.due(Instant::now()));
        assert!(!app.dirty, "terminal output uses its frame schedule");
        assert_eq!(app.editor.document.len(), 0);

        app.terminal.close_to_editor();
        app.event_subsystem
            .push_custom_event(lsp::Event {
                id: u64::MAX,
                result: Err("stale result".into()),
            })
            .unwrap();
        // SDL's Rust queue API does not support pushing TextInput. Mix the
        // queued worker result with synthetic text in the application's order.
        for event in [
            Event::TextInput {
                timestamp: 0,
                window_id: 0,
                text: "a".into(),
            },
            pump.wait_event_timeout(Duration::from_secs(1))
                .expect("queued background event"),
            Event::TextInput {
                timestamp: 0,
                window_id: 0,
                text: "b".into(),
            },
        ] {
            assert_eq!(
                app.send_application(event, &mut frames),
                EventFlow::Continue
            );
        }
        assert_eq!(app.editor.document.text().unwrap(), "ab");
        assert_eq!(app.other_editor.document.len(), 0);
        assert!(
            !app.command_bar.is_active(),
            "stale results do not open a panel"
        );
        assert!(app.dirty);
        assert_eq!(
            app.send_application(Event::Quit { timestamp: 0 }, &mut frames),
            EventFlow::Quit
        );
    });
}

#[test]
#[cfg(unix)]
#[ignore = "Headless SDL LSP delivery; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn application_lsp_delivery_synchronizes_edits_and_focuses_the_definition_pane() {
    struct WorkingDirectory(std::path::PathBuf);
    impl Drop for WorkingDirectory {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.0).unwrap();
        }
    }
    fn queued_lsp(pump: &mut sdl3::EventPump) -> Event {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(Instant::now() < deadline, "language-server reply timed out");
            if let Some(event) = pump.wait_event_timeout(Duration::from_millis(20))
                && matches!(event, Event::User { .. })
            {
                // This fixture's only worker is LSP. Decoding here would
                // consume SDL's payload before application dispatch sees it.
                return event;
            }
        }
    }

    let files = FolderFixture::new();
    std::fs::create_dir(files.0.join("config")).unwrap();
    let server =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lsp_server.py");
    std::fs::write(
        files.0.join("config/lsp.toml"),
        format!(
            "enabled=true\n[[servers]]\nname='mock'\ncommand='python3'\nargs=[{},'incremental']\nextensions=['cs']\nlanguage_id='csharp'\n",
            serde_json::to_string(server.to_str().unwrap()).unwrap()
        ),
    )
    .unwrap();
    let source = files.0.join("source.cs");
    let destination = files.0.join("definition.cs");
    std::fs::write(&source, "transform").unwrap();
    std::fs::write(&destination, "a🦀b\n").unwrap();
    let _cwd = WorkingDirectory(std::env::current_dir().unwrap());
    std::env::set_current_dir(&files.0).unwrap();

    with_fixture(|app| {
        let sdl = sdl3::init().unwrap();
        let mut pump = sdl.event_pump().unwrap();
        for _ in pump.poll_iter() {}
        app.event_subsystem
            .register_custom_event::<lsp::Event>()
            .unwrap();
        let mut frames = terminal::FrameSchedule::default();
        app.editor.open(source.to_str().unwrap()).unwrap();
        app.editor.document.move_cursor(9).unwrap();
        app.other_editor = app.editor.duplicate_view();
        app.split_mode = true;
        app.renderer.set_split_mode(true);
        app.mode(KeybindingMode::Vim);
        app.key(Keycode::A, Mod::NOMOD);
        app.text("a"); // Consume the paired text for Vim's insert command.
        // Start the request with synchronized views, then verify that its
        // asynchronous edit synchronizes them again at application dispatch.
        app.editor.insert(".").unwrap();
        app.vim.record_text(".");
        synchronize_pane_views(&mut app.editor, &mut app.other_editor, &mut app.renderer).unwrap();
        app.lsp_ui
            .typed_member_trigger(".", &app.editor, &app.other_editor, &mut app.command_bar);
        app.send_application(queued_lsp(&mut pump), &mut frames);
        assert_eq!(
            app.lsp_ui.completion_display().unwrap().labels,
            ["position", "Translate"]
        );
        app.key(Keycode::Down, Mod::NOMOD);
        app.key(Keycode::Return, Mod::NOMOD);
        app.dirty = false;
        app.send_application(queued_lsp(&mut pump), &mut frames);
        assert_eq!(app.editor.document.text().unwrap(), "transform.Translate");
        assert_eq!(
            app.other_editor.document.text().unwrap(),
            "transform.Translate"
        );
        assert_eq!(app.editor.document.cursor.position, 19);
        assert_eq!(app.vim.mode(), vim::VimMode::Insert);
        assert!(app.dirty);
        assert_eq!(std::fs::read_to_string(&source).unwrap(), "transform");
        app.key(Keycode::Escape, Mod::NOMOD);
        app.key(Keycode::U, Mod::NOMOD);
        assert_eq!(app.editor.document.text().unwrap(), "transform.");
        app.key(Keycode::U, Mod::NOMOD);
        assert_eq!(app.editor.document.text().unwrap(), "transform");
        assert_eq!(app.other_editor.document.text().unwrap(), "transform");

        app.other_editor = Editor::new(app.other_editor.config.clone()).unwrap();
        app.other_editor
            .open(destination.to_str().unwrap())
            .unwrap();
        app.other_vim
            .handle_key(
                &mut app.other_editor,
                &app.clipboard,
                Keycode::I,
                Mod::NOMOD,
                false,
                10,
            )
            .unwrap();
        app.editor.insert("unsaved ").unwrap();
        let unsaved = app.editor.document.text().unwrap();
        app.execute_command(":lsp definition", false);
        let response = queued_lsp(&mut pump)
            .as_user_event_type::<lsp::Event>()
            .unwrap();
        assert!(matches!(response.result, Ok(lsp::Reply::Definition(_))));
        // Retain the real request ID, but target the other open file instead
        // of the mock server's usual same-file location. Deliver it via SDL.
        app.event_subsystem
            .push_custom_event(lsp::Event {
                id: response.id,
                result: Ok(lsp::Reply::Definition(vec![lsp::Location {
                    path: destination.clone(),
                    line: 0,
                    character: 3,
                }])),
            })
            .unwrap();
        app.dirty = false;
        app.send_application(queued_lsp(&mut pump), &mut frames);
        assert_eq!(app.active_pane, 1);
        assert_eq!(app.editor.path().as_ref(), Some(&destination));
        assert_eq!(app.editor.document.cursor.position, 5);
        assert_eq!(app.vim.mode(), vim::VimMode::Insert);
        assert_eq!(app.other_editor.path().as_ref(), Some(&source));
        assert_eq!(app.other_editor.document.text().unwrap(), unsaved);
        assert!(app.other_editor.is_dirty());
        assert!(!app.command_bar.is_active());
        assert!(app.dirty);
        app.lsp_ui.stop();
    });
}

fn terminal_link_position(app: &mut Fixture<'_>, action: &TerminalAction) -> (f32, f32) {
    let left = if app.split_mode {
        app.renderer.terminal_pane() as i32 * 400
    } else {
        0
    };
    let width = if app.split_mode { 400 } else { 800 };
    for y in (44..450).step_by(8) {
        for x in (left + 12..left + width - 10).step_by(8) {
            if app
                .renderer
                .terminal_action_at(&mut app.terminal, x, y)
                .unwrap()
                .as_ref()
                == Some(action)
            {
                return (x as f32, y as f32);
            }
        }
    }
    panic!("expected a visible link for {action:?}");
}

fn render_terminal_fixture(app: &mut Fixture<'_>) {
    if app.split_mode {
        app.renderer
            .render_split_terminal(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &mut app.terminal,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
    } else {
        app.renderer.render_terminal(&mut app.terminal).unwrap();
    }
}

fn click_terminal_action(
    app: &mut Fixture<'_>,
    action: &TerminalAction,
    modifiers: Mod,
    drag: bool,
) {
    render_terminal_fixture(app);
    let (x, y) = terminal_link_position(app, action);
    app.keyboard.set_mod_state(modifiers);
    app.send(
        Event::MouseButtonDown {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x,
            y,
        },
        true,
    );
    app.keyboard.set_mod_state(Mod::NOMOD);
    app.send(
        Event::MouseButtonUp {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x: x + if drag { 20.0 } else { 0.0 },
            y,
        },
        true,
    );
}

fn open_folder_fixture(app: &mut Fixture<'_>, folder: &FolderFixture) {
    open_folder_workspace(
        &folder.0,
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
}

#[test]
#[ignore = "Headless SDL terminal input; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn terminal_typed_paths_open_in_the_terminal_pane() {
    let folder = FolderFixture::new();
    let original = folder.0.join("original.txt");
    std::fs::write(folder.0.join("notes with spaces.txt"), "notes\n").unwrap();
    with_fixture(|app| {
        for split in [false, true] {
            for pane in [0, 1] {
                if !split && pane == 1 {
                    continue;
                }
                for (input, contents, read_only) in [
                    ("./sample.txt", "project file\n", false),
                    ("\"notes with spaces.txt\"", "notes\n", false),
                    ("edit ./sample.txt:1:3", "project file\n", false),
                    ("view ./sample.txt", "project file\n", true),
                ] {
                    app.editor = Editor::new(app.editor.config.clone()).unwrap();
                    app.other_editor = Editor::new(app.other_editor.config.clone()).unwrap();
                    open_folder_fixture(app, &folder);
                    focus_pane(
                        pane,
                        &mut app.active_pane,
                        &mut app.editor,
                        &mut app.other_editor,
                        &mut app.vim,
                        &mut app.other_vim,
                        &mut app.renderer,
                    );
                    app.renderer.place_terminal_in_active_pane();
                    app.split_mode = split;
                    app.renderer.set_split_mode(split);
                    std::fs::write(&original, "original").unwrap();
                    app.editor.open(original.to_str().unwrap()).unwrap();
                    app.editor.insert_text("saved ").unwrap();
                    app.other_editor.insert_text("other pane draft").unwrap();
                    app.text(input);
                    app.key(Keycode::Return, Mod::NOMOD);
                    assert_eq!(app.active_pane, pane, "{input}");
                    assert_eq!(app.split_mode, split);
                    assert!(!app.terminal.is_active(), "{:?}", app.terminal.status());
                    assert_eq!(app.editor.document.text().unwrap(), contents);
                    assert_eq!(app.editor.is_read_only(), read_only);
                    assert_eq!(
                        app.other_editor.document.text().unwrap(),
                        "other pane draft"
                    );
                    assert!(app.other_editor.is_dirty());
                    assert_eq!(
                        std::fs::read_to_string(&original).unwrap(),
                        "saved original"
                    );
                    if input.starts_with("edit ") {
                        assert_eq!(app.editor.document.cursor_line_column().unwrap(), (0, 2));
                    }
                }
            }
        }
    });
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
                    focus_pane(
                        pane,
                        &mut app.active_pane,
                        &mut app.editor,
                        &mut app.other_editor,
                        &mut app.vim,
                        &mut app.other_vim,
                        &mut app.renderer,
                    );
                    app.renderer.place_terminal_in_active_pane();
                    app.split_mode = split;
                    app.renderer.set_split_mode(split);
                    click_terminal_action(app, &file, modifiers, true);
                    assert_eq!(app.split_mode, split, "dragging must not open a split");
                    assert!(app.editor.path().is_none() && app.other_editor.path().is_none());
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
            focus_pane(
                0,
                &mut app.active_pane,
                &mut app.editor,
                &mut app.other_editor,
                &mut app.vim,
                &mut app.other_vim,
                &mut app.renderer,
            );
            app.split_mode = false;
            app.renderer.set_split_mode(false);
            click_terminal_action(
                app,
                &TerminalAction::EnterDirectory(folder.0.join("nested folder")),
                modifiers,
                false,
            );
            finish_folder_listing(app);
            assert!(!app.split_mode);
            assert!(app.terminal.is_active());
            assert_eq!(
                app.terminal.resolve_path("new.txt").unwrap(),
                folder.0.join("nested folder/new.txt")
            );
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
        assert!(
            app.terminal
                .status()
                .unwrap()
                .contains("selected pane has unsaved")
        );
        assert_eq!(app.editor.document.text().unwrap(), "hidden unsaved");
        assert!(app.other_editor.document.is_empty());
        // Modified clicks must not fall back into the terminal's clean document either.
        app.editor = Editor::new(app.editor.config.clone()).unwrap();
        app.other_editor.insert_text("destination unsaved").unwrap();
        click_terminal_action(app, &file, Mod::LGUIMOD, false);
        assert_eq!(app.active_pane, 0);
        assert!(app.terminal.is_active());
        assert!(
            app.terminal
                .status()
                .unwrap()
                .contains("selected pane has unsaved")
        );
        assert!(app.editor.document.is_empty());
        assert_eq!(
            app.other_editor.document.text().unwrap(),
            "destination unsaved"
        );
        // An already-open file keeps its in-memory changes and undo history.
        app.other_editor
            .open(folder.0.join("sample.txt").to_str().unwrap())
            .unwrap();
        app.other_editor.insert_text("unsaved ").unwrap();
        let text = app.other_editor.document.text().unwrap();
        let history = app.other_editor.undo_stack.len();
        click_terminal_action(app, &file, Mod::LCTRLMOD, false);
        assert_eq!(app.active_pane, 1);
        assert_eq!(app.editor.document.text().unwrap(), text);
        assert_eq!(app.editor.undo_stack.len(), history);
        assert!(app.terminal.is_active());
        assert!(app.other_editor.path().is_none());
    });
}

#[test]
#[ignore = "Headless SDL terminal clicks; run with SDL_VIDEODRIVER=dummy in a separate process"]
fn terminal_clicks_autosave_named_files_in_the_selected_pane() {
    for modifiers in [Mod::NOMOD, Mod::LCTRLMOD, Mod::LGUIMOD] {
        let folder = FolderFixture::new();
        let original = folder.0.join("original.txt");
        std::fs::write(&original, "original file").unwrap();
        let file = TerminalAction::ListedFile(folder.0.join("sample.txt"));
        with_fixture(|app| {
            open_folder_fixture(app, &folder);
            // Folder opening leaves the right pane active, with the terminal left.
            let target = if modifiers == Mod::NOMOD {
                &mut app.other_editor
            } else {
                &mut app.editor
            };
            target.open(original.to_str().unwrap()).unwrap();
            target.insert_text("saved ").unwrap();
            click_terminal_action(app, &file, modifiers, false);
            assert_eq!(
                std::fs::read_to_string(&original).unwrap(),
                "saved original file"
            );
            assert_eq!(app.editor.document.text().unwrap(), "project file\n");
            assert_eq!(app.active_pane, if modifiers == Mod::NOMOD { 0 } else { 1 });
            assert!(!app.editor.is_dirty());
        });
    }
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
        assert_eq!(app.editor.path(), app.other_editor.path());
        app.editor.document.move_cursor(0).unwrap();
        app.text("left ");
        assert_eq!(
            app.editor.document.text().unwrap(),
            app.other_editor.document.text().unwrap()
        );
        focus_pane(
            1,
            &mut app.active_pane,
            &mut app.editor,
            &mut app.other_editor,
            &mut app.vim,
            &mut app.other_vim,
            &mut app.renderer,
        );
        app.key(Keycode::Z, Mod::LCTRLMOD);
        assert_eq!(app.editor.document.text().unwrap(), expected);
        assert_eq!(
            app.editor.document.text().unwrap(),
            app.other_editor.document.text().unwrap()
        );
        app.key(Keycode::S, Mod::LCTRLMOD);
        assert!(!app.editor.is_dirty() && !app.other_editor.is_dirty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);
        // Reopen terminal over this view and Ctrl-click the same file into a clean empty pane.
        app.other_editor = Editor::new(app.other_editor.config.clone()).unwrap();
        app.renderer.place_terminal_in_active_pane();
        app.terminal.open(None);
        click_terminal_action(app, &file, Mod::LCTRLMOD, false);
        assert_eq!(app.active_pane, 0);
        assert!(app.editor.shares_document_with(&app.other_editor));
        app.text("right ");
        assert_eq!(
            app.editor.document.text().unwrap(),
            app.other_editor.document.text().unwrap()
        );
        // Closing a view leaves the live document and its history usable.
        app.terminal.close_to_editor();
        assert!(close_focused_pane(
            &mut app.split_mode,
            &mut app.active_pane,
            &mut app.editor,
            &mut app.other_editor,
            &mut app.vim,
            &mut app.other_vim,
            &mut app.renderer,
            &mut app.terminal
        ));
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
        focus_pane(
            1,
            &mut app.active_pane,
            &mut app.editor,
            &mut app.other_editor,
            &mut app.vim,
            &mut app.other_vim,
            &mut app.renderer,
        );
        app.key(Keycode::I, Mod::NOMOD);
        app.text("i");
        app.key(Keycode::C, Mod::NOMOD);
        app.text("cc");
        app.key(Keycode::Escape, Mod::NOMOD);
        focus_pane(
            0,
            &mut app.active_pane,
            &mut app.editor,
            &mut app.other_editor,
            &mut app.vim,
            &mut app.other_vim,
            &mut app.renderer,
        );
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
        let left = if app.active_pane == 1 && app.split_mode {
            app.renderer.split_divider()
        } else {
            0
        };
        for x in (left..800).step_by(2) {
            if app
                .renderer
                .cursor_target_at(&mut app.editor.document, x, y)
                .unwrap()
                == Some((line, column))
            {
                return (x as f32, y as f32);
            }
        }
    }
    panic!("no mouse target for {line}:{column}");
}

fn mouse_press(app: &mut Fixture<'_>, point: (f32, f32), clicks: u8, modifiers: Mod) {
    app.keyboard.set_mod_state(modifiers);
    app.send(
        Event::MouseButtonDown {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks,
            x: point.0,
            y: point.1,
        },
        true,
    );
}

fn mouse_move(app: &mut Fixture<'_>, point: (f32, f32)) {
    app.send(
        Event::MouseMotion {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mousestate: sdl3::mouse::MouseState::from_sdl_state(1),
            x: point.0,
            y: point.1,
            xrel: 0.0,
            yrel: 0.0,
        },
        true,
    );
}

fn mouse_release(app: &mut Fixture<'_>, point: (f32, f32)) {
    app.send(
        Event::MouseButtonUp {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x: point.0,
            y: point.1,
        },
        true,
    );
    app.keyboard.set_mod_state(Mod::NOMOD);
}

fn selected_document_text(app: &Fixture<'_>) -> String {
    let doc = &app.editor.document;
    String::from_utf8(
        doc.read_range(
            doc.selection_start(),
            doc.selection_end() - doc.selection_start(),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
#[ignore = "Headless SDL mouse selection; run with SDL_VIDEODRIVER=dummy"]
fn mouse_interactions_select_edit_extend_words_and_lines() {
    with_fixture(|app| {
        app.editor
            .insert_text("hello café_東京!\nsecond line\nthird")
            .unwrap();
        app.editor.document.move_cursor(0).unwrap();
        app.renderer
            .render(&mut app.editor.document, &app.search_ui, &app.command_bar)
            .unwrap();
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
        assert_eq!(
            app.editor.document.text().unwrap(),
            "hello café_東京!\nreplacement\nthird"
        );
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
        app.renderer
            .render_split(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
        mouse_press(app, (400.0, 150.0), 1, Mod::NOMOD);
        mouse_move(app, (280.0, 150.0));
        mouse_release(app, (280.0, 150.0));
        assert_eq!(app.renderer.split_divider(), 280);
        assert_eq!(app.renderer.pane_at(300), 1);
        app.renderer
            .render_split(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
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
            editor
                .insert_text(&format!("{}\n", "long ".repeat(100)).repeat(80))
                .unwrap();
            editor.document.move_cursor(0).unwrap();
        }
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
        let wheel = |app: &mut Fixture<'_>, pane, x, y| {
            app.send(
                Event::MouseWheel {
                    timestamp: 0,
                    window_id: 0,
                    which: 0,
                    x,
                    y,
                    direction: sdl3::mouse::MouseWheelDirection::Normal,
                    mouse_x: pane as f32 * 400.0 + 200.0,
                    mouse_y: 60.0,
                    integer_x: 0,
                    integer_y: 0,
                },
                true,
            );
        };
        for _ in 0..3 {
            wheel(app, 0, 0.0, -0.25);
        }
        wheel(app, 1, 0.0, -0.25);
        assert_eq!(
            app.renderer
                .cursor_target_at(&mut app.editor.document, 600, 50)
                .unwrap()
                .unwrap()
                .0,
            0
        );
        wheel(app, 0, -0.25, -0.25);
        assert_eq!(
            app.renderer
                .cursor_target_at(&mut app.editor.document, 200, 50)
                .unwrap()
                .unwrap()
                .0,
            1
        );
        let before = app
            .renderer
            .cursor_target_at(&mut app.editor.document, 200, 50)
            .unwrap()
            .unwrap()
            .1;
        wheel(app, 0, -0.25, 0.0);
        let after = app
            .renderer
            .cursor_target_at(&mut app.editor.document, 200, 50)
            .unwrap()
            .unwrap()
            .1;
        assert!(
            after > before,
            "fractional horizontal wheel movement must not disappear"
        );
    });
}

#[test]
#[ignore = "Headless SDL Vim mouse selection; run with SDL_VIDEODRIVER=dummy"]
fn mouse_interactions_vim_can_yank_and_delete_mouse_selection() {
    with_fixture(|app| {
        app.editor.insert_text("one two three").unwrap();
        app.mode(KeybindingMode::Vim);
        app.renderer
            .render(&mut app.editor.document, &app.search_ui, &app.command_bar)
            .unwrap();
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
        app.editor
            .insert_text(&"line of code\n".repeat(150))
            .unwrap();
        app.editor.document.move_cursor(0).unwrap();
        app.renderer
            .render(&mut app.editor.document, &app.search_ui, &app.command_bar)
            .unwrap();
        let start = document_mouse_point(app, 0, 0);
        mouse_press(app, start, 1, Mod::NOMOD);
        mouse_move(app, (start.0, 620.0));
        let before = app.editor.document.cursor.line;
        assert!(app.mouse_state.wait_timeout() <= std::time::Duration::from_millis(40));
        std::thread::sleep(std::time::Duration::from_millis(45));
        assert!(
            app.mouse_state
                .tick(&mut app.editor, &mut app.renderer, &mut app.vim, false, 0)
                .unwrap()
        );
        assert!(app.editor.document.cursor.line > before);
        mouse_release(app, (start.0, 620.0));
        assert_eq!(app.mouse_state.wait_timeout(), std::time::Duration::MAX);
        mouse_press(app, start, 1, Mod::NOMOD);
        mouse_move(app, (start.0, 620.0));
        app.send(
            Event::Window {
                timestamp: 0,
                window_id: 0,
                win_event: WindowEvent::FocusLost,
            },
            true,
        );
        assert_eq!(app.mouse_state.wait_timeout(), std::time::Duration::MAX);
        assert!(
            !app.mouse_state
                .tick(&mut app.editor, &mut app.renderer, &mut app.vim, false, 0)
                .unwrap()
        );
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

#[test]
#[ignore = "Headless SDL resize cursors; run with SDL_VIDEODRIVER=dummy"]
fn mouse_interactions_window_edges_show_resize_cursors_without_editing() {
    use sdl3::mouse::SystemCursor::*;
    with_fixture(|app| {
        app.editor.insert_text("unchanged document").unwrap();
        let position = app.editor.document.cursor.position;
        for ((x, y), expected) in [
            ((2.0, 300.0), SizeWE),
            ((798.0, 300.0), SizeWE),
            ((400.0, 2.0), SizeNS),
            ((400.0, 598.0), SizeNS),
            ((2.0, 2.0), SizeNWSE),
            ((798.0, 598.0), SizeNWSE),
            ((798.0, 2.0), SizeNESW),
            ((2.0, 598.0), SizeNESW),
        ] {
            mouse_move(app, (x, y));
            assert_eq!(app.mouse_state.cursor_kind(), Some(expected));
            mouse_press(app, (x, y), 1, Mod::NOMOD);
            assert_eq!(app.editor.document.cursor.position, position);
            mouse_release(app, (x, y));
        }
        mouse_move(app, (100.0, 100.0));
        assert_eq!(app.mouse_state.cursor_kind(), Some(Arrow));
        app.split_mode = true;
        app.renderer.set_split_mode(true);
        mouse_move(app, (400.0, 120.0));
        assert_eq!(app.mouse_state.cursor_kind(), Some(SizeWE));
        app.send(
            Event::Window {
                timestamp: 0,
                window_id: 0,
                win_event: WindowEvent::MouseLeave,
            },
            true,
        );
        assert_eq!(app.mouse_state.cursor_kind(), Some(Arrow));
        assert_eq!(app.editor.document.text().unwrap(), "unchanged document");
    });
}

fn experimental_wait_idle(app: &mut Fixture<'_>) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        super::super::experimental::poll(&mut app.renderer, &mut app.vim, &mut app.other_vim);
        if app.renderer.experimental_ready() && !app.renderer.experimental_running() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "managed terminal did not finish: {}",
            app.renderer.experimental_contents()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "SDL embedded terminal integration; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_embeds_and_routes_input_without_changing_the_document() {
    with_fixture(|app| {
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.text("keep this document");
        app.terminal.insert_text("legacy draft");
        app.execute_command(":term-new", false);
        assert!(app.renderer.experimental_focused());
        assert!(!app.terminal.is_active());
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.renderer.experimental_ready() {
            super::super::experimental::poll(&mut app.renderer, &mut app.vim, &mut app.other_vim);
            assert!(Instant::now() < deadline, "shell did not initialize");
            std::thread::sleep(Duration::from_millis(5));
        }
        #[cfg(unix)]
        let variable = "export POTYI_VALUE=retained";
        #[cfg(windows)]
        let variable = "set POTYI_VALUE=retained";
        experimental_wait_idle(app);
        app.text(variable);
        app.key(Keycode::Return, Mod::NOMOD);
        experimental_wait_idle(app);
        #[cfg(unix)]
        {
            // Like :term, ordinary commands have EOF stdin and command-local
            // environment. The editable prompt remains usable while they run.
            app.text("read POTYI_REPLY");
            app.key(Keycode::Return, Mod::NOMOD);
            experimental_wait_idle(app);
            app.text("sleep 0.2");
            app.key(Keycode::Return, Mod::NOMOD);
            app.text("draft");
            experimental_wait_idle(app);
            assert_eq!(app.renderer.experimental_prompt(), "draft");
            for _ in 0..5 {
                app.key(Keycode::Backspace, Mod::NOMOD);
            }
            app.text("printf 'ANSWER_%s\\n' \"${POTYI_VALUE-unset}\"");
            app.key(Keycode::Return, Mod::NOMOD);
            experimental_wait_idle(app);
            assert!(
                app.renderer
                    .experimental_contents()
                    .lines()
                    .any(|line| line == "ANSWER_unset")
            );
        }
        #[cfg(windows)]
        {
            app.text(
                "if defined POTYI_VALUE (echo FAILED_PERSISTENCE) else (echo WIN_FRESH_READY)",
            );
            app.key(Keycode::Return, Mod::NOMOD);
            experimental_wait_idle(app);
            assert!(
                app.renderer
                    .experimental_contents()
                    .lines()
                    .any(|line| line.trim() == "WIN_FRESH_READY")
            );
        }
        experimental_wait_idle(app);
        app.renderer
            .render_experimental(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &mut app.terminal,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
        assert_eq!(app.editor.document.text().unwrap(), "keep this document");
        assert_eq!(app.terminal.input(), "legacy draft");
        app.key(Keycode::Grave, Mod::LCTRLMOD);
        assert!(!app.renderer.experimental_visible());
        app.execute_command(":term-new", false);
        assert!(app.renderer.experimental_focused());
        #[cfg(unix)]
        assert!(
            app.renderer
                .experimental_contents()
                .contains("ANSWER_unset")
        );
        #[cfg(windows)]
        assert!(
            app.renderer
                .experimental_contents()
                .contains("WIN_FRESH_READY")
        );
        app.key(Keycode::P, Mod::LCTRLMOD);
        assert!(app.command_bar.is_active());
    });
}

#[test]
#[ignore = "SDL embedded terminal integration; build the application first"]
fn experimental_terminal_shell_edit_view_keep_the_terminal_and_protect_unsaved_documents() {
    with_fixture(|app| {
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        let root = std::env::temp_dir().join(format!("potyi-embedded-file-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("é notes.rs");
        std::fs::write(&path, "first\nsecond\nthird\n").unwrap();
        app.execute_command(":split", false);
        app.execute_command(":term-new", false);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.renderer.experimental_ready() {
            super::super::experimental::poll(&mut app.renderer, &mut app.vim, &mut app.other_vim);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        #[cfg(unix)]
        let change = format!("cd \"{}\"", root.display());
        #[cfg(windows)]
        let change = format!("cd /d \"{}\"", root.display());
        app.text(&change);
        app.key(Keycode::Return, Mod::NOMOD);
        while app.renderer.experimental_directory() != Some(root.canonicalize().unwrap().as_path())
        {
            super::super::experimental::poll(&mut app.renderer, &mut app.vim, &mut app.other_vim);
            assert!(app.renderer.experimental_request().is_none());
            assert!(
                Instant::now() < deadline,
                "{}",
                app.renderer.experimental_contents()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        experimental_wait_idle(app);
        app.text("view \"é notes.rs:2:1\"; pwd");
        app.key(Keycode::Return, Mod::NOMOD);
        let request = loop {
            super::super::experimental::poll(&mut app.renderer, &mut app.vim, &mut app.other_vim);
            if let Some(link) = app.renderer.experimental_request() {
                break link;
            }
            assert!(
                Instant::now() < deadline,
                "{}",
                app.renderer.experimental_contents()
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        super::super::experimental::open_link(
            request,
            &mut app.editor,
            &mut app.other_editor,
            &mut app.renderer,
            &mut app.terminal,
            &mut app.active_pane,
            app.split_mode,
            &mut app.vim,
            &mut app.other_vim,
        );
        assert_eq!(
            app.editor.path().as_ref().unwrap().canonicalize().unwrap(),
            path.canonicalize().unwrap()
        );
        assert!(app.editor.is_read_only());
        assert_eq!(app.editor.document.cursor.line, 1);
        assert!(app.renderer.experimental_visible());
        assert!(!app.renderer.experimental_focused());
        app.editor = Editor::new(app.editor.config.clone()).unwrap();
        app.editor.insert_text("unsaved").unwrap();
        let link = crate::experimental_terminal::links::FileLink {
            path: path.clone(),
            line: Some(2),
            column: Some(1),
            byte_column: false,
            read_only: false,
            other_pane: true,
        };
        super::super::experimental::open_link(
            link,
            &mut app.editor,
            &mut app.other_editor,
            &mut app.renderer,
            &mut app.terminal,
            &mut app.active_pane,
            app.split_mode,
            &mut app.vim,
            &mut app.other_vim,
        );
        assert_eq!(app.editor.document.text().unwrap(), "unsaved");
        assert!(app.editor.path().is_none());
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[test]
#[ignore = "SDL embedded terminal integration; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_pane_chords_do_not_send_their_text_to_the_shell() {
    with_fixture(|app| {
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.mode(KeybindingMode::Vim);
        app.execute_command(":split", false);
        app.execute_command(":term-new", false);
        app.key(Keycode::W, Mod::LCTRLMOD);
        app.key(Keycode::W, Mod::NOMOD);
        app.text("w");
        assert!(!app.renderer.experimental_focused());
        app.key(Keycode::W, Mod::LCTRLMOD);
        app.key(Keycode::W, Mod::NOMOD);
        app.text("w");
        assert!(app.renderer.experimental_focused());
        assert_eq!(app.editor.document.text().unwrap(), "");
        assert_eq!(app.other_editor.document.text().unwrap(), "");
    });
}

fn experimental_open_requests(app: &mut Fixture<'_>) {
    super::super::experimental::poll(&mut app.renderer, &mut app.vim, &mut app.other_vim);
    while let Some(link) = app.renderer.experimental_request() {
        super::super::experimental::open_link(
            link,
            &mut app.editor,
            &mut app.other_editor,
            &mut app.renderer,
            &mut app.terminal,
            &mut app.active_pane,
            app.split_mode,
            &mut app.vim,
            &mut app.other_vim,
        );
    }
}

#[test]
#[ignore = "SDL embedded terminal browsing; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_native_browsing_completion_quick_open_and_return() {
    with_fixture(|app| {
        let root = std::env::temp_dir().join(format!("potyi-native-browse-{}", std::process::id()));
        std::fs::create_dir_all(root.join("folder")).unwrap();
        std::fs::write(root.join("é notes.txt"), "first\nsecond\n").unwrap();
        std::fs::write(root.join(".hidden"), "hidden").unwrap();
        std::fs::write(root.join("binary"), [0, 1, 2, 0]).unwrap();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.editor.insert_text("unsaved document").unwrap();
        app.renderer.open_experimental(&root, Some("ls")).unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        app.renderer
            .render_experimental(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &mut app.terminal,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
        let rows = app.renderer.experimental_browser_rows();
        assert!(rows.iter().any(|row| row.path.as_deref()
            == Some(root.canonicalize().unwrap().join(".hidden").as_path())));
        assert!(
            rows.iter()
                .any(|row| row.kind == crate::experimental_terminal::browser::Kind::Binary)
        );
        app.text("é");
        app.key(Keycode::Tab, Mod::NOMOD);
        assert!(app.renderer.experimental_prompt().contains("é notes.txt"));
        app.key(Keycode::Return, Mod::NOMOD);
        experimental_open_requests(app);
        assert_eq!(app.editor.document.text().unwrap(), "unsaved document");
        assert!(app.renderer.experimental_focused());
        app.editor = Editor::new(app.editor.config.clone()).unwrap();
        app.text("\"é notes.txt\"");
        app.key(Keycode::Return, Mod::NOMOD);
        experimental_open_requests(app);
        assert_eq!(
            app.editor.path().as_ref().unwrap().canonicalize().unwrap(),
            root.join("é notes.txt").canonicalize().unwrap()
        );
        assert!(!app.renderer.experimental_visible());
        app.key(Keycode::Grave, Mod::LCTRLMOD);
        assert!(app.renderer.experimental_focused());
        assert_eq!(
            app.renderer.experimental_directory(),
            Some(root.canonicalize().unwrap().as_path())
        );
        app.renderer
            .render_experimental(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &mut app.terminal,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
        let folder = root.join("folder").canonicalize().unwrap();
        let (x, y) = app.renderer.experimental_path_point(&folder).unwrap();
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
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
                y,
            },
            true,
        );
        experimental_wait_idle(app);
        assert_eq!(
            app.renderer.experimental_directory(),
            Some(folder.as_path())
        );
        app.key(Keycode::Escape, Mod::NOMOD);
        app.text("cd ..");
        app.key(Keycode::Return, Mod::NOMOD);
        experimental_wait_idle(app);
        app.renderer
            .render_experimental(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &mut app.terminal,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
        app.key(Keycode::Escape, Mod::NOMOD);
        assert!(!app.renderer.experimental_visible());
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[cfg(unix)]
#[test]
#[ignore = "SDL managed terminal stdin ownership; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_directory_click_never_writes_into_a_running_program() {
    with_fixture(|app| {
        let root = std::env::temp_dir().join(format!("potyi-safe-input-{}", std::process::id()));
        std::fs::create_dir_all(root.join("next")).unwrap();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer.open_experimental(&root, None).unwrap();
        experimental_wait_idle(app);
        app.text("sleep 5");
        app.key(Keycode::Return, Mod::NOMOD);
        assert!(app.renderer.experimental_running());
        let link = crate::experimental_terminal::links::FileLink {
            path: root.join("next"),
            line: None,
            column: None,
            byte_column: false,
            read_only: false,
            other_pane: false,
        };
        super::super::experimental::open_link(
            link,
            &mut app.editor,
            &mut app.other_editor,
            &mut app.renderer,
            &mut app.terminal,
            &mut app.active_pane,
            app.split_mode,
            &mut app.vim,
            &mut app.other_vim,
        );
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(
            app.renderer.experimental_directory(),
            Some(root.canonicalize().unwrap().as_path())
        );
        app.text("my answer");
        app.key(Keycode::Return, Mod::NOMOD);
        assert_eq!(app.renderer.experimental_prompt(), "my answer");
        assert!(app.renderer.experimental_running());
        app.key(Keycode::C, Mod::LCTRLMOD);
        experimental_wait_idle(app);
        assert_eq!(app.renderer.experimental_prompt(), "my answer");
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[cfg(unix)]
#[test]
#[ignore = "SDL asynchronous browsing cancellation; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_failed_or_cancelled_cd_does_not_launch_a_queued_command() {
    with_fixture(|app| {
        let root = std::env::temp_dir().join(format!("potyi-cancel-cd-{}", std::process::id()));
        std::fs::create_dir_all(root.join("next")).unwrap();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer.open_experimental(&root, None).unwrap();
        experimental_wait_idle(app);
        app.text("cd nonexistent");
        app.key(Keycode::Return, Mod::NOMOD);
        app.text("printf wrong > unintended.txt");
        app.key(Keycode::Return, Mod::NOMOD);
        experimental_wait_idle(app);
        assert!(!root.join("unintended.txt").exists());
        app.text("cd next");
        app.key(Keycode::Return, Mod::NOMOD);
        app.text("printf cancelled > cancelled.txt");
        app.key(Keycode::Return, Mod::NOMOD);
        app.key(Keycode::C, Mod::LCTRLMOD);
        experimental_wait_idle(app);
        assert!(!root.join("cancelled.txt").exists());
        assert!(!root.join("next/cancelled.txt").exists());
        app.text("cd next");
        app.key(Keycode::Return, Mod::NOMOD);
        app.text("printf cleared > cleared.txt");
        app.key(Keycode::Return, Mod::NOMOD);
        app.key(Keycode::L, Mod::LCTRLMOD);
        experimental_wait_idle(app);
        assert!(!root.join("cleared.txt").exists());
        assert!(!root.join("next/cleared.txt").exists());
        app.text("cd next");
        app.key(Keycode::Return, Mod::NOMOD);
        app.text("printf correct > correct.txt");
        app.key(Keycode::Return, Mod::NOMOD);
        experimental_wait_idle(app);
        assert_eq!(
            std::fs::read_to_string(root.join("next/correct.txt")).unwrap(),
            "correct"
        );
        assert!(!root.join("correct.txt").exists());
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[test]
#[ignore = "SDL queued editor return and glyph release; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_queued_editor_return_redraws_and_releases_glyphs() {
    with_fixture(|app| {
        let root = std::env::temp_dir().join(format!("potyi-queued-editor-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer.open_experimental(&root, None).unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        assert!(app.renderer.experimental_cached_glyphs());
        app.renderer.experimental_send("ls").unwrap();
        app.renderer.experimental_send("editor").unwrap();
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut repaint = false;
        while app.renderer.experimental_visible() {
            repaint = super::super::experimental::poll(
                &mut app.renderer,
                &mut app.vim,
                &mut app.other_vim,
            );
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(repaint, "hidden transition must request a redraw");
        assert!(!app.renderer.experimental_cached_glyphs());
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[cfg(unix)]
#[test]
#[ignore = "SDL unified browsing/command transcript; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_unified_transcript_keeps_listing_command_tail_and_next_listing() {
    with_fixture(|app| {
        let root = std::env::temp_dir().join(format!("potyi-unified-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("listing-marker.txt"), "file\n").unwrap();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer.open_experimental(&root, Some("ls")).unwrap();
        experimental_wait_idle(app);
        app.renderer.experimental_send("printf 'UNI_%s\\n' FIRST; i=0; while [ $i -lt 300 ]; do printf 'row:%s\\n' \"$i\"; i=$((i+1)); done; printf 'TAIL_%s' LAST").unwrap();
        experimental_wait_idle(app);
        app.renderer.experimental_send("pwd").unwrap();
        experimental_wait_idle(app);
        app.renderer.experimental_send("ls -1").unwrap();
        experimental_wait_idle(app);
        let text = app.renderer.experimental_contents();
        assert!(
            text.find("listing-marker.txt").unwrap() < text.find("UNI_FIRST").unwrap(),
            "{text}"
        );
        assert_eq!(text.matches("UNI_FIRST").count(), 1, "{text}");
        assert_eq!(text.matches("row:299").count(), 1, "{text}");
        assert_eq!(text.matches("TAIL_LAST").count(), 1, "{text}");
        assert!(
            text.find("TAIL_LAST").unwrap() < text.rfind("listing-marker.txt").unwrap(),
            "{text}"
        );
        app.renderer
            .render_experimental(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &mut app.terminal,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
        assert!(
            app.renderer
                .experimental_browser_rows()
                .iter()
                .any(|row| row.text.contains("listing-marker.txt"))
        );
        app.key(Keycode::L, Mod::LCTRLMOD);
        assert!(!app.renderer.experimental_contents().contains("UNI_FIRST"));
        std::fs::remove_dir_all(root).unwrap();
    });
}

fn experimental_wait_output(app: &mut Fixture<'_>) {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        super::super::experimental::poll(&mut app.renderer, &mut app.vim, &mut app.other_vim);
        app.renderer
            .render_experimental(
                &mut app.editor.document,
                &mut app.other_editor.document,
                &mut app.terminal,
                &app.search_ui,
                &app.command_bar,
            )
            .unwrap();
        if !app.renderer.experimental_output_pending() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "output layout/navigation did not settle"
        );
        std::thread::sleep(Duration::from_millis(3));
    }
}

#[test]
#[ignore = "SDL projected file columns; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_column_click_opens_on_release_and_drag_keeps_pane() {
    with_fixture(|app| {
        let root = std::env::temp_dir().join(format!("potyi-columns-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for name in ["a.txt", "b.txt", "c.txt", "d.txt"] {
            std::fs::write(root.join(name), name).unwrap();
        }
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer.open_experimental(&root, Some("ls")).unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        let rows = app.renderer.experimental_browser_rows();
        let points: Vec<_> = rows
            .iter()
            .filter_map(|row| {
                row.path
                    .as_ref()
                    .filter(|path| path.is_file())
                    .and_then(|path| {
                        app.renderer
                            .experimental_path_point(path)
                            .map(|point| (path.clone(), point))
                    })
            })
            .collect();
        let pair = points
            .iter()
            .enumerate()
            .find_map(|(i, a)| {
                points
                    .iter()
                    .skip(i + 1)
                    .find(|b| a.1.1 == b.1.1 && a.1.0 != b.1.0)
                    .map(|b| (a.clone(), b.clone()))
            })
            .expect("small listing should use columns");
        let (x, y) = pair.0.1;
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
            },
            true,
        );
        assert!(app.renderer.experimental_visible());
        app.send(
            Event::MouseMotion {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mousestate: sdl3::mouse::MouseState::from_sdl_state(1),
                x: pair.1.1.0,
                y,
                xrel: 4.0,
                yrel: 0.0,
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
                x: pair.1.1.0,
                y,
            },
            true,
        );
        experimental_wait_output(app);
        assert!(app.renderer.experimental_visible());
        assert!(app.editor.path().is_none());
        app.key(Keycode::Escape, Mod::NOMOD);
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
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
                y,
            },
            true,
        );
        assert_eq!(
            app.editor.path().as_ref().unwrap().canonicalize().unwrap(),
            pair.0.0.canonicalize().unwrap()
        );
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[cfg(unix)]
#[test]
#[ignore = "SDL output navigation and copy; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_output_vim_unicode_copy_and_prompt_focus() {
    with_fixture(|app| {
        app.mode(KeybindingMode::Vim);
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.execute_command(":term-new", false);
        experimental_wait_idle(app);
        app.renderer
            .experimental_send("printf 'alpha界 beta\\nsecond row\\n'")
            .unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        let (x, y) = app
            .renderer
            .experimental_text_point("alpha界 beta")
            .unwrap();
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 2,
                x,
                y,
            },
            true,
        );
        app.send(
            Event::MouseButtonUp {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 2,
                x,
                y,
            },
            true,
        );
        experimental_wait_output(app);
        app.key(Keycode::C, Mod::LGUIMOD);
        experimental_wait_output(app);
        assert_eq!(app.clipboard.clipboard_text().unwrap(), "alpha界");
        app.key(Keycode::Escape, Mod::NOMOD);
        assert!(app.renderer.experimental_visible());
        app.text("draft");
        assert_eq!(app.renderer.experimental_prompt(), "draft");
        app.key(Keycode::C, Mod::LCTRLMOD);
        assert_eq!(app.renderer.experimental_prompt(), "draft");
        for _ in 0..5 {
            app.key(Keycode::Backspace, Mod::NOMOD);
        }
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
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
                y,
            },
            true,
        );
        experimental_wait_output(app);
        app.clipboard.set_clipboard_text("keep clipboard").unwrap();
        app.key(Keycode::Y, Mod::NOMOD);
        app.text("y");
        experimental_wait_output(app);
        assert_eq!(app.clipboard.clipboard_text().unwrap(), "keep clipboard");
        app.key(Keycode::V, Mod::NOMOD);
        app.text("v");
        app.key(Keycode::W, Mod::NOMOD);
        app.text("w");
        app.key(Keycode::Y, Mod::NOMOD);
        app.text("y");
        experimental_wait_output(app);
        assert_eq!(app.renderer.experimental_prompt(), "");
        assert_eq!(app.clipboard.clipboard_text().unwrap(), "alpha界 b");
    });
}

#[cfg(unix)]
#[test]
#[ignore = "SDL exact full transcript copy; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_copy_all_reads_saved_output_once_and_clear_cancels_selection() {
    with_fixture(|app| {
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.execute_command(":term-new", false);
        experimental_wait_idle(app);
        app.renderer.experimental_send("i=0; while [ $i -lt 220 ]; do printf 'COPY_%s\\n' \"$i\"; i=$((i+1)); done; printf 'UNICODE_界é'").unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        app.key(Keycode::C, Mod::LGUIMOD);
        experimental_wait_output(app);
        let text = app.clipboard.clipboard_text().unwrap();
        assert_eq!(text.matches("COPY_219").count(), 1);
        assert_eq!(text.matches("COPY_0\n").count(), 1);
        assert_eq!(text.matches("UNICODE_界é").count(), 2, "header plus output");
        let (x, y) = app.renderer.experimental_text_point("UNICODE_界é").unwrap();
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 2,
                x,
                y,
            },
            true,
        );
        app.key(Keycode::L, Mod::LCTRLMOD);
        experimental_wait_output(app);
        assert!(app.renderer.experimental_output_cursor().is_none());
        assert!(app.renderer.experimental_contents().is_empty());
    });
}

#[cfg(windows)]
#[test]
#[ignore = "SDL native Windows command/copy limits; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_windows_unicode_copy_and_fresh_command_environment() {
    with_fixture(|app| {
        let root = std::env::temp_dir().join(format!("potyi-win-copy-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("界é notes.txt"), "hello\n").unwrap();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer.open_experimental(&root, None).unwrap();
        experimental_wait_idle(app);
        app.renderer
            .experimental_send("for /L %i in (1,1,220) do @echo WINCOPY_%i")
            .unwrap();
        experimental_wait_idle(app);
        app.renderer.experimental_send("echo UNICODE_界é").unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        app.key(Keycode::C, Mod::LGUIMOD);
        experimental_wait_output(app);
        let text = app.clipboard.clipboard_text().unwrap();
        assert_eq!(text.matches("WINCOPY_220").count(), 1);
        assert_eq!(text.matches("UNICODE_界é").count(), 2);
        app.renderer
            .experimental_send("set POTYI_ONLY_THIS_COMMAND=retained")
            .unwrap();
        experimental_wait_idle(app);
        app.renderer
            .experimental_send(
                "if defined POTYI_ONLY_THIS_COMMAND (echo BAD_STATE) else (echo FRESH_STATE)",
            )
            .unwrap();
        experimental_wait_idle(app);
        assert!(
            app.renderer
                .experimental_contents()
                .lines()
                .any(|line| line.trim() == "FRESH_STATE")
        );
        app.renderer.experimental_send("ls -1").unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        let (x, y) = app
            .renderer
            .experimental_path_point(&root.canonicalize().unwrap().join("界é notes.txt"))
            .unwrap();
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
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
                y,
            },
            true,
        );
        assert_eq!(app.editor.document.text().unwrap(), "hello\n");
        std::fs::remove_dir_all(root).unwrap();
    });
}

fn experimental_git_fixture() -> (std::path::PathBuf, String) {
    let mut random = [0; 8];
    getrandom::getrandom(&mut random).unwrap();
    let root =
        std::env::temp_dir().join(format!("potyi-pane-git-{:x}", u64::from_le_bytes(random)));
    std::fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(&root)
            .args([
                "-c",
                "user.name=Potyi Test",
                "-c",
                "user.email=potyi@example.invalid",
                "-c",
                "commit.gpgSign=false",
                "-c",
                "core.hooksPath=.no-hooks",
                "-c",
                "core.autocrlf=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };
    git(&["init", "--quiet"]);
    std::fs::write(root.join("space 界.rs"), "first\nsecond\nthird\n").unwrap();
    git(&["add", "--", "space 界.rs"]);
    git(&["commit", "--quiet", "-m", "initial"]);
    std::fs::write(root.join("space 界.rs"), "first\nchanged\nthird\n").unwrap();
    git(&["commit", "--quiet", "-am", "changed"]);
    let hash = git(&["rev-parse", "HEAD"]);
    (root.canonicalize().unwrap(), hash)
}

#[test]
#[ignore = "SDL saved Git Back restoration; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_git_back_restores_saved_output_draft_and_again_after_clear() {
    with_fixture(|app| {
        let (root, hash) = experimental_git_fixture();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer.open_experimental(&root, None).unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        app.text("git log --oneline");
        app.key(Keycode::Return, Mod::NOMOD);
        experimental_wait_idle(app);
        experimental_wait_output(app);
        app.text("unsubmitted draft");
        let before = app.renderer.experimental_contents();
        let stats = app.renderer.experimental_resource_stats();
        let (x, y) = app.renderer.experimental_text_point(&hash[..7]).unwrap();
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
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
                y,
            },
            true,
        );
        assert!(app.renderer.experimental_back_available());
        experimental_wait_idle(app);
        experimental_wait_output(app);
        let detail = app.renderer.experimental_contents();
        assert!(detail.contains("changed"), "{detail}");
        assert_eq!(app.renderer.experimental_prompt(), "");
        app.key(Keycode::L, Mod::LCTRLMOD);
        experimental_wait_output(app);
        assert!(app.renderer.experimental_contents().is_empty());
        assert!(app.renderer.experimental_back_available());
        let (x, y) = app.renderer.experimental_back_point();
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
            },
            true,
        );
        experimental_wait_output(app);
        assert!(!app.renderer.experimental_back_available());
        assert_eq!(app.renderer.experimental_contents(), before);
        assert_eq!(app.renderer.experimental_resource_stats(), stats);
        assert_eq!(app.renderer.experimental_prompt(), "unsubmitted draft");
        assert_eq!(app.renderer.experimental_directory(), Some(root.as_path()));
        app.key(Keycode::Up, Mod::NOMOD);
        assert_eq!(app.renderer.experimental_prompt(), "git log --oneline");
        app.key(Keycode::Down, Mod::NOMOD);
        assert_eq!(app.renderer.experimental_prompt(), "unsubmitted draft");
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[test]
#[ignore = "SDL Git Back during active detail; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_git_alt_left_restores_log_while_detail_is_loading() {
    with_fixture(|app| {
        let (root, hash) = experimental_git_fixture();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer.open_experimental(&root, None).unwrap();
        experimental_wait_idle(app);
        app.renderer.experimental_send("git log --oneline").unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        let before = app.renderer.experimental_contents();
        let (x, y) = app.renderer.experimental_text_point(&hash[..7]).unwrap();
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
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
                y,
            },
            true,
        );
        app.key(Keycode::Left, Mod::LALTMOD);
        experimental_wait_output(app);
        assert!(!app.renderer.experimental_back_available());
        assert_eq!(app.renderer.experimental_contents(), before);
        assert_eq!(app.renderer.experimental_directory(), Some(root.as_path()));
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[cfg(unix)]
#[test]
#[ignore = "SDL keyboard-only output focus and linewise paste; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_keyboard_focus_and_linewise_yank_keep_editor_paste_mode() {
    with_fixture(|app| {
        app.mode(KeybindingMode::Vim);
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.execute_command(":term-new", false);
        experimental_wait_idle(app);
        app.renderer
            .experimental_send("printf 'alpha beta\\nsecond row\\n'")
            .unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        app.key(Keycode::F6, Mod::NOMOD);
        experimental_wait_output(app);
        assert!(app.renderer.experimental_output_cursor().is_some());
        app.key(Keycode::F6, Mod::NOMOD);
        experimental_wait_output(app);
        assert!(app.renderer.experimental_output_cursor().is_none());
        app.key(Keycode::Up, Mod::LSHIFTMOD);
        experimental_wait_output(app);
        assert!(app.renderer.experimental_output_cursor().is_some());
        app.key(Keycode::Escape, Mod::NOMOD);
        let (x, y) = app
            .renderer
            .experimental_terminal_text_point("alpha beta")
            .unwrap();
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
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
                y,
            },
            true,
        );
        experimental_wait_output(app);
        app.key(Keycode::V, Mod::LSHIFTMOD);
        app.key(Keycode::Y, Mod::NOMOD);
        experimental_wait_output(app);
        assert_eq!(app.clipboard.clipboard_text().unwrap(), "alpha beta\n");
        app.key(Keycode::Grave, Mod::LCTRLMOD);
        app.editor.insert_text("editor row\n").unwrap();
        app.editor
            .document
            .move_cursor_to_line_column(0, 0)
            .unwrap();
        app.key(Keycode::P, Mod::NOMOD);
        assert_eq!(
            app.editor.document.text().unwrap(),
            "editor row\nalpha beta\n"
        );
    });
}

#[cfg(unix)]
#[test]
#[ignore = "SDL source-scoped wrapped compiler links; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_wrapped_diagnostic_opens_saved_command_file_and_line() {
    with_fixture(|app| {
        let root = std::env::temp_dir().join(format!("potyi-wrapped-link-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let name = "a long source filename with Unicode 界 and many descriptive words to cross several terminal rows and keep the original path through wrapping into diagnostics.rs";
        std::fs::write(root.join(name), "first\nsecond\nthird\n").unwrap();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.renderer.open_experimental(&root, None).unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        app.renderer
            .experimental_send(&format!("printf 'error: \"{name}:2:3\" failed\\n'"))
            .unwrap();
        experimental_wait_idle(app);
        experimental_wait_output(app);
        let (x, y) = app
            .renderer
            .experimental_terminal_text_point("several")
            .unwrap();
        app.send(
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
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
                y,
            },
            true,
        );
        experimental_wait_output(app);
        experimental_open_requests(app);
        assert_eq!(
            app.editor.path().as_ref().unwrap().canonicalize().unwrap(),
            root.join(name).canonicalize().unwrap()
        );
        assert_eq!(app.editor.document.cursor_line_column().unwrap().0, 1);
        std::fs::remove_dir_all(root).unwrap();
    });
}

#[path = "terminal_startup_probe.rs"]
mod terminal_startup_probe;

#[path = "terminal_resource_probe.rs"]
mod terminal_resource_probe;

#[path = "terminal_visual_probe.rs"]
mod terminal_visual_probe;

#[path = "terminal_keyboard_links.rs"]
mod terminal_keyboard_links;

#[test]
#[ignore = "SDL configured clipboard and non-repeatable terminal actions; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_configured_paste_flattens_lines_and_actions_ignore_repeat() {
    with_fixture(|app| {
        let binding = std::env::temp_dir().join(format!(
            "potyi-terminal-paste-binding-{}.toml",
            std::process::id()
        ));
        std::fs::write(&binding, "[[bindings]]\nkey = \"B\"\nmodifiers = [\"Ctrl\"]\ncommand = \"Paste\"\nrepeatable = true\n").unwrap();
        app.key_bindings = KeyBindings::load(binding.to_str().unwrap()).unwrap();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.execute_command(":term-new", false);
        experimental_wait_idle(app);
        let banner = app.renderer.experimental_contents();
        assert!(banner.starts_with("--- Pötyi terminal"));
        app.clipboard
            .set_clipboard_text("echo first\r\necho second")
            .unwrap();
        app.key(Keycode::F6, Mod::NOMOD);
        experimental_wait_output(app);
        app.key(Keycode::B, Mod::LCTRLMOD);
        assert_eq!(
            app.renderer.experimental_prompt(),
            "echo first  echo second"
        );
        app.key_repeat(Keycode::B, Mod::LCTRLMOD, true);
        app.key_repeat(Keycode::Return, Mod::NOMOD, true);
        app.key_repeat(Keycode::L, Mod::LCTRLMOD, true);
        assert_eq!(
            app.renderer.experimental_prompt(),
            "echo first  echo second"
        );
        assert!(!app.renderer.experimental_running());
        assert_eq!(app.renderer.experimental_contents(), banner);
        app.key(Keycode::C, Mod::LCTRLMOD);
        assert_eq!(
            app.renderer.experimental_prompt(),
            "echo first  echo second"
        );
        app.key(Keycode::Escape, Mod::NOMOD);
        app.key(Keycode::Grave, Mod::LCTRLMOD);
        assert_eq!(
            app.renderer.experimental_prompt(),
            "echo first  echo second"
        );
        app.key(Keycode::Backspace, Mod::NOMOD);
        assert_eq!(app.renderer.experimental_prompt(), "echo first  echo secon");
        std::fs::remove_file(binding).unwrap();
    });
}

#[test]
#[ignore = "SDL command-bar terminal opens use the other split pane; SDL_VIDEODRIVER=dummy"]
fn experimental_terminal_command_bar_file_open_preserves_terminal_and_unsaved_split() {
    with_fixture(|app| {
        let file = std::env::temp_dir().join(format!(
            "potyi-term-command-open-{}.txt",
            std::process::id()
        ));
        std::fs::write(&file, "first\nsecond\n").unwrap();
        app.mode(KeybindingMode::Vim);
        app.execute_command(":split", false);
        app.editor.insert_text("unsaved original").unwrap();
        app.renderer
            .experimental_events(&app.event_subsystem)
            .unwrap();
        app.execute_command(&format!(":term-new view \"{}:2\"", file.display()), false);
        experimental_open_requests(app);
        assert_eq!(
            app.editor.path().as_ref().unwrap().canonicalize().unwrap(),
            file.canonicalize().unwrap()
        );
        assert!(app.editor.is_read_only());
        assert_eq!(app.editor.document.cursor_line_column().unwrap().0, 1);
        assert_eq!(
            app.other_editor.document.text().unwrap(),
            "unsaved original"
        );
        assert!(app.renderer.experimental_visible());
        assert!(!app.renderer.experimental_focused());
        std::fs::remove_file(file).unwrap();
    });
}
