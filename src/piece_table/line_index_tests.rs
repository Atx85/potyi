// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::Document as PieceTable;
use super::*;
use crate::{Editor, benchmarks::scaling::with_counters, config::EditorConfig};

struct Fixture(PathBuf);
impl Fixture {
    fn new(bytes: &[u8]) -> Self {
        let path = std::env::temp_dir().join(format!(
            "potyi-line-index-{}-{}",
            std::process::id(),
            next_document_revision(),
        ));
        fs::write(&path, bytes).unwrap();
        Self(path)
    }
    fn open(&self) -> PieceTable {
        PieceTable::open(self.0.to_str().unwrap()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn table(text: &str) -> PieceTable {
    let mut table = PieceTable::empty().unwrap();
    table.insert(0, text).unwrap();
    table
}

fn verify(table: &mut PieceTable, expected: &str) {
    assert_eq!(table.text().unwrap(), expected);
    let lines: Vec<_> = expected.split('\n').collect();
    assert_eq!(table.line_count().unwrap(), lines.len());
    for (index, line) in lines.iter().enumerate() {
        assert_eq!(
            table.line_text(index).unwrap(),
            line.strip_suffix('\r').unwrap_or(line)
        );
    }
    let mut cold = self::table(expected);
    for position in expected
        .char_indices()
        .map(|(byte, _)| byte)
        .chain([expected.len()])
    {
        assert_eq!(
            table.line_column_at(position).unwrap(),
            cold.line_column_at(position).unwrap()
        );
    }
}

#[test]
fn cached_prefix_survives_unicode_newline_crlf_and_eof_edits() {
    let original = "α🙂\r\nbravo\n中\tcharlie\r\nlast";
    let boundaries: Vec<_> = original
        .char_indices()
        .map(|(byte, _)| byte)
        .chain([original.len()])
        .collect();
    for (index, position) in boundaries.iter().copied().enumerate() {
        for inserted in ["X", "\n", "\r", "\r\n", "🙂"] {
            let mut table = table(original);
            table.line_count().unwrap();
            let keep = table
                .line_cache
                .iter()
                .filter(|line| line.end < position)
                .count();
            table.insert(position, inserted).unwrap();
            assert_eq!(table.line_cache.len(), keep);
            let mut expected = original.to_owned();
            expected.insert_str(position, inserted);
            verify(&mut table, &expected);
        }
        // Include two-character removals to cross/join CRLF and newline boundaries.
        for end in boundaries.iter().skip(index + 1).take(2).copied() {
            let mut table = table(original);
            table.line_count().unwrap();
            let keep = table
                .line_cache
                .iter()
                .filter(|line| line.end < position)
                .count();
            table.delete(position, end - position).unwrap();
            assert_eq!(table.line_cache.len(), keep);
            let mut expected = original.to_owned();
            expected.replace_range(position..end, "");
            verify(&mut table, &expected);
        }
    }
    let mut empty = table("last\r\n");
    empty.line_count().unwrap();
    empty.delete(0, empty.len()).unwrap();
    verify(&mut empty, "");
    empty.insert(0, "🙂\r\n").unwrap();
    verify(&mut empty, "🙂\r\n");
}

#[test]
fn short_line_discovery_reuses_one_bounded_read_without_eager_validation() {
    let text = ("a".repeat(63) + "\n").repeat(2048) + "tail";
    let fixture = Fixture::new(text.as_bytes());
    let mut table = fixture.open();
    let (_, counters) = with_counters(|| table.ensure_line_cached(1000).unwrap());
    assert!(counters.read_bytes <= 2 * PIECE_TABLE_CHUNK_SIZE as u64);
    assert!(counters.read_calls <= 2);
    assert!(!table.all_lines_cached);

    // Reading ahead is allowed; validating/indexing unread later lines is not.
    let invalid = Fixture::new(b"valid\n\xff\n");
    let mut table = invalid.open();
    assert_eq!(table.line_text(0).unwrap(), "valid");
    assert_eq!(
        table.line_text(1).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}

#[test]
fn bulk_line_scan_preserves_unicode_and_crlf_across_refills() {
    // Cold file scans use 64 KiB; edits start with 256 bytes and grow.
    for boundary in [256, 768, PIECE_TABLE_CHUNK_SIZE, PIECE_TABLE_CHUNK_SIZE * 2] {
        for character in ["é", "中", "🙂"] {
            for overlap in 1..character.len() {
                let prefix = "a".repeat(boundary - overlap);
                let original = format!("{prefix}{character}\r\nnext\n");
                let fixture = Fixture::new(original.as_bytes());
                let mut cold = fixture.open();
                let mut edited = table(&original);
                for table in [&mut cold, &mut edited] {
                    assert_eq!(table.line_length(0).unwrap(), prefix.len() + 1);
                    assert_eq!(
                        table.line_start(1).unwrap(),
                        prefix.len() + character.len() + 2
                    );
                    assert_eq!(table.line_text(1).unwrap(), "next");
                    assert_eq!(table.line_count().unwrap(), 3);
                    assert_eq!(
                        table
                            .line_column_at(prefix.len() + character.len())
                            .unwrap(),
                        (0, prefix.len() + 1),
                    );
                }
            }
        }
        let original = format!("{}\r\n\ntail", "a".repeat(boundary - 1));
        let fixture = Fixture::new(original.as_bytes());
        let mut cold = fixture.open();
        let mut edited = table(&original);
        for table in [&mut cold, &mut edited] {
            assert_eq!(table.line_length(0).unwrap(), boundary - 1);
            assert_eq!(table.line_start(1).unwrap(), boundary + 1);
            assert_eq!(table.line_text(1).unwrap(), "");
            assert_eq!(table.line_text(2).unwrap(), "tail");
            assert_eq!(table.line_count().unwrap(), 3);
        }
    }
}

#[test]
fn bulk_line_scan_rejects_malformed_utf8_without_validating_later_lines() {
    for invalid in [
        &b"\xff"[..],
        b"\xc0\x80",
        b"\xe0\x80\x80",
        b"\xed\xa0\x80",
        b"\xf4\x90\x80\x80",
        b"\xf0\x9f",
        b"\xe2\n",
        b"\xe2x\x80",
    ] {
        for boundary in [256, PIECE_TABLE_CHUNK_SIZE] {
            for overlap in 1..=invalid.len() {
                let mut original = b"valid\n".to_vec();
                original.resize(boundary - overlap, b'a');
                original.extend_from_slice(invalid);
                let fixture = Fixture::new(&original);
                let mut cold = fixture.open();
                assert_eq!(cold.line_text(0).unwrap(), "valid");
                assert_eq!(cold.line_cache.len(), 1);
                assert_eq!(
                    cold.line_length(1).unwrap_err().kind(),
                    io::ErrorKind::InvalidData
                );
                // Failed scans publish no partial line and remain retryable.
                assert_eq!(cold.line_cache.len(), 1);
                assert_eq!(
                    cold.line_length(1).unwrap_err().kind(),
                    io::ErrorKind::InvalidData
                );
                // Exercise the small post-edit refill against file-backed data.
                let mut edited = fixture.open();
                edited.insert(0, "X").unwrap();
                assert_eq!(edited.line_text(0).unwrap(), "Xvalid");
                assert_eq!(
                    edited.line_length(1).unwrap_err().kind(),
                    io::ErrorKind::InvalidData
                );
            }
        }
    }
}

#[test]
fn deep_editor_edits_and_history_do_not_rediscover_the_unchanged_prefix() {
    let fixture = Fixture::new(("a".repeat(63) + "\n").repeat(16384).as_bytes());
    let mut editor = Editor::new(EditorConfig::default()).unwrap();
    editor.open(fixture.0.to_str().unwrap()).unwrap();
    let position = 786_432;
    editor.document.move_cursor(position).unwrap();
    let mut other = editor.duplicate_view();
    for _ in 0..8 {
        let (_, counters) = with_counters(|| {
            editor.insert_text("x").unwrap();
            Editor::synchronize_views(&mut editor, &mut other).unwrap();
            editor.undo().unwrap();
            Editor::synchronize_views(&mut editor, &mut other).unwrap();
            editor.redo().unwrap();
            Editor::synchronize_views(&mut editor, &mut other).unwrap();
            editor.backspace().unwrap();
            Editor::synchronize_views(&mut editor, &mut other).unwrap();
        });
        assert!(
            counters.line_discoveries <= 4,
            "unchanged 12,288-line prefix was rescanned"
        );
        assert!(counters.read_bytes < 5 * PIECE_TABLE_CHUNK_SIZE as u64);
        assert_eq!(editor.document.cursor.position, position);
        assert_eq!(editor.document.len(), 1024 * 1024);
        assert_eq!(other.document.byte_at(position).unwrap(), Some(b'a'));
    }
}

#[test]
fn scan_buffer_is_discarded_after_edits_and_snapshot_swaps() {
    let mut table = table("first\nsecond\nthird\n");
    table.line_text(0).unwrap(); // Buffer contains unread later lines.
    table.insert(7, "🙂\n").unwrap();
    verify(&mut table, "first\ns🙂\necond\nthird\n");
    let mut snapshot = table
        .replace_ranges([(7, 12)].into_iter(), "X")
        .unwrap()
        .unwrap();
    let replaced = table.text().unwrap();
    verify(&mut table, &replaced);
    table.swap_snapshot(&mut snapshot);
    verify(&mut table, "first\ns🙂\necond\nthird\n");
    table.swap_snapshot(&mut snapshot);
    verify(&mut table, &replaced);
}

#[test]
fn short_line_edit_refills_do_not_read_thousands_of_unrelated_fragments() {
    let fixture = Fixture::new(("a".repeat(63) + "\n").repeat(16384).as_bytes());
    let mut table = fixture.open();
    for position in (1..10_000).rev() {
        table.insert(position * 100, "f").unwrap();
    }
    let position = table.len() / 2;
    table.move_cursor(position).unwrap();
    let (_, counters) = with_counters(|| {
        table.insert(position, "x").unwrap();
        table.move_cursor(position + 1).unwrap();
    });
    assert!(
        counters.read_bytes < 1024,
        "{} read bytes",
        counters.read_bytes
    );
    assert!(
        counters.read_calls < 32,
        "{} positional reads",
        counters.read_calls
    );
    assert_eq!(table.byte_at(position).unwrap(), Some(b'x'));
    let (line, column) = table.line_column_at(position + 1).unwrap();
    assert!(line > 8000 && column > 0);
}

#[test]
fn bounded_index_reconstructs_evicted_lines_positions_and_terminal_rows() {
    let row = "é\t🦀\r\n";
    // Cross two checkpoint compactions as well as many detailed-window evictions.
    let rows = 300_000;
    let fixture = Fixture::new(row.repeat(rows).as_bytes());
    let mut table = fixture.open();
    assert_eq!(table.line_count().unwrap(), rows + 1);
    let (details, points, allocation) = table.line_cache.metadata();
    assert!(details <= line_index::DETAIL_LIMIT);
    assert!(points <= line_index::CHECKPOINT_LIMIT);
    assert!(
        allocation <= 128 * 1024,
        "{allocation} allocated metadata bytes"
    );
    assert!(table.line_scan.borrow().allocated_bytes() <= PIECE_TABLE_CHUNK_SIZE);
    for line in [0, rows / 2, 5, rows - 1, rows, rows / 3, 4095, rows] {
        assert_eq!(
            table.line_text(line).unwrap(),
            if line == rows { "" } else { "é\t🦀" }
        );
        assert_eq!(table.line_start(line).unwrap(), row.len() * line);
        assert_eq!(table.line_column_at(row.len() * line).unwrap(), (line, 0));
        if line < rows {
            assert_eq!(
                table
                    .line_column_at(row.len() * line + "é\t".len())
                    .unwrap(),
                (line, 2)
            );
            assert_eq!(table.line_visual_column(line, 3, 4).unwrap(), 5);
        }
        assert_eq!(table.line_count().unwrap(), rows + 1);
        assert!(table.line_scan.borrow().allocated_bytes() <= PIECE_TABLE_CHUNK_SIZE);
    }
    // EOF of an unterminated row must not become an extra empty row when the
    // detailed terminal entry has been evicted.
    let fixture = Fixture::new(("abc\n".repeat(9000) + "tail").as_bytes());
    let mut table = fixture.open();
    assert_eq!(table.line_count().unwrap(), 9001);
    table.line_text(0).unwrap();
    assert_eq!(table.line_column_at(table.len()).unwrap(), (9000, 4));
    assert_eq!(table.line_count().unwrap(), 9001);
}

#[test]
fn saturated_page_movement_reloads_evicted_eof_details_and_clamps_columns() {
    let row = "é\t🙂\r\n";
    let fixture = Fixture::new((row.repeat(80_000) + "tail").as_bytes());
    let mut table = fixture.open();
    assert_eq!(table.line_count().unwrap(), 80_001);
    table.move_cursor_to_line_column(0, 2).unwrap();
    assert!(table.line_cache.get(80_000).is_none());
    table.move_page(true, usize::MAX, false).unwrap();
    assert_eq!((table.cursor.line, table.cursor.column), (80_000, 2));
    assert_eq!(table.cursor.position, 80_000 * row.len() + 2);
    assert_eq!(table.line_start(usize::MAX).unwrap(), table.len());
    assert_eq!(table.line_text(usize::MAX).unwrap(), "");
    table.move_page(false, usize::MAX, false).unwrap();
    assert_eq!((table.cursor.line, table.cursor.column), (0, 2));
    assert_eq!(table.cursor.position, "é\t".len());
}

#[test]
fn edits_outside_the_detailed_window_invalidate_sparse_coordinates() {
    let row = "é🦀\r\n";
    let original = row.repeat(20_000) + "tail";
    for position in [0, row.len() * 70 + 2, row.len() * 15_000, original.len()] {
        let mut table = table(&original);
        table.line_count().unwrap();
        table.line_text(8000).unwrap();
        let inserted = "中\r\n";
        table.insert(position, inserted).unwrap();
        let mut expected = original.clone();
        expected.insert_str(position, inserted);
        assert_eq!(table.line_count().unwrap(), expected.split('\n').count());
        for line in [0, 70, 8000, 15_000, 20_000, 20_001] {
            assert_eq!(
                table.line_text(line).unwrap(),
                expected
                    .split('\n')
                    .nth(line)
                    .unwrap()
                    .trim_end_matches('\r')
            );
        }
        table.delete(position, inserted.len()).unwrap();
        assert_eq!(table.line_count().unwrap(), 20_001);
        assert_eq!(table.line_column_at(table.len()).unwrap(), (20_000, 4));
        assert_eq!(table.text().unwrap(), original);
    }
}

#[test]
fn distant_shared_panes_reconstruct_evicted_windows_after_edit_undo_and_snapshot() {
    let mut a = Editor::new(EditorConfig::default()).unwrap();
    let original = "α\t🙂\r\n".repeat(20_000);
    a.insert_text(&original).unwrap();
    a.document.move_cursor(7).unwrap();
    let mut b = a.duplicate_view();
    b.document.move_cursor(original.len()).unwrap();
    a.insert_text("X\n").unwrap();
    Editor::synchronize_views(&mut a, &mut b).unwrap();
    assert_eq!(b.document.cursor.line, 20_001);
    b.undo().unwrap();
    Editor::synchronize_views(&mut b, &mut a).unwrap();
    assert_eq!(a.document.line_count().unwrap(), 20_001);
    assert_eq!(a.document.line_text(0).unwrap(), "α\t🙂");
    b.redo().unwrap();
    assert_eq!(b.document.line_count().unwrap(), 20_002);
    a.apply_formatted(&mut io::Cursor::new("replaced\n"))
        .unwrap();
    b.undo().unwrap();
    assert_eq!(b.document.line_count().unwrap(), 20_002);
    assert_eq!(b.document.line_text(20_001).unwrap(), "");
    let (details, points, allocation) = b.document.line_index_metadata();
    assert!(details <= line_index::DETAIL_LIMIT && points <= line_index::CHECKPOINT_LIMIT);
    assert!(allocation <= 128 * 1024);
}
