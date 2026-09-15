# Pötyi benchmark suite

Run from the repository root on macOS or Linux:

```sh
python3 tools/benchmarks/run.py
```

Requirements: Rust, Python 3, Pötyi's normal native build dependencies, a POSIX shell, `grep`, and `ps`. No benchmark package or additional Python dependencies are needed. The default idle measurement opens three temporary native app windows for about eight seconds each. Avoid interacting with them during sampling. The script builds the release test executable, runs one discarded warm-up and seven measured repetitions, then samples the actual native release app's idle memory and CPU three times. It never starts an LSP server. It only terminates app processes it created for its temporary fixture.

Results and raw probe logs go to ignored `temp/benchmarks/latest.json` and `.log`. JSON contains every sample, medians/minima/maxima, UTC date, compiler/target, shell, OS/CPU, source fingerprint and Git dirty state. Open/search/edit probes live in `src/benchmarks.rs`; terminal probes reuse the real terminal and renderer in `src/renderer.rs`. They have correctness checks but no timing pass/fail thresholds.

```sh
# Shorter run; at least three measured samples are required.
python3 tools/benchmarks/run.py --samples 3 --idle-samples 1
# Skip app-level idle sampling where a native display or process metrics are unavailable.
python3 tools/benchmarks/run.py --idle-samples 0
# Generate a standalone local HTML report from reviewed data.
python3 tools/benchmarks/publish.py temp/benchmarks/latest.json
# Render the recorded baseline (output defaults to temp/benchmarks/report.html).
python3 tools/benchmarks/publish.py tools/benchmarks/results.json
# Verify that local report against its source data.
python3 tools/benchmarks/publish.py tools/benchmarks/results.json --check
```

Compare the same workloads, shell, compiler target, display driver, build profile and machine. For a before/after comparison, keep both raw reports. Do not compare these core timings with another editor's complete startup time.

Fixtures are real, non-sparse 1 MiB, 100 MiB, and 1 GB (1,000,000,000 bytes) files, created outside the timer and already warm in the filesystem cache. Opening is lazy; accessing the first 40 lines is separately timed and checked against the fixture text. The 1 GB load test also records combined opening and first-40-line access, without reading the entire gigabyte. Temporary fixtures are removed after each probe; allow about 1.2 GB of free temporary disk space. Search and editing use only the smaller fixtures: search scans for a missing literal, and editing times 1,000 one-byte insert/delete pairs in the middle of the document, without rendering or editor undo bookkeeping. Terminal timing uses an 800×600 headless SDL renderer, not a desktop compositor. The append/layout fixture is exactly 2,060,800 bytes.

Idle sampling opens a small temporary document in the actual release binary with default settings, LSP off, and SDL's native display driver. After three seconds of settling, `ps` samples resident memory and cumulative CPU time at the start and end of a five-second interval. RSS is not peak memory. CPU is a percentage of one logical core and has OS-dependent time quantization; 0% means below measurement resolution. These measurements do not establish large-file memory use, other display drivers, complete application startup time, or LSP server costs.

The generated report includes timings and measurement limits; raw samples remain in the input JSON. Never replace missing results with estimates, use one fastest run as the headline, or present headless measurements as a comparison against another editor. Benchmark scripts do not publish or push to GitHub.

## Recorded results

`results.json` preserves the reviewed 11 September 2026 baseline. The public
website does not include a benchmark page. Despite its historical name,
`publish.py` only writes a standalone local report; it does not update `docs/`
or publish anything. Use `--output path/to/report.html` to choose its location.
