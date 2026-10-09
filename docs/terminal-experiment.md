# Embedded terminal integration

`:term-new [command]` opens the new integration in Pötyi's custom window. The
original `:term` stays available for comparison. Use Ctrl+P to enter the command,
or launch `potyi --term-new`. Ctrl+grave returns to the same terminal after
opening a document. **Editor**, or Escape from the command row, returns to the
document. Closing Pötyi ends its owned shell and command processes.

## Supported behaviour

- File browsing and quick opens: portable `ls`, `cd`, `pwd`, `touch`, `clear`,
  `help`, `exit`, `editor`, `edit` and `view`; dotfiles, parent navigation,
  column listings, Git status colours, exact stored paths, Unicode and spaces.
  Ctrl/Cmd-click opens in the other pane. Literal existing filenames take
  precedence over location suffixes. A standalone text filename opens in Pötyi
  without shadowing an installed command.
- Command row: Unicode editing, cursor movement, history with draft restoration,
  bounded paste, Tab/Shift+Tab completion, busy protection, Stop and Again.
  Failed or cancelled directory changes discard waiting commands.
- Output: one ordered transcript for listings, commands and completion, colour,
  word-aware wrapping, width reflow, scrolling, mouse/keyboard/Vim selection,
  selected Copy and retained Copy All. F6 switches command/output focus.
  Shift+Up starts output selection. In output focus, move onto an underlined
  item and press Enter (or keypad Enter) to open it; Ctrl/Cmd+Enter opens a file
  in the other split pane. Enter on ordinary text returns to command input.
  Linewise yanks keep linewise editor paste.
- Git: saved log/diff/show, clickable log hashes, commit detail, Again, and **Back**
  or Alt+Left. Back restores output, scroll, draft, history and selection without
  rerunning Git. Commit opening requires an idle pane. Other commands return to
  the base transcript first.
- File links: wrapped diagnostics with locations, quotes, spaces and Unicode.
  Relative links require completed commands with retained directory provenance.
  Helper directory changes veto them. Publication checks reject late Clear,
  eviction and directory notifications. Absolute links can work during a command.
  Browser paths and authenticated edit/view helpers use exact paths.
- Integration: document reuse, unsaved-document protection, split panes,
  protected file drops, folder navigation, and return to the same embedded pane.

Pötyi owns command completion; shell output does not need to look like a prompt.
Listings retain filesystem order, like the original asynchronous listing.

## Shells and intentional limits

POSIX platforms use private script files and a persistent PTY driver. Command
IDs, authenticated completion and process ownership protect dispatch and Stop.
Production commands run in fresh noninteractive shells, with EOF stdin, like
the original `:term`. Environment, directory and trap changes stay inside that
command; `exit` and `exec` leave the pane available. sh, bash, zsh and fish have
specialized helpers; other selected shells use the original `-c` command
contract through an internal sh dispatcher. Noninteractive startup files run
once per command; login and interactive profiles do not run. Stop kills the
owned command process groups and restarts the private driver asynchronously.

Windows managed commands use fresh cmd processes, closed stdin, hidden ordinary
pipes and Job objects owning process trees. Environment changes inside one
command do not persist to the next. Completion requires process exit and both
output streams ending. Unicode arguments and UTF-8 output have native handling.
Structured Git uses the same owner. Raw ConPTY remains a backend comparison API.

CMD receives a safe local-drive directory in ordinary `C:\...` form. Windows
canonical paths can include `\\?\`; passing that spelling directly to CMD can
make it fall back to the Windows directory. The conversion happens only at the
CMD boundary. UNC/device directories, paths of 260 or more UTF-16 units and
extended-only names are rejected before dispatch instead of running in another
folder. Browser/completion paths retain their canonical spelling; direct Git
execution is unchanged. Native Windows runtime verification remains pending.

Builds ship `potyi-term-helper` beside the app for the same target. It shares the
existing request protocol without loading graphics libraries. `tools/dev.py run`
builds it before starting Pötyi. A copied app without its helper uses the existing
app entry point as a fallback; packaging both enables the smaller helper.

This is a limited embedded terminal. Interactive stdin, full-screen terminal
programs, mouse reporting, terminal graphics, OSC clipboard/hyperlinks, modern
keyboard protocols and automatic font fallback remain outside scope. Normal
output uses the original text rules: ANSI sequences are stripped, tabs and
trailing spaces are preserved, and carriage returns separate lines. The bundled font can show
missing-glyph boxes. Completion uses the latest bounded listing rather than a
recursive search. Transcript search and pointer-drag auto-scroll are not claimed
original `:term` features.

Relative diagnostic provenance uses a launch/final-directory heuristic. It
cannot prove that an arbitrary program did not change directory internally and
return. Observed helper changes veto it; unfinished/evicted commands do not
qualify.

## Resource design

The permanent transcript is one fixed 8 MiB temporary disk ring: 7 MiB records
and a 1 MiB disk index, at most 65,536 records. This budget includes metadata,
so it retains less plaintext than the original 8 MiB text budget. Copy All
therefore operates on different retained byte counts after eviction. Git detail adds one separate
8 MiB ring only while open. Back transfers saved state ownership; it does not
copy the transcript or create a third layout store. Normal drop removes files;
a crash can leave temporary files.

Transcript I/O uses fixed 40 KiB write buffers and an 80 KiB read cache.
Layout retains valid checkpoints after ring eviction and keeps at most 1,024
sparse checkpoints and visible projections capped at
256 KiB each, including fragment-vector slack, text capacity and segment capacity.
Published projections are shared between layout and pane; repeated ready frames
reuse the same buffers and compatibility grid. A growing projection uses bounded
copy-on-write storage. Active and saved Git state together account for at most
2 MiB of projection capacities; this excludes object headers and the separately
bounded layout flow, grid, transcript and renderer caches. This is not a total
terminal-memory cap. At most 2,048 visible zero-column native
filename ink rectangles are kept for accurate clicks, without copying paths
or text. Normal output uses the pane's visible row
count; output/listings use up to 1,000 columns, matching the
original width limit. Unfinished output keeps at most 60 KiB of UTF-8 text and
four UTF-8 carry bytes; completed chunks go directly to the disk transcript.
Normal output bypasses the comparison VT grid; only the private completion
protocol is parsed there. The comparison grid is capped at 160 rows/256 columns. Long selection,
copy and file probes use bounded workers. Copy is limited to 8 MiB actual string
capacity and keeps the clipboard unchanged on failure. Diagnostic assembly is
limited to 64 KiB/256 rows and bounded file probes. Epoch/source checks reject
stale selection and links after Clear or ring reuse.

Browser queues hold eight batches, normally 16 KiB/64 rows each. One row is
limited to 64 KiB; completion to 4,096 paths/256 KiB; recursive browsing depth to
64. Git status collection is on demand and cancellable. Command text is capped
at 64 KiB and prompt history at 100 entries/256 KiB. PTY output queues hold
512 KiB plus a pending 16 KiB chunk. Parsing uses 1 KiB slices, a 128 KiB per-turn
ceiling and a 2 ms budget checked between slices. OSC strings are capped at
1 KiB; glyph textures at 8 MiB/512 entries.

Workers block when idle and coalesce wakes. There is no idle terminal animation
or recurring browser scan. Children have separate resource costs. Bounds alone
do not establish whole-application CPU/RAM parity.

The integration adds explicit size bounds beyond the original command field:
64 KiB per command, 256 KiB total prompt history, 256 listing path arguments,
and 64 recursive directory levels. These are additional resource limits, not
claims that the original terminal had the same bounds.

## Validation and comparison

Build Pötyi before managed helper tests:

```sh
cargo build --locked --bins
POTYI_TERM_TEST_CLIENT="$PWD/target/debug/potyi-term-helper" SDL_VIDEODRIVER=dummy cargo test --locked --bin potyi experimental_terminal -- --include-ignored --test-threads=1
```

The [backend harness](../tools/qa/terminal-backend/README.md) compiles production
modules without SDL. The dedicated workflow has native Linux x64, Windows x64,
Intel macOS and Apple Silicon macOS jobs. A prepared workflow or successful
cross compilation does not establish native runtime validation.

The [resource runner](../tools/qa/terminal-resources.py) uses one release test
binary and an explicit release helper, validated against a frozen build manifest,
for alternating old/new fresh process pairs. Identical phases cover idle,
small commands, three output streams crossing the cap, resize, selected copy and
retained copy-all. Raw samples, phase logs, hashes and architecture are saved.
App and descendant CPU/RSS are separate. The dummy SDL fixture does not measure
native window delivery or GPU memory; summed RSS can count shared pages twice.

Local x86_64 execution on this Apple Silicon laptop uses Rosetta. Native
Windows/Intel macOS/Linux x86_64 runtime gates require those machines. fish runtime checks
need a host with fish installed. Final local evidence is in
`docs/terminal-experiment/`.

The Enter activation follow-up is recorded separately in
[`keyboard-links/evidence.json`](terminal-experiment/keyboard-links/evidence.json):
six focused keyboard tests and all 215 embedded-terminal tests passed locally.
The earlier comparison bundle and resource measurements describe their frozen
build before this keyboard change.

Changes are generated against a captured baseline and reconciled with the
current shared project. The original terminal's 12 implementation/layout/cache/
keyboard files are byte-identical in the isolated integration and are not
written during shared integration. Other models' shared changes remain theirs.

The [2026-10-09 optimization report](terminal-experiment/performance-2026-10-09/report.md)
records a separate frozen build after the Enter follow-up. Output counting skips
discarded text/span allocation, visible frames share storage, safe ASCII spans
are copied in bulk, and each bounded parse slice reuses its transcript lock.
Production POSIX commands avoid a redundant syntax-check shell while preserving
fresh-shell `-c` semantics and exact command bytes; raw mode and fish retain their
existing preflight. Helper requests use one bounded framed write and TCP_NODELAY.

In that frozen build the new terminal used 26.2% less streaming app CPU and 29.6%
less resize app CPU than before those optimizations. Warm tiny commands improved
by 5.3%, to 134 ms; a 2.64-second first-command outlier remained. Streaming CPU was
still 1.51–1.63 times the original terminal, and settled RSS did not improve.
These Rosetta/dummy-SDL measurements do not establish native-window or other-OS
performance. See the report for raw evidence and the remaining targets.

The later [latency report](terminal-experiment/latency-2026-10-09.md) records the
small helper, fewer POSIX command-management processes and bounded ASCII output
counting. Warm commands improved from 125 to 85 ms, streaming app CPU fell 36%
(29% after normalization against the unchanged control), and resize elapsed time
fell 58%. Streaming CPU is now about 1.04–1.12 times the original per stream in
that run. Memory ranges overlap and tiny commands remain slower than original.
All 246 terminal app checks, 55 exact RGB comparisons and 21 clipboard comparisons
passed for the final merged snapshot. New evidence is stored in one compressed
archive; earlier evidence retains its original scope.

Optional helper diagnostics require `POTYI_TERM_TRACE_FILE` to name an absolute
file path. Newly created trace files are private, bounded to 64 KiB and use nonblocking locks;
they record static operation/stage labels and timing, without command text,
paths or authentication tokens. Tracing is off by default and was off during
the acceptance measurements. Helper-entry timing excludes executable loading.
