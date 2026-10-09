# Pötyi scaling plan

Updated: 9 October 2026.

Improve sustained editing of large files and long sessions while preserving
file-backed text storage, lazy opening, predictable memory use, and quiet idle
behaviour. Deliver the stages below separately and compare each with the same
baseline workloads. The three correctness fixes and their regression tests,
the measurement harness, prefix preservation, shared document ownership, and
bounded line index are implemented. Stage 5 has been evaluated: retain the vector for this delivery, using the
measured refill improvement and documenting its remaining linear costs.

## Implementation progress

See the [initial comparison](testing/scaling-2026-10-08/comparison.md) and
[Stages 3–5 results](testing/scaling-2026-10-08/stages-3-5.md) for raw
samples, source identity, reproduction commands and limitations.

The [9 October first-navigation improvement](testing/distant-navigation-2026-10-09/report.md)
reduces the near-end jump in the 1 GB short-line fixture from 3.64 s to 0.81 s
using bulk UTF-8 validation and newline searches. Index and scan-buffer budgets
remain unchanged; paired text-only measurements reduce whole-process CPU time
by 64% with effectively unchanged peak RSS. First navigation still scans the
prefix on demand, with no background indexing.

- Stage 1: added production editor/undo/synchronization probes, separate redraw
  timings, read/copy counters and bounded workloads. Recorded all 23 baseline
  workloads, including seven timeouts. RSS could not be sampled in this sandbox.
- Stage 2: retained valid line-index prefixes, reused a bounded scan buffer,
  preserved earlier viewport checkpoints on both panes, and changed cached-line
  cursor lookups to binary search. All 23 final workloads completed within the
  same eight-second process limit. The base 1 MiB middle insert reads 65,605 bytes
  instead of 536,936,517 and discovers one line instead of 8,193.
- Stage 3: panes share one text layout, line index, backing stores, recovery
  journal, revision, file state and undo/redo history. Each pane keeps its cursors
  and input mode. Synchronization and switching no longer copy document records.
  In the 1,000-history-entry case, synchronization copies zero records and takes
  about 0.0023 ms, versus about 0.216 ms after Stage 2.
- Stage 4: cap detailed records at 4,096 and sparse checkpoints at 2,048. On
  this 64-bit build their retained vector allocations total at most 128 KiB.
  Reuse a scan buffer capped at 64 KiB. After edits refill 256 bytes first and
  grow only if scanning continues, reducing unrelated reads in fragmented text.
  The existing bounded viewport checkpoints remain separate; these limits are
  not a whole-process memory budget.
- Stage 5: a serial independent comparison of fixed/adaptive refills completed
  all 24 processes at 1,000, 10,000 and 100,000 dispersed edits. At about 200,000
  pieces, later inserts dropped from 7.285 ms to 0.629 ms; positional reads fell
  from 11,433 to 65. Retain the vector for the tested range. Lookup/splicing and
  initial fragmented scanning still grow with piece count.
- Verification: 619 regular tests passed, including the additional saturated-page
  regression protecting evicted EOF details. All 23 final release workloads and both
  isolated shared-pane checks passed. The production compile check and six runner
  regressions passed. Independent
  eviction, Unicode/CRLF, page-clamping, snapshot and shared-pane checks passed.
- Coordination: the event refactor and other LLM changes were preserved.
  Application adaptations use shared file-state accessors and remove the old
  history copy on pane switching. The other chat is independently reviewing the
  bounded index in an isolated copy.

## Completed fixes and regression coverage

Eight regular regression tests were added:

| Area | Added tests | Source |
| --- | --- | --- |
| Save permissions | `save_preserves_executable_and_private_permissions`; `save_as_overwrite_uses_destination_permissions_and_new_files_stay_exclusive` | [Save tests](../src/piece_table/save_tests.rs) |
| Symbolic links | `save_follows_relative_symlink_chains_and_preserves_target_permissions`; `save_does_not_replace_dangling_links_or_overwrite_links_for_new_files` | [Save tests](../src/piece_table/save_tests.rs) |
| Shared cursors and selections | `shared_views_preserve_middle_selection_across_compound_edits_and_grouped_undo`; `shared_views_preserve_middle_selection_across_replace_all_and_snapshot_undo` | [Editor tests](../src/editor/shared_view_tests.rs) |
| Synchronization history lifetime | `view_history_is_released_after_sync_or_dropping_a_stale_view`; `several_views_keep_edits_until_the_oldest_view_catches_up` | [Shared-view tests](../src/piece_table/shared_view.rs) |

The permission tests check that executable `755` and private `600` modes survive
saving. The link tests check relative link chains, repeated saves, preserved
links with updated target contents, dangling-link failures, and exclusive creation. The cursor tests
check Unicode selections through edits at both ends of a document, replace-all,
grouped undo, and snapshot undo/redo.

Shared views now map positions through actual edits. Pending maps are released
when all live views catch up or stale views close. Ordinary single-pane edits
do not allocate or lock a synchronization log. Snapshot maps are prepared in
both directions so undo/redo does not rebuild them.

### Initial verification of the three fixes

Before the scaling changes, the local macOS run passed **605 regular tests**, with **66 optional tests
ignored**. Two existing headless integration tests were also run individually
and passed:

- `app::input::tests::terminal_clicks_open_shared_views_and_edit_undo_save_from_either_pane`
- `app::input::tests::shared_views_keep_vim_insert_groups_separate_when_switching_panes`

The Unix permission and link tests have not been run on Linux, and native
Windows behaviour remains unverified. Headless checks do not establish native
keyboard, mouse, or clipboard delivery.

Run the regular suite from the repository root:

```sh
cargo test --locked --bin potyi -- --test-threads=1
```

Run the two headless checks in separate processes:

```sh
SDL_VIDEODRIVER=dummy cargo test --locked --bin potyi app::input::tests::terminal_clicks_open_shared_views_and_edit_undo_save_from_either_pane -- --ignored --exact --test-threads=1
SDL_VIDEODRIVER=dummy cargo test --locked --bin potyi app::input::tests::shared_views_keep_vim_insert_groups_separate_when_switching_panes -- --ignored --exact --test-threads=1
```

## Stage 1: Establish sustained-editing measurements

Extend the [existing benchmark suite](../tools/benchmarks/README.md). Its current
large-file opening probe and raw piece-table edits do not establish full editor
typing performance deep inside a document.

Measure these workloads using the production editing paths:

- Files of 1 MiB, 100 MiB, and 1 GB, with many short lines and separate long-line
  fixtures. Record sizes and line counts explicitly.
- Cursor movement followed by typing at the beginning, middle, and near the end.
- Repeated edits after warming the line index, with undo/redo included.
- One pane and two panes viewing the same document, with short and long histories.
- Edits spread across many locations to accumulate fragmented pieces.
- Recovery off and on, reported separately to distinguish indexing and copying
  costs from journal synchronization.

Record edit-handler and redraw times separately, median and tail latency,
process memory, bytes read, lines rescanned, piece count, and metadata copied.
Keep fixture creation and initial navigation outside repeated-typing timers;
report first navigation and first edit separately. Use bounded run durations
and report unfinished workloads instead of silently dropping them.

**Completion criteria:** retain raw samples, environment and source identity,
and a reproducible baseline. The local bounded baseline is now recorded in the
comparison above. Memory measurements remain unavailable in this sandbox;
collect them on an environment allowing process sampling before setting a
memory budget. Performance targets should follow these results.

## Stage 2: Preserve valid line-index work

Primary code: [piece table](../src/piece_table.rs) and
[viewport checkpoints](../src/piece_table/line_view.rs).

Originally each mutation cleared the entire line index. Start with the smallest
correct change: retain cached lines before the earliest affected line and
invalidate the affected line and following entries. Rebuild later information
only when navigation or rendering requests it.

Determine the affected line using the pre-edit index. Account for insertion at
line boundaries, newline deletion that joins lines, CRLF, and edits at EOF.
Compound operations should invalidate from their earliest changed location.
Snapshot restoration may initially retain conservative invalidation.

Reuse unread bytes from the scanning buffer across short-line discovery;
avoid repeatedly reading a large overlapping block for each short line.
Preserve unaffected viewport checkpoints where their coordinates remain valid.
Use binary search over ordered cached line records for cursor and line-boundary
lookup so retaining a large prefix does not leave a linear metadata walk.

**Completion criteria:** deterministic read-count checks show that repeated
typing deep in an indexed file no longer rescans its unchanged prefix. Unicode,
CRLF, newline edits, selection, navigation, and undo/redo tests must pass. The
same sustained-editing benchmarks must show the practical improvement.

## Stage 3: Share one document across panes — implemented

Primary code: [editor](../src/editor.rs),
[shared views](../src/piece_table/shared_view.rs), and
[application navigation](../src/app/navigation.rs).

Move document-wide state into one owner: piece layout, line index, backing
stores, revision, path, dirty/read-only state, recovery, and undo/redo history.
Each pane should reference that document and retain its own cursor, selection,
scroll position, and keyboard-mode state.

Keep UI document ownership on the application thread. Language-server and
formatter workers should continue receiving explicit snapshots or file-backed
inputs rather than live mutable UI state. Avoid adding document locks to every
UI access merely to share state between panes.

Reuse the edit-position maps introduced by the cursor fix. Remove full history,
piece-layout, and line-index copies from pane synchronization and switching.
Keep position maps only as long as a live view needs them.

**Completion criteria:** two views contain one document history and piece layout;
typing and switching panes do not clone them. Verify editing and undo from
either pane, Vim grouping, Save As, workspace refactoring, and closing one view
while the other remains editable. Memory and synchronization cost should stop
doubling with document metadata and history size.

## Stage 4: Bound line-index memory — implemented

After the prefix-preservation change is measured, replace unrestricted dense
line caching with a budgeted detailed cache and compact sparse checkpoints.
Evict detailed entries and compact checkpoints as density grows. Reconstruct
line and Unicode-column information near the requested position using bounded
read buffers.

Account separately for line metadata, checkpoint metadata, and cached text.
Do not introduce full-file background scanning. First navigation to an
unindexed distant region will still require scanning; measure and document
that cost rather than claiming every jump is constant-time.

The chosen budget stores one contiguous recent window of 4,096 exact line
records and at most 2,048 ordered restart points. Checkpoint stride starts at
64 lines and doubles when the checkpoint vector fills. Detailed records evict
from the front; logical line numbers and the known prefix/count survive
eviction. Navigation reloads missing details from the nearest checkpoint.
Edits use exact resident coordinates or conservatively invalidate from an
earlier checkpoint. Snapshot swaps invalidate the index. No timer, worker,
idle scan, or document text copy was introduced.

The cap trades memory for navigation work. Revisiting evicted regions rescans
part of the file, and edits before a distant pane can require rescanning its
invalidated suffix. First visits to unindexed regions still scan the prefix.

**Completion criteria:** visiting more lines respects the chosen index-memory
budget. Cache eviction and reconstruction preserve exact line, character, and
visual-column results. Repeated navigation remains practical on the baseline
fixtures, without adding idle work.

## Stage 5: Address fragmented pieces if justified — evaluated, vector retained

Use Stage 1's fragmentation measurements to decide whether replacing the flat
piece vector is necessary. If piece lookup, splicing, and metadata copying
remain material costs after the earlier stages, introduce a balanced structure
with chunked leaves and subtree byte counts.

Keep text in the original file and append-only edit store. Share unchanged
structure across snapshots where practical. Track newline aggregates only for
indexed regions so creating the structure does not force eager file scanning.
Preserve efficient streaming traversal and bulk replacement.

The [controlled refill comparison](testing/scaling-2026-10-08/fragment-review/findings.md)
compares immutable Stage 4 executables in alternating serial runs. At 100,000
dispersed insertions (200,002 final pieces), later insert median/p95 is
0.629/0.659 ms with adaptive refill, versus 7.285/7.557 ms with fixed refill.
Backspace median is 1.184 ms; its p95 is 2.228 ms. A tree was not implemented
because the cheaper refill change removed the dominant measured edit cost.

This decision is limited to the tested workload. The vector still does linear
lookup/splicing, setup takes about 7.4 seconds for the heaviest fixture, and
initial fragmented navigation costs roughly 65 ms with about 103,000 reads.
Revisit a balanced structure if supported workloads grow beyond this range,
handler tails exceed an agreed latency budget, or profiles attribute production
delays to vector lookup/splicing. A byte-count tree alone would not remove the
fragmented scan's individual file reads.

**Completion criteria for a future tree:** edits and position lookup scale with tree depth and
affected leaves instead of total piece count. Compare with the vector on both
small documents and heavily fragmented sessions; retain the vector if the
measured benefit does not justify the added complexity.

## Added ownership and bounded-index regressions

In addition to the original bug fixes, the new tests verify:

- One shared layout/history allocation and zero copied records through duplicate,
  edit, synchronization, undo and redo with 1,000 existing history entries.
- Immediate shared path/dirty/read-only state and editing after one view closes.
- Refreshing a stale pane's cursor before recording its edit and history.
- Eviction and multiple checkpoint compactions over 300,000 Unicode/CRLF rows,
  allocated metadata bounds, visual columns, terminated/unterminated EOF, and
  saturated page movement when terminal detail has been evicted.
- Edits outside retained detail, sparse-coordinate invalidation, deletion,
  distant shared panes, undo/redo and snapshot restoration.
- Small line edits in 10,000-fragment fixtures avoid thousands of unrelated
  positional reads.

Sources: [shared editor tests](../src/editor/shared_view_tests.rs),
[line-index tests](../src/piece_table/line_index_tests.rs), and the existing
[workspace tests](../src/workspace_edit/tests.rs).

## Delivery order

Implement measurements, then line-index preservation, then shared document
ownership. Use the resulting evidence to set memory budgets and decide on the
piece structure. Each stage should be independently reviewable, pass the
relevant correctness and headless checks, and retain before/after evidence.
Native platform checks remain necessary for platform-sensitive file behaviour.
