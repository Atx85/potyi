// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in measurements: no timing assertions and no production runtime work.
use std::{fs::{self, File}, io::Write, time::{Duration, Instant}};
use crate::{PieceTable, Searcher};

pub(crate) fn record(name: &str, elapsed: Duration, bytes: usize) {
    if std::env::var_os("POTYI_BENCHMARK_JSON").is_some() {
        println!("POTYI_BENCH {}", serde_json::json!({
            "name":name,"milliseconds":elapsed.as_secs_f64()*1000.0,"bytes":bytes
        }));
    }
}

#[test]
#[ignore = "Opt-in benchmark; run tools/benchmarks/run.py"]
fn editor_performance_probe() {
    let directory = std::env::temp_dir().join(format!("potyi-benchmark-{}",std::process::id()));
    fs::create_dir(&directory).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }
    let _cleanup = Cleanup(directory.clone());
    // Real non-sparse files. Fixture generation and searcher setup are untimed.
    let chunk = b"let value = 42; // repeatable benchmark input\n".repeat(1024);
    for (tag, size) in [("1mib", 1024usize * 1024), ("100mib", 100 * 1024 * 1024), ("1gb", 1_000_000_000)] {
        let path = directory.join(format!("{tag}.txt"));
        let mut output = File::create(&path).unwrap();
        let mut remaining = size;
        while remaining > 0 { let n = remaining.min(chunk.len()); output.write_all(&chunk[..n]).unwrap(); remaining -= n; }
        output.sync_all().unwrap(); drop(output);
        let start = Instant::now();
        let mut table = PieceTable::open(path.to_str().unwrap()).unwrap();
        let open_elapsed = start.elapsed();
        let read_start = Instant::now();
        let first_lines: Vec<_> = (0..40).map(|line| table.line_text(line).unwrap()).collect();
        let read_elapsed = read_start.elapsed();
        let open_and_read_elapsed = start.elapsed();
        record(&format!("open_{tag}"),open_elapsed,size);
        record(&format!("first_40_lines_{tag}"),read_elapsed,size);
        assert_eq!(table.len(),size);
        assert!(first_lines.iter().all(|line| line.trim_end() == "let value = 42; // repeatable benchmark input"));
        if tag == "1gb" {
            record("open_and_first_40_lines_1gb",open_and_read_elapsed,size);
            // Loading probe only: no full-file search or editing for the 1 GB fixture.
            continue;
        }
        let searcher = Searcher::new("missing_benchmark_needle");
        let start = Instant::now();
        let result = searcher.find_forward(&table,0).unwrap();
        record(&format!("literal_scan_{tag}"),start.elapsed(),size);
        assert!(result.is_none());
        let start = Instant::now();
        for _ in 0..1000 { table.insert(size/2,"x").unwrap(); table.delete(size/2,1).unwrap(); }
        record(&format!("1000_insert_delete_pairs_{tag}"),start.elapsed(),size);
        assert_eq!(table.len(),size);
    }
}
