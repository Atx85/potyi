// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

struct Source {
    epoch: u64,
    first: u64,
    rows: Vec<RecordText>,
    reads: usize,
}
impl Source {
    fn new(rows: &[(&str, bool)]) -> Self {
        Self {
            epoch: 1,
            first: 10,
            rows: rows
                .iter()
                .map(|(text, join_next)| RecordText {
                    text: (*text).into(),
                    join_next: *join_next,
                    omitted: false,
                    terminated: false,
                })
                .collect(),
            reads: 0,
        }
    }
    fn at(&self, index: usize, offset: usize) -> Anchor {
        Anchor {
            epoch: self.epoch,
            record_id: self.first + index as u64,
            utf8_byte_offset: offset,
        }
    }
}
impl TextSource for Source {
    fn bounds(&self) -> Bounds {
        Bounds {
            epoch: self.epoch,
            first: self.first,
            end: self.first + self.rows.len() as u64,
        }
    }
    fn text(&mut self, record_id: u64) -> io::Result<RecordText> {
        self.reads += 1;
        self.rows
            .get((record_id - self.first) as usize)
            .cloned()
            .ok_or_else(|| io::Error::other("Missing test row"))
    }
}

#[test]
fn logical_movement_uses_utf8_and_treats_soft_wrap_as_one_line() {
    let mut source = Source::new(&[("a界", true), ("bc", false), ("next", false)]);
    let mut selection = Selection::default();
    let start = source.at(0, 0);
    selection.apply(&mut source, start, false).unwrap();
    selection
        .move_logical(&mut source, Motion::NextCharacter, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(source.at(0, 1)));
    selection
        .move_logical(&mut source, Motion::NextCharacter, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(source.at(1, 0)));
    selection
        .move_logical(&mut source, Motion::PreviousCharacter, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(source.at(0, 1)));
    selection
        .move_logical(&mut source, Motion::LineLastCharacter, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(source.at(1, 1)));
    selection
        .move_logical(&mut source, Motion::LineStart, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(start));
    selection
        .move_logical(&mut source, Motion::NextWord, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(source.at(2, 0)));
    selection
        .move_logical(&mut source, Motion::PreviousWord, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(start));
}

#[test]
fn selection_survives_append_reflow_and_clamps_only_eviction_or_clear() {
    let mut source = Source::new(&[("one", false), ("界 two", false), ("three", false)]);
    let mut selection = Selection::default();
    let start = source.at(0, 1);
    let end = source.at(1, 3);
    selection.apply(&mut source, start, false).unwrap();
    selection.apply(&mut source, end, true).unwrap();
    source.rows.push(RecordText {
        text: "appended".into(),
        join_next: false,
        omitted: false,
        terminated: false,
    });
    selection.clamp(&mut source).unwrap();
    assert_eq!(
        selection.range(&mut source).unwrap(),
        Some(Range { start, end })
    );
    source.rows.remove(0);
    source.first += 1;
    selection.clamp(&mut source).unwrap();
    assert_eq!(selection.anchor(), Some(source.at(0, 0)));
    assert_eq!(selection.cursor(), Some(end));
    source.epoch += 1;
    selection.clamp(&mut source).unwrap();
    assert!(!selection.focused());
    assert_eq!(selection.cursor(), None);
}

#[test]
fn stale_mouse_and_navigation_targets_never_rebind_after_clear() {
    let mut source = Source::new(&[("old", false)]);
    let target = source.at(0, 1);
    source.epoch += 1;
    source.first += 1;
    source.rows[0].text = "new".into();
    let mut selection = Selection::default();
    assert!(selection.apply(&mut source, target, false).is_err());
    assert!(
        selection
            .mouse_begin(&mut source, target, 1, false)
            .is_err()
    );
    assert_eq!(selection.cursor(), None);
    assert!(!selection.focused());
}

#[test]
fn conventional_and_vim_keys_return_layout_requests_and_visual_copy_ranges() {
    let mut selection = Selection::default();
    assert_eq!(
        selection.key(
            Key::PageDown,
            Modifiers {
                shift: true,
                ..Modifiers::default()
            },
            false
        ),
        Request::Pages {
            delta: 1,
            extend: true
        }
    );
    assert_eq!(
        selection.key(Key::Home, Modifiers::default(), false),
        Request::VisualEdge {
            end: false,
            extend: false
        }
    );
    assert_eq!(
        selection.key(Key::G, Modifiers::default(), true),
        Request::None
    );
    assert_eq!(
        selection.key(Key::G, Modifiers::default(), true),
        Request::Move {
            motion: Motion::First,
            extend: false
        }
    );
    assert_eq!(
        selection.key(
            Key::G,
            Modifiers {
                shift: true,
                ..Modifiers::default()
            },
            true
        ),
        Request::Move {
            motion: Motion::LastLineStart,
            extend: false
        }
    );
    let mut source = Source::new(&[("héllo", false), ("world", false)]);
    let position = source.at(0, 1);
    selection.apply(&mut source, position, false).unwrap();
    selection.key(Key::V, Modifiers::default(), true);
    assert_eq!(selection.copy(&mut source, false, false).unwrap(), "é");
    selection.key(
        Key::V,
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
        true,
    );
    assert_eq!(
        selection.copy(&mut source, false, false).unwrap(),
        "héllo\n"
    );
    selection.finish_yank(&mut source).unwrap();
    assert_eq!(selection.visual(), Visual::Off);
    assert_eq!(selection.cursor(), Some(source.at(0, 0)));
    assert_eq!(
        selection.key(Key::Escape, Modifiers::default(), true),
        Request::FocusPrompt
    );
    assert!(!selection.focused());
}

#[test]
fn double_word_triple_line_and_shift_drag_use_logical_anchors() {
    let mut source = Source::new(&[("one two", true), (" continued", false), ("last", false)]);
    let mut selection = Selection::default();
    let position = source.at(0, 5);
    selection
        .mouse_begin(&mut source, position, 2, false)
        .unwrap();
    assert_eq!(selection.copy(&mut source, false, false).unwrap(), "two");
    let position = source.at(0, 0);
    selection.mouse_drag(&mut source, position).unwrap();
    assert_eq!(
        selection.copy(&mut source, false, false).unwrap(),
        "one two"
    );
    let position = source.at(1, 3);
    selection
        .mouse_begin(&mut source, position, 3, false)
        .unwrap();
    assert_eq!(
        selection.copy(&mut source, false, false).unwrap(),
        "one two continued\n"
    );
    selection.cancel_mouse();
    let position = source.at(2, 2);
    selection
        .mouse_begin(&mut source, position, 1, true)
        .unwrap();
    assert_eq!(
        selection.copy(&mut source, false, false).unwrap(),
        "one two continued\nla"
    );
}

#[test]
fn selected_and_all_copy_join_wraps_without_ansi_and_fail_atomically_at_cap() {
    let mut source = Source::new(&[("ab", true), ("界", false), ("end", false)]);
    let start = source.at(0, 1);
    let end = source.at(2, 2);
    assert_eq!(
        copy_range(&mut source, Range { start, end }, 100).unwrap(),
        "b界\nen"
    );
    assert!(copy_range(&mut source, Range { start, end }, 4).is_err());
    let mut selection = Selection::default();
    assert_eq!(
        selection.copy(&mut source, true, false).unwrap(),
        "ab界\nend"
    );
    let mut formatted = vt100::Parser::new(1, 20, 0);
    formatted.process(b"\x1b[31mred\x1b[0m");
    let record = Record {
        id: 0,
        epoch: 1,
        source: 1,
        cwd: std::env::temp_dir(),
        data: RecordData::Terminal {
            cols: 20,
            wrapped: false,
            formatted: formatted.screen().row_scrollback_formatted(0),
        },
    };
    assert_eq!(
        record_text(&record).unwrap(),
        RecordText {
            text: "red".into(),
            join_next: false,
            omitted: false,
            terminated: false,
        }
    );
}

#[test]
fn soft_wrap_cannot_join_a_different_source_or_native_record() {
    let mut first = Record {
        id: 3,
        epoch: 1,
        source: 9,
        cwd: std::env::temp_dir(),
        data: RecordData::Terminal {
            cols: 20,
            wrapped: true,
            formatted: b"first".to_vec(),
        },
    };
    let mut second = Record {
        id: 4,
        epoch: 1,
        source: 9,
        cwd: std::env::temp_dir(),
        data: RecordData::Terminal {
            cols: 20,
            wrapped: false,
            formatted: b"second".to_vec(),
        },
    };
    assert!(joins_next(&first, &second));
    let mut a = record_text(&first).unwrap();
    let b = record_text(&second).unwrap();
    second.source = 10;
    assert!(!joins_next(&first, &second));
    a.join_next = joins_next(&first, &second);
    let mut source = Source::new(&[]);
    source.rows = vec![a, b];
    assert_eq!(
        Selection::default().copy(&mut source, true, false).unwrap(),
        "first\nsecond"
    );
    second.source = first.source;
    second.data = RecordData::Header("another command".into());
    assert!(!joins_next(&first, &second));
    first.epoch += 1;
    assert!(!joins_next(&first, &second));
}

#[test]
fn copy_capacity_stays_within_cap_near_non_power_of_two_row_sizes() {
    let limit = 1000;
    let mut text = String::new();
    let row = "x".repeat(63);
    for _ in 0..15 {
        append_checked(&mut text, &row, limit).unwrap();
        assert!(text.capacity() <= limit);
    }
    let previous = text.clone();
    assert!(append_checked(&mut text, &row, limit).is_err());
    assert_eq!(text, previous);
}

#[test]
fn future_tail_ids_keep_selection_when_snapshot_rows_become_committed() {
    let mut source = Source::new(&[("saved", false), ("active first", true), ("尾tail", false)]);
    let mut selection = Selection::default();
    let start = source.at(1, 0);
    let end = source.at(2, "尾".len());
    selection.apply(&mut source, start, false).unwrap();
    selection.apply(&mut source, end, true).unwrap();
    assert_eq!(
        selection.copy(&mut source, false, false).unwrap(),
        "active first尾"
    );
    // The TextSource keeps the same IDs while moving rows from its bounded
    // snapshot to disk; display wrapping and storage location are immaterial.
    selection.clamp(&mut source).unwrap();
    assert_eq!(
        selection.range(&mut source).unwrap(),
        Some(Range { start, end })
    );
}

#[test]
fn long_word_navigation_decodes_each_record_only_once() {
    let mut source = Source::new(&[("", false)]);
    source.rows[0].text = "a".repeat(64 * 1024);
    let mut selection = Selection::default();
    let start = source.at(0, 0);
    selection.apply(&mut source, start, false).unwrap();
    source.reads = 0;
    selection
        .move_logical(&mut source, Motion::NextWord, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(source.at(0, 64 * 1024)));
    assert!(source.reads <= 5, "Decoded {} times", source.reads);
}

fn presentation_source(mut records: Vec<Record>) -> Source {
    for (index, record) in records.iter_mut().enumerate() {
        record.id = 10 + index as u64;
    }
    let mut rows = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let mut row = record_text_at_end(record, index + 1 == records.len()).unwrap();
        row.join_next &= records
            .get(index + 1)
            .is_some_and(|next| joins_next(record, next));
        if matches!(
            record.data,
            RecordData::Terminal { .. } | RecordData::Native(_)
        ) && records.get(index + 1).is_some_and(|next| {
            next.source == record.source && matches!(next.data, RecordData::Result { .. })
        }) {
            row.terminated = true;
        }
        rows.push(row);
    }
    Source {
        epoch: 1,
        first: 10,
        rows,
        reads: 0,
    }
}
fn presentation_record(source: u64, data: RecordData) -> Record {
    Record {
        id: 0,
        epoch: 1,
        source,
        cwd: std::env::temp_dir(),
        data,
    }
}
fn completed() -> RecordData {
    RecordData::Result {
        relative_safe: true,
        status: 0,
        text: "Exit 0".into(),
    }
}
fn output(text: &str) -> RecordData {
    RecordData::Terminal {
        cols: 80,
        wrapped: false,
        formatted: text.as_bytes().to_vec(),
    }
}

#[test]
fn completed_metadata_is_invisible_in_copy_and_navigation_between_commands() {
    let mut source = presentation_source(vec![
        presentation_record(1, RecordData::Header("printf first".into())),
        presentation_record(1, output("first")),
        presentation_record(1, completed()),
        presentation_record(2, RecordData::Header("$ printf second".into())),
        presentation_record(2, output("second")),
        presentation_record(2, completed()),
    ]);
    let mut selection = Selection::default();
    assert_eq!(
        selection.copy(&mut source, true, false).unwrap(),
        "$ printf first\nfirst\n$ printf second\nsecond\n"
    );
    selection
        .move_logical(&mut source, Motion::End, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(source.at(5, 0)));
    let first_end = source.at(1, 5);
    selection.apply(&mut source, first_end, false).unwrap();
    selection
        .move_logical(&mut source, Motion::NextCharacter, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(source.at(3, 0)));
    selection
        .move_logical(&mut source, Motion::PreviousCharacter, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(first_end));
    let hidden = source.at(2, 0);
    selection.apply(&mut source, hidden, false).unwrap();
    assert_eq!(selection.cursor(), Some(source.at(3, 0)));
}

#[test]
fn select_all_and_linewise_copy_include_legacy_final_newline_but_manual_end_excludes_it() {
    let mut source = presentation_source(vec![
        presentation_record(1, RecordData::Header("pwd".into())),
        presentation_record(1, completed()),
    ]);
    let mut selection = Selection::default();
    selection.select_all(&mut source).unwrap();
    assert_eq!(selection.cursor(), Some(source.at(1, 0)));
    assert_eq!(
        selection.copy(&mut source, false, false).unwrap(),
        "$ pwd\n"
    );
    let first = source.at(0, 0);
    let end = source.at(0, 5);
    selection.apply(&mut source, first, false).unwrap();
    selection.apply(&mut source, end, true).unwrap();
    assert_eq!(selection.copy(&mut source, false, false).unwrap(), "$ pwd");
    selection.key(
        Key::V,
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
        true,
    );
    assert_eq!(selection.copy(&mut source, false, true).unwrap(), "$ pwd\n");
}

#[test]
fn copy_keeps_incomplete_live_partial_unterminated_and_metadata_only_empty() {
    let mut source = presentation_source(vec![
        presentation_record(1, RecordData::Header("read name".into())),
        presentation_record(1, output("partial")),
    ]);
    let mut selection = Selection::default();
    assert_eq!(
        selection.copy(&mut source, true, false).unwrap(),
        "$ read name\npartial"
    );
    let mut empty = presentation_source(vec![presentation_record(1, completed())]);
    assert_eq!(selection.copy(&mut empty, true, false).unwrap(), "");
}

#[test]
fn result_display_matches_legacy_without_changing_raw_provenance() {
    for (status, text, expected, omitted) in [
        (0, "Exit 0", "", true),
        (7, "Exit 7", "[finished with exit code 7]", false),
        (130, "Exit 130", "[command terminated]", false),
        (130, "Browsing stopped", "[listing stopped]", false),
        (130, "[listing stopped]", "[listing stopped]", false),
    ] {
        let record = presentation_record(
            1,
            RecordData::Result {
                relative_safe: false,
                status,
                text: text.into(),
            },
        );
        let projected = record_text(&record).unwrap();
        assert_eq!(projected.text, expected);
        assert_eq!(projected.omitted, omitted);
        let RecordData::Result {
            text: raw,
            relative_safe,
            ..
        } = record.data
        else {
            panic!()
        };
        assert_eq!(raw, text);
        assert!(!relative_safe);
    }
    let title = "Commit deadbeef · Back / Alt+Left returns to the log";
    assert_eq!(
        record_text(&presentation_record(1, RecordData::Header(title.into())))
            .unwrap()
            .text,
        title
    );
}

#[test]
fn fixed_native_columns_copy_persisted_spaces_and_join_only_same_source_entries() {
    fn entry(name: &str, padding: u8, join: bool) -> RecordData {
        let mut bytes = vec![0u8; 16];
        bytes[..4].copy_from_slice(&(name.len() as u32).to_le_bytes());
        bytes[14] = 16 | if join { 8 } else { 0 };
        bytes[15] = padding;
        bytes.extend_from_slice(name.as_bytes());
        RecordData::Native(bytes)
    }
    let mut source = presentation_source(vec![
        presentation_record(1, RecordData::Header("ls".into())),
        presentation_record(1, entry("alpha", 3, true)),
        presentation_record(1, entry("é.rs", 2, true)),
        presentation_record(1, entry("last", 0, false)),
        presentation_record(1, completed()),
        presentation_record(2, RecordData::Header("pwd".into())),
        presentation_record(2, entry("new", 4, true)),
        presentation_record(3, RecordData::Header("other".into())),
        presentation_record(3, completed()),
    ]);
    assert_eq!(
        Selection::default().copy(&mut source, true, false).unwrap(),
        "$ ls\nalpha   é.rs  last\n$ pwd\nnew    \n$ other\n"
    );
    assert!(source.rows[1].join_next);
    assert!(
        !source.rows[6].join_next,
        "fixed entries cannot absorb a command header"
    );
}

#[test]
fn failed_command_footer_owns_empty_eof_and_copies_one_final_newline() {
    let mut source = presentation_source(vec![
        presentation_record(1, RecordData::Header("false".into())),
        presentation_record(
            1,
            RecordData::Result {
                relative_safe: true,
                status: 7,
                text: "Exit 7".into(),
            },
        ),
    ]);
    let mut selection = Selection::default();
    selection
        .move_logical(&mut source, Motion::End, false)
        .unwrap();
    assert_eq!(
        selection.cursor(),
        Some(source.at(1, "[finished with exit code 7]\n".len()))
    );
    assert_eq!(
        selection.copy(&mut source, true, false).unwrap(),
        "$ false\n[finished with exit code 7]\n"
    );
    selection
        .move_logical(&mut source, Motion::LineStart, false)
        .unwrap();
    assert_eq!(
        selection.cursor(),
        Some(source.at(1, "[finished with exit code 7]\n".len()))
    );
}

#[test]
fn plain_command_output_copy_preserves_tabs_trailing_spaces_and_long_row_continuations() {
    let mut source = presentation_source(vec![
        presentation_record(1, RecordData::Header("printf literal".into())),
        presentation_record(
            1,
            RecordData::Terminal {
                cols: 0,
                wrapped: true,
                formatted: "\t東京  ".as_bytes().to_vec(),
            },
        ),
        presentation_record(
            1,
            RecordData::Terminal {
                cols: 0,
                wrapped: false,
                formatted: b"trail  ".to_vec(),
            },
        ),
        presentation_record(1, completed()),
    ]);
    assert_eq!(
        Selection::default().copy(&mut source, true, false).unwrap(),
        "$ printf literal\n\t東京  trail  \n"
    );
    let raw = presentation_record(
        1,
        RecordData::Terminal {
            cols: 0,
            wrapped: true,
            formatted: b"raw".to_vec(),
        },
    );
    let mut vt = presentation_record(1, output("grid"));
    let mut raw = raw;
    raw.id = 1;
    vt.id = 2;
    assert!(!joins_next(&raw, &vt));
}

#[test]
fn native_listing_final_blank_line_survives_eof_and_a_following_command() {
    let native = |text: &str| {
        let mut bytes = vec![0u8; 16];
        bytes[..4].copy_from_slice(&(text.len() as u32).to_le_bytes());
        bytes[8] = 4;
        bytes.extend_from_slice(text.as_bytes());
        bytes
    };
    let rows = || {
        vec![
            presentation_record(1, RecordData::Header("ls".into())),
            presentation_record(1, RecordData::Native(native("file.txt"))),
            presentation_record(1, RecordData::Native(native(""))),
            presentation_record(1, completed()),
        ]
    };
    let mut source = presentation_source(rows());
    let mut selection = Selection::default();
    assert_eq!(
        selection.copy(&mut source, true, false).unwrap(),
        "$ ls\nfile.txt\n\n"
    );
    selection
        .move_logical(&mut source, Motion::End, false)
        .unwrap();
    assert_eq!(selection.cursor(), Some(source.at(3, 0)));
    selection
        .move_logical(&mut source, Motion::PreviousCharacter, true)
        .unwrap();
    assert_eq!(selection.copy(&mut source, false, false).unwrap(), "\n");
    let mut records = rows();
    records.extend([
        presentation_record(2, RecordData::Header("pwd".into())),
        presentation_record(2, output("directory")),
        presentation_record(2, completed()),
    ]);
    let mut source = presentation_source(records);
    assert_eq!(
        Selection::default().copy(&mut source, true, false).unwrap(),
        "$ ls\nfile.txt\n\n$ pwd\ndirectory\n"
    );
}
