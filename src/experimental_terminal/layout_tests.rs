// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::super::transcript::Transcript;
use super::*;
use std::{path::PathBuf, sync::Arc};

fn cwd() -> PathBuf {
    std::env::temp_dir().canonicalize().unwrap()
}
fn native(text: &str, cell: Option<u8>) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(text.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&[
        0,
        0,
        1,
        150,
        220,
        165,
        u8::from(cell.is_some()),
        cell.unwrap_or(0),
    ]);
    bytes.extend_from_slice(text.as_bytes());
    bytes
}
fn layout(shared: SharedTranscript, columns: u16) -> Layout {
    let mut layout = Layout::new(shared, Arc::new(|| {})).unwrap();
    layout.configure(Metrics {
        columns,
        tab_width: 4,
    });
    drain(&mut layout);
    layout
}
fn drain(layout: &mut Layout) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while layout.pending() {
        layout.poll().unwrap();
        assert!(Instant::now() < deadline, "Layout bounded work timed out");
    }
}
fn view(layout: &mut Layout, start: usize, count: usize) -> Vec<Fragment> {
    for _ in 0..8 {
        let rows = layout.visible(start, count).unwrap();
        if layout.query.as_ref().is_some_and(|query| {
            query.start == start.saturating_add(layout.origin)
                && query.count == count
                && query.ready
                && query.revision == layout.projection_revision
        }) {
            return rows;
        }
        if layout.complete && layout.len() == 0 {
            return rows;
        }
        drain(layout);
    }
    panic!("Viewport did not reach the requested projection")
}
fn shared_view(layout: &mut Layout, start: usize, count: usize) -> Arc<Vec<Fragment>> {
    for _ in 0..8 {
        let rows = layout.visible_shared(start, count).unwrap();
        if layout.query.as_ref().is_some_and(|query| {
            query.start == start.saturating_add(layout.origin)
                && query.count == count
                && query.ready
                && query.revision == layout.projection_revision
        }) {
            return rows;
        }
        if layout.complete && layout.len() == 0 {
            return rows;
        }
        drain(layout);
    }
    panic!("Shared viewport did not reach the requested projection")
}
fn frame_bytes(rows: &Vec<Fragment>) -> usize {
    rows.iter().map(Fragment::bytes).sum::<usize>()
        + (rows.capacity() - rows.len()) * mem::size_of::<Fragment>()
}
fn row_at(layout: &mut Layout, anchor: Anchor) -> Option<usize> {
    let value = layout.row_at(anchor).unwrap();
    if value.is_some() {
        return value;
    }
    drain(layout);
    layout.row_at(anchor).unwrap()
}

fn assert_counting_matches_projection(
    shared: &SharedTranscript,
    metrics: Metrics,
    font_columns: bool,
    pixels: Option<PixelMetrics>,
) {
    let (epoch, base, end) = {
        let store = shared.lock().unwrap();
        (
            store.epoch(),
            store.base_id(),
            store.base_id() + store.len() as u64,
        )
    };
    let source = Source {
        store: shared,
        disk_end: end,
        tail: None,
    };
    let mut projected = Flow::new(Cursor { id: base, byte: 0 });
    let mut counted = Flow::counting(Cursor { id: base, byte: 0 });
    let mut projected_widths = ScalarWidths::default();
    let mut counted_widths = ScalarWidths::default();
    let mut measure_projected = |character: char| if character == '\u{301}' { 3 } else { 7 };
    let mut measure_counted = |character: char| if character == '\u{301}' { 3 } else { 7 };
    let mut projected_measure: Option<&mut dyn FnMut(char) -> i32> = Some(&mut measure_projected);
    let mut counted_measure: Option<&mut dyn FnMut(char) -> i32> = Some(&mut measure_counted);
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut fragments = 0;
    loop {
        let mut projected_budget = Budget {
            deadline,
            reads: usize::MAX,
        };
        let mut counted_budget = Budget {
            deadline,
            reads: usize::MAX,
        };
        let full = projected
            .next(
                &source,
                end,
                epoch,
                metrics,
                font_columns,
                pixels,
                &mut projected_widths,
                &mut projected_measure,
                &mut projected_budget,
            )
            .unwrap();
        let shape = counted
            .next(
                &source,
                end,
                epoch,
                metrics,
                font_columns,
                pixels,
                &mut counted_widths,
                &mut counted_measure,
                &mut counted_budget,
            )
            .unwrap();
        assert_eq!(projected.cursor, counted.cursor);
        assert_eq!(projected.output_hard, counted.output_hard);
        match (full, shape) {
            (Some((start, full)), Some((count_start, shape))) => {
                assert_eq!(
                    (start, full.start, full.end),
                    (count_start, shape.start, shape.end)
                );
                assert_eq!(
                    shape.text.capacity(),
                    0,
                    "Counting must not allocate rendered text"
                );
                assert_eq!(
                    shape.segments.capacity(),
                    0,
                    "Counting must not allocate hit/style spans"
                );
                fragments += 1;
            }
            (None, None) => {
                assert!(projected.finished(end) && counted.finished(end));
                break;
            }
            _ => panic!("Counting and projection disagreed on a visual row"),
        }
        assert!(Instant::now() < deadline);
    }
    assert!(fragments > 0);
}

fn assert_ascii_counting_matches_scalar<const READS: usize>(
    shared: &SharedTranscript,
    metrics: Metrics,
    font_columns: bool,
    pixels: Option<PixelMetrics>,
) {
    let (epoch, base, end) = {
        let store = shared.lock().unwrap();
        (
            store.epoch(),
            store.base_id(),
            store.base_id() + store.len() as u64,
        )
    };
    let source = Source {
        store: shared,
        disk_end: end,
        tail: None,
    };
    let mut bulk = Flow::counting(Cursor { id: base, byte: 0 });
    let mut scalar = Flow::counting(Cursor { id: base, byte: 0 });
    let mut bulk_widths = ScalarWidths::default();
    let mut scalar_widths = ScalarWidths::default();
    let mut measure_bulk = |character: char| if character == '\u{301}' { 3 } else { 7 };
    let mut measure_scalar = |character: char| if character == '\u{301}' { 3 } else { 7 };
    let mut bulk_measure: Option<&mut dyn FnMut(char) -> i32> = Some(&mut measure_bulk);
    let mut scalar_measure: Option<&mut dyn FnMut(char) -> i32> = Some(&mut measure_scalar);
    // A wall deadline cannot synchronize two serial traversals. Make it
    // nonbinding here and bound the number of steps independently instead.
    let deadline = Instant::now() + Duration::from_secs(3600);
    for _ in 0..4096 {
        // A join can require reloading the preceding row after word-wrap
        // rewind, then reading its successor. Joined fixtures need two reads;
        // one-read yielding is checked separately on a nonjoining row.
        let mut bulk_budget = Budget {
            deadline,
            reads: READS,
        };
        let mut scalar_budget = Budget {
            deadline,
            reads: READS,
        };
        let fast = bulk
            .next_inner::<true>(
                &source,
                end,
                epoch,
                metrics,
                font_columns,
                pixels,
                &mut bulk_widths,
                &mut bulk_measure,
                &mut bulk_budget,
            )
            .unwrap();
        let original = scalar
            .next_inner::<false>(
                &source,
                end,
                epoch,
                metrics,
                font_columns,
                pixels,
                &mut scalar_widths,
                &mut scalar_measure,
                &mut scalar_budget,
            )
            .unwrap();
        assert_eq!(fast, original);
        assert_eq!(bulk.cursor, scalar.cursor);
        assert_eq!(
            bulk.loaded.as_ref().map(|row| row.id),
            scalar.loaded.as_ref().map(|row| row.id)
        );
        assert_eq!(
            (bulk.hard, bulk.output_hard),
            (scalar.hard, scalar.output_hard)
        );
        assert_eq!(bulk_budget.reads, scalar_budget.reads);
        match (&bulk.building, &scalar.building) {
            (Some(a), Some(b)) => {
                assert_eq!(
                    (
                        a.start,
                        a.hard,
                        &a.fragment,
                        a.text_bytes,
                        a.segment_count,
                        &a.last_segment
                    ),
                    (
                        b.start,
                        b.hard,
                        &b.fragment,
                        b.text_bytes,
                        b.segment_count,
                        &b.last_segment
                    )
                );
                assert_eq!(
                    (a.width, a.tab_column, a.wrap_pixels),
                    (b.width, b.tab_column, b.wrap_pixels)
                );
                match (&a.mode, &b.mode) {
                    (Mode::Text { last_break: a }, Mode::Text { last_break: b }) => {
                        assert_eq!(a, b)
                    }
                    (Mode::Packed { .. }, Mode::Packed { .. }) => {}
                    _ => panic!("Bulk traversal changed its wrapping mode"),
                }
            }
            (None, None) => {}
            _ => panic!("Bulk traversal changed a bounded yield"),
        }
        if bulk.finished(end) {
            assert!(scalar.finished(end));
            return;
        }
    }
    panic!(
        "Counting exceeded its bounded test steps at {:?}",
        bulk.cursor
    );
}

#[test]
fn ascii_counting_keeps_one_read_yield_at_64_scalars() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    shared
        .lock()
        .unwrap()
        .append_plain(&cwd, 1, &"A".repeat(128), false)
        .unwrap();
    let (epoch, base, end) = {
        let store = shared.lock().unwrap();
        (
            store.epoch(),
            store.base_id(),
            store.base_id() + store.len() as u64,
        )
    };
    let source = Source {
        store: &shared,
        disk_end: end,
        tail: None,
    };
    let metrics = Metrics {
        columns: 1000,
        tab_width: 4,
    };
    let mut flow = Flow::counting(Cursor { id: base, byte: 0 });
    let mut widths = ScalarWidths::default();
    let mut measure = None;
    let mut budget = Budget {
        deadline: Instant::now() + Duration::from_secs(3600),
        reads: 1,
    };
    assert!(
        flow.next_inner::<true>(
            &source,
            end,
            epoch,
            metrics,
            true,
            None,
            &mut widths,
            &mut measure,
            &mut budget
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(budget.reads, 0);
    assert_eq!(flow.cursor, Cursor { id: base, byte: 64 });
    assert_eq!(flow.building.as_ref().unwrap().text_bytes, 64);
    assert_ascii_counting_matches_scalar::<1>(&shared, metrics, true, None);
}

#[test]
fn ascii_counting_preserves_scalar_wrapping_anchors_styles_and_yields() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    {
        let mut store = shared.lock().unwrap();
        store.append_header(&cwd, 1, "printf").unwrap();
        let printable: String = (b' '..=b'~').map(char::from).collect();
        store
            .append_plain(&cwd, 1, &format!("{} {}", printable, printable), true)
            .unwrap();
        store
            .append_plain(
                &cwd,
                1,
                " continuing words\tASCII\u{7f} 東京 a\u{301}\u{a0}end   ",
                false,
            )
            .unwrap();
        store
            .append_plain(&cwd, 2, "error: source.rs:12:9 plain after range", false)
            .unwrap();
        for name in ["two word filename.txt", "ASCII 東京\tend"] {
            store.append_native(&cwd, 2, &native(name, None)).unwrap();
        }
        store
            .append_terminal(&cwd, 2, 80, false, b"\x1b[31mred ASCII\x1b[0m rest")
            .unwrap();
        store.append_result(&cwd, 2, 1, "Exit 1").unwrap();
    }
    for columns in [1, 2, 7, 63, 64, 65, 127, 1000] {
        for font_columns in [false, true] {
            let metrics = Metrics {
                columns,
                tab_width: 4,
            };
            assert_ascii_counting_matches_scalar::<2>(&shared, metrics, font_columns, None);
            for width in [1, usize::from(columns) * 7, usize::from(columns) * 7 + 3] {
                assert_ascii_counting_matches_scalar::<2>(
                    &shared,
                    metrics,
                    font_columns,
                    Some(PixelMetrics {
                        width,
                        cell: 7,
                        font_key: 1,
                    }),
                );
            }
        }
    }
}

#[test]
fn ascii_counting_preserves_fragment_caps_and_long_separator_runs() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    {
        let mut store = shared.lock().unwrap();
        store
            .append_plain(&cwd, 1, &" ".repeat(MAX_FRAGMENT_BYTES), true)
            .unwrap();
        store
            .append_plain(&cwd, 1, &"x".repeat(MAX_FRAGMENT_BYTES), true)
            .unwrap();
        store.append_plain(&cwd, 1, " tail", false).unwrap();
        store
            .append_plain(&cwd, 2, &"\txa ".repeat(MAX_FRAGMENT_SEGMENTS + 3), false)
            .unwrap();
    }
    for font_columns in [false, true] {
        assert_ascii_counting_matches_scalar::<2>(
            &shared,
            Metrics {
                columns: u16::MAX,
                tab_width: 4,
            },
            font_columns,
            None,
        );
        assert_ascii_counting_matches_scalar::<2>(
            &shared,
            Metrics {
                columns: 1000,
                tab_width: 4,
            },
            font_columns,
            Some(PixelMetrics {
                width: MAX_FRAGMENT_BYTES * 7 + 3,
                cell: 7,
                font_key: 1,
            }),
        );
    }
}

#[test]
fn shape_only_counting_matches_wrapping_styles_packing_and_source_boundaries() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    {
        let mut store = shared.lock().unwrap();
        store.append_header(&cwd, 1, "ls").unwrap();
        for name in ["../", "á東京", "space name", "tail"] {
            store
                .append_native(&cwd, 1, &native(name, Some(12)))
                .unwrap();
        }
        store
            .append_plain(&cwd, 2, "error: 東京\ta\u{301} word  ", true)
            .unwrap();
        store
            .append_plain(&cwd, 2, "continued longwordwithoutspaces", false)
            .unwrap();
        store
            .append_plain(&cwd, 3, "source changes prevent joining", true)
            .unwrap();
        store
            .append_plain(&cwd, 4, "new source\n\nlast  ", false)
            .unwrap();
        store
            .append_terminal(&cwd, 4, 32, false, b"\x1b[31m-red\x1b[0m\tplain")
            .unwrap();
        store.append_result(&cwd, 4, 7, "Exit 7").unwrap();
    }
    for columns in [1, 7, 24, 80] {
        for font_columns in [false, true] {
            assert_counting_matches_projection(
                &shared,
                Metrics {
                    columns,
                    tab_width: 4,
                },
                font_columns,
                None,
            );
            assert_counting_matches_projection(
                &shared,
                Metrics {
                    columns,
                    tab_width: 4,
                },
                font_columns,
                Some(PixelMetrics {
                    width: usize::from(columns) * 7,
                    cell: 7,
                    font_key: 1,
                }),
            );
        }
    }
}

#[test]
fn shape_only_counting_keeps_fragment_byte_and_segment_boundaries() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    {
        let mut store = shared.lock().unwrap();
        store
            .append_plain(&cwd, 1, &"\t".repeat(MAX_FRAGMENT_SEGMENTS + 17), false)
            .unwrap();
        store
            .append_plain(&cwd, 1, &" ".repeat(MAX_FRAGMENT_BYTES), true)
            .unwrap();
        store
            .append_plain(&cwd, 1, " remainder\u{301} ", false)
            .unwrap();
    }
    assert_counting_matches_projection(
        &shared,
        Metrics {
            columns: u16::MAX,
            tab_width: 4,
        },
        true,
        None,
    );
}

#[test]
fn shared_view_reuses_ready_frame_without_copying_text_or_segments() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    {
        let mut store = shared.lock().unwrap();
        store.append_header(&cwd, 1, "ls").unwrap();
        store
            .append_native(&cwd, 1, &native("file with spaces.txt", Some(24)))
            .unwrap();
        store
            .append_plain(&cwd, 2, "error: source.rs:2:3\t東京a\u{301}  ", false)
            .unwrap();
        store.append_result(&cwd, 2, 3, "Exit 3").unwrap();
    }
    let mut layout = layout(shared, 32);
    let first = shared_view(&mut layout, 0, 20);
    let text = first[0].text.as_ptr();
    let spans = first[0].segments.as_ptr();
    let expected = (*first).clone();
    for _ in 0..20 {
        layout.request_visible(0, 20).unwrap();
        layout.poll().unwrap();
        let next = layout.visible_shared(0, 20).unwrap();
        assert!(Arc::ptr_eq(&first, &next));
        assert!(Arc::ptr_eq(
            &next,
            &layout.query.as_ref().unwrap().fragments
        ));
        assert!(Arc::ptr_eq(&next, &layout.published.as_ref().unwrap().rows));
        assert_eq!(next[0].text.as_ptr(), text);
        assert_eq!(next[0].segments.as_ptr(), spans);
        assert_eq!(&*next, &expected);
    }
}

#[test]
fn growing_shared_frame_preserves_published_anchors_and_byte_budget() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    {
        let mut store = shared.lock().unwrap();
        for text in ["first\t東京", "second a\u{301}", "third", "fourth  "] {
            store.append_plain(&cwd, 1, text, false).unwrap();
        }
    }
    let mut layout = layout(shared, 40);
    layout.request_visible(0, 10).unwrap();
    // Publish exactly one completed visual row before a later poll resumes the
    // query. This exercises partial frames without depending on a time budget.
    let mut query = layout.query.take().unwrap();
    let source = Source {
        store: &layout.store,
        disk_end: layout.disk_end,
        tail: None,
    };
    let mut budget = Budget {
        deadline: Instant::now() + Duration::from_secs(20),
        reads: usize::MAX,
    };
    let (_, first) = query
        .flow
        .next(
            &source,
            query.end,
            layout.epoch,
            layout.metrics,
            layout.font_columns,
            layout.pixels,
            &mut layout.scalar_widths,
            &mut None,
            &mut budget,
        )
        .unwrap()
        .unwrap();
    query.bytes = first.bytes();
    query.fragments_mut().push(first);
    query.row += 1;
    layout.query = Some(query);
    let captured = layout.visible_shared(0, 10).unwrap();
    let before = (*captured).clone();
    assert_eq!(captured.len(), 1);
    assert!(Arc::ptr_eq(
        &captured,
        &layout.query.as_ref().unwrap().fragments
    ));
    drain(&mut layout);
    let current = layout.visible_shared(0, 10).unwrap();
    assert_eq!(current.len(), 4);
    assert!(!Arc::ptr_eq(&captured, &current));
    assert_eq!(
        &*captured, &before,
        "Painting and hit-test anchors stay immutable"
    );
    assert_eq!(current[0], captured[0]);
    assert!(frame_bytes(&captured) <= MAX_VISIBLE_BYTES);
    assert!(frame_bytes(&current) <= MAX_VISIBLE_BYTES);
    assert!(frame_bytes(&captured) + frame_bytes(&current) <= 2 * MAX_VISIBLE_BYTES);
    assert!(Arc::ptr_eq(
        &current,
        &layout.query.as_ref().unwrap().fragments
    ));
    assert!(Arc::ptr_eq(
        &current,
        &layout.published.as_ref().unwrap().rows
    ));
}

#[test]
fn old_and_new_shared_viewports_each_keep_the_original_content_cap() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    {
        let mut store = shared.lock().unwrap();
        for row in 0..500 {
            store
                .append_plain(&cwd, 1, &format!("{row:03} {}", "w".repeat(900)), false)
                .unwrap();
        }
    }
    let mut layout = layout(shared.clone(), 1000);
    let previous = shared_view(&mut layout, 0, 500);
    assert!(
        previous.len() > 160,
        "The viewport must retain its original content allowance"
    );
    assert!(
        previous.len() < 500,
        "This fixture reaches the original byte cap"
    );
    assert!(frame_bytes(&previous) <= MAX_VISIBLE_BYTES);
    let anchors = (previous[0].start, previous.last().unwrap().end);
    let next = shared_view(&mut layout, 250, 250);
    assert!(!Arc::ptr_eq(&previous, &next));
    assert_eq!((previous[0].start, previous.last().unwrap().end), anchors);
    assert!(next[0].text.starts_with("250 "));
    assert!(frame_bytes(&next) <= MAX_VISIBLE_BYTES);
    assert!(frame_bytes(&previous) + frame_bytes(&next) <= 2 * MAX_VISIBLE_BYTES);
    drop(next);
    // Model a Pane still painting the previous frame while Layout retains a
    // more recent publication and prepares another viewport plus navigation.
    layout.request_visible(0, 500).unwrap();
    assert!(layout.navigation_row(400).unwrap().is_none());
    drain(&mut layout);
    let buffers = [
        &previous,
        &layout.published.as_ref().unwrap().rows,
        &layout.query.as_ref().unwrap().fragments,
        &layout.navigation.as_ref().unwrap().fragments,
    ];
    for (index, rows) in buffers.iter().enumerate() {
        assert!(frame_bytes(rows) <= MAX_VISIBLE_BYTES);
        for earlier in &buffers[..index] {
            assert!(!Arc::ptr_eq(rows, earlier));
        }
    }
    assert!(buffers.iter().map(|rows| frame_bytes(rows)).sum::<usize>() <= 4 * MAX_VISIBLE_BYTES);
    // The owned projection stays bounded even while an immutable old frame is
    // retained for the previous paint. Clear drops every Layout-owned frame.
    shared.lock().unwrap().clear().unwrap();
    assert!(layout.visible_shared(0, 500).unwrap().is_empty());
    assert!(layout.query.is_none());
    assert!(layout.published.is_none());
    assert_eq!((previous[0].start, previous.last().unwrap().end), anchors);
}

#[test]
fn native_columns_are_row_major_and_padding_has_no_source_anchor() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    let anchors = {
        let mut store = shared.lock().unwrap();
        ["../", "a.rs", "b.rs", "c.rs"]
            .into_iter()
            .map(|text| {
                store
                    .append_native(&cwd, 1, &native(text, Some(7)))
                    .unwrap()
            })
            .collect::<Vec<_>>()
    };
    let mut layout = layout(shared, 12);
    let rows = view(&mut layout, 0, 20);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        ["../    a.rs", "b.rs   c.rs"]
    );
    assert_eq!(rows[0].segments.len(), 2);
    assert_eq!(rows[0].segments[0].range, 0..3);
    assert_eq!(rows[0].segments[1].range, 7..11);
    assert_eq!(rows[0].segments[1].column, 7);
    assert_eq!(rows[0].segments[1].start, anchors[1]);
    assert_eq!(row_at(&mut layout, anchors[0]), Some(0));
    assert_eq!(row_at(&mut layout, anchors[1]), Some(0));
    assert_eq!(row_at(&mut layout, anchors[2]), Some(1));
    assert_eq!(rows[0].anchor_column(5), rows[0].segments[0].end);
    assert!(rows[0].start < rows[0].end && rows[0].end < rows[1].start);
}

#[test]
fn one_per_line_and_recursive_headers_stop_packing() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    {
        let mut store = shared.lock().unwrap();
        store.append_native(&cwd, 1, &native("a", Some(5))).unwrap();
        store
            .append_native(&cwd, 1, &native("child:", None))
            .unwrap();
        store.append_native(&cwd, 1, &native("b", Some(5))).unwrap();
        store.append_native(&cwd, 1, &native("c", Some(5))).unwrap();
        store.append_native(&cwd, 2, &native("one", None)).unwrap();
        store.append_native(&cwd, 2, &native("two", None)).unwrap();
    }
    let mut layout = layout(shared, 20);
    assert_eq!(
        view(&mut layout, 0, 20)
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>(),
        ["a", "child:", "b    c", "one", "two"]
    );
}

#[test]
fn word_wrap_retains_unicode_combining_tabs_long_words_and_blank_lines() {
    let shared = Transcript::shared().unwrap();
    let text = "東京🙂\ta\u{301} word\n\nlongwordwithoutspaces";
    shared
        .lock()
        .unwrap()
        .append_native(&cwd(), 1, &native(text, None))
        .unwrap();
    let mut layout = layout(shared, 8);
    let rows = view(&mut layout, 0, 40);
    assert!(rows.iter().any(|row| row.text.is_empty()));
    assert!(rows.iter().any(|row| row.text.contains("a\u{301}")));
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<String>(),
        text.replace('\t', "  ").replace('\n', "")
    );
    for row in &rows {
        assert!(row.text.trim_end().width() <= 8);
        for segment in &row.segments {
            assert!(row.text.is_char_boundary(segment.range.start));
            assert!(row.text.is_char_boundary(segment.range.end));
        }
    }
}

#[test]
fn font_columns_reflow_missing_wide_glyphs_without_changing_byte_anchors() {
    let shared = Transcript::shared().unwrap();
    let text = "東京a\u{301}b";
    let start = shared
        .lock()
        .unwrap()
        .append_native(&cwd(), 1, &native(text, None))
        .unwrap();
    let mut layout = layout(shared, 4);
    assert_eq!(view(&mut layout, 0, 8).len(), 2);
    assert!(layout.set_font_columns(true));
    drain(&mut layout);
    let rows = view(&mut layout, 0, 8);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].text, text);
    assert_eq!(rows[0].text_columns(text), 4);
    assert_eq!(rows[0].start, start);
    assert_eq!(rows[0].anchor_column(2).utf8_byte_offset, "東京".len());
    let after_accent = Anchor {
        utf8_byte_offset: "東京a\u{301}".len(),
        ..start
    };
    assert_eq!(rows[0].anchor_visual_column(after_accent), Some(3));
    assert_eq!(row_at(&mut layout, after_accent), Some(0));
    assert!(!layout.set_font_columns(true));
    assert!(layout.set_font_columns(false));
    drain(&mut layout);
    assert_eq!(view(&mut layout, 0, 8).len(), 2);
}

#[test]
fn terminal_soft_wrap_seams_keep_byte_anchors_and_command_boundaries() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    let (first, second) = {
        let mut store = shared.lock().unwrap();
        let first = store.append_terminal(&cwd, 1, 20, true, b"hello ").unwrap();
        let second = store
            .append_terminal(&cwd, 1, 20, false, b"world longer")
            .unwrap();
        store.append_terminal(&cwd, 2, 20, true, b"other").unwrap();
        store
            .append_terminal(&cwd, 3, 20, false, b"command")
            .unwrap();
        (first, second)
    };
    let mut layout = layout(shared, 8);
    let rows = view(&mut layout, 0, 20);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        ["hello ", "world ", "longer", "other", "command"]
    );
    assert_eq!(rows[0].start, first);
    assert_eq!(rows[1].segments[0].record_id, second.record_id);
    assert_eq!(row_at(&mut layout, second), Some(1));
    assert!(rows[2].end < rows[3].start);
}

#[test]
fn main_output_uses_legacy_normal_styles_after_ansi_reflow() {
    let shared = Transcript::shared().unwrap();
    shared
        .lock()
        .unwrap()
        .append_terminal(
            &cwd(),
            1,
            80,
            false,
            b"\x1b[1;3;4;7;38;2;10;20;30;48;2;40;50;60mstyled\x1b[0m plain \x1b[2mdim",
        )
        .unwrap();
    let mut layout = layout(shared, 8);
    let rows = view(&mut layout, 0, 20);
    for segment in rows.iter().flat_map(|row| &row.segments) {
        assert_eq!(segment.style, Style::default());
    }
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<String>(),
        "styled plain dim"
    );
}

#[test]
fn token_across_saved_physical_rows_is_one_reflowable_line() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    let (first, second) = {
        let mut store = shared.lock().unwrap();
        (
            store.append_terminal(&cwd, 1, 4, true, b"long").unwrap(),
            store.append_terminal(&cwd, 1, 4, false, b"word").unwrap(),
        )
    };
    let mut layout = layout(shared, 6);
    let rows = view(&mut layout, 0, 10);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        ["longwo", "rd"]
    );
    assert_eq!(rows[0].start, first);
    assert!(
        rows[0]
            .segments
            .iter()
            .any(|segment| segment.record_id == second.record_id)
    );
    assert_eq!(row_at(&mut layout, second), Some(0));
    assert_eq!(
        row_at(
            &mut layout,
            Anchor {
                utf8_byte_offset: 2,
                ..second
            }
        ),
        Some(1)
    );
}

#[test]
fn saved_live_seam_and_incremental_sealing_keep_future_ids_stable() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    shared
        .lock()
        .unwrap()
        .append_terminal(&cwd, 7, 4, true, b"long")
        .unwrap();
    let mut layout = layout(shared.clone(), 6);
    let mut live = vt100::Parser::new(2, 4, 0);
    live.process(b"word\r\nnext");
    layout.set_tail(live.screen(), 0, 2, 7).unwrap();
    drain(&mut layout);
    let rows = view(&mut layout, 0, 20);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        ["longwo", "rd", "next"]
    );
    let remaining = rows[2].start;
    assert_eq!(remaining.record_id, 2);
    assert_eq!(row_at(&mut layout, remaining), Some(2));
    shared
        .lock()
        .unwrap()
        .append_terminal(
            &cwd,
            7,
            4,
            false,
            &live.screen().row_scrollback_formatted(0),
        )
        .unwrap();
    layout.set_tail(live.screen(), 1, 1, 7).unwrap();
    drain(&mut layout);
    let sealed = view(&mut layout, 0, 20);
    assert_eq!(sealed[2].start, remaining);
    assert_eq!(
        sealed
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>(),
        ["longwo", "rd", "next"]
    );
    assert_eq!(shared.lock().unwrap().len(), 2);
    shared
        .lock()
        .unwrap()
        .append_terminal(
            &cwd,
            7,
            4,
            false,
            &live.screen().row_scrollback_formatted(1),
        )
        .unwrap();
    layout.set_tail(live.screen(), 0, 0, 7).unwrap();
    drain(&mut layout);
    assert_eq!(view(&mut layout, 0, 20)[2].start, remaining);
    assert!(layout.tail.is_none());
}

#[test]
fn live_grid_bounds_and_primary_snapshot_do_not_duplicate_disk_history() {
    let shared = Transcript::shared().unwrap();
    let mut layout = layout(shared.clone(), 20);
    let oversized = vt100::Parser::new(161, 1, 0);
    assert!(layout.set_tail(oversized.screen(), 0, 1, 1).is_err());
    let valid = vt100::Parser::new(160, 256, 0);
    layout.set_tail(valid.screen(), 0, 160, 1).unwrap();
    assert_eq!(layout.tail.as_ref().unwrap().count, 160);
    layout.set_tail(valid.screen(), 0, 0, 1).unwrap();
    let mut live = vt100::Parser::new(2, 20, 0);
    live.process(b"primary\x1b[?1049halt");
    layout.set_tail(live.screen(), 0, 1, 1).unwrap();
    drain(&mut layout);
    assert_eq!(view(&mut layout, 0, 20)[0].text, "primary");
    assert_eq!(shared.lock().unwrap().len(), 0);
}

#[test]
fn continuous_append_color_and_follow_targets_publish_bounded_frames() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    let first = shared
        .lock()
        .unwrap()
        .append_native(&cwd, 1, &native("old", None))
        .unwrap();
    for index in 0..300 {
        shared
            .lock()
            .unwrap()
            .append_header(&cwd, 2, &format!("row{index}"))
            .unwrap();
    }
    let mut layout = layout(shared.clone(), 80);
    assert!(!view(&mut layout, 0, 40).is_empty());
    let mut publications = 0;
    for index in 0..150 {
        let mut color = native("old", None);
        color[11] = index as u8;
        {
            let mut store = shared.lock().unwrap();
            store.replace_native(first.record_id, &color).unwrap();
            store
                .append_header(&cwd, 2, &format!("new{index}"))
                .unwrap();
        }
        layout.poll().unwrap();
        let start = layout.len().saturating_sub(40);
        if !layout.visible(start, 40).unwrap().is_empty() {
            publications += 1;
        }
    }
    assert!(publications > 100);
    drain(&mut layout);
    let start = layout.len().saturating_sub(40);
    let latest = view(&mut layout, start, 40);
    assert!(latest.iter().any(|row| row.text == "$ new149"));
    assert!(layout.query.as_ref().unwrap().bytes <= MAX_VISIBLE_BYTES);
}

#[test]
fn repeated_live_updates_preserve_a_ready_scrolled_frame_during_rewind() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    for index in 0..100 {
        shared
            .lock()
            .unwrap()
            .append_header(&cwd, 1, &format!("past{index}"))
            .unwrap();
    }
    let mut layout = layout(shared, 20);
    let mut live = vt100::Parser::new(20, 20, 0);
    live.process(b"before");
    layout.set_tail(live.screen(), 0, 20, 2).unwrap();
    drain(&mut layout);
    assert!(!view(&mut layout, 100, 20).is_empty());
    for index in 0..30 {
        live.process(format!("\x1b[Hchanged{index}").as_bytes());
        layout.set_tail(live.screen(), 0, 20, 2).unwrap();
        assert!(!layout.visible(100, 20).unwrap().is_empty());
        layout.poll().unwrap();
    }
    drain(&mut layout);
    assert!(view(&mut layout, 100, 20)[0].text.starts_with("changed29"));
    assert_eq!(layout.cached_start(), Some(100));
    assert!(layout.published_tail().is_some());
}

#[test]
fn continuously_mutating_initial_tail_finishes_captured_index_passes() {
    let shared = Transcript::shared().unwrap();
    let mut layout = layout(shared, 20);
    let mut live = vt100::Parser::new(100, 20, 0);
    for index in 0..100 {
        live.process(format!("\x1b[Hupdate{index}").as_bytes());
        layout.set_tail(live.screen(), 0, 100, 3).unwrap();
        layout.poll().unwrap();
    }
    assert_eq!(layout.len(), 100);
    drain(&mut layout);
    let rows = view(&mut layout, 99, 1);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].start.record_id, 99);
}

#[test]
fn huge_tab_runs_and_dense_style_changes_stay_inside_projection_budget() {
    let shared = Transcript::shared().unwrap();
    shared
        .lock()
        .unwrap()
        .append_header(&cwd(), 1, &"\t".repeat(64 * 1024))
        .unwrap();
    let mut layout = layout(shared, 1);
    let rows = view(&mut layout, 0, usize::MAX);
    assert!(!rows.is_empty());
    assert!(rows.iter().map(Fragment::bytes).sum::<usize>() <= MAX_VISIBLE_BYTES);
    assert!(
        rows.iter()
            .all(|row| row.segments.len() <= MAX_FRAGMENT_SEGMENTS)
    );
    assert!(rows.iter().all(|row| row.text.len() <= MAX_FRAGMENT_BYTES));
}

#[test]
fn resize_append_and_color_only_revision_preserve_logical_positions() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    let anchor = shared
        .lock()
        .unwrap()
        .append_native(&cwd, 1, &native("one two three", None))
        .unwrap();
    let mut layout = layout(shared.clone(), 8);
    let before = view(&mut layout, 0, 20);
    assert_eq!(before.len(), 2);
    let target = Anchor {
        utf8_byte_offset: 8,
        ..anchor
    };
    assert_eq!(row_at(&mut layout, target), Some(1));
    layout.configure(Metrics {
        columns: 20,
        tab_width: 4,
    });
    drain(&mut layout);
    assert_eq!(row_at(&mut layout, target), Some(0));
    let checkpoints = layout
        .checkpoints
        .iter()
        .map(|point| (point.row, point.cursor))
        .collect::<Vec<_>>();
    let mut changed = native("one two three", None);
    changed[11..14].copy_from_slice(&[1, 2, 3]);
    shared
        .lock()
        .unwrap()
        .replace_native(anchor.record_id, &changed)
        .unwrap();
    layout.poll().unwrap();
    assert_eq!(
        layout
            .checkpoints
            .iter()
            .map(|point| (point.row, point.cursor))
            .collect::<Vec<_>>(),
        checkpoints
    );
    assert_eq!(
        view(&mut layout, 0, 20)[0].segments[0].color,
        Some((1, 2, 3))
    );
    shared
        .lock()
        .unwrap()
        .append_header(&cwd, 2, "new output")
        .unwrap();
    layout.poll().unwrap();
    drain(&mut layout);
    assert_eq!(row_at(&mut layout, target), Some(0));
    assert_eq!(layout.len(), 2);
}

#[test]
fn sparse_checkpoints_and_visible_projection_are_fixed_bounded_caches() {
    let shared = Transcript::shared().unwrap();
    shared
        .lock()
        .unwrap()
        .append_header(&cwd(), 1, &"x".repeat(64 * 1024))
        .unwrap();
    let mut layout = layout(shared, 1);
    // The command header contributes its "$ " prefix as one visual row.
    assert_eq!(layout.len(), 64 * 1024 + 1);
    assert!(layout.checkpoints.len() <= CHECKPOINTS);
    assert!(layout.checkpoints.capacity() * mem::size_of::<Checkpoint>() <= 128 * 1024);
    assert!(layout.stride > 16);
    let rows = view(&mut layout, 60_000, usize::MAX);
    assert!(rows.len() > 160 && rows.len() <= MAX_VISIBLE_BYTES / mem::size_of::<Fragment>());
    assert!(rows.iter().map(Fragment::bytes).sum::<usize>() <= MAX_VISIBLE_BYTES);
    assert_eq!(rows[0].start.utf8_byte_offset, 60_001);
}

#[test]
fn eviction_and_clear_reject_old_anchors_and_do_not_grow_the_index() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    let first = shared
        .lock()
        .unwrap()
        .append_header(&cwd, 1, "old")
        .unwrap();
    let mut layout = layout(shared.clone(), 80);
    {
        let mut store = shared.lock().unwrap();
        for _ in 0..150 {
            store
                .append_header(&cwd, 2, &"x".repeat(64 * 1024))
                .unwrap();
        }
    }
    layout.poll().unwrap();
    drain(&mut layout);
    assert_eq!(row_at(&mut layout, first), None);
    assert!(layout.checkpoints.len() <= CHECKPOINTS);
    shared.lock().unwrap().clear().unwrap();
    layout.poll().unwrap();
    assert_eq!(layout.len(), 0);
    assert!(layout.complete());
    assert!(view(&mut layout, 0, 20).is_empty());
    assert_eq!(row_at(&mut layout, first), None);
}

#[test]
fn repeated_cap_eviction_reuses_hard_line_suffix_and_rebases_navigation() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    let text = "word ".repeat(36);
    {
        let mut store = shared.lock().unwrap();
        for _ in 0..40000 {
            store.append_plain(&cwd, 1, &text, false).unwrap();
        }
    }
    let mut layout = layout(shared.clone(), 79);
    let old_base = layout.base;
    let retained_anchor = Anchor {
        epoch: layout.epoch,
        record_id: layout.end - 200,
        utf8_byte_offset: 5,
    };
    let retained_row = row_at(&mut layout, retained_anchor).unwrap();
    view(&mut layout, retained_row, 4);
    let mut changed_base = 0;
    for _ in 0..8 {
        let previous_end = layout.end;
        let previous_checkpoints = layout.checkpoints.len();
        {
            let mut store = shared.lock().unwrap();
            for _ in 0..256 {
                store.append_plain(&cwd, 1, &text, false).unwrap();
            }
        }
        layout.poll().unwrap();
        if layout.base != old_base {
            changed_base += 1;
            assert!(layout.origin != 0 || layout.rebase.is_some());
            assert!(
                layout.checkpoints.len() > previous_checkpoints / 2,
                "Eviction discarded the retained sparse index"
            );
            assert!(
                layout.flow.cursor.id >= previous_end - 1,
                "Eviction restarted the main pass at the retained base"
            );
        }
        drain(&mut layout);
    }
    assert!(changed_base > 0);
    let mut fresh = super::tests::layout(shared.clone(), 79);
    assert_eq!(layout.len(), fresh.len());
    let rows = layout.len();
    assert_eq!(view(&mut layout, 0, 8), view(&mut fresh, 0, 8));
    assert_eq!(
        view(&mut layout, rows - 8, 8),
        view(&mut fresh, rows - 8, 8)
    );
    assert_eq!(
        row_at(&mut layout, retained_anchor),
        row_at(&mut fresh, retained_anchor)
    );
    let edge = layout.row_edge(retained_anchor, true).unwrap();
    drain(&mut layout);
    assert_eq!(
        edge.or_else(|| layout.row_edge(retained_anchor, true).unwrap()),
        fresh.row_edge(retained_anchor, true).unwrap().or_else(|| {
            drain(&mut fresh);
            fresh.row_edge(retained_anchor, true).unwrap()
        })
    );
}

#[test]
fn eviction_of_continued_lines_reflows_from_a_true_hard_boundary() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    {
        let mut store = shared.lock().unwrap();
        // Periodic hard lines with awkward lengths make an evicted soft-line
        // prefix wrap differently; checkpoints inside that prefix are unsafe.
        for index in 0..35000 {
            let text = if index % 11 == 0 {
                "boundary".repeat(16)
            } else {
                "abcdef ".repeat(20)
            };
            store
                .append_plain(&cwd, 1, &text, index % 11 != 10)
                .unwrap();
        }
    }
    let mut existing = layout(shared.clone(), 43);
    {
        let mut store = shared.lock().unwrap();
        for index in 0..1500 {
            store
                .append_plain(&cwd, 1, &"x y z ".repeat(24), index % 7 != 6)
                .unwrap();
        }
    }
    existing.poll().unwrap();
    drain(&mut existing);
    let mut fresh = layout(shared, 43);
    assert_eq!(existing.len(), fresh.len());
    assert_eq!(view(&mut existing, 0, 12), view(&mut fresh, 0, 12));
    let start = existing.len() - 12;
    assert_eq!(view(&mut existing, start, 12), view(&mut fresh, start, 12));
}

#[test]
fn a_head_scroll_waits_for_eviction_origin_without_hiding_retained_rows() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    let text = "word ".repeat(36);
    {
        let mut store = shared.lock().unwrap();
        for _ in 0..40000 {
            store.append_plain(&cwd, 1, &text, false).unwrap();
        }
    }
    let mut projected = layout(shared.clone(), 79);
    let bottom = projected.len() - 4;
    let retained = view(&mut projected, bottom, 4);
    let previous_base = projected.base;
    while shared.lock().unwrap().base_id() == previous_base {
        shared
            .lock()
            .unwrap()
            .append_plain(&cwd, 1, &text, false)
            .unwrap();
    }
    // visible() observes eviction before poll() can establish its row origin.
    assert_eq!(projected.visible(0, 4).unwrap(), retained);
    assert!(projected.rebase.is_some());
    assert!(
        projected
            .query
            .as_ref()
            .is_some_and(|query| query.start == bottom)
    );
    drain(&mut projected);
    let mut fresh = layout(shared, 79);
    assert_eq!(view(&mut projected, 0, 4), view(&mut fresh, 0, 4));
    assert_eq!(projected.cached_start(), Some(0));
}

#[test]
fn vertical_navigation_is_independent_of_viewport_requests() {
    let shared = Transcript::shared().unwrap();
    let anchor = shared
        .lock()
        .unwrap()
        .append_native(&cwd(), 1, &native("abcd efgh ijkl", None))
        .unwrap();
    let mut layout = layout(shared, 5);
    assert_eq!(row_at(&mut layout, anchor), Some(0));
    assert_eq!(layout.neighbour(anchor, 1, 2).unwrap(), None);
    layout.visible(0, 20).unwrap();
    drain(&mut layout);
    let down = layout.neighbour(anchor, 1, 2).unwrap().unwrap();
    assert_eq!(down.utf8_byte_offset, 7);
    assert_eq!(row_at(&mut layout, down), Some(1));
    if layout.row_edge(down, true).unwrap().is_none() {
        drain(&mut layout);
    }
    assert_eq!(
        layout
            .row_edge(down, true)
            .unwrap()
            .unwrap()
            .utf8_byte_offset,
        10
    );
}

#[test]
fn plain_tail_preserves_literal_text_and_seals_with_stable_anchors() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    {
        let mut store = shared.lock().unwrap();
        store.append_header(&cwd, 41, "printf").unwrap();
        store.append_plain(&cwd, 41, "\ttrail  ", false).unwrap();
    }
    let mut layout = layout(shared.clone(), 80);
    layout.set_plain_tail("long á東京  ", 41).unwrap();
    drain(&mut layout);
    let rows = view(&mut layout, 0, 20);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        ["$ printf", "    trail  ", "long á東京  "]
    );
    let anchor = rows[2].start;
    let (published, source, first) = layout.published_plain_tail().unwrap();
    assert_eq!(
        (&*published, source, first),
        ("long á東京  ", 41, anchor.record_id)
    );
    assert!(layout.published_tail().is_none());
    assert_eq!(shared.lock().unwrap().len(), 2);
    shared
        .lock()
        .unwrap()
        .append_plain(&cwd, 41, &published, false)
        .unwrap();
    layout.set_plain_tail("", 41).unwrap();
    drain(&mut layout);
    assert_eq!(view(&mut layout, 0, 20)[2].start, anchor);
    assert_eq!(row_at(&mut layout, anchor), Some(2));
    assert!(layout.tail.is_none());
}

#[test]
fn plain_tail_bounds_are_atomic_and_published_text_is_pinned() {
    let shared = Transcript::shared().unwrap();
    shared
        .lock()
        .unwrap()
        .append_header(&cwd(), 42, "build")
        .unwrap();
    let mut layout = layout(shared.clone(), 80);
    layout
        .set_plain_tail("error: source.rs:2:3 failure  ", 42)
        .unwrap();
    drain(&mut layout);
    let rows = view(&mut layout, 0, 20);
    assert!(
        rows[1]
            .segments
            .iter()
            .all(|segment| segment.style.fg == vt100::Color::Rgb(245, 145, 145))
    );
    assert!(
        rows[1]
            .segments
            .iter()
            .any(|segment| segment.style.underline)
    );
    let (snapshot, _, _) = layout.published_plain_tail().unwrap();
    assert!(
        layout
            .set_plain_tail(&"x".repeat(MAX_PLAIN_TAIL_BYTES + 1), 42)
            .is_err()
    );
    assert!(layout.set_plain_tail("two\nlines", 42).is_err());
    assert_eq!(&*layout.published_plain_tail().unwrap().0, &*snapshot);
    layout.set_plain_tail("new", 42).unwrap();
    drain(&mut layout);
    view(&mut layout, 0, 20);
    assert_eq!(&*snapshot, "error: source.rs:2:3 failure  ");
    assert_eq!(&*layout.published_plain_tail().unwrap().0, "new");
    assert_eq!(shared.lock().unwrap().len(), 1);
    layout
        .set_plain_tail(&"x".repeat(MAX_PLAIN_TAIL_BYTES), 42)
        .unwrap();
    assert!(
        layout
            .pending_tail
            .as_ref()
            .and_then(Option::as_ref)
            .map_or_else(
                || layout.tail.as_ref().unwrap().plain.as_ref().unwrap().len(),
                |tail| tail.plain.as_ref().unwrap().len()
            )
            == MAX_PLAIN_TAIL_BYTES
    );
    layout.set_plain_tail("", 42).unwrap();
    drain(&mut layout);
}

#[test]
fn changing_plain_tail_finishes_captured_index_and_viewport_work() {
    let shared = Transcript::shared().unwrap();
    let mut layout = layout(shared.clone(), 1);
    let text = "x".repeat(100);
    layout.set_plain_tail(&text, 43).unwrap();
    layout.visible(90, 10).unwrap();
    let mut published = false;
    for tick in 0..100 {
        layout
            .set_plain_tail(&format!("{}{}", tick % 10, &text[1..]), 43)
            .unwrap();
        layout.poll().unwrap();
        published |= !layout.visible(90, 10).unwrap().is_empty();
    }
    assert!(layout.len() >= 100, "Live index must advance under output");
    assert!(
        published,
        "A captured visible query must publish under output"
    );
    drain(&mut layout);
    assert_eq!(shared.lock().unwrap().len(), 0);
}

#[test]
fn font_tab_stops_count_combining_scalars_separately_from_glyph_pixels() {
    let shared = Transcript::shared().unwrap();
    shared
        .lock()
        .unwrap()
        .append_plain(&cwd(), 44, "a\u{301}\tb  ", false)
        .unwrap();
    let mut layout = layout(shared, 80);
    layout.set_font_columns(true);
    drain(&mut layout);
    let rows = view(&mut layout, 0, 20);
    assert_eq!(rows[0].text, "a\u{301}  b  ");
    // Legacy tab stops count both a and its accent, while the accent glyph
    // occupies zero pixels. The following b is therefore at visual column3.
    assert_eq!(
        rows[0].anchor_visual_column(Anchor {
            utf8_byte_offset: "a\u{301}\t".len(),
            ..rows[0].start
        }),
        Some(3)
    );
    assert_eq!(rows[0].anchor_column(2).utf8_byte_offset, "a\u{301}".len());
}

#[test]
fn native_listing_blank_and_eof_are_distinct_rows_and_survive_later_output() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    let eof = {
        let mut store = shared.lock().unwrap();
        store.append_header(&cwd, 1, "ls").unwrap();
        store
            .append_native(&cwd, 1, &native("file.txt", None))
            .unwrap();
        store.append_native(&cwd, 1, &native("", None)).unwrap();
        store.append_result(&cwd, 1, 0, "Done").unwrap()
    };
    let mut layout = layout(shared.clone(), 80);
    let rows = view(&mut layout, 0, 20);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        ["$ ls", "file.txt", "", ""]
    );
    assert_eq!(row_at(&mut layout, eof), Some(3));
    {
        let mut store = shared.lock().unwrap();
        store.append_header(&cwd, 2, "pwd").unwrap();
        store
            .append_native(&cwd, 2, &native("directory", None))
            .unwrap();
        store.append_result(&cwd, 2, 0, "Done").unwrap();
    }
    layout.poll().unwrap();
    drain(&mut layout);
    let rows = view(&mut layout, 0, 20);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        ["$ ls", "file.txt", "", "$ pwd", "directory", ""]
    );
}

#[test]
fn original_wide_and_tall_normal_geometry_keeps_existing_byte_budget() {
    let shared = Transcript::shared().unwrap();
    let cwd = cwd();
    {
        let mut store = shared.lock().unwrap();
        store
            .append_plain(&cwd, 45, &"w".repeat(900), false)
            .unwrap();
        for _ in 0..300 {
            store.append_plain(&cwd, 45, "", false).unwrap();
        }
    }
    let mut layout = layout(shared, 1000);
    let rows = view(&mut layout, 0, 400);
    assert_eq!(rows.len(), 301);
    assert_eq!(rows[0].text.len(), 900);
    assert_eq!(rows[0].end.utf8_byte_offset, 900);
    assert_eq!(rows[0].anchor_column(899).utf8_byte_offset, 899);
    assert!(rows.iter().map(Fragment::bytes).sum::<usize>() <= MAX_VISIBLE_BYTES);
    let query = layout.query.as_ref().unwrap();
    assert!(
        query.bytes
            + (query.fragments.capacity() - query.fragments.len()) * mem::size_of::<Fragment>()
            <= MAX_VISIBLE_BYTES
    );
    layout.configure(Metrics {
        columns: u16::MAX,
        tab_width: 4,
    });
    assert_eq!(layout.metrics.columns, 1000);
}

#[test]
fn measured_wrap_matches_scalar_pixels_and_unmeasured_hidden_output_suspends() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let shared = Transcript::shared().unwrap();
    shared
        .lock()
        .unwrap()
        .append_plain(
            &cwd(),
            46,
            "folder with spaces/  e\u{301}_combining.txt     README.md",
            false,
        )
        .unwrap();
    let wakes = Arc::new(AtomicUsize::new(0));
    let notify = wakes.clone();
    let mut layout = Layout::new(
        shared,
        Arc::new(move || {
            notify.fetch_add(1, Ordering::Relaxed);
        }),
    )
    .unwrap();
    layout.configure(Metrics {
        columns: 36,
        tab_width: 4,
    });
    layout.set_font_columns(true);
    layout.set_wrap_pixels(396, 11, 18f32.to_bits());
    layout.poll().unwrap();
    assert!(layout.scalar_widths.needed);
    assert!(
        !layout.pending(),
        "No polling loop while a hidden pane needs its font"
    );
    let before = wakes.load(Ordering::Relaxed);
    for _ in 0..30 {
        assert!(!layout.poll().unwrap());
    }
    assert_eq!(wakes.load(Ordering::Relaxed), before);
    let mut measures = 0;
    while !layout.complete() || layout.pending() {
        layout
            .poll_with_measure(|character| {
                assert_eq!(character, '\u{301}');
                measures += 1;
                9
            })
            .unwrap();
    }
    let rows = view(&mut layout, 0, 20);
    assert_eq!(
        rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
        [
            "folder with spaces/  ",
            "e\u{301}_combining.txt     README.md"
        ]
    );
    assert_eq!(measures, 1);
    assert_eq!(rows[1].text_columns("e\u{301}"), 1);
    assert!(layout.scalar_widths.cached.len() <= 512);
    assert!(layout.set_wrap_pixels(396, 11, 19f32.to_bits()));
    assert!(layout.scalar_widths.cached.is_empty());
}
