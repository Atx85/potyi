# Sustained editing baseline

Recorded: 2026-10-09T14:33:03.440372+00:00.

Profile: **release**. OS: macOS-15.8.1-arm64-arm-64bit-Mach-O.
Source stable during build: **True**.

Times are milliseconds. First and later edits are separate; setup includes navigation and initial drawing.
Only complete measured runs contribute to medians. Failed and timed-out runs retain partial events in JSON.

| Case | Completed measured runs | Unfinished runs | Last unfinished phase | Sampled RSS maximum, MiB |
| --- | ---: | --- | --- | ---: |
| 1000000000b-line64-pos99 | 3 | none | — | unavailable |

## 1000000000b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.011 | 0.012 | 0.012 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.008 | 0.015 | 0.015 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 3.952 | 4.043 | 4.043 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.084 | 0.104 | 0.104 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.012 | 0.021 | 0.021 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3640.485 | 3666.901 | 3666.901 | 990052354 | 15468751 | 0 / 0 / 0 |
| open | setup | 3 | 1620.016 | 1751.448 | 1751.448 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.327 | 2.341 | 2.341 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.179 | 2.339 | 2.339 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.052 | 0.058 | 0.058 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## Limits

- Local measurements with freshly written, warm filesystem fixtures; no cold-disk or other-editor comparison.
- Headless rendering measures SDL draw work, not native desktop presentation, input dispatch or LSP work.
- RSS is sampled about every 100ms across fixture/setup/editing, not true peak memory or an allocation budget; ps may be unavailable.
- Read counters count successful positional file reads in the piece table, including rereads; not physical disk I/O or recovery writes.
- Line discoveries count discovery calls, including reconstructed evicted entries and EOF handling; not unique lines.
- Copy counters count top-level records at view/history cloning sites, not bytes or deep allocations in undo snapshots.
- p95/p99 use nearest ranks; small sample counts and correlated operations limit tail claims.
- The first edit and later edits are reported separately. Timeout includes fixture and setup; partial timings remain in raw events.
- Unfinished warmups stop that case's remaining runs; no timing threshold determines correctness.
- Concurrent checkout changes after building cannot alter the saved probe. Different before/after build hashes make source provenance inconclusive.
