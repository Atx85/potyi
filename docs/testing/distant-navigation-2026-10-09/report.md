# Faster first navigation without larger text caches

Recorded 9 October 2026. The first jump to byte 990,000,000 in the warm
1,000,000,000-byte, 64-byte-line fixture fell from **3,640.485 ms to
812.876 ms**, a **4.48× improvement** in the matched headless editing probe.
Opening remains lazy, with no background indexing, new worker or idle timer.

## Implementation

`Document::discover_next_line` searches for newlines using `memchr`, validates
UTF-8 in borrowed line/block slices, and counts Unicode characters in bulk.
Previously it classified and validated each character separately. `memchr`
was already in the locked dependency graph; it is now a direct dependency.

The same scan buffer and line index are reused. Validation stops at the
requested line, so malformed bytes in a later line do not prevent reading an
earlier valid one. A character crossing a refill boundary still uses the
bounded character reader. CRLF at a buffer boundary retains its existing line
and column behavior. No document-sized string, carry allocation, eager index
or persistent cache was added.

## Matched measurements

Both executables use release x86_64 builds on the same ARM macOS host, hidden
800×600 SDL, freshly written warm files, one discarded warmup and three measured
processes, with four edit/undo/redo/backspace cycles. The configuration and
navigation counters are identical before and after.

| First near-end navigation | Before | After |
| --- | ---: | ---: |
| Median wall time | 3,640.485 ms | 812.876 ms |
| Positional bytes read | 990,052,354 | 990,052,354 |
| Positional read calls | 15,109 | 15,109 |
| Discovered line entries | 15,468,751 | 15,468,751 |
| Retained detailed records | 4,096 | 4,096 |
| Retained sparse checkpoints | 1,888 | 1,888 |
| Line-index vector capacity | 131,072 bytes | 131,072 bytes |
| Scan buffer limit | 65,536 bytes | 65,536 bytes |

The final full matrix completed all **23 workloads / 92 processes**, including
large and long-line files, shared panes, history, fragmentation and recovery.
Read/copy counters and correctness assertions remain available in the raw
reports. Microsecond edit timings vary; this change does not claim an undo,
redo or rendering speedup.

Evidence: [before](before/results.json), [after](after/results.json),
[full final matrix](after/summary.md).

## CPU and memory

`compare_resources.py` alternates frozen executable order, discards one warmup
pair and measures three pairs with macOS `/usr/bin/time -l`. No builds or other
test runs were active during measurement. CPU and peak RSS cover the entire
test process, including fixture creation and four edit cycles, rather than only
the timed navigation handler.

| Whole-process resource median | Before | After |
| --- | ---: | ---: |
| CPU time, with headless drawing | 4.55 s | 1.68 s |
| CPU time, without drawing | 4.51 s | 1.61 s |
| Peak RSS, without drawing | 7.05 MiB | 7.03 MiB |
| Peak RSS, with headless drawing | 14.14 MiB | 14.54 MiB |

The text-only run cuts CPU time by **64.3%** while keeping peak RSS effectively
unchanged. The SDL-inclusive run records a 0.41 MiB median RSS increase and
process-to-process variation; it includes graphics/font allocations. Neither
run establishes a universal whole-application memory guarantee. Retained text
index allocation and scan-buffer limits are unchanged, and regression checks
enforce the existing 128 KiB / 64 KiB bounds after discovery and reconstruction.

Evidence: [text-only samples](resources-text/results.json),
[SDL-inclusive samples](resources/results.json). Raw logs retain all samples,
including the discarded warmups.

## Verification

- Final ordinary suite: **832 passed, 0 failed, 101 optional tests ignored**.
  [Full log](core.log). This includes the new refill-boundary UTF-8/CRLF tests,
  malformed-data/lazy-validation checks, and scan-buffer capacity assertions.
- Two additional isolated headless SDL checks passed: Unicode/tab viewport and
  mouse targeting ([log](viewport.log)); shared-pane editing, undo and saving
  from either pane ([log](shared-panes.log)).
- The non-test application build passed with the faster production scanner.
  [Build log](production-build.log). Existing compiler warnings remain.
- All 23 final benchmark workloads completed their four processes and
  correctness assertions. The benchmark itself is opt-in and is not included
  in the ordinary-suite count.

The ordinary suite ran outside the restricted sandbox because the terminal
tests require localhost sockets. The headless viewport and split-pane checks
ran inside the sandbox. No native desktop input/clipboard or other-platform
performance result is inferred from these checks.

## Provenance and reproduction

Both benchmark builds recorded matching source hashes before and after
compilation. The following saved executables are in ignored `target/qa/`;
the reports and build logs are retained here.

| Executable | SHA-256 |
| --- | --- |
| `target/qa/distant-navigation-20261009/before-probe` | `f80f11118fb8c7acc58a64511d3e5328e8a48fc97db76f8855a269da75286441` |
| `target/qa/distant-navigation-20261009/after-probe` | `15fad839170547f85ea105a38f90aa5415ac267b44f3050c4f4c5bd87584c6c2` |

The two UTF-8/CRLF regression tests were included in the measured final build.
The scan-buffer capacity assertions and test-only accessor were added afterward;
production code was not changed after measurement.

Run a fresh full matrix from the repository root (choose a new output folder):

```sh
python3 tools/benchmarks/scaling.py --samples 3 --warmups 1 --iterations 4 --timeout 20 --no-memory
```

Repeat the paired macOS resource check against the saved executables:

```sh
python3 docs/testing/distant-navigation-2026-10-09/compare_resources.py \
  target/qa/distant-navigation-20261009/before-probe \
  target/qa/distant-navigation-20261009/after-probe \
  target/qa/distant-navigation-resources-new --no-render
```

Omit `--no-render` for the SDL-inclusive check. The output folder must be new.
Resource-statistics access may require execution outside a restricted sandbox.

These are warm-filesystem observations with small sample counts, not native
keyboard/display latency or cold-storage guarantees. A first uncached jump
still scans the prefix to determine exact line numbers; this change reduces
CPU spent scanning rather than hiding work in the background. Windows, Linux
and native ARM performance were not measured in this run.
