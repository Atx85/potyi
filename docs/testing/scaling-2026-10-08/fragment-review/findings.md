# Independent fragmentation/refill findings

Repository archive note: raw reports, scripts and logs are retained here; executable copies are excluded. The runnable original output directory is recorded in `provenance.json`. Run `compare.py` from that original directory, or restore its `fixed64/probe` and `adaptive/probe` from the ignored saved executables before rerunning an archived copy.

8 October 2026. **Retain adaptive refills and the flat vector for this delivery.** The measured short-edit regression primarily came from reading 64 KiB across fragmented pieces after each edit. A 256-byte refill growing to 64 KiB removes that cost. Remaining vector work is linear; this comparison does not demonstrate logarithmic scaling or performance beyond roughly 200,000 pieces.

Copied immutable release probes were checked against both original recorded SHA-256 hashes. Both builds reported stable source hashes. The probes use shared storage and the bounded line index; their relevant difference is fixed 64 KiB versus adaptive refills. Fresh 1 MiB / 64-byte-line files received dispersed insertions, then middle navigation and four production Editor edit/undo/redo/backspace cycles. Serial execution alternated variant ordering. One discarded warmup plus three measured runs per variant/case gives 3 first-operation and 9 correlated later-operation samples. All 24 processes completed within a 60-second limit. Rendering, recovery, extra panes and memory sampling were disabled.

Times below are milliseconds. p95/p99 equal the maximum at these sample counts. Reads are successful positional file reads, including rereads, rather than physical disk I/O.

| Dispersed edits | Later operation | Fixed median / p95/p99 | Adaptive median / p95/p99 | Fixed bytes / calls | Adaptive bytes / calls |
| ---: | --- | ---: | ---: | ---: | ---: |
| 1,000 | insert | 0.098 / 0.142 | 0.017 / 0.021 | 65605 / 135 | 325 / 9 |
| 1,000 | backspace | 0.097 / 0.120 | 0.018 / 0.085 | 65546 / 135 | 266 / 9 |
| 10,000 | insert | 0.836 / 1.074 | 0.077 / 0.088 | 65606 / 1249 | 326 / 15 |
| 10,000 | backspace | 0.902 / 1.450 | 0.124 / 0.125 | 65546 / 1247 | 266 / 13 |
| 100,000 | insert | 7.285 / 7.557 | 0.629 / 0.659 | 65611 / 11433 | 331 / 65 |
| 100,000 | backspace | 7.933 / 9.631 | 1.183 / 2.228 | 65546 / 11421 | 266 / 53 |

At 100,000 dispersed edits, later insert median improves **11.6×**. Adaptive first insert median/p95 is 0.674/0.696 ms; first backspace is 2.298/2.352 ms. Insert/backspace discover one line per operation. First and later timings for every case are in [details.md](details.md).

| Dispersed edits | Later undo fixed → adaptive median, ms | Later redo fixed → adaptive median, ms | Final pieces | Navigation fixed → adaptive median, ms | Whole-process fixed → adaptive median, s |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 0.008 → 0.008 | 0.003 → 0.003 | 2,002 | 2.901 → 2.894 | 0.114 → 0.113 |
| 10,000 | 0.066 → 0.067 | 0.020 → 0.020 | 20,002 | 9.328 → 9.570 | 0.216 → 0.216 |
| 100,000 | 1.019 → 0.656 | 0.189 → 0.189 | 200,002 | 63.093 → 64.987 | 7.424 → 7.481 |

Undo/redo read two/one bytes in two/one calls and discover no lines. At 100,000 edits, adaptive later undo p95/p99 is 1.814 ms and redo is 0.199 ms; first undo median/p95 is 1.933/1.963 ms. Their timing variation should not be attributed to refills, because they retain history and vector work.

Fragmentation preparation is not separately timed. Whole-process elapsed includes fixture creation, open, preparation, navigation, editing, cleanup and roughly 100 ms polling granularity. Both 100,000-edit variants still take about 7.4 seconds overall. Their initial navigation still needs roughly 103,000 read calls. A byte-count tree alone would not remove those reads.

Every run retained 4,096 detailed lines, 128 sparse checkpoints and 100,352 allocated metadata bytes. Copy counters stayed zero for pieces, lines and history in all timed operations; this does not count every allocator copy or deep undo allocation. No whole-process memory claim follows.

Revisit a balanced tree if real workloads accumulate materially more pieces, handler tails exceed an agreed latency budget, or profiles identify vector lookup/splicing as a material delay. Local warm-filesystem measurements with three independent runs do not establish native UI latency, cold-disk behavior, a maximum supported piece count, recovery costs or tree superiority. No production source files were modified and no failures/timeouts were omitted.

Evidence: [results.json](results.json) contains all raw events and summaries; [provenance.json](provenance.json) contains identities and environment; `fixed64/` and `adaptive/` contain copied probes, original reports and every raw log. [details.md](details.md) has full first/later, tail and read tables. Run `python3 compare.py` to repeat using saved probes, or `python3 report.py` to regenerate documents.
