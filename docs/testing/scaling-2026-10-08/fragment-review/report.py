#!/usr/bin/env python3
"""Render the saved fragmentation comparison without rerunning probes."""
import json
from pathlib import Path
import statistics

root = Path(__file__).resolve().parent
report = json.loads((root / "results.json").read_text())


def row(case, variant, stage, bucket):
    return next(r for r in case["variants"][variant]["summary"]
                if r["stage"] == stage and r["bucket"] == bucket)


def complete_rows(case, variant):
    return [next(event for event in run["events"] if event["type"] == "complete")
            for run in case["variants"][variant]["runs"] if not run["warmup"]]


assert len(report["execution_order"]) == 24
assert all(run["status"] == "complete" for case in report["cases"]
           for variant in case["variants"].values() for run in variant["runs"])
assert all(all(event["counters"][key] == 0 for key in
               ("copied_piece_records", "copied_line_records", "copied_history_entries"))
           for case in report["cases"] for variant in case["variants"].values()
           for run in variant["runs"] for event in run["events"] if event["type"] == "measurement")

lines = [
    "# Independent fragmentation/refill comparison",
    "",
    "Recorded 8 October 2026. No production source files were changed by this review.",
    "",
    "**Recommendation: retain the adaptive refill and the flat piece vector for the tested range.** "
    "The large fragmented-edit regression primarily came from reading an unrelated 64 KiB after "
    "each edit. Starting with 256 bytes removes that cost without a piece-tree refactor. "
    "A tree remains a possible later improvement to linear lookup/splicing; these measurements "
    "do not demonstrate logarithmic scaling or establish performance beyond roughly 200,000 pieces.",
    "",
    "## Controlled comparison",
    "",
    "Two immutable release test executables were copied with their original result files and "
    "verified against the recorded SHA-256 hashes. Both original builds reported stable source hashes. "
    "They use the shared-document and bounded-index implementation. The parent identifies their "
    "relevant difference as fixed 64 KiB refills versus 256-byte refills after invalidation, growing "
    "geometrically to 64 KiB. This review measures that comparison; it does not compare a vector with a tree.",
    "",
    "Each case creates a fresh 1 MiB file with 64-byte lines, disperses 1,000 / 10,000 / 100,000 "
    "insertions, navigates to the middle, and executes four production Editor insert/undo/redo/backspace "
    "cycles. One process warmup per executable/case is discarded; three measured processes remain. "
    "The variants ran serially and their ordering alternated within each pair. No rendering, recovery, "
    "second pane, or memory sampling was enabled. The full parent matrix had finished before this run.",
    "",
    "All **24 processes completed**, including the six discarded warmups. Every process had a "
    "60-second wall-time limit, which includes fixture creation and fragmentation setup.",
    "",
    "## Insert and backspace",
    "",
    "Times are milliseconds. First has 3 samples; later has 9 samples. p95 and p99 are nearest-rank "
    "statistics and both equal the sample maximum at these small sample counts.",
    "",
    "| Dispersed edits | Operation | Bucket | Fixed median / p95 / p99 | Adaptive median / p95 / p99 | Fixed reads, bytes / calls | Adaptive reads, bytes / calls |",
    "| ---: | --- | --- | ---: | ---: | ---: | ---: |",
]
for case in report["cases"]:
    edits = case["config"]["fragmented_edits"]
    for stage in ("insert", "backspace"):
        for bucket in ("first", "later"):
            fixed = row(case, "fixed64", stage, bucket)
            adaptive = row(case, "adaptive", stage, bucket)
            times = lambda r: " / ".join(f"{r[key]:.6f}" for key in ("median_ms", "p95_ms", "p99_ms"))
            reads = lambda r: " / ".join(str(int(r["counters"][key]["median"])) for key in ("read_bytes", "read_calls"))
            lines.append(f"| {edits:,} | {stage} | {bucket} | {times(fixed)} | {times(adaptive)} | {reads(fixed)} | {reads(adaptive)} |")
lines += [
    "",
    "At 100,000 dispersed edits, later insert median improves **11.6×**, from 7.285250 to 0.629167 ms; "
    "its p95/p99 improves from 7.557375 to 0.658708 ms. Insert reads fall from 65,611 bytes / 11,433 calls "
    "to 331 bytes / 65 calls. Later backspace median improves from 7.933291 to 1.183500 ms, with "
    "adaptive p95/p99 of 2.227542 ms. Both discover one affected line per insert/backspace. "
    "Those reads are successful positional file reads, including rereads, rather than physical disk I/O.",
    "",
    "## Undo and redo",
    "",
    "| Dispersed edits | Operation | Bucket | Fixed median / p95 / p99, ms | Adaptive median / p95 / p99, ms | Reads, bytes / calls (both) |",
    "| ---: | --- | --- | ---: | ---: | ---: |",
]
for case in report["cases"]:
    edits = case["config"]["fragmented_edits"]
    for stage in ("undo", "redo"):
        for bucket in ("first", "later"):
            fixed = row(case, "fixed64", stage, bucket)
            adaptive = row(case, "adaptive", stage, bucket)
            assert reads(fixed) == reads(adaptive)
            lines.append(f"| {edits:,} | {stage} | {bucket} | {times(fixed)} | {times(adaptive)} | {reads(fixed)} |")
lines += [
    "",
    "Undo/redo discover no lines in these cycles. They continue to include history and piece-vector work; "
    "the refill change does not remove that work. Variation in undo timing should not be presented as "
    "a guaranteed speedup from refills because these operations perform only two or one positional reads.",
    "",
    "## Setup, navigation, and retained metadata",
    "",
    "| Dispersed edits | Final piece count | Fixed navigation median / p95, ms | Adaptive navigation median / p95, ms | Fixed navigation bytes / calls | Adaptive navigation bytes / calls | Fixed / adaptive whole-process median, s |",
    "| ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
]
for case in report["cases"]:
    fixed = row(case, "fixed64", "navigation", "setup")
    adaptive = row(case, "adaptive", "navigation", "setup")
    endings = complete_rows(case, "fixed64") + complete_rows(case, "adaptive")
    assert len({event["final_pieces"] for event in endings}) == 1
    assert all(event["line_index_metadata"] == [4096, 128, 100352] for event in endings)
    walls = [statistics.median(run["seconds"] for run in case["variants"][variant]["runs"] if not run["warmup"])
             for variant in ("fixed64", "adaptive")]
    lines.append(f"| {case['config']['fragmented_edits']:,} | {endings[0]['final_pieces']:,} | "
                 f"{fixed['median_ms']:.6f} / {fixed['p95_ms']:.6f} | "
                 f"{adaptive['median_ms']:.6f} / {adaptive['p95_ms']:.6f} | "
                 f"{reads(fixed)} | {reads(adaptive)} | {walls[0]:.3f} / {walls[1]:.3f} |")
lines += [
    "",
    "All runs retained 4,096 detailed line records and 128 sparse checkpoints, with 100,352 bytes "
    "of retained vector allocation reported by the probe. The known line prefix was 8,193 lines, "
    "which is different from the 4,096 retained details. No piece, line, or history copy counters "
    "incremented in any timed operation. These counters do not count every allocator copy or deep undo allocation.",
    "",
    "Fragmentation preparation is outside the operation timers and has no separate timer in this saved probe. "
    "Whole-process elapsed therefore includes fixture creation, open, preparation, navigation, editing, "
    "cleanup and roughly 100 ms polling granularity. At 100,000 dispersed edits both variants take about "
    "7.4 seconds overall. Initial navigation still costs roughly 63–65 ms and about 103,000 read calls. "
    "The adaptive strategy fixes short edit refills; it does not fix setup or scanning across many pieces.",
    "",
    "## Piece-tree decision and limits",
    "",
    "The flat vector still performs linear work with piece count. The final fixture contains 200,002 pieces "
    "(roughly 200,001 after dispersion plus the timed editing boundary), and adaptive edit latency rises "
    "between the 1,000 and 100,000 cases. The 100,000 case demonstrates that this remaining cost is "
    "measurable, even though the tested insert/undo/redo/backspace handlers stay in the low milliseconds.",
    "",
    "Keep the vector for this delivery. Revisit a balanced structure if supported production workloads "
    "accumulate materially more pieces, if measured handler tails exceed an agreed latency budget, "
    "or if profiles attribute real navigation/editing delays to vector lookup or splicing. A tree "
    "comparison should retain streaming traversal and separately measure fragmented scan/read costs; "
    "a byte-count tree alone does not remove the roughly 103,000 positional reads in this initial scan.",
    "",
    "These are local warm-filesystem measurements on the original recorded macOS environment, not cold-disk "
    "measurements or native UI latency. There are only three independent measured processes per case; "
    "the nine later samples are correlated within those processes. No result establishes a whole-process "
    "memory budget, the maximum supported piece count, large multi-cursor edits, snapshot replacement, "
    "recovery costs, or tree superiority. No failures or timeouts were dropped from these results.",
    "",
    "## Reproduction and evidence",
    "",
    "Run `python3 compare.py` from this isolated directory to repeat serial measurements using its copied "
    "executables (this replaces results and logs). Run `python3 report.py` to regenerate this document "
    "from the saved JSON without executing benchmarks. The runner imports the saved scaling harness.",
    "",
    "- `provenance.json`: original source identity, environment, original/copy executable paths and hashes.",
    "- `fixed64/original-results.json`, `adaptive/original-results.json`: complete original reports.",
    "- `results.json`: raw per-operation events, status, elapsed time, ordering and summaries for all 24 processes.",
    "- `fixed64/fragments*-run*.log`, `adaptive/fragments*-run*.log`: unmodified probe output.",
    "- `run.log`: serial comparison progress and completion.",
]
(root / "details.md").write_text("\n".join(lines) + "\n")
short = [
    "# Independent fragmentation/refill findings",
    "",
    "8 October 2026. **Retain adaptive refills and the flat vector for this delivery.** "
    "The measured short-edit regression primarily came from reading 64 KiB across fragmented pieces "
    "after each edit. A 256-byte refill growing to 64 KiB removes that cost. Remaining vector work "
    "is linear; this comparison does not demonstrate logarithmic scaling or performance beyond roughly 200,000 pieces.",
    "",
    "Copied immutable release probes were checked against both original recorded SHA-256 hashes. "
    "Both builds reported stable source hashes. The probes use shared storage and the bounded line index; "
    "their relevant difference is fixed 64 KiB versus adaptive refills. Fresh 1 MiB / 64-byte-line files "
    "received dispersed insertions, then middle navigation and four production Editor edit/undo/redo/backspace "
    "cycles. Serial execution alternated variant ordering. One discarded warmup plus three measured runs "
    "per variant/case gives 3 first-operation and 9 correlated later-operation samples. All 24 processes "
    "completed within a 60-second limit. Rendering, recovery, extra panes and memory sampling were disabled.",
    "",
    "Times below are milliseconds. p95/p99 equal the maximum at these sample counts. Reads are successful "
    "positional file reads, including rereads, rather than physical disk I/O.",
    "",
    "| Dispersed edits | Later operation | Fixed median / p95/p99 | Adaptive median / p95/p99 | Fixed bytes / calls | Adaptive bytes / calls |",
    "| ---: | --- | ---: | ---: | ---: | ---: |",
]
for case in report["cases"]:
    for stage in ("insert", "backspace"):
        fixed = row(case, "fixed64", stage, "later")
        adaptive = row(case, "adaptive", stage, "later")
        short.append(f"| {case['config']['fragmented_edits']:,} | {stage} | "
                     f"{fixed['median_ms']:.3f} / {fixed['p95_ms']:.3f} | "
                     f"{adaptive['median_ms']:.3f} / {adaptive['p95_ms']:.3f} | {reads(fixed)} | {reads(adaptive)} |")
short += [
    "",
    "At 100,000 dispersed edits, later insert median improves **11.6×**. Adaptive first insert "
    "median/p95 is 0.674/0.696 ms; first backspace is 2.298/2.352 ms. Insert/backspace discover "
    "one line per operation. First and later timings for every case are in [details.md](details.md).",
    "",
    "| Dispersed edits | Later undo fixed → adaptive median, ms | Later redo fixed → adaptive median, ms | Final pieces | Navigation fixed → adaptive median, ms | Whole-process fixed → adaptive median, s |",
    "| ---: | ---: | ---: | ---: | ---: | ---: |",
]
for case in report["cases"]:
    edits = case["config"]["fragmented_edits"]
    pairs = []
    for stage, bucket in (("undo", "later"), ("redo", "later"), ("navigation", "setup")):
        pairs.append(" → ".join(f"{row(case, variant, stage, bucket)['median_ms']:.3f}"
                                for variant in ("fixed64", "adaptive")))
    walls = " → ".join(f"{statistics.median(run['seconds'] for run in case['variants'][variant]['runs'] if not run['warmup']):.3f}"
                         for variant in ("fixed64", "adaptive"))
    short.append(f"| {edits:,} | {pairs[0]} | {pairs[1]} | {complete_rows(case, 'adaptive')[0]['final_pieces']:,} | {pairs[2]} | {walls} |")
short += [
    "",
    "Undo/redo read two/one bytes in two/one calls and discover no lines. At 100,000 edits, adaptive "
    "later undo p95/p99 is 1.814 ms and redo is 0.199 ms; first undo median/p95 is 1.933/1.963 ms. "
    "Their timing variation should not be attributed to refills, because they retain history and vector work.",
    "",
    "Fragmentation preparation is not separately timed. Whole-process elapsed includes fixture creation, "
    "open, preparation, navigation, editing, cleanup and roughly 100 ms polling granularity. Both "
    "100,000-edit variants still take about 7.4 seconds overall. Their initial navigation still needs "
    "roughly 103,000 read calls. A byte-count tree alone would not remove those reads.",
    "",
    "Every run retained 4,096 detailed lines, 128 sparse checkpoints and 100,352 allocated metadata bytes. "
    "Copy counters stayed zero for pieces, lines and history in all timed operations; this does not "
    "count every allocator copy or deep undo allocation. No whole-process memory claim follows.",
    "",
    "Revisit a balanced tree if real workloads accumulate materially more pieces, handler tails exceed "
    "an agreed latency budget, or profiles identify vector lookup/splicing as a material delay. Local "
    "warm-filesystem measurements with three independent runs do not establish native UI latency, "
    "cold-disk behavior, a maximum supported piece count, recovery costs or tree superiority. "
    "No production source files were modified and no failures/timeouts were omitted.",
    "",
    "Evidence: [results.json](results.json) contains all raw events and summaries; [provenance.json](provenance.json) "
    "contains identities and environment; `fixed64/` and `adaptive/` contain copied probes, original reports "
    "and every raw log. [details.md](details.md) has full first/later, tail and read tables. "
    "Run `python3 compare.py` to repeat using saved probes, or `python3 report.py` to regenerate documents.",
]
(root / "findings.md").write_text("\n".join(short) + "\n")
print(root / "findings.md")
