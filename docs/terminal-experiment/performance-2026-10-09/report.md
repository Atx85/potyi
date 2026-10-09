# Terminal optimization results

The changes reduce output-processing and resize costs. Command startup improves modestly; its first-use delay remains unresolved.

Final frozen source: `d6f0a4f7f68dfcf5cdc77a4c84c1bb9b9252b4dcf5625663ef2efdb6f4b66d44`. Independent assessment verified release build, source, test executable and real Pötyi helper identities. Helper tracing was disabled in both timing experiments. Assessment: [assessment.json](assessment/assessment.json) and [assessment.md](assessment/assessment.md).

| Measurement | New integration before | New integration after | Result |
|---|---:|---:|---|
| App CPU for three 12 MiB streams | 3.81 s | 2.81 s | 26.2% less |
| App CPU for nine resizes | 2.33 s | 1.64 s | 29.6% less |
| Nine resizes, elapsed | 2412 ms | 1710 ms | 29.1% faster |
| Tiny warm commands, median | 141.87 ms | 134.30 ms | 5.3% faster |
| First command, median | 141.98 ms | 142.10 ms | Essentially unchanged |
| First command, observed p95 | 2832 ms | 2636 ms | Seconds-long outlier remains |
| Combined RSS after the third stream | 54.45 MiB | 54.52 MiB | Essentially unchanged |
| Combined RSS after Copy All | 82.45 MiB | 85.79 MiB | 3.34 MiB higher point estimate |
| Idle redraws during each settled window | 0 | 0 | Preserved |

The unchanged original terminal also varied between resource batches. Matching old/new by repetition, the new integration's total streaming CPU ratio improved from 2.03× to 1.57× original, a 22.5% reduction after accounting for the old control. Individual final stream CPU ratios remain 1.51–1.63× original, so the 1.3× target was missed. Final resize elapsed time is 1710 ms versus 2301 ms original, and combined RSS during resize is 60.95 versus 78.99 MiB original. After all output settles, the new integration still uses more combined RSS than original: 54.52 versus 49.89 MiB.

Startup uses ten fresh old/new pairs for each variant, one first command and five warm commands per Session. Both variants used fresh identical-byte artifact copies. The final warm range is 120.86–159.02 ms, and the first-command range is 128.21–2636.16 ms. The original terminal's final warm median is 16.66 ms. The 100 ms warm target and 250 ms first-command p95 target were both missed. First-command outliers remain included; the work does not claim the initial helper delay is fixed.

Memory continues growing across the output streams. The strict no-memory-regression target was missed by point estimates, especially after Copy All; three runs and allocator variation do not establish statistical significance. Idle app CPU remained at the same observed 0.01-second counter increment over each two-second window (about 0.5% of one core), with no extra redraws.

The implementation removes discarded layout text/span allocations, shares immutable visible output snapshots, avoids redundant helper writes and production syntax-check processes, copies safe ASCII spans in bulk, and reuses a transcript lock within each existing bounded parse slice. These changes preserve the existing file-backed transcript and queue/partial-output limits. Source review found no defect in the Session/literal-output changes; behavior and visual acceptance results are separate gates.

These are release x86_64 macOS runs under Rosetta on ARM hardware, using dummy SDL. They measure app scheduling through settled output/rendering, excluding fixture/window/font creation and physical keyboard/display latency. Fresh processes and fresh artifact paths do not prove cold OS-cache launches. Resource CPU covers valid process-snapshot intervals; boundary-crossing snapshots were excluded. Combined RSS is calculated from simultaneous app/descendant samples and may double-count shared pages. Copy All stores different retained text amounts in the two integrations (about 8 MiB original versus 4.56 MiB new), so its old/new timing or memory is not an equal-byte comparison.


## Behavior, visuals and integration

All 238 embedded-terminal application checks passed in the frozen release build.
The fresh old/new visual run compared 55 frames as exact RGB and 21 clipboard
files as exact bytes. That includes both standard and leading-combining-character
fixtures plus an actual click opening the accented filename. The comparisons use
the same fresh directory paths, file timestamps, Git commits, bundled font and
SDL settings for each pair. They do not cover arbitrary fonts, themes, DPI or
native compositors. See [visual summary](visual/summary.json).

Original and new images are retained as lossless PNGs; the
[conversion manifest](visual/lossless-png-manifest.json) connects every original
BMP hash to its PNG and exact RGB pixel hash. Capture provenance retains the
original filenames and hashes. The raw BMP copies remain in the local temporary
measurement workspace rather than adding 182 MiB to the repository.

[Source review](final-layout-review.md) documents the unchanged 256 KiB allowance
per projection and the conservative 2 MiB active-plus-saved projection-capacity
ceiling. This excludes separately bounded queues, flow scratch, grids, transcript
I/O and glyph caches; it is not a total terminal RAM limit.

The shared checkout was updated only after checking every owned file against the
captured baseline. All compiled source inputs then matched the frozen measured
source. The original terminal's 12 implementation/layout/cache/keyboard files
and Git index remained unchanged. See [integration check](integration-check.json)
and [exact optimization patch](optimization.patch). No branch, commit or upload
was created during this optimization.

The startup collector and probe are retained alongside the evidence. The
[configuration](startup-config-final.json) records the immutable local paths used
for this run; those temporary paths are historical provenance, not a portable
configuration. To reproduce, rebuild and freeze each variant, replace the paths
with new absolute paths and matching build manifests, and run the documented
collector without concurrent builds or samplers. The patch plus baseline source
manifest identifies the changes; historical evidence has not been overwritten.


## Platform checks

The frozen source passed the native Apple Silicon SDL-free backend suite (207
checks) and both vendored-parser checks. Linux ARM64 in the offline test container
passed the same 207 backend checks, two parser checks, the full application build
and all 238 embedded-terminal application checks. The Windows x64 GNU backend
and test sources compiled; this is a compilation gate, not native Windows runtime.
The release application behavior/visual/performance runs used macOS x86_64 under
Rosetta. Native Windows, Intel macOS and Linux x64 runtime, plus fish-specific
runtime, remain external gates. Dedicated CI is prepared but was not dispatched.
Build/test logs are retained beside this report.

A final [isolated driver diagnostic](startup-diagnostic/README.md) found distributed
process/driver costs, including helper launch; it did not establish a single slow
terminal-mode call or fix the first-use outlier. Its instrumented/mock-server
results are kept separate from the acceptance numbers.
