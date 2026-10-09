# Scaling progress: measurements and line-index reuse

Recorded: 8 October 2026. Stages 1 and 2 of the
[scaling plan](../../scaling-plan.md) are implemented in the shared checkout.
Application event-handling files were left untouched.

## Changes

- Added a sustained-editing probe and bounded runner using production editor
  insertion, undo, redo, backspace and shared-view synchronization. Drawing is
  timed separately. The probe checks document length, inserted bytes, cursor
  position, shared revisions and recovery errors.
- Retained exact cached lines before the earliest affected line. Mutations
  invalidate that line and the following entries; snapshot restoration remains
  conservative. Newline joins, CRLF and EOF are covered by regression tests.
- Reused unread scan bytes between lazy line discoveries, bounded to one 64 KiB
  block per view. Reading ahead does not eagerly validate later lines.
- Preserved unchanged earlier viewport checkpoints on the edited view and on
  synchronized peer views, using the actual edit maps.
- Replaced linear cached-line searches for cursor coordinates and current line
  boundaries with binary search.

## Recorded comparison

Both runs use the release profile, headless 800×600 SDL renderer, freshly written
warm fixtures, one discarded warmup and three measured processes per case.
Each process performs four edit/undo/redo/backspace cycles and has an **eight-second
limit including fixture creation and setup**. Later-edit medians below contain
nine samples; first edits and setup timings remain separate in the raw reports.

The Rust executable targets **x86_64** on the local macOS machine. These results
do not establish native arm64, Windows or Linux behaviour. Source hashes matched
before and after each build, and the final source hash also matched the checkout
after verification. Separate saved executables prevented other builds replacing
the probe during measurement.

| Workload | Insert median before, ms | Insert median after, ms | File bytes read per insert before | File bytes read per insert after |
| --- | ---: | ---: | ---: | ---: |
| 1 MiB, 64-byte lines, middle, one pane | 39.326 | 0.017 | 536,936,517 | 65,605 |
| 1 MiB, 64-byte lines, 99%, one pane | 72.556 | 0.017 | 1,039,365,665 | 10,566 |
| 100 MiB, 64-byte lines, middle | Unfinished process | 0.019 | — | 65,605 |
| 1 GB, 64-byte lines, middle | Unfinished process | 0.016 | — | 65,605 |
| 1 MiB, middle, two shared panes | 32.084 | 0.018 | 536,936,517 | 65,605 |
| 1 MiB, middle, two panes, 1,000 history entries | 32.177 | 0.017 | 536,936,517 | 65,605 |
| 1 MiB, middle, 1,000 dispersed insertions | 702.792 | 0.098 | 536,936,517 | 65,605 |
| 1 MiB, middle, recovery enabled | 49.243 | 8.591 | 536,936,517 | 65,605 |

Synchronization and redraw are separate operations, so insertion time alone is
not complete UI latency. Detailed counter tables are retained in the raw reports.

The baseline completed **16 of 23 workloads**; seven reached the process limit
and retain their partial output. The final run completed **all 23 workloads**,
including the recovery/history/fragmentation combination. An unfinished process
does not imply that one insert alone took eight seconds.

For the base 1 MiB middle case, later inserts discover **one line**, down from
**8,193**. The regular deep-edit regression test checks that an entire
edit/sync/undo/sync/redo/sync/backspace/sync cycle discovers at most four lines
while preserving a 12,288-line prefix. These deterministic checks establish
avoided work independently of variable wall-clock timings.

Recovery-enabled insertions remain around 8.6 ms in this run. Recovery durability
policy is unchanged. Shared-view synchronization still copies document metadata:
with 1,000 existing history entries, the later insert synchronization median
copies **4 piece records, 8,193 line records and 1,005 history entries**. Stage 3
will remove those copies. The dense line index still grows with visited lines;
Stage 4 will address its memory budget.

## Evidence and reproduction

- [Baseline summary](baseline/summary.md) and [raw baseline samples](baseline/results.json).
- [Final summary](after/summary.md) and [raw final samples](after/results.json).
- Raw process logs and build logs are alongside each report, including timed-out
  baseline cases. The measured executables remain in the local ignored
  `target/qa/scaling-baseline-20261008/` and
  `target/qa/scaling-index-final-20261008/` folders.
- [Runner methodology and limitations](../../../tools/benchmarks/SCALING.md).

```sh
python3 tools/benchmarks/scaling.py --suite full --samples 3 --warmups 1 --iterations 4 --timeout 8
python3 -m unittest discover -s tools/benchmarks -p test_scaling.py -v
```

Latency can vary with concurrent work on the machine. p95/p99 values have small,
correlated sample counts. Headless drawing excludes desktop presentation and
event dispatch. RSS sampling was prohibited by the local sandbox, so both
reports mark memory unavailable. Read counters measure bytes returned by the
piece table's positional file reads, including rereads, rather than physical
disk traffic. Copy counters are top-level records, not deep allocation sizes.

## Verification

The final saved release test executable passed **612 regular tests**, with
**69 optional tests ignored**. Two ignored headless integration checks passed
in separate processes:

- `app::input::tests::terminal_clicks_open_shared_views_and_edit_undo_save_from_either_pane`
- `app::input::tests::shared_views_keep_vim_insert_groups_separate_when_switching_panes`

The five Python runner regressions also passed. Verification logs are saved in
this folder. New regular coverage includes Unicode/newline/CRLF/EOF edits,
bounded scan reuse and lazy validation, deep editing with history and shared
views, snapshot invalidation, and viewport checkpoint preservation on both panes.
The existing cross-block UTF-8, selection, navigation and undo tests also pass.

The next stage is shared document ownership. It should remove metadata/history
copies while preserving pane-local cursor, selection and keyboard state. The
application integration should be coordinated with the other LLM's event refactor.

## Later stages

Shared document ownership, bounded line metadata and fragmentation/refill
results are recorded in [Stages 3–5](stages-3-5.md). The results above remain the
Stage 1–2 comparison.
