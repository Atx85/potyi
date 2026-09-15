// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in edit-handler latency measurements; no production instrumentation.

use crate::{Editor, config::EditorConfig};
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn timed(action: impl FnOnce()) -> f64 {
    let start = Instant::now();
    action();
    start.elapsed().as_secs_f64() * 1000.0
}

#[test]
#[ignore = "Opt-in recovery typing probe; run tools/qa/recovery_latency.py"]
fn typing_latency_probe() {
    let size: usize = std::env::var("POTYI_RECOVERY_LATENCY_BYTES")
        .expect("explicit file size required")
        .parse()
        .unwrap();
    assert!(size <= 1024 * 1024 * 1024);
    let recovery = match std::env::var("POTYI_RECOVERY_LATENCY_MODE").as_deref() {
        Ok("on") => true,
        Ok("off") => false,
        _ => panic!("mode must be on or off"),
    };
    let root = std::env::temp_dir().join(format!(
        "potyi-recovery-latency-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let _cleanup = Cleanup(root.clone());
    let path = root.join("source.txt");
    // Real text, physically written (not a sparse allocation). Fixture creation
    // is outside all edit timers; source data is warm in the OS filesystem cache.
    let block = b"let value = 42; // latency probe\n".repeat(1024);
    let mut file = File::create(&path).unwrap();
    let mut remaining = size;
    while remaining > 0 {
        let count = remaining.min(block.len());
        file.write_all(&block[..count]).unwrap();
        remaining -= count;
    }
    file.sync_all().unwrap();
    drop(file);

    let mut editor = Editor::new(EditorConfig::default()).unwrap();
    editor.open(path.to_str().unwrap()).unwrap();
    // Simulate the already displayed top viewport without indexing the entire file.
    for line in 0..if size == 0 { 1 } else { 40 } {
        editor.document.line_text(line).unwrap();
    }
    if recovery {
        editor
            .document
            .enable_recovery_at(root.join("recovery"), Some(&path));
    }
    let first_ms = timed(|| editor.insert_text("X").unwrap());
    assert_eq!(editor.document.len(), size + 1);

    let mut insert_ms = Vec::with_capacity(200);
    for _ in 0..200 {
        insert_ms.push(timed(|| editor.insert_text("x").unwrap()));
    }
    let mut backspace_ms = Vec::with_capacity(200);
    for _ in 0..200 {
        backspace_ms.push(timed(|| editor.backspace().unwrap()));
    }
    assert_eq!(editor.document.len(), size + 1);

    let mut replace_ms = Vec::with_capacity(50);
    for index in 0..50 {
        editor.set_cursor_and_anchor(1, 0).unwrap();
        replace_ms.push(timed(|| {
            editor
                .insert_text(if index % 2 == 0 { "Y" } else { "X" })
                .unwrap()
        }));
    }
    assert_eq!(editor.document.byte_at(0).unwrap(), Some(b'X'));
    assert_eq!(editor.document.len(), size + 1);

    let mut paced_ms = Vec::with_capacity(30);
    for _ in 0..30 {
        // Delay between dispatches is deliberately outside the timer. This is
        // not an OS key event, input queue, redraw or display-latency measurement.
        std::thread::sleep(Duration::from_millis(50));
        paced_ms.push(timed(|| editor.insert_text("x").unwrap()));
    }
    assert_eq!(editor.document.len(), size + 31);
    let save_ms = timed(|| editor.save().unwrap());
    assert_eq!(fs::metadata(&path).unwrap().len(), (size + 31) as u64);
    let after_save_ms = timed(|| editor.insert_text("z").unwrap());
    assert_eq!(editor.document.len(), size + 32);
    editor.undo().unwrap();
    assert_eq!(editor.document.len(), size + 31);
    editor.redo().unwrap();
    assert_eq!(editor.document.len(), size + 32);
    assert!(editor.document.take_recovery_warning().is_none());
    println!(
        "POTYI_RECOVERY_LATENCY {}",
        serde_json::json!({
            "bytes":size,"recovery":recovery,"first_ms":first_ms,"insert_ms":insert_ms,
            "backspace_ms":backspace_ms,"replace_ms":replace_ms,"paced_ms":paced_ms,
            "save_ms":save_ms,"after_save_ms":after_save_ms,
        })
    );
}
