// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::{
    io::{Seek, SeekFrom},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
pub(super) struct Temp(pub(super) PathBuf);
impl Temp {
    pub(super) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "potyi recovery test {} {} {}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn table(&self, text: &str) -> PieceTable {
        let source = self.0.join("source.txt");
        fs::write(&source, text).unwrap();
        let mut table = PieceTable::open(source.to_str().unwrap()).unwrap();
        table.enable_recovery_at(self.0.join("recovery"), Some(&source));
        table
    }
    fn restore(&self) -> (String, bool) {
        let entries = list(&self.0.join("recovery")).unwrap();
        assert_eq!(entries.len(), 1);
        let (file, partial) = restore(&self.0.join("recovery"), &entries[0].id).unwrap();
        (fs::read_to_string(file).unwrap(), partial)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn recovery_replays_unicode_deletions_piece_undo_and_batches() {
    let temp = Temp::new();
    let mut table = temp.table("hello 🦀\r\nworld");
    table.insert(6, "é!").unwrap();
    let captured = table.capture_range(0, 5).unwrap();
    table.delete(0, 5).unwrap();
    table.insert_pieces(0, &captured).unwrap();
    let mut snapshot = table
        .replace_ranges([(0, 5), (14, 19)].into_iter(), "hi")
        .unwrap()
        .unwrap();
    table.swap_snapshot(&mut snapshot);
    table.swap_snapshot(&mut snapshot);
    let expected = table.text().unwrap();
    assert!(table.take_recovery_warning().is_none());
    assert!(
        list(&temp.0.join("recovery")).unwrap().is_empty(),
        "live session must be locked"
    );
    drop(table);
    assert_eq!(temp.restore(), (expected, false));
}

#[test]
fn recovery_handles_format_undo_redo_and_empty_documents() {
    let temp = Temp::new();
    let mut table = temp.table("original\n");
    table.insert(0, "before ").unwrap();
    let mut snapshot = table
        .replace_from_reader(&mut &b"formatted\r\n"[..], 1024)
        .unwrap()
        .unwrap();
    table.swap_snapshot(&mut snapshot);
    table.swap_snapshot(&mut snapshot);
    let len = table.len();
    table.delete(0, len).unwrap();
    assert!(table.take_recovery_warning().is_none());
    drop(table);
    assert_eq!(temp.restore(), (String::new(), false));
}

#[test]
fn recovery_untitled_save_then_undo_keeps_immutable_original() {
    let temp = Temp::new();
    let mut table = PieceTable::empty().unwrap();
    table.enable_recovery_at(temp.0.join("recovery"), None);
    table.insert(0, "unsaved é").unwrap();
    drop(table);
    assert_eq!(temp.restore().0, "unsaved é");
    let temp = Temp::new();
    let mut table = temp.table("original");
    table.insert(8, " edited").unwrap();
    let destination = temp.0.join("source.txt");
    table.write_to(&destination).unwrap();
    table.recovery_saved(&destination);
    assert!(!table.recovery.journal.as_ref().unwrap().dirty);
    table.delete(8, 7).unwrap();
    fs::write(&destination, "outside modification").unwrap();
    assert_eq!(
        table.text().unwrap(),
        "original",
        "live table also uses immutable snapshot"
    );
    drop(table);
    assert_eq!(temp.restore().0, "original");
    assert_eq!(
        fs::read_to_string(destination).unwrap(),
        "outside modification"
    );
}

#[test]
fn recovery_snapshots_the_open_handle_after_source_path_is_replaced() {
    let temp = Temp::new();
    let mut table = temp.table("opened original");
    let source = temp.0.join("source.txt");
    let moved = temp.0.join("moved.txt");
    fs::rename(&source, &moved).unwrap();
    fs::write(&source, "replacement at the same path").unwrap();
    table.insert(0, "edited ").unwrap();
    fs::write(&moved, "outside change to the original inode").unwrap();
    assert_eq!(table.text().unwrap(), "edited opened original");
    assert!(table.take_recovery_warning().is_none());
    drop(table);
    assert_eq!(temp.restore(), ("edited opened original".into(), false));
    assert_eq!(fs::read_to_string(source).unwrap(), "replacement at the same path");
}

#[test]
fn successful_saves_clear_recovery_but_failed_saves_do_not() {
    let temp = Temp::new();
    let mut table = temp.table("saved");
    table.insert(5, " change").unwrap();
    let directory = table.recovery.journal.as_ref().unwrap().directory.clone();
    assert!(table.write_to(&temp.0.join("missing/file.txt")).is_err());
    drop(table);
    assert_eq!(temp.restore().0, "saved change");
    let mut copy = PieceTable::open(temp.0.join("source.txt").to_str().unwrap()).unwrap();
    copy.recovered_from(directory.clone());
    copy.recovery_saved(&temp.0.join("copy.txt"));
    assert!(list(&temp.0.join("recovery")).unwrap().is_empty());
    let temp = Temp::new();
    let mut table = temp.table("a");
    table.insert(1, "b").unwrap();
    let directory = table.recovery.journal.as_ref().unwrap().directory.clone();
    let path = temp.0.join("renamed.cs");
    table.write_to(&path).unwrap();
    table.recovery_saved(&path);
    table.insert(2, "c").unwrap();
    drop(table);
    let entry = list(&temp.0.join("recovery")).unwrap().pop().unwrap();
    assert_eq!(entry.source, Some(path.clone()));
    let (out, _) = restore(&temp.0.join("recovery"), &entry.id).unwrap();
    assert_eq!(out.extension().unwrap(), "cs");
    assert_eq!(fs::read_to_string(out).unwrap(), "abc");
    let mut table = PieceTable::open(path.to_str().unwrap()).unwrap();
    table.enable_recovery_at(temp.0.join("other"), Some(&path));
    table.insert(0, "new").unwrap();
    let clean = table.recovery.journal.as_ref().unwrap().directory.clone();
    table.write_to(&path).unwrap();
    table.recovery_saved(&path);
    drop(table);
    assert!(!clean.exists());
    assert!(directory.exists());
}

#[test]
fn recovery_ignores_every_truncated_tail_and_checksum_damage() {
    let temp = Temp::new();
    let mut table = temp.table("base");
    table.insert(4, " first").unwrap();
    let directory = table.recovery.journal.as_ref().unwrap().directory.clone();
    let log = directory.join("journal");
    let checkpoint = fs::metadata(&log).unwrap().len();
    table.insert(table.len(), " second").unwrap();
    drop(table);
    let complete = fs::read(&log).unwrap();
    for length in checkpoint as usize..complete.len() {
        fs::write(&log, &complete[..length]).unwrap();
        let (table, partial) = format::replay(&directory, &metadata(&directory).unwrap()).unwrap();
        assert_eq!(table.text().unwrap(), "base first");
        assert_eq!(partial, length != checkpoint as usize);
    }
    let mut damaged = complete;
    damaged[checkpoint as usize + 26] ^= 1;
    fs::write(&log, damaged).unwrap();
    assert_eq!(temp.restore(), ("base first".into(), true));
    assert!(
        directory.join("add").is_file(),
        "replay must not delete the backing edit store"
    );
}

#[test]
fn recovery_rejects_missing_sources_and_unbounded_frame_lengths() {
    let temp = Temp::new();
    let mut table = temp.table("a");
    table.insert(1, "b").unwrap();
    let directory = table.recovery.journal.as_ref().unwrap().directory.clone();
    drop(table);
    fs::write(directory.join("original"), "").unwrap();
    assert!(temp.restore_error().contains("incomplete"));
    fs::write(directory.join("original"), "a").unwrap();
    let mut log = OpenOptions::new()
        .write(true)
        .open(directory.join("journal"))
        .unwrap();
    log.seek(SeekFrom::Start(8)).unwrap();
    log.write_all(&u64::MAX.to_le_bytes()).unwrap();
    drop(log);
    assert!(temp.restore_error().contains("No complete"));
    assert!(restore(&temp.0.join("recovery"), "../source.txt").is_err());
}
impl Temp {
    fn restore_error(&self) -> String {
        let entries = list(&self.0.join("recovery")).unwrap();
        restore(&self.0.join("recovery"), &entries[0].id)
            .unwrap_err()
            .to_string()
    }
}

#[test]
fn recovery_failure_keeps_edits_and_reports_a_warning() {
    let temp = Temp::new();
    let mut table = PieceTable::empty().unwrap();
    let blocked = temp.0.join("not a directory");
    fs::write(&blocked, "occupied").unwrap();
    table.enable_recovery_at(blocked, None);
    table.insert(0, "still editable").unwrap();
    assert!(
        table
            .take_recovery_warning()
            .unwrap()
            .contains("recovery stopped")
    );
    table.insert(table.len(), "!").unwrap();
    assert_eq!(table.text().unwrap(), "still editable!");
    assert!(
        table.take_recovery_warning().is_none(),
        "warn once, without repeating during typing"
    );
}

#[test]
fn recovery_uses_fixed_buffers_and_small_per_edit_records() {
    assert!(std::mem::size_of::<State>() < 512);
    assert_eq!(COPY_BUFFER, 64 * 1024);
    let temp = Temp::new();
    let source = temp.0.join("source.txt");
    let file = File::create(&source).unwrap();
    file.set_len(8 * 1024 * 1024).unwrap();
    drop(file);
    let mut table = PieceTable::open(source.to_str().unwrap()).unwrap();
    table.enable_recovery_at(temp.0.join("recovery"), Some(&source));
    for _ in 0..100 {
        table.insert(0, "x").unwrap();
    }
    let directory = table.recovery.journal.as_ref().unwrap().directory.clone();
    assert_eq!(
        fs::metadata(directory.join("original")).unwrap().len(),
        8 * 1024 * 1024
    );
    assert!(
        fs::metadata(directory.join("journal")).unwrap().len() < 200 * 100,
        "normal typing must not write whole-piece-table snapshots"
    );
    assert!(table.take_recovery_warning().is_none());
}

#[test]
#[ignore = "child fixture for a real process-kill recovery test"]
fn crash_fixture() {
    let root = PathBuf::from(std::env::var_os("POTYI_RECOVERY_FIXTURE").unwrap());
    let mut table = PieceTable::open(root.join("source.txt").to_str().unwrap()).unwrap();
    table.enable_recovery_at(root.join("recovery"), Some(&root.join("source.txt")));
    table.insert(0, "recovered ").unwrap();
    table.delete(table.len() - 1, 1).unwrap();
    assert!(table.take_recovery_warning().is_none());
    fs::write(root.join("ready"), "ready").unwrap();
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[test]
fn recovery_survives_process_kill_and_skips_live_processes() {
    let temp = Temp::new();
    fs::write(temp.0.join("source.txt"), "text!").unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "piece_table::recovery::tests::crash_fixture",
            "--ignored",
            "--nocapture",
        ])
        .env("POTYI_RECOVERY_FIXTURE", &temp.0)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !temp.0.join("ready").exists() && Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let ready = temp.0.join("ready").exists();
    let live = list(&temp.0.join("recovery")).unwrap();
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    assert!(ready, "{}", String::from_utf8_lossy(&output.stderr));
    assert!(live.is_empty());
    assert_eq!(temp.restore(), ("recovered text".into(), false));
}

#[test]
#[ignore = "memory probe; run separately with /usr/bin/time and POTYI_RECOVERY_MEMORY=on or off"]
fn recovery_memory_probe() {
    let temp = Temp::new();
    let source = temp.0.join("large.txt");
    let file = File::create(&source).unwrap();
    file.set_len(256 * 1024 * 1024).unwrap();
    drop(file);
    let mut table = PieceTable::open(source.to_str().unwrap()).unwrap();
    if std::env::var("POTYI_RECOVERY_MEMORY").as_deref() == Ok("on") {
        table.enable_recovery_at(temp.0.join("recovery"), Some(&source));
    }
    for _ in 0..1000 {
        table.insert(0, "x").unwrap();
    }
    assert!(table.take_recovery_warning().is_none());
    assert_eq!(table.len(), 256 * 1024 * 1024 + 1000);
}

#[test]
fn damaged_metadata_does_not_hide_other_recovery_sessions() {
    let temp = Temp::new();
    let mut table = temp.table("a");
    table.insert(1, "b").unwrap();
    let directory = table.recovery.journal.as_ref().unwrap().directory.clone();
    drop(table);
    fs::write(directory.join("metadata.json"), "broken").unwrap();
    let mut table = temp.table("c");
    table.insert(1, "d").unwrap();
    drop(table);
    let entries = list(&temp.0.join("recovery")).unwrap();
    assert_eq!(entries.len(), 2);
    let good = entries.iter().find(|e| e.source.is_some()).unwrap();
    let (output, partial) = restore(&temp.0.join("recovery"), &good.id).unwrap();
    assert!(!partial);
    assert_eq!(fs::read_to_string(output).unwrap(), "cd");
}

#[test]
fn cross_filesystem_add_copy_and_failed_format_keep_journal_consistent() {
    let temp = Temp::new();
    let mut table = temp.table("old");
    table.insert(3, " text").unwrap();
    // Exercise the fallback used when temp and recovery directories are on different volumes.
    let journal = table.recovery.journal.as_mut().unwrap();
    let path = journal.directory.join("add");
    fs::remove_file(&path).unwrap();
    create_private(&path).unwrap();
    journal.linked_add = false;
    journal.copied_add = 0;
    table.insert(0, "new ").unwrap();
    let expected = table.text().unwrap();
    assert!(
        table
            .replace_from_reader(&mut &[b'a', 0xff][..], 100)
            .is_err()
    );
    table.insert(table.len(), "!").unwrap();
    assert!(table.take_recovery_warning().is_none());
    drop(table);
    assert_eq!(temp.restore().0, format!("{expected}!"));
}

#[test]
fn saving_after_a_journal_failure_restarts_from_the_complete_current_state() {
    let temp = Temp::new();
    let mut table = temp.table("base");
    table.insert(4, " one").unwrap();
    let journal = table.recovery.journal.as_mut().unwrap();
    let path = journal.directory.join("journal");
    journal.log = Some(File::open(&path).unwrap()); // inject a journal write failure
    table.insert(table.len(), " unjournaled").unwrap();
    assert!(table.take_recovery_warning().is_some());
    assert!(table.recovery.failed);
    table.recovery.journal.as_mut().unwrap().log = Some(
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap(),
    );
    let destination = temp.0.join("saved.txt");
    table.write_to(&destination).unwrap();
    table.recovery_saved(&destination);
    assert!(!table.recovery.failed);
    assert!(table.recovery.journal.is_none());
    table.insert(table.len(), " after save").unwrap();
    let expected = table.text().unwrap();
    assert!(table.take_recovery_warning().is_none());
    drop(table);
    assert_eq!(temp.restore().0, expected);
}
