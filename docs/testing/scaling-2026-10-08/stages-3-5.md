# Scaling stages 3–5: shared ownership, bounded indexing, fragmentation

Recorded 8 October 2026. The [scaling plan](../../scaling-plan.md) is updated.
Stages 3 and 4 are implemented; Stage 5 was evaluated and the piece vector is
retained for this delivery. Other LLM changes in the shared checkout were
preserved. The independent reviewer used an isolated copy, and the fragmentation
helper used saved immutable executables with separate output paths.

## Result

Panes now share text layout, backing stores, revision, recovery, line index,
file path, dirty/read-only state, and undo/redo history. Cursors, selections and
input-mode state remain local to each pane. Ownership stays on the UI thread
through reference-counted borrowing; no document mutex was introduced. Worker
inputs continue using explicit file-backed data/snapshots. Switching panes and
synchronizing their cursors no longer clone document layout or history.

The line index retains at most 4,096 detailed records and 2,048 sparse restart
points. Checkpoint stride starts at 64 lines and doubles as points accumulate.
The detailed window evicts old records; the discovered logical prefix and exact
known count survive eviction. Missing detail is reconstructed lazily. Edits
outside detail conservatively invalidate from an earlier known point. There is
no full-file background scan or idle work.

On this 64-bit build, retained detailed/checkpoint vector capacity totals at most
**128 KiB**, plus one reusable scan buffer capped at **64 KiB**. Existing bounded
viewport checkpoints, pieces, history, temporary caller reads and allocator
bookkeeping are separate. This is an index allocation budget, not measured RSS
or a whole-process memory budget.

After an edit, scanning starts with a 256-byte refill and grows geometrically
to 64 KiB only when needed. The initial cold scan still starts at 64 KiB.
Refills reuse the allocation and discard partial state on I/O errors. This
avoids reading thousands of unrelated fragments to rediscover one short line.

## Comparable full-matrix measurements

Both the Stage 2 and final runs use release x86_64 executables on the same macOS
host, an 800×600 headless SDL renderer, freshly written warm fixtures, one
discarded warmup plus three measured processes, four edit cycles and the same
**eight-second whole-process limit**. All 23 final workloads completed all runs.
Later-edit medians below have nine correlated samples; navigation has three.

| Workload / operation | Stage 2 median, ms | Final median, ms | Stage 2 / final positional read bytes |
| --- | ---: | ---: | ---: |
| 1 MiB short lines, middle insert | 0.017333 | 0.014750 | 65,605 / 325 |
| 1 MiB short lines, middle insert after 1,000 dispersed edits | 0.098333 | 0.018875 | 65,605 / 325 |
| Two panes, 1,000 history entries, cursor synchronization | 0.216334 | 0.002208 | 2 / 2 |
| 1 GB short lines, 99% insert | 0.017125 | 0.012375 | 65,605 / 325 |
| First navigation to 99% of 1 GB short-line fixture | 3,774.299 | 3,955.098 | 990,052,354 / 990,052,354 |

The shared-history synchronization case previously copied four pieces, 8,193
line records and 1,005 top-level history entries per later insert. It now
copies **zero** such records. Regression tests also establish common storage
and history allocations rather than relying only on counters.

Near 99% of the 1 GB short-line fixture, the final probe knows a prefix of
15,468,751 lines while retaining only 4,096 detailed records, 1,888 checkpoints
and 131,072 bytes of line-index vector allocation. The old detailed-record
payload alone for that prefix was about 354 MiB. The new budget trades memory
for reconstruction work; first distant navigation still scans the prefix.

Insertion, synchronization and redraw are separate measurements. These are
not native input-to-display latency guarantees. Recovery durability policy is
unchanged and recovery-enabled timings remain separate in the raw report.

## Fragmentation decision

The helper compared the pre/post-refill Stage 4 executables serially, alternating
variant order. Both use shared ownership and the bounded index. Rendering,
recovery, a second pane and RSS sampling were disabled. Each 1 MiB short-line
case accumulated 1,000, 10,000 or 100,000 dispersed insertions. Six warmup and
18 measured processes all completed under a 60-second process limit.

| Dispersed edits | Fixed-refill later insert median / p95, ms | Adaptive median / p95, ms | Fixed / adaptive positional read calls |
| ---: | ---: | ---: | ---: |
| 1,000 | 0.098 / 0.142 | 0.017 / 0.021 | 135 / 9 |
| 10,000 | 0.836 / 1.074 | 0.077 / 0.088 | 1,249 / 15 |
| 100,000 | 7.285 / 7.557 | 0.629 / 0.659 | 11,433 / 65 |

At 100,000 edits, the final layout contains 200,002 pieces. Adaptive insert reads
331 bytes instead of 65,611, and later backspace median/p95 is 1.184/2.228 ms.
This cheaper refill change removed the dominant observed edit cost, so a piece
tree was not implemented.

The vector still does linear lookup and splicing. Initial navigation in the
heaviest fragmented fixture still takes about 65 ms and 103,000 positional reads;
whole-process setup/editing takes about 7.4 seconds. Undo/redo continue to include
vector/history work, and the refill change does not establish an undo speedup.
Revisit a tree for larger supported piece counts, exceeded agreed handler-tail
budgets, or profiles attributing production delays to lookup/splicing. A
byte-count tree alone would not remove the fragmented scan's individual reads.

See the helper's [findings](fragment-review/findings.md),
[full timing tables](fragment-review/details.md),
[raw samples](fragment-review/results.json), and
[verified executable provenance](fragment-review/provenance.json).

## Independent review

Three supplemental production-API tests passed: saturated page movement after
EOF detail eviction, edits/deletion in evicted Unicode/CRLF regions with snapshot
undo/redo checked against a fresh document, and distant shared-pane history.
The permanent suite now includes the saturated-page regression.

A separate 64 MiB / 1,048,577-line probe alternated two distant 40-row windows.
Median reconstruction time was 0.177 ms near checkpoints, 0.338 ms mid-gap and
0.468 ms near gap ends, with 88, 590 and 1,102 line discoveries respectively.
The maximum observed time was 0.490 ms. Retained index allocation stayed at
131,072 bytes. This measures document line-window reconstruction before the
adaptive-refill change, not native rendering or edit invalidation of a distant
suffix. Editing before a distant pane can still require rescanning that suffix.

Evidence: [supplemental tests](independent-review/line_budget_tests.rs),
[initial test log](independent-review/tests.log),
[final-buffer rerun](independent-review/tests-final-buffer.log),
[distant-pane samples](independent-review/distant-panes.json) and
[raw output](independent-review/distant-panes.log).

## Verification and provenance

The final checkout passed **619 regular tests**, with **70 optional tests ignored**.
The non-test production build also passed its compile check. Two ignored
shared-pane save/undo and Vim-grouping checks passed separately on
the saved final release executable. Six runner regressions passed, including
fallback to the owned child when a sandbox denies process-group termination.

- [Final regular suite](final/core-delivery.log), [production compile check](final/production-check.log).
- [Shared-pane save/undo](final/shared-save.log), [Vim grouping](final/shared-vim.log).
- [Runner tests](final/runner-tests.log).
- [Final full-matrix summary](final/summary.md), [raw samples](final/results.json).
- [Shared-ownership intermediate report](shared/summary.md), [raw samples](shared/results.json).
- [Bounded-index report before adaptive refills](bounded/summary.md), [raw samples](bounded/results.json).

Every benchmark build recorded equal before/after source hashes and a saved
executable hash. The final production code was unchanged after its benchmark
build; the saturated-page regression was added afterward and passed in the
final regular suite. The executable copies remain in ignored local
`target/qa/scaling-{shared,bounded,final}-20261008/` folders. Raw process and
build logs are archived beside their JSON reports; binaries are not committed.
The helper's scripts, source identities and raw logs are archived under
`fragment-review/`; its runnable executable copies remain in the original
isolated directory recorded by `provenance.json`.

The shared-ownership intermediate full-matrix run was interrupted when the
sandbox denied a timeout group signal during 1 GB preparation; its five variants
were finished against the same immutable binary, and missing intermediate cases
are documented. The pre-refill bounded-index run had one near-EOF 1 GB timeout.
Neither incomplete run was silently dropped. The final full run completed all
23 cases with the same original limit.

RSS could not be sampled in this sandbox, so no process-memory reduction is
claimed. Timings are local warm-filesystem observations with small sample counts;
concurrent work can affect them. Windows, Linux, native arm64 and desktop input/
presentation checks remain unverified by these runs.

## Reproduction

```sh
cargo test --offline --locked --bin potyi -- --test-threads=1
python3 tools/benchmarks/scaling.py --suite full --samples 3 --warmups 1 --iterations 4 --timeout 8 --no-memory
python3 tools/benchmarks/scaling.py --suite fragments --samples 3 --warmups 1 --iterations 4 --timeout 60 --no-render --no-memory
python3 -m unittest discover -s tools/benchmarks -p test_scaling.py -v
```

Use the [runner methodology](../../../tools/benchmarks/SCALING.md) for counter
semantics, source provenance and headless measurement limits. Keep the Stage 1–2
[comparison](comparison.md) as the original baseline record.
