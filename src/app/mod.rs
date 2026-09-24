// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! SDL application setup, event polling, and frame scheduling.
use crate::{
    FONT_DATA,
    clipboard::{copy_selection, cut_selection, paste, read_text},
    command_bar::{self, CommandBar, CommandBarHit, GotoMode, ParsedCommand, quote_argument},
    config::{EditorConfig, KeybindingMode, LineNumberMode},
    editor::{Editor, file_is_open_in},
    emacs, embedded_config, formatting,
    keybindings::{Command, KeyBindings},
    line_numbers, lsp, lsp_setup, lsp_ui,
    piece_table::{self, PieceTable},
    renderer::{Renderer, TerminalHit, WindowControl},
    search::{SearchMode, SearchResult},
    search_ui::SearchUi,
    startup,
    terminal::{self, OutputCommand, Terminal, TerminalAction, TerminalEvent},
    vim::{self, VimController, VimUiAction},
    window, workspace_edit,
};
use sdl3::{
    event::{DisplayEvent, Event, WindowEvent},
    keyboard::{Keycode, Mod},
    mouse::MouseButton,
};
use std::{
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

pub(crate) mod commands;
mod input;
pub(crate) mod navigation;
pub(crate) mod search;
use commands::*;
use navigation::*;
use search::*;

pub(crate) fn run() -> Result<(), String> {
    piece_table::recovery::enable_for_application();
    let argument = std::env::args().nth(1);
    let launch = startup::LaunchTarget::resolve(
        argument.as_deref(),
        &std::env::current_dir().map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("Could not open launch path: {error}"))?;
    if let Some(root) = launch.workspace_root() {
        // Set once, before configuration loads and worker threads start.
        // Terminal navigation has its own cwd and does not change this root.
        std::env::set_current_dir(root)
            .map_err(|error| format!("Could not open folder {}: {error}", root.display()))?;
    }

    let editor_config = EditorConfig::load("config/editor.toml")
        .map_err(|e| format!("Could not load config/editor.toml: {e}"))?;

    println!(
        "EDITOR CONFIG: tab_width={}, insert_spaces={}, line_numbers={:?}, keybindings={:?}",
        editor_config.tab_width,
        editor_config.insert_spaces,
        editor_config.line_numbers,
        editor_config.keybinding_mode,
    );
    let key_bindings = KeyBindings::load("config/keybindings.toml")
        .map_err(|e| format!("Could not load config/keybindings.toml: {e}"))?;

    let startup_start = Instant::now();

    let mut editor = Editor::new(editor_config.clone()).map_err(|e| e.to_string())?;

    let mut other_editor = Editor::new(editor_config).map_err(|e| e.to_string())?;

    let mut emacs = emacs::Controller::default();
    let mut vim_enabled = editor.config.keybinding_mode == KeybindingMode::Vim;

    if let startup::LaunchTarget::File { path, location } = &launch {
        editor.open(path).map_err(|e| format!("Could not open file {path}: {e}"))?;
        if let Some((line, column)) = location {
            editor
                .document
                .move_cursor_to_line_column(line.saturating_sub(1), column.unwrap_or(0))
                .map_err(|e| e.to_string())?;
        }
    }

    println!(
        "Editor startup / PieceTable open: {:?}",
        startup_start.elapsed()
    );

    // Match linux/potyi.desktop so Wayland can use its launcher icon.
    sdl3::hint::set("SDL_APP_ID", "potyi");
    let sdl = sdl3::init().map_err(|e| e.to_string())?;

    let video = sdl.video().map_err(|e| format!("Could not initialize the display: {e}"))?;

    let clipboard = video.clipboard();
    let keyboard = sdl.keyboard();

    let driver = video.current_video_driver();
    let (window, window_hit_test) = window::create_window(&video, "Pötyi", 800, 600)
        .map_err(|e| format!("Could not create the window ({driver}): {e}"))?;

    video.text_input().start(&window);

    let canvas = sdl3::render::create_renderer(window, None)
        .map_err(|e| format!("Could not create the renderer ({driver}): {e}"))?;

    let ttf_context = sdl3::ttf::init().map_err(|e| e.to_string())?;

    let logical_font_size = editor.config.font_size as f32;

    let logical_font_stream =
        sdl3::iostream::IOStream::from_bytes(FONT_DATA).map_err(|e| e.to_string())?;

    let logical_font = ttf_context
        .load_font_from_iostream(logical_font_stream, logical_font_size)
        .map_err(|e| e.to_string())?;

    let raster_font_stream =
        sdl3::iostream::IOStream::from_bytes(FONT_DATA).map_err(|e| e.to_string())?;

    let raster_font = ttf_context
        .load_font_from_iostream(raster_font_stream, logical_font_size)
        .map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();
    let mut renderer = Renderer::new(
        canvas,
        &texture_creator,
        logical_font,
        raster_font,
        logical_font_size,
        (
            ttf_context
                .load_font_from_iostream(
                    sdl3::iostream::IOStream::from_bytes(FONT_DATA).map_err(|e| e.to_string())?,
                    command_bar::COMMAND_FONT_SIZE,
                )
                .map_err(|e| e.to_string())?,
            ttf_context
                .load_font_from_iostream(
                    sdl3::iostream::IOStream::from_bytes(FONT_DATA).map_err(|e| e.to_string())?,
                    command_bar::COMMAND_FONT_SIZE,
                )
                .map_err(|e| e.to_string())?,
        ),
        window_hit_test,
    )?;

    renderer.set_tab_width(editor.config.tab_width);

    renderer.set_line_number_mode(editor.config.line_numbers);

    if let Some(path) = editor.path.as_deref() {
        renderer.set_file_path(Some(path));
    }
    let mut search_ui = SearchUi::new();

    let mut command_bar = CommandBar::new();
    match piece_table::recovery::root().and_then(|root| piece_table::recovery::list(&root)) {
        Ok(entries) if !entries.is_empty() => {
            command_bar.open(":recover");
            command_bar.show_info(
                &piece_table::recovery::describe()
                    .unwrap_or_else(|e| format!("Could not list recovered work: {e}")),
            );
        }
        Err(error) => {
            command_bar.open(":recover");
            command_bar.show_info(&format!("Could not check crash recovery: {error}"));
        }
        _ => (),
    }

    let mut vim = VimController::new();
    let mut other_vim = VimController::new();

    renderer.set_mode_label(editor_mode_label(&editor, &vim));

    let terminal_directory = editor
        .path
        .as_deref()
        .and_then(|path| path.parent())
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let mut terminal = Terminal::new(terminal_directory).map_err(|error| error.to_string())?;

    let event_subsystem = sdl.event().map_err(|error| error.to_string())?;

    event_subsystem
        .register_custom_event::<TerminalEvent>()
        .map_err(|error| error.to_string())?;

    terminal.set_events(event_subsystem.clone());
    let mut split_mode = false;
    let mut active_pane = 0usize;
    if let Some(root) = launch.workspace_root() {
        open_folder_workspace(
            root, &mut split_mode, &mut active_pane, &mut editor, &mut other_editor,
            &mut vim, &mut other_vim, &mut renderer, &mut terminal,
        )?;
    }
    event_subsystem
        .register_custom_event::<lsp::Event>()
        .map_err(|e| e.to_string())?;
    event_subsystem
        .register_custom_event::<lsp_setup::Event>()
        .map_err(|e| e.to_string())?;
    let mut lsp_ui = lsp_ui::LspUi::new(event_subsystem.clone());

    let mut event_pump = sdl.event_pump().map_err(|e| e.to_string())?;

    if !renderer.window_mut().show() {
        return Err(sdl3::get_error().to_string());
    }

    let mut dirty = true;
    let mut pending_events = Vec::with_capacity(32);
    let mut mouse_state = input::MouseState::default();

    // ----------------------------------------------------------------------
    // Event loop
    // ----------------------------------------------------------------------

    let mut terminal_frames = terminal::FrameSchedule::default();
    'event_loop: loop {
        pending_events.clear();
        if !renderer.terminal_visible(&terminal) {
            terminal_frames.clear();
        }
        if terminal
            .poll_background()
            .map_err(|error| error.to_string())?
            && renderer.terminal_visible(&terminal)
        {
            terminal_frames.changed();
        }

        if !dirty
            && let Some(event) = event_pump.wait_event_timeout(terminal_frames.wait(
                Instant::now(),
                dirty,
                terminal.has_pending_work(),
            ).min(mouse_state.wait_timeout()))
        {
            pending_events.push(event);
        }

        pending_events.extend(event_pump.poll_iter().take(64));

        for mut event in pending_events.drain(..) {
            command_bar.refresh_formatters(editor.path.as_deref());
            if let Some(terminal_event) = event.as_user_event_type::<TerminalEvent>() {
                if terminal
                    .handle_event(terminal_event)
                    .map_err(|error| error.to_string())?
                    && renderer.terminal_visible(&terminal)
                {
                    terminal_frames.changed();
                }
                continue;
            }

            if let Some(event) = event.as_user_event_type::<lsp_setup::Event>() {
                lsp_ui.accept_setup(event, &editor, &other_editor, &mut command_bar);
                dirty = true;
                continue;
            }
            if let Some(event) = event.as_user_event_type::<lsp::Event>() {
                lsp_ui.validate_completion(
                    &editor,
                    &other_editor,
                    !renderer.terminal_focused(&terminal)
                        && !command_bar.is_active()
                        && (!vim_enabled || vim.mode() == vim::VimMode::Insert),
                );
                let outcome =
                    lsp_ui.accept(event, &mut editor, &mut other_editor, &mut command_bar);
                synchronize_pane_views(&mut editor, &mut other_editor, &mut renderer).map_err(|e| e.to_string())?;
                if outcome.document_changed && vim_enabled {
                    vim.finish_formatting(&mut editor);
                }
                if outcome.focus_other {
                    focus_pane(
                        1 - active_pane,
                        &mut active_pane,
                        &mut editor,
                        &mut other_editor,
                        &mut vim,
                        &mut other_vim,
                        &mut renderer,
                    );
                }
                if outcome.cursor_changed {
                    search_ui.close();
                    if vim_enabled && outcome.document_reloaded {
                        vim.reset();
                    }
                    renderer.set_file_path(editor.path.as_deref());
                    renderer.set_mode_label(editor_mode_label(&editor, &vim));
                    renderer.invalidate_scroll_cache();
                    renderer.update_cursor(&editor.document);
                    renderer.ensure_cursor_visible(&mut editor.document);
                }
                dirty = true;
                continue;
            }

            let coordinates_converted = renderer.convert_event_coordinates(&mut event);

            if input::dispatch(
                input::InputContext {
                    mouse_state: &mut mouse_state,
                    editor: &mut editor,
                    other_editor: &mut other_editor,
                    renderer: &mut renderer,
                    terminal: &mut terminal,
                    vim: &mut vim,
                    other_vim: &mut other_vim,
                    emacs: &mut emacs,
                    lsp_ui: &mut lsp_ui,
                    search_ui: &mut search_ui,
                    command_bar: &mut command_bar,
                    key_bindings: &key_bindings,
                    keyboard: &keyboard,
                    clipboard: &clipboard,
                    event_subsystem: &event_subsystem,
                    dirty: &mut dirty,
                    split_mode: &mut split_mode,
                    active_pane: &mut active_pane,
                    vim_enabled: &mut vim_enabled,
                },
                event,
                coordinates_converted,
            )? == input::EventFlow::Quit
            {
                break 'event_loop;
            }
        }

        dirty |= mouse_state.tick(&mut editor, &mut renderer, &mut vim, vim_enabled, active_pane)?;

        for document in [&mut editor.document, &mut other_editor.document] {
            if let Some(warning) = document.take_recovery_warning() {
                command_bar.open(":recover");
                command_bar.show_info(&warning);
                dirty = true;
            }
        }

        lsp_ui.validate_completion(
            &editor,
            &other_editor,
            !renderer.terminal_focused(&terminal)
                && !command_bar.is_active()
                && (!vim_enabled || vim.mode() == vim::VimMode::Insert),
        );
        lsp_ui.discard_dismissed_preview(&command_bar);
        lsp_ui.reconcile(&editor, &other_editor);

        dirty |= renderer.terminal_visible(&terminal) && terminal_frames.due(Instant::now());
        if dirty {
            command_bar.refresh_formatters(editor.path.as_deref());
            let start = Instant::now();

            renderer.update_window_size()?;
            renderer.set_completion(lsp_ui.completion_display());

            if renderer.terminal_visible(&terminal) {
                if split_mode {
                    renderer.render_split_terminal(
                        &mut editor.document,
                        &mut other_editor.document,
                        &mut terminal,
                        &search_ui,
                        &command_bar,
                    )?;
                } else {
                    renderer.render_terminal(&mut terminal)?;
                }
                terminal_frames.rendered(Instant::now());
            } else if split_mode {
                renderer.render_split(
                    &mut editor.document,
                    &mut other_editor.document,
                    &search_ui,
                    &command_bar,
                )?;
            } else {
                renderer.render(&mut editor.document, &search_ui, &command_bar)?;
            }

            let render_time = start.elapsed();

            if render_time > Duration::from_millis(20) {
                println!("Render: {:?}", render_time);
            }

            dirty = false;
        }
    }

    Ok(())
}
