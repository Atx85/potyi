// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Navigation and editing after eviction through the production document API.
use super::*;
use super::Document as PieceTable;
use crate::{Editor, config::EditorConfig};

struct Fixture(PathBuf);
impl Fixture {
    fn new(text: &str) -> Self {
        let path = std::env::temp_dir().join(format!("potyi-line-budget-{}-{}", std::process::id(), next_document_revision()));
        fs::write(&path, text).unwrap();
        Self(path)
    }
    fn open(&self) -> PieceTable { PieceTable::open(self.0.to_str().unwrap()).unwrap() }
}
impl Drop for Fixture {
    fn drop(&mut self) { let _ = fs::remove_file(&self.0); }
}

fn check_budget(table: &PieceTable) {
    let (details, checkpoints, bytes) = table.line_cache.metadata();
    assert!(bytes <= 128 * 1024);
    assert!(details <= 4096 && checkpoints <= 2048);
}

#[test]
fn evicted_unicode_crlf_lines_reconstruct_exact_columns_and_page_destinations() {
    let text = "é\t🙂\r\n".repeat(80_000) + "tail";
    let fixture = Fixture::new(&text);
    let mut table = fixture.open();
    assert_eq!(table.line_count().unwrap(), 80_001);
    assert!(table.line_cache.get(0).is_none());
    for line in [0, 40_123, 17, 79_999, 8_123, 80_000, 511] {
        table.move_cursor_to_line_column(line, 2).unwrap();
        assert_eq!((table.cursor.line, table.cursor.column), (line, 2));
        assert_eq!(table.cursor.position, line * 9 + if line == 80_000 { 2 } else { 3 });
        assert_eq!(table.line_text(line).unwrap(), if line == 80_000 { "tail" } else { "é\t🙂" });
        if line < 80_000 {
            assert_eq!(table.line_column_at(line * 9 + 8).unwrap(), (line, 4));
            assert_eq!(table.line_window(line, 0, 12, 4).unwrap().text, "é\t🙂");
            assert_eq!(table.line_visual_column(line, 3, 4).unwrap(), 5);
        }
        check_budget(&table);
    }
    // The final page can be evicted while EOF remains known. A clamped jump
    // must reconstruct it rather than index an absent dense record.
    for line in (0..6_000).step_by(128) { table.line_start(line).unwrap(); }
    table.move_cursor(0).unwrap();
    table.move_page(true, usize::MAX, false).unwrap();
    assert_eq!(table.cursor.line, 80_000);
    assert_eq!(table.line_start(usize::MAX).unwrap(), table.len());
    assert_eq!(table.line_text(usize::MAX).unwrap(), "");
}

#[test]
fn edits_in_evicted_regions_and_snapshot_undo_discard_stale_checkpoints() {
    let mut expected = "α🙂\r\n".repeat(80_000) + "last";
    let fixture = Fixture::new(&expected);
    let mut table = fixture.open();
    table.line_count().unwrap();
    let position = 12_003 * 8 + 2;
    assert!(!table.line_cache.contains_position(position));
    table.insert(position, "X\n中").unwrap();
    expected.insert_str(position, "X\n中");
    table.delete(2, 4).unwrap(); // Remove an emoji before every retained checkpoint.
    expected.replace_range(2..6, "");
    let before = expected.clone();
    let mut snapshot = table.replace_ranges([(4, 12)].into_iter(), "🙂\r\n").unwrap().unwrap();
    expected.replace_range(4..12, "🙂\r\n");
    for text in [&expected, &before, &expected] {
        let reference_fixture = Fixture::new(text);
        let mut reference = reference_fixture.open();
        assert_eq!(table.line_count().unwrap(), reference.line_count().unwrap());
        for line in [0, 1, 9, 12_002, 12_003, 40_123, 79_998, 80_000] {
            assert_eq!(table.line_text(line).unwrap(), reference.line_text(line).unwrap());
            assert_eq!(table.line_start(line).unwrap(), reference.line_start(line).unwrap());
        }
        for position in text.char_indices().step_by(11_111).map(|(byte, _)| byte).chain([text.len()]) {
            assert_eq!(table.line_column_at(position).unwrap(), reference.line_column_at(position).unwrap());
        }
        check_budget(&table);
        table.swap_snapshot(&mut snapshot);
    }
}

#[test]
fn shared_views_edit_and_undo_correctly_after_line_metadata_eviction() {
    let fixture = Fixture::new(&("a".repeat(63) + "\n").repeat(80_000));
    let mut first = Editor::new(EditorConfig::default()).unwrap();
    first.open(fixture.0.to_str().unwrap()).unwrap();
    let mut second = first.duplicate_view();
    first.document.move_cursor(64 * 12).unwrap();
    second.document.move_cursor(64 * 70_000 + 8).unwrap();
    first.insert_text("🙂\n").unwrap();
    Editor::synchronize_views(&mut first, &mut second).unwrap();
    assert_eq!(second.document.cursor.line, 70_001);
    assert_eq!(second.document.cursor.column, 8);
    second.undo().unwrap();
    Editor::synchronize_views(&mut second, &mut first).unwrap();
    assert_eq!(second.document.line_text(70_000).unwrap(), "a".repeat(63));
    second.redo().unwrap();
    Editor::synchronize_views(&mut second, &mut first).unwrap();
    assert_eq!(second.document.line_text(70_001).unwrap(), "a".repeat(63));
    assert_eq!(first.document.line_text(12).unwrap(), "🙂");
    check_budget(&first.document.storage.borrow());
}

#[test]
#[ignore = "Independent bounded-memory and distant-pane performance measurement"]
fn alternating_distant_panes_report_reconstruction_cost() {
    use crate::benchmarks::scaling::with_counters;
    let path = std::env::temp_dir().join(format!("potyi-pane-budget-probe-{}-{}", std::process::id(), next_document_revision()));
    let fixture = Fixture(path);
    let mut file = File::create(&fixture.0).unwrap();
    let block = ("a".repeat(63) + "\n").repeat(1024);
    for _ in 0..1024 { file.write_all(block.as_bytes()).unwrap(); }
    drop(file);
    let mut a = Editor::new(EditorConfig::default()).unwrap();
    a.open(fixture.0.to_str().unwrap()).unwrap();
    let started = Instant::now();
    assert_eq!(a.document.line_count().unwrap(), 1_048_577);
    let full_scan_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut b = a.duplicate_view();
    let mut samples = Vec::new();
    for (left, right) in [(517, 786_947), (767, 787_199), (1023, 787_455)] {
      a.document.move_cursor(left * 64).unwrap();
      b.document.move_cursor(right * 64).unwrap();
      for iteration in 0..9 {
        let (((), milliseconds), counters) = with_counters(|| {
            let start = Instant::now();
            for editor in [&mut a, &mut b] {
                let first = editor.document.cursor.line;
                for line in first..first + 40 {
                    assert_eq!(editor.document.line_window(line, 0, 80, 4).unwrap().text.len(), 63);
                }
            }
            ((), start.elapsed().as_secs_f64() * 1000.0)
        });
        samples.push(serde_json::json!({"left_line":left,"right_line":right,"iteration":iteration,"milliseconds":milliseconds,"counters":counters}));
      }
    }
    println!("POTYI_LINE_BUDGET_REVIEW {}", serde_json::json!({
        "arch":std::env::consts::ARCH,"fixture_bytes":67_108_864,"lines":1_048_577,
        "full_scan_ms":full_scan_ms,"line_index_metadata":a.document.line_index_metadata(),
        "two_panes_visible_rows":40,"samples":samples,
    }));
}
