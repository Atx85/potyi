# Independent fragmentation/refill comparison

Repository archive note: raw reports, scripts and logs are retained here; executable copies are excluded. The runnable original output directory is recorded in `provenance.json`. Run `compare.py` from that original directory, or restore its `fixed64/probe` and `adaptive/probe` from the ignored saved executables before rerunning an archived copy.

Recorded 8 October 2026. No production source files were changed by this review.

**Recommendation: retain the adaptive refill and the flat piece vector for the tested range.** The large fragmented-edit regression primarily came from reading an unrelated 64 KiB after each edit. Starting with 256 bytes removes that cost without a piece-tree refactor. A tree remains a possible later improvement to linear lookup/splicing; these measurements do not demonstrate logarithmic scaling or establish performance beyond roughly 200,000 pieces.

## Controlled comparison

Two immutable release test executables were copied with their original result files and verified against the recorded SHA-256 hashes. Both original builds reported stable source hashes. They use the shared-document and bounded-index implementation. The parent identifies their relevant difference as fixed 64 KiB refills versus 256-byte refills after invalidation, growing geometrically to 64 KiB. This review measures that comparison; it does not compare a vector with a tree.

Each case creates a fresh 1 MiB file with 64-byte lines, disperses 1,000 / 10,000 / 100,000 insertions, navigates to the middle, and executes four production Editor insert/undo/redo/backspace cycles. One process warmup per executable/case is discarded; three measured processes remain. The variants ran serially and their ordering alternated within each pair. No rendering, recovery, second pane, or memory sampling was enabled. The full parent matrix had finished before this run.

All **24 processes completed**, including the six discarded warmups. Every process had a 60-second wall-time limit, which includes fixture creation and fragmentation setup.

## Insert and backspace

Times are milliseconds. First has 3 samples; later has 9 samples. p95 and p99 are nearest-rank statistics and both equal the sample maximum at these small sample counts.

| Dispersed edits | Operation | Bucket | Fixed median / p95 / p99 | Adaptive median / p95 / p99 | Fixed reads, bytes / calls | Adaptive reads, bytes / calls |
| ---: | --- | --- | ---: | ---: | ---: | ---: |
| 1,000 | insert | first | 0.118916 / 0.124625 / 0.124625 | 0.036209 / 0.039291 / 0.039291 | 65605 / 135 | 325 / 9 |
| 1,000 | insert | later | 0.098166 / 0.142291 / 0.142291 | 0.017000 / 0.021125 / 0.021125 | 65605 / 135 | 325 / 9 |
| 1,000 | backspace | first | 0.098333 / 0.134500 / 0.134500 | 0.030125 / 0.034875 / 0.034875 | 65546 / 135 | 266 / 9 |
| 1,000 | backspace | later | 0.096833 / 0.119959 / 0.119959 | 0.018167 / 0.084833 / 0.084833 | 65546 / 135 | 266 / 9 |
| 10,000 | insert | first | 0.872833 / 0.889875 / 0.889875 | 0.100292 / 0.123208 / 0.123208 | 65606 / 1249 | 326 / 15 |
| 10,000 | insert | later | 0.836458 / 1.074333 / 1.074333 | 0.077084 / 0.087750 / 0.087750 | 65606 / 1249 | 326 / 15 |
| 10,000 | backspace | first | 0.990459 / 1.016417 / 1.016417 | 0.284458 / 0.309250 / 0.309250 | 65546 / 1247 | 266 / 13 |
| 10,000 | backspace | later | 0.901709 / 1.450084 / 1.450084 | 0.123958 / 0.124875 / 0.124875 | 65546 / 1247 | 266 / 13 |
| 100,000 | insert | first | 7.285542 / 7.614833 / 7.614833 | 0.674209 / 0.695583 / 0.695583 | 65611 / 11433 | 331 / 65 |
| 100,000 | insert | later | 7.285250 / 7.557375 / 7.557375 | 0.629167 / 0.658708 / 0.658708 | 65611 / 11433 | 331 / 65 |
| 100,000 | backspace | first | 8.996750 / 9.019209 / 9.019209 | 2.297584 / 2.352208 / 2.352208 | 65546 / 11421 | 266 / 53 |
| 100,000 | backspace | later | 7.933291 / 9.631250 / 9.631250 | 1.183500 / 2.227542 / 2.227542 | 65546 / 11421 | 266 / 53 |

At 100,000 dispersed edits, later insert median improves **11.6×**, from 7.285250 to 0.629167 ms; its p95/p99 improves from 7.557375 to 0.658708 ms. Insert reads fall from 65,611 bytes / 11,433 calls to 331 bytes / 65 calls. Later backspace median improves from 7.933291 to 1.183500 ms, with adaptive p95/p99 of 2.227542 ms. Both discover one affected line per insert/backspace. Those reads are successful positional file reads, including rereads, rather than physical disk I/O.

## Undo and redo

| Dispersed edits | Operation | Bucket | Fixed median / p95 / p99, ms | Adaptive median / p95 / p99, ms | Reads, bytes / calls (both) |
| ---: | --- | --- | ---: | ---: | ---: |
| 1,000 | undo | first | 0.071000 / 0.074041 / 0.074041 | 0.075958 / 0.076792 / 0.076792 | 2 / 2 |
| 1,000 | undo | later | 0.008208 / 0.026959 / 0.026959 | 0.008041 / 0.023416 / 0.023416 | 2 / 2 |
| 1,000 | redo | first | 0.004875 / 0.006417 / 0.006417 | 0.004459 / 0.007375 / 0.007375 | 1 / 1 |
| 1,000 | redo | later | 0.003167 / 0.004292 / 0.004292 | 0.003167 / 0.003792 / 0.003792 | 1 / 1 |
| 10,000 | undo | first | 0.246375 / 0.323875 / 0.323875 | 0.266875 / 0.269000 / 0.269000 | 2 / 2 |
| 10,000 | undo | later | 0.066459 / 0.231459 / 0.231459 | 0.066792 / 0.182500 / 0.182500 | 2 / 2 |
| 10,000 | redo | first | 0.021167 / 0.022000 / 0.022000 | 0.021667 / 0.262792 / 0.262792 | 1 / 1 |
| 10,000 | redo | later | 0.019875 / 0.026416 / 0.026416 | 0.020125 / 0.020959 / 0.020959 | 1 / 1 |
| 100,000 | undo | first | 1.890459 / 2.000417 / 2.000417 | 1.932500 / 1.963291 / 1.963291 | 2 / 2 |
| 100,000 | undo | later | 1.018709 / 1.886125 / 1.886125 | 0.656459 / 1.814042 / 1.814042 | 2 / 2 |
| 100,000 | redo | first | 0.188084 / 0.201709 / 0.201709 | 0.187792 / 0.201000 / 0.201000 | 1 / 1 |
| 100,000 | redo | later | 0.188792 / 0.276708 / 0.276708 | 0.189250 / 0.199292 / 0.199292 | 1 / 1 |

Undo/redo discover no lines in these cycles. They continue to include history and piece-vector work; the refill change does not remove that work. Variation in undo timing should not be presented as a guaranteed speedup from refills because these operations perform only two or one positional reads.

## Setup, navigation, and retained metadata

| Dispersed edits | Final piece count | Fixed navigation median / p95, ms | Adaptive navigation median / p95, ms | Fixed navigation bytes / calls | Adaptive navigation bytes / calls | Fixed / adaptive whole-process median, s |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 2,002 | 2.901458 / 2.999209 | 2.894416 / 2.960416 | 589826 / 1135 | 589571 / 1143 | 0.114 / 0.113 |
| 10,000 | 20,002 | 9.327500 / 9.501375 | 9.570417 / 12.652625 | 589827 / 11156 | 589570 / 11157 | 0.216 / 0.216 |
| 100,000 | 200,002 | 63.093208 / 64.008500 | 64.986958 / 66.252750 | 589826 / 102716 | 589570 / 102675 | 7.424 / 7.481 |

All runs retained 4,096 detailed line records and 128 sparse checkpoints, with 100,352 bytes of retained vector allocation reported by the probe. The known line prefix was 8,193 lines, which is different from the 4,096 retained details. No piece, line, or history copy counters incremented in any timed operation. These counters do not count every allocator copy or deep undo allocation.

Fragmentation preparation is outside the operation timers and has no separate timer in this saved probe. Whole-process elapsed therefore includes fixture creation, open, preparation, navigation, editing, cleanup and roughly 100 ms polling granularity. At 100,000 dispersed edits both variants take about 7.4 seconds overall. Initial navigation still costs roughly 63–65 ms and about 103,000 read calls. The adaptive strategy fixes short edit refills; it does not fix setup or scanning across many pieces.

## Piece-tree decision and limits

The flat vector still performs linear work with piece count. The final fixture contains 200,002 pieces (roughly 200,001 after dispersion plus the timed editing boundary), and adaptive edit latency rises between the 1,000 and 100,000 cases. The 100,000 case demonstrates that this remaining cost is measurable, even though the tested insert/undo/redo/backspace handlers stay in the low milliseconds.

Keep the vector for this delivery. Revisit a balanced structure if supported production workloads accumulate materially more pieces, if measured handler tails exceed an agreed latency budget, or if profiles attribute real navigation/editing delays to vector lookup or splicing. A tree comparison should retain streaming traversal and separately measure fragmented scan/read costs; a byte-count tree alone does not remove the roughly 103,000 positional reads in this initial scan.

These are local warm-filesystem measurements on the original recorded macOS environment, not cold-disk measurements or native UI latency. There are only three independent measured processes per case; the nine later samples are correlated within those processes. No result establishes a whole-process memory budget, the maximum supported piece count, large multi-cursor edits, snapshot replacement, recovery costs, or tree superiority. No failures or timeouts were dropped from these results.

## Reproduction and evidence

Run `python3 compare.py` from this isolated directory to repeat serial measurements using its copied executables (this replaces results and logs). Run `python3 report.py` to regenerate this document from the saved JSON without executing benchmarks. The runner imports the saved scaling harness.

- `provenance.json`: original source identity, environment, original/copy executable paths and hashes.
- `fixed64/original-results.json`, `adaptive/original-results.json`: complete original reports.
- `results.json`: raw per-operation events, status, elapsed time, ordering and summaries for all 24 processes.
- `fixed64/fragments*-run*.log`, `adaptive/fragments*-run*.log`: unmodified probe output.
- `run.log`: serial comparison progress and completion.
