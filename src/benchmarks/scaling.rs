// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in sustained editing measurements. Counters exist only in test builds.
use crate::{
    Editor, command_bar::CommandBar, config::EditorConfig, renderer::Renderer, search_ui::SearchUi,
};
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
    time::Instant,
};

#[derive(Clone, Copy, Default, Serialize)]
pub(crate) struct Counters {
    pub read_bytes: u64,
    pub read_calls: u64,
    pub line_discoveries: u64,
    pub copied_piece_records: u64,
    pub copied_line_records: u64,
    pub copied_history_entries: u64,
}

thread_local! {
    static COUNTERS: Cell<Option<Counters>> = const { Cell::new(None) };
}

fn count(update: impl FnOnce(&mut Counters)) {
    COUNTERS.with(|cell| {
        if let Some(mut counters) = cell.get() {
            update(&mut counters);
            cell.set(Some(counters));
        }
    });
}

pub(crate) fn record_read(bytes: usize) {
    count(|c| {
        c.read_bytes += bytes as u64;
        c.read_calls += 1;
    });
}

pub(crate) fn record_line_discovery() {
    count(|c| c.line_discoveries += 1);
}

pub(crate) fn record_copies(pieces: usize, lines: usize, history: usize) {
    count(|c| {
        c.copied_piece_records += pieces as u64;
        c.copied_line_records += lines as u64;
        c.copied_history_entries += history as u64;
    });
}

fn emit(value: serde_json::Value) {
    println!("POTYI_SCALING {value}");
    io::stdout().flush().unwrap();
}

fn phase(name: &str) {
    emit(serde_json::json!({"type":"phase","name":name}));
}

pub(crate) fn with_counters<T>(action: impl FnOnce() -> T) -> (T, Counters) {
    COUNTERS.with(|cell| cell.set(Some(Counters::default())));
    let result = action();
    let counters = COUNTERS.with(|cell| cell.take().unwrap());
    (result, counters)
}

fn measure<T>(stage: &str, iteration: Option<usize>, action: impl FnOnce() -> T) -> T {
    let ((result, milliseconds), counters) = with_counters(|| {
        let start = Instant::now();
        let result = action();
        (result, start.elapsed().as_secs_f64() * 1000.0)
    });
    emit(serde_json::json!({
        "type":"measurement", "stage":stage, "iteration":iteration,
        "milliseconds":milliseconds, "counters":counters,
    }));
    result
}

#[derive(Deserialize)]
struct Case {
    bytes: usize,
    line_bytes: usize,
    position_percent: usize,
    panes: usize,
    history_entries: usize,
    fragmented_edits: usize,
    recovery: bool,
    render: bool,
    iterations: usize,
    fixture_dir: PathBuf,
}

struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture(config: &Case, path: &std::path::Path) -> io::Result<()> {
    let mut file = File::create(path)?;
    let mut block = [0u8; 64 * 1024];
    let mut offset = 0;
    while offset < config.bytes {
        let size = block.len().min(config.bytes - offset);
        for (index, byte) in block[..size].iter_mut().enumerate() {
            *byte = if (offset + index + 1) % config.line_bytes == 0 {
                b'\n'
            } else {
                b'a'
            };
        }
        file.write_all(&block[..size])?;
        offset += size;
    }
    file.sync_all()
}

fn sync(editor: &mut Editor, other: &mut Option<Editor>) -> io::Result<()> {
    if let Some(other) = other {
        Editor::synchronize_views(editor, other)?;
    }
    Ok(())
}

fn run(config: &Case, mut renderer: Option<&mut Renderer<'_>>) -> io::Result<()> {
    fs::create_dir(&config.fixture_dir)?;
    let _cleanup = Cleanup(config.fixture_dir.clone());
    let path = config.fixture_dir.join("fixture.txt");
    phase("fixture");
    fixture(config, &path)?;
    emit(serde_json::json!({
        "type":"fixture", "bytes":config.bytes,
        "line_bytes":config.line_bytes, "line_count":config.bytes / config.line_bytes + 1,
        "rust_arch":std::env::consts::ARCH,
    }));

    let mut editor = Editor::new(EditorConfig::default())?;
    measure("open", None, || editor.open(path.to_str().unwrap()))?;
    phase("prepare_history");
    // A bounded first-line replacement creates real history entries without
    // growing the document or deliberately fragmenting its piece layout.
    for index in 0..config.history_entries {
        editor.set_cursor_and_anchor(1, 0)?;
        editor.insert_text(if index % 2 == 0 { "b" } else { "a" })?;
    }
    phase("prepare_fragmentation");
    // Setup only: build dispersed pieces without including navigation costs
    // in fragmentation preparation. Timed operations below use Editor methods.
    for index in (1..=config.fragmented_edits).rev() {
        let position = ((config.bytes as u128 * index as u128)
            / (config.fragmented_edits as u128 + 1)) as usize;
        editor.document.insert(position, "f")?;
    }
    if config.fragmented_edits > 0 {
        editor.set_dirty(true);
    }

    let position =
        ((editor.document.len() as u128 * config.position_percent as u128) / 100) as usize;
    phase("navigation");
    measure("navigation", None, || editor.document.move_cursor(position))?;
    let mut other = if config.panes == 2 {
        Some(measure("duplicate_view", None, || editor.duplicate_view()))
    } else {
        None
    };
    if config.recovery {
        editor
            .document
            .enable_recovery_at(config.fixture_dir.join("recovery"), Some(&path));
    }

    let search = SearchUi::new();
    let bar = CommandBar::new();
    if let Some(renderer) = renderer.as_deref_mut() {
        renderer.set_split_mode(config.panes == 2);
        if let Some(other) = &mut other {
            renderer.set_active_pane(1);
            renderer.ensure_cursor_visible(&mut other.document);
        }
        renderer.set_active_pane(0);
        renderer.ensure_cursor_visible(&mut editor.document);
        phase("initial_draw");
        measure("initial_draw", None, || {
            if let Some(other) = &mut other {
                renderer.render_split(&mut editor.document, &mut other.document, &search, &bar)
            } else {
                renderer.render(&mut editor.document, &search, &bar)
            }
        })
        .map_err(io::Error::other)?;
    }

    let initial_len = editor.document.len();
    phase("editing");
    for iteration in 0..config.iterations {
        measure("insert", Some(iteration), || editor.insert_text("x"))?;
        measure("synchronize", Some(iteration), || {
            sync(&mut editor, &mut other)
        })?;
        assert_eq!(editor.document.len(), initial_len + 1);
        assert_eq!(editor.document.byte_at(position)?, Some(b'x'));
        if let Some(renderer) = renderer.as_deref_mut() {
            measure("redraw", Some(iteration), || {
                renderer.invalidate_scroll_cache();
                renderer.ensure_cursor_visible(&mut editor.document);
                if let Some(other) = &mut other {
                    renderer.render_split(&mut editor.document, &mut other.document, &search, &bar)
                } else {
                    renderer.render(&mut editor.document, &search, &bar)
                }
            })
            .map_err(io::Error::other)?;
        }
        measure("undo", Some(iteration), || editor.undo())?;
        measure("synchronize_undo", Some(iteration), || {
            sync(&mut editor, &mut other)
        })?;
        assert_eq!(editor.document.len(), initial_len);
        measure("redo", Some(iteration), || editor.redo())?;
        measure("synchronize_redo", Some(iteration), || {
            sync(&mut editor, &mut other)
        })?;
        assert_eq!(editor.document.len(), initial_len + 1);
        assert_eq!(editor.document.byte_at(position)?, Some(b'x'));
        measure("backspace", Some(iteration), || editor.backspace())?;
        measure("synchronize_backspace", Some(iteration), || {
            sync(&mut editor, &mut other)
        })?;
        assert_eq!(editor.document.len(), initial_len);
        assert_eq!(editor.document.cursor.position, position);
        if let Some(other) = &other {
            assert_eq!(other.document.len(), initial_len);
            assert_eq!(other.document.revision(), editor.document.revision());
        }
        assert!(editor.document.take_recovery_warning().is_none());
    }
    emit(serde_json::json!({
        "type":"complete", "iterations":config.iterations,
        "final_pieces":editor.document.pieces().len(),
        "final_cached_lines":editor.document.cached_line_count(),
        "line_index_metadata":editor.document.line_index_metadata(),
        "final_history_entries":editor.undo_stack.len(),
    }));
    Ok(())
}

#[test]
#[ignore = "Opt-in sustained editor benchmark; run tools/benchmarks/scaling.py"]
fn sustained_editing_probe() {
    let config: Case = serde_json::from_str(
        &std::env::var("POTYI_SCALING_CASE").expect("explicit benchmark case required"),
    )
    .unwrap();
    assert!((2..=1_000_000_000).contains(&config.bytes));
    assert!((2..=1024 * 1024).contains(&config.line_bytes));
    assert!(config.position_percent <= 99 && matches!(config.panes, 1 | 2));
    assert!((1..=1000).contains(&config.iterations));
    assert!(config.history_entries <= 100_000 && config.fragmented_edits <= 100_000);
    if config.render {
        assert_eq!(std::env::var("SDL_VIDEODRIVER").as_deref(), Ok("dummy"));
        phase("graphics");
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video
            .window("scaling probe", 800, 600)
            .hidden()
            .build()
            .unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || {
            ttf.load_font_from_iostream(
                sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(),
                18.0,
            )
            .unwrap()
        };
        let canvas = window.into_canvas();
        let textures = canvas.texture_creator();
        let mut renderer = Renderer::new(
            canvas,
            &textures,
            font(),
            font(),
            18.0,
            (font(), font()),
            crate::window::WindowHitTestState::new(800, 1.0),
        )
        .unwrap();
        run(&config, Some(&mut renderer)).unwrap();
    } else {
        run(&config, None).unwrap();
    }
}

#[test]
fn measurement_counters_are_scoped_to_the_measured_operation() {
    record_read(100);
    assert!(COUNTERS.with(Cell::get).is_none());
    COUNTERS.with(|cell| cell.set(Some(Counters::default())));
    record_read(7);
    record_line_discovery();
    record_copies(2, 3, 4);
    let counters = COUNTERS.with(Cell::take).unwrap();
    assert_eq!(counters.read_bytes, 7);
    assert_eq!(counters.read_calls, 1);
    assert_eq!(counters.line_discoveries, 1);
    assert_eq!(counters.copied_piece_records, 2);
    assert_eq!(counters.copied_line_records, 3);
    assert_eq!(counters.copied_history_entries, 4);
    record_read(100);
    assert!(COUNTERS.with(Cell::get).is_none());
}
