# Sustained editing measurements

The [8 October 2026 comparison](../../docs/testing/scaling-2026-10-08/comparison.md)
retains the baseline and each completed optimization result.

Run from the repository root on macOS or Linux:

```sh
python3 tools/benchmarks/scaling.py
```

The runner builds an optimized release test executable offline using the locked
dependencies. It saves a private copy of that executable in a new output folder
so another build in the shared checkout cannot replace it during measurement.
Source hashes before and after the build, the executable hash, compiler, profile,
platform, Git commit and dirty state are retained. If source files change during
the build, the report explicitly marks source provenance inconclusive. Repeat
the build when the checkout is stable before using that run for comparisons.

## Workloads

The full suite covers real, non-sparse 1 MiB, 100 MiB and 1 GB
(1,000,000,000 bytes) files. Each has either 64-byte lines or 1 MiB lines,
with editing positions at 0%, 50% and 99%. Newlines count toward line size;
the last line may be shorter or empty. The JSON records exact file sizes and
line counts. Fixtures are freshly written and warm in the filesystem cache.

Five additional 1 MiB short-line cases isolate two shared panes, two panes with
1,000 existing history entries, 1,000 dispersed insertions, recovery enabled,
and those features combined. History setup replaces the first character without
growing the document. Fragmentation setup uses raw insertions outside timers;
measured operations use the production `Editor` methods. The separate
`fragments` suite extends this to 1,000, 10,000 and 100,000 dispersed insertions;
use a longer process limit for its setup. It does not change the full matrix.

Each process reports opening, first navigation, view duplication and initial
drawing, then repeated insertion, pane synchronization, undo, redo, backspace
and redraw. It checks text length, inserted bytes, cursor position, shared
revisions and recovery errors. Edit operations and synchronization are separate;
sum the relevant operations when assessing a complete shared-pane interaction.
This does not time the application event dispatcher or language servers.

Redraw uses an 800×600 hidden headless SDL window. Both panes are drawn where
applicable. Native desktop presentation and GPU behaviour are outside the probe.

## Bounded execution and output

Each case gets one discarded warmup and three measured process runs by default.
Each process has a 30-second limit including fixture creation and setup. A failed
or timed-out run stops further repetitions of that case, retains its partial
events and raw log, and leaves other cases running. Only complete measured runs
contribute to summaries. The runner kills only the process group it started, falling back to its own
child if a sandbox denies group signals. The probe has no descendants. It
removes its temporary fixture even after a timeout.

Results go to a fresh `target/qa/scaling-<UTC timestamp>/` folder:

- `results.json`: settings, provenance, every raw event and run status, summaries.
- `summary.md`: readable case status and timing/counter tables.
- `case*-run*.log`: raw probe output, including errors and partial runs.
- `build.jsonl`, `build.log`, `probe`: build evidence and the measured executable.

First-iteration timings are separate from later iterations. Medians and nearest
rank p95/p99 values are reported with sample counts; these small, correlated
samples do not establish production tail guarantees. There are no timing
assertions. Failed correctness checks make the runner exit unsuccessfully;
timeouts remain reported measurements, including their last completed phase.

## Counters and memory

Counters are compiled only into test builds and are active only during the
measured operation. They record successful positional reads in the piece table,
bytes returned (including rereads), line-discovery calls, and top-level piece,
line and history records copied at view-cloning sites. Discovery calls include
reconstructed evicted entries and EOF handling. Copy counts exclude deep undo
allocations and are not byte-accurate memory accounting. Recovery writes are
included in latency but excluded from read counters.

The runner samples process RSS about every 100 ms across fixture creation, setup
and editing. The reported maximum is a sampled maximum, not true peak memory or
an allocation budget. If `ps` is unavailable or prohibited, the report retains
that limitation and continues without inventing a memory value.

Completed probes also report `line_index_metadata`: retained detailed records,
sparse checkpoints, and their allocated capacity in bytes. These are
deterministic index measurements, separate from RSS, the scan buffer, viewport
checkpoints, piece layout and history allocations. `final_cached_lines` continues
to mean the discovered logical prefix, including evicted detailed records.

## Useful commands

```sh
# Faster initial coverage; 1 MiB base fixtures plus the five variants.
python3 tools/benchmarks/scaling.py --suite quick

# Restrict to a specific matrix dimension.
python3 tools/benchmarks/scaling.py --only line64-pos50 --timeout 10

# Measure editing without redraw or process-memory sampling.
python3 tools/benchmarks/scaling.py --no-render --no-memory

# Reuse the optimized development build for harness checks.
# Report and compare this profile separately from release results.
python3 tools/benchmarks/scaling.py --suite quick --profile dev

# Heavy fragmentation, without rendering.
python3 tools/benchmarks/scaling.py --suite fragments --timeout 60 --no-render --no-memory

# Runner regressions: timeout output, failed samples and sample accounting.
python3 -m unittest discover -s tools/benchmarks -p test_scaling.py -v
```

Use identical settings, machine, profile, compiler, display driver and source
provenance for before/after comparisons. Concurrent compilation or other work
can distort latency and memory; deterministic read/copy counts still identify
algorithmic work. Keep both reports and all incomplete cases.
