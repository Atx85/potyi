// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! A companion window keeps the experimental path out of legacy pane routing.
use super::{
    input,
    painting::{self, FOOTER, GlyphCache, MARGIN, Point, Selection},
    session::{Session, Wake},
};
use crate::dpi_text::DpiTextMetrics;
use sdl3::{
    event::{Event, WindowEvent},
    keyboard::Keycode,
    mouse::MouseButton,
    render::Canvas,
    video::Window,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Debug)]
struct Ready;

pub(super) fn run(command: Option<String>, smoke: bool) -> Result<String, String> {
    let sdl = sdl3::init().map_err(|error| error.to_string())?;
    let video = sdl.video().map_err(|error| error.to_string())?;
    let window = video
        .window("Pötyi — experimental terminal", 900, 600)
        .resizable()
        .high_pixel_density()
        .build()
        .map_err(|error| error.to_string())?;
    video.text_input().start(&window);
    let mut canvas =
        sdl3::render::create_renderer(window, None).map_err(|error| error.to_string())?;
    let creator = canvas.texture_creator();
    let ttf = sdl3::ttf::init().map_err(|error| error.to_string())?;
    let logical_font = ttf
        .load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA)
                .map_err(|error| error.to_string())?,
            16.0,
        )
        .map_err(|error| error.to_string())?;
    let mut raster_font = ttf
        .load_font_from_iostream(
            sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA)
                .map_err(|error| error.to_string())?,
            16.0,
        )
        .map_err(|error| error.to_string())?;
    let cell_width = logical_font
        .size_of("M")
        .map_err(|error| error.to_string())?
        .0
        .max(1) as i32;
    let cell_height = logical_font.height().max(1) + 2;
    let events = sdl.event().map_err(|error| error.to_string())?;
    // SDL's custom-event registry can outlive the SDL context in test processes.
    if let Err(error) = events.register_custom_event::<Ready>() {
        if error.to_string() != "The same event type can not be registered twice!" {
            return Err(error.to_string());
        }
    }
    let sender = events.event_sender();
    let wake: Wake = Arc::new(move || {
        let _ = sender.push_custom_event(Ready);
    });
    let (rows, cols) = dimensions(&canvas, cell_width, cell_height);
    let directory = std::env::current_dir().map_err(|error| error.to_string())?;
    let mut session =
        Session::open(&directory, rows, cols, wake).map_err(|error| error.to_string())?;
    if let Some(command) = command {
        // One atomic queue item; a partial enqueue must never run half a command.
        session
            .send(format!("{command}\r").into_bytes())
            .map_err(|error| error.to_string())?;
    }
    let clipboard = video.clipboard();
    let keyboard = sdl.keyboard();
    let mut pump = sdl.event_pump().map_err(|error| error.to_string())?;
    let mut selection = Selection::default();
    let mut cache = GlyphCache::default();
    let mut metrics = DpiTextMetrics::default();
    let mut dirty = true;
    let mut output_dirty = false;
    let mut focused = true;
    let mut next_frame = Instant::now();
    let start = Instant::now();
    let mut wheel = 0.0f32;
    loop {
        if session.poll() {
            selection.clear(); // Selection references the current displayed grid.
            output_dirty = true;
        }
        let now = Instant::now();
        if dirty || (output_dirty && now >= next_frame) {
            let (width, height) = canvas.window().size();
            canvas
                .set_logical_size(
                    width.max(1),
                    height.max(1),
                    sdl3::sys::render::SDL_LOGICAL_PRESENTATION_STRETCH,
                )
                .map_err(|error| error.to_string())?;
            let scale = canvas.window().display_scale();
            if metrics.needs_update(scale) {
                metrics = DpiTextMetrics::new(scale);
                raster_font
                    .set_size(metrics.raster_font_size(16.0))
                    .map_err(|error| error.to_string())?;
                cache.clear();
            }
            let (rows, cols) = dimensions(&canvas, cell_width, cell_height);
            if session
                .resize(rows, cols)
                .map_err(|error| error.to_string())?
            {
                selection.clear();
            }
            painting::draw(
                &mut canvas,
                &creator,
                &mut raster_font,
                &mut cache,
                session.screen(),
                &selection,
                session.status.as_deref(),
                cell_width,
                cell_height,
                metrics.raster_scale(),
                focused,
            )?;
            dirty = false;
            output_dirty = false;
            next_frame = Instant::now() + Duration::from_millis(16);
        }
        if smoke && session.closed && !session.work_pending && !output_dirty && !dirty {
            return Ok(session.screen().contents());
        }
        if smoke && start.elapsed() > Duration::from_secs(10) {
            return Err("Experimental terminal smoke test timed out".into());
        }
        let wait = if session.work_pending {
            Duration::ZERO
        } else if output_dirty {
            next_frame.saturating_duration_since(Instant::now())
        } else {
            Duration::from_secs(1)
        };
        let first = pump.wait_event_timeout(wait);
        let pending: Vec<_> = first.into_iter().chain(pump.poll_iter().take(64)).collect();
        for mut event in pending {
            if event.as_user_event_type::<Ready>().is_some() {
                continue;
            }
            // SDL reports window coordinates; our logical resolution matches
            // those coordinates, independent of the framebuffer's pixel scale.
            match &mut event {
                Event::Quit { .. }
                | Event::Window {
                    win_event: WindowEvent::CloseRequested,
                    ..
                } => return Ok(session.screen().contents()),
                Event::Window {
                    win_event: WindowEvent::FocusGained,
                    ..
                } => {
                    focused = true;
                    dirty = true;
                }
                Event::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } => {
                    focused = false;
                    selection.dragging = false;
                    dirty = true;
                }
                Event::Window { .. } | Event::Display { .. } => {
                    dirty = true;
                }
                Event::TextInput { text, .. } => {
                    // Command/Control keys are handled by KeyDown. Do not send
                    // a second printable event for the same shortcut.
                    let modifiers = keyboard.mod_state();
                    if (!input::control(modifiers) || input::alt_graph(modifiers))
                        && !input::command(modifiers)
                    {
                        let bytes = if input::alt(modifiers) && !input::alt_graph(modifiers) {
                            [b"\x1b".as_slice(), text.as_bytes()].concat()
                        } else {
                            text.as_bytes().to_vec()
                        };
                        if let Err(error) = session.send(bytes) {
                            session.status = Some(error.to_string());
                        }
                        selection.clear();
                        dirty = true;
                    }
                }
                Event::KeyDown {
                    keycode: Some(key),
                    keymod,
                    ..
                } => {
                    let copy = *key == Keycode::C
                        && (input::command(*keymod)
                            || (input::control(*keymod) && input::shift(*keymod)));
                    let paste = *key == Keycode::V
                        && (input::command(*keymod)
                            || (input::control(*keymod) && input::shift(*keymod)));
                    let result = if copy {
                        let selected = selection.text(session.screen());
                        let text = if selected.is_empty() {
                            session.screen().contents()
                        } else {
                            selected
                        };
                        clipboard
                            .set_clipboard_text(&text)
                            .map_err(|error| error.to_string())
                    } else if paste {
                        clipboard
                            .clipboard_text()
                            .map_err(|error| error.to_string())
                            .and_then(|text| {
                                session.paste(&text).map_err(|error| error.to_string())
                            })
                    } else if input::shift(*keymod)
                        && matches!(*key, Keycode::PageUp | Keycode::PageDown)
                    {
                        session.scroll(if *key == Keycode::PageUp {
                            rows as isize / 2 + 1
                        } else {
                            -(rows as isize / 2 + 1)
                        });
                        selection.clear();
                        Ok(())
                    } else if !input::command(*keymod) {
                        input::key_bytes(*key, *keymod, session.screen().application_cursor())
                            .map_or(Ok(()), |bytes| {
                                selection.clear();
                                session.send(bytes).map_err(|error| error.to_string())
                            })
                    } else {
                        Ok(())
                    };
                    if let Err(error) = result {
                        session.status = Some(error);
                    }
                    dirty = true;
                }
                Event::MouseWheel { y, .. } => {
                    wheel += *y * 3.0;
                    let lines = wheel.trunc() as isize;
                    wheel -= lines as f32;
                    if lines != 0 {
                        session.scroll(lines);
                        selection.clear();
                        dirty = true;
                    }
                }
                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    x,
                    y,
                    ..
                } => {
                    let point = point(*x, *y, session.screen(), cell_width, cell_height);
                    selection = Selection {
                        anchor: Some(point),
                        end: Some(point),
                        dragging: true,
                        spans: Vec::new(),
                    };
                    dirty = true;
                }
                Event::MouseMotion { x, y, .. } if selection.dragging => {
                    selection.end = Some(point(*x, *y, session.screen(), cell_width, cell_height));
                    dirty = true;
                }
                Event::MouseButtonUp {
                    mouse_btn: MouseButton::Left,
                    ..
                } => {
                    selection.dragging = false;
                }
                _ => {}
            }
        }
    }
}

fn dimensions(canvas: &Canvas<Window>, width: i32, height: i32) -> (u16, u16) {
    let (w, h) = canvas.window().size();
    (
        ((h as i32 - MARGIN * 2 - FOOTER) / height).clamp(1, super::session::MAX_ROWS as i32)
            as u16,
        ((w as i32 - MARGIN * 2) / width).clamp(1, super::session::MAX_COLS as i32) as u16,
    )
}

fn point(x: f32, y: f32, screen: &vt100::Screen, width: i32, height: i32) -> Point {
    let (rows, cols) = screen.size();
    Point {
        row: ((y as i32 - MARGIN).max(0) / height).min(i32::from(rows) - 1) as u16,
        col: ((x as i32 - MARGIN).max(0) / width).min(i32::from(cols)) as u16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires SDL video (SDL_VIDEODRIVER=dummy is supported)"]
    fn experimental_terminal_window_runs_a_real_shell_and_renders_output() {
        #[cfg(unix)]
        let command = "printf '\\033[31mWINDOW_READY\\033[0m\\n'; exit";
        #[cfg(windows)]
        let command = "echo WINDOW_READY & exit";
        let output = run(Some(command.into()), true).unwrap();
        assert!(output.contains("WINDOW_READY"), "{output:?}");
    }
}
