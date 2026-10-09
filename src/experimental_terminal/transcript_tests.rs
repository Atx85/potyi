// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn cwd() -> PathBuf {
    std::env::temp_dir().canonicalize().unwrap()
}

#[test]
fn typed_records_keep_order_sources_directories_and_terminal_metadata() {
    let mut transcript = Transcript::new().unwrap();
    let cwd = cwd();
    let native_source = transcript.next_source();
    let shell_source = transcript.next_source();
    assert_ne!(native_source, shell_source);
    let header = transcript
        .append_header(&cwd, native_source, "$ ls")
        .unwrap();
    let native = transcript
        .append_native(&cwd, native_source, b"native row")
        .unwrap();
    let terminal = transcript
        .append_terminal(&cwd, shell_source, 87, true, b"\x1b[31merror\x1b[0m")
        .unwrap();
    let result = transcript
        .append_result(&cwd, shell_source, 2, "Exit 2")
        .unwrap();
    assert_eq!(transcript.len(), 4);
    assert_eq!(transcript.serial(), 4);
    for (index, anchor) in [header, native, terminal, result].into_iter().enumerate() {
        let record = transcript.read(index).unwrap();
        assert_eq!(record.id, anchor.record_id);
        assert_eq!(record.epoch, anchor.epoch);
        assert_eq!(record.cwd, cwd);
        assert_eq!(transcript.read_anchor(anchor).unwrap(), record);
    }
    assert_eq!(
        transcript.read(1).unwrap().data,
        RecordData::Native(b"native row".to_vec())
    );
    assert_eq!(
        transcript.read(2).unwrap().data,
        RecordData::Terminal {
            cols: 87,
            wrapped: true,
            formatted: b"\x1b[31merror\x1b[0m".to_vec(),
        }
    );
    assert_eq!(
        transcript.read(3).unwrap().data,
        RecordData::Result {
            relative_safe: true,
            status: 2,
            text: "Exit 2".into()
        }
    );
}

#[test]
fn clear_invalidates_empty_views_and_never_reuses_record_ids() {
    let mut transcript = Transcript::new().unwrap();
    let cwd = cwd();
    let first = transcript.append_header(&cwd, 1, "one").unwrap();
    transcript.clear().unwrap();
    assert!(transcript.read_anchor(first).is_err());
    assert!(transcript.read_id(first.record_id).is_err());
    let epoch = transcript.epoch();
    let revision = transcript.serial();
    transcript.clear().unwrap();
    assert!(transcript.epoch() > epoch);
    assert!(transcript.serial() > revision);
    let next = transcript.append_header(&cwd, 2, "two").unwrap();
    assert!(next.record_id > first.record_id);
    assert_ne!(next.epoch, first.epoch);
    assert_eq!(transcript.base_id(), next.record_id);
}

#[test]
fn complete_record_size_is_validated_before_eviction_or_id_allocation() {
    let mut transcript = Transcript::new().unwrap();
    let cwd = cwd();
    let first = transcript.append_header(&cwd, 1, "kept").unwrap();
    let revision = transcript.serial();
    let size = MAX_PAYLOAD_BYTES;
    assert!(
        transcript
            .append_native(&cwd, 1, &vec![0; size + 1])
            .is_err()
    );
    assert!(
        transcript
            .append_header(Path::new("relative"), 1, "wrong cwd")
            .is_err()
    );
    assert!(
        transcript
            .append_terminal(&cwd, 1, 0, false, b"wrong width")
            .is_err()
    );
    let oversized_metadata = cwd.join("x".repeat(MAX_RECORD_BYTES));
    assert!(
        transcript
            .append_native(&oversized_metadata, 1, b"small")
            .is_err()
    );
    assert_eq!(transcript.serial(), revision);
    assert_eq!(transcript.len(), 1);
    assert_eq!(
        transcript.read_anchor(first).unwrap().data,
        RecordData::Header("kept".into())
    );
    let last = transcript.append_native(&cwd, 1, &vec![0; size]).unwrap();
    assert_eq!(last.record_id, first.record_id + 1);
    assert_eq!(
        transcript.read_anchor(last).unwrap().data,
        RecordData::Native(vec![0; size])
    );
}

#[test]
fn eviction_keeps_absolute_ids_and_memory_buffers_bounded() {
    let mut transcript = Transcript::new().unwrap();
    let cwd = cwd();
    let first = transcript.append_header(&cwd, 1, "evicted").unwrap();
    let mut last = first;
    for index in 0..80_000 {
        last = transcript
            .append_header(&cwd, 1, &format!("{index:06} {}", "x".repeat(160)))
            .unwrap();
    }
    assert!(transcript.len() < 80_001);
    assert!(transcript.len() <= 65_536);
    assert_eq!(transcript.buffer_capacity(), 120 * 1024);
    assert!(transcript.base_id() > first.record_id);
    assert!(transcript.read_anchor(first).is_err());
    assert_eq!(
        transcript.read_id(last.record_id).unwrap().id,
        last.record_id
    );
    assert_eq!(transcript.read(0).unwrap().id, transcript.base_id());
}

#[test]
fn batch_reads_bound_rows_and_metadata_plus_payload_bytes() {
    let mut transcript = Transcript::new().unwrap();
    let cwd = cwd();
    for _ in 0..600 {
        transcript.append_header(&cwd, 1, "small").unwrap();
    }
    assert_eq!(
        transcript.read_rows(0, usize::MAX).unwrap().len(),
        MAX_READ_ROWS
    );
    assert!(
        transcript
            .read_rows(usize::MAX, usize::MAX)
            .unwrap()
            .is_empty()
    );
    transcript.clear().unwrap();
    let size = MAX_PAYLOAD_BYTES;
    for _ in 0..8 {
        transcript.append_native(&cwd, 1, &vec![0; size]).unwrap();
    }
    let batch = transcript.read_rows(0, usize::MAX).unwrap();
    assert_eq!(batch.len(), 3);
    assert!(batch.iter().map(record_size).sum::<usize>() <= MAX_READ_BYTES);
}

#[test]
fn shared_owners_append_to_one_ordered_store() {
    let shared = Transcript::shared().unwrap();
    let other = shared.clone();
    let cwd = cwd();
    shared
        .lock()
        .unwrap()
        .append_header(&cwd, 1, "native")
        .unwrap();
    other
        .lock()
        .unwrap()
        .append_terminal(&cwd, 2, 80, false, b"shell")
        .unwrap();
    assert_eq!(shared.lock().unwrap().len(), 2);
    assert!(Arc::ptr_eq(&shared, &other));
}

#[test]
fn native_replacement_changes_revision_without_moving_anchors() {
    let mut transcript = Transcript::new().unwrap();
    let cwd = cwd();
    let anchor = transcript.append_native(&cwd, 7, b"old").unwrap();
    let revision = transcript.serial();
    transcript.replace_native(anchor.record_id, b"new").unwrap();
    let record = transcript.read_anchor(anchor).unwrap();
    assert_eq!(record.data, RecordData::Native(b"new".to_vec()));
    assert_eq!(record.source, 7);
    assert_eq!(transcript.serial(), revision + 1);
    assert!(
        transcript
            .replace_native(anchor.record_id, b"too long")
            .is_err()
    );
    assert_eq!(transcript.serial(), revision + 1);
    let next = transcript.append_header(&cwd, 8, "next").unwrap();
    assert_eq!(next.record_id, anchor.record_id + 1);
    assert!(transcript.replace_native(next.record_id, b"next").is_err());
    assert_eq!(transcript.read_anchor(anchor).unwrap(), record);
}

#[test]
fn full_bounded_command_header_is_preserved_with_directory_metadata() {
    let mut transcript = Transcript::new().unwrap();
    let cwd = cwd();
    let header = format!("$ {}", "x".repeat(MAX_PAYLOAD_BYTES));
    let anchor = transcript.append_header(&cwd, 1, &header).unwrap();
    assert_eq!(
        transcript.read_anchor(anchor).unwrap().data,
        RecordData::Header(header)
    );
    assert!(
        transcript
            .append_header(&cwd, 1, &"x".repeat(MAX_PAYLOAD_BYTES + 3))
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn native_directory_bytes_survive_round_trip() {
    use std::os::unix::ffi::OsStringExt;
    let cwd = PathBuf::from(std::ffi::OsString::from_vec(b"/tmp/invalid-\xff".to_vec()));
    let mut transcript = Transcript::new().unwrap();
    let anchor = transcript.append_header(&cwd, 1, "one").unwrap();
    assert_eq!(transcript.read_anchor(anchor).unwrap().cwd, cwd);
}

#[test]
fn raw_plain_rows_are_utf8_exact_and_zero_width_requires_the_durable_flag() {
    let mut t = Transcript::new().unwrap();
    let cwd = cwd();
    let source = t.next_source();
    t.append_header(&cwd, source, "printf text").unwrap();
    let text = "\t東京 trail  ";
    let anchor = t.append_plain(&cwd, source, text, true).unwrap();
    let record = t.read_anchor(anchor).unwrap();
    assert_eq!(
        super::super::output_selection::record_text(&record)
            .unwrap()
            .text,
        text
    );
    assert!(matches!(
        record.data,
        RecordData::Terminal {
            cols: 0,
            wrapped: true,
            ..
        }
    ));
    let encoded = t.history.row(1).unwrap().bytes;
    assert_eq!(encoded[2], 3, "raw and continuation bits are durable");
    let mut invalid = encoded.clone();
    invalid[2] = 1;
    assert!(decode(t.epoch(), anchor.record_id, &invalid).is_err());
    invalid = encoded.clone();
    invalid[4..6].copy_from_slice(&80u16.to_le_bytes());
    assert!(decode(t.epoch(), anchor.record_id, &invalid).is_err());
    invalid = encoded.clone();
    *invalid.last_mut().unwrap() = 0xff;
    assert!(decode(t.epoch(), anchor.record_id, &invalid).is_err());
    assert!(
        t.append_terminal(&cwd, source, 0, false, b"still invalid")
            .is_err()
    );
    let before = (t.len(), t.serial());
    assert!(
        t.append_plain(&cwd, source, &"x".repeat(64 * 1024 + 1), false)
            .is_err()
    );
    assert_eq!((t.len(), t.serial()), before);
}

#[test]
fn raw_diagnostic_style_replacement_preserves_raw_flags_and_original_text() {
    let mut t = Transcript::new().unwrap();
    let cwd = cwd();
    let source = t.next_source();
    t.append_header(&cwd, source, "cargo check").unwrap();
    let first = t.append_plain(&cwd, source, "error", true).unwrap();
    t.append_plain(&cwd, source, "[E0308]: mismatch  ", false)
        .unwrap();
    let record = t.read_anchor(first).unwrap();
    let RecordData::Terminal {
        cols,
        formatted,
        wrapped,
    } = &record.data
    else {
        panic!()
    };
    assert_eq!(*cols, 0);
    assert!(*wrapped);
    assert_eq!(
        super::super::output_colors::stored_style(formatted).0,
        Some(Tone::Removed)
    );
    assert_eq!(
        super::super::output_selection::record_text(&record)
            .unwrap()
            .text,
        "error"
    );
    assert_eq!(t.history.row(1).unwrap().bytes[2], 3);
}
