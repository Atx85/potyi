# Feature test report — 2026-09-15

This run follows the [51-feature acceptance plan](plan.md). The source tree was already modified; no application code was changed during this test run. Test tools, the plan and evidence were added.

**565 distinct top-level Rust test cases passed; 0 failed.** This includes 535 ordinary tests, 23 additional opt-in integration/live-tool tests, six distinct resource probes, and the shared live-install test exercised separately for each eligible recipe. Repeated samples and recipes are not counted as new test cases. Child fixture tests run through their parents.

These are local results, not complete cross-platform acceptance. Feature evidence overlaps, so the per-feature counts below must not be added together. Native drag-and-drop is unverified; passing file-loader/rendering tests do not establish native event delivery.

Environment: `macOS-15.7.3-arm64-arm-64bit-Mach-O`. The tested release executable is **x86_64 macOS running on an ARM Mac**. This run does not verify an ARM-native package, Windows or Linux. The native idle check launched a separate real app window successfully. Interactive Computer Use returned `Computer Use permissions are not granted`; native clicks, key routing, clipboard interoperability and package icons remain unverified.

Source fingerprint: `167a7a5990ff2390839b40c1323212bbb96bae8a0a5cc6670058e1ced5365ad4`. [Environment and binary fingerprint](2026-09-15/environment.json).

## Executed groups

- **Core:** 535 passed, 33 initially ignored. [Raw log](2026-09-15/core.log).
- **Headless SDL integration:** 18 passed in isolated processes. Covers rendering, clicks/hit targets, terminal execution, Git browsing, command routing helpers, LSP UI and the Unity missing-server regression.
- **Existing real tools:** five opt-in tests passed: rustfmt editor integration; clangd hover/definition; rust-analyzer hover/definition/rename; real Rust quick fixes/extract-module refactoring; and Rust/C++ member completion.
- **Resources:** six probe types passed their correctness assertions; resource values are measurements, not timing-budget pass/fail claims. [Repeated benchmark samples](2026-09-15/benchmarks.json), [other resource probes](2026-09-15/resources.json).
- **Installer recipes:** 11/15 passed real installation/reuse and initialization; 4 unverified due to missing runtimes. [Evidence](2026-09-15/installers.json).
- **Formatter commands:** 2/8 installed tool presets passed live stdin/stdout checks. All eight presets passed configuration tests; missing tools are not counted as live passes. [Evidence](2026-09-15/formatters.json).

## Installer results

| Recipe | Result | Evidence / reason |
| --- | --- | --- |
| rust | PASS | [Log](2026-09-15/install-rust.log) |
| clangd | PASS | [Log](2026-09-15/install-clangd.log) |
| python | PASS | [Log](2026-09-15/install-python.log) |
| typescript | PASS | [Log](2026-09-15/install-typescript.log) |
| php | PASS | [Log](2026-09-15/install-php.log) |
| bash | PASS | [Log](2026-09-15/install-bash.log) |
| lua | PASS | [Log](2026-09-15/install-lua.log) |
| luau | PASS | [Log](2026-09-15/install-luau.log) |
| web | PASS | [Log](2026-09-15/install-web.log) |
| yaml | PASS | [Log](2026-09-15/install-yaml.log) |
| toml | PASS | [Log](2026-09-15/install-toml.log) |
| csharp | UNVERIFIED | No .NET SDK installed |
| go | UNVERIFIED | No Go toolchain installed |
| java | UNVERIFIED | No JDK installed; macOS java launcher is only a stub |
| sql | UNVERIFIED | No Go toolchain installed |

Successful initialization checks the connection/profile setup; it does not prove every language feature on a real project. Unity still needs testing against an actual generated Unity project. Node-based installers used the existing Node 24 runtime rather than the shell's older Node 16. Installs used temporary server directories; package managers may retain their normal caches. Existing Rust toolchain components may be reused/checked.

## Resource observations

- Native small-file idle sample: **47.51 MiB RSS** and **0.00% of one core** over 5.01 seconds. Zero means below measurement resolution; this is one sample with LSP off.
- Recovery probe, 256 MiB sparse source plus 1,000 insertions: peak RSS **5.79 MiB off**, **6.06 MiB on**, difference **0.273 MiB**. This measures the isolated test process, not the graphical editor.
- Recovery on/off elapsed times include initial backup and journal synchronization. They should not be interpreted as isolated per-keystroke latency. Raw logs retain the disk-I/O/time cost.
- Median lazy open plus first 40 lines of a warm 1 GB fixture: **1.089 ms**. This is not full-file parsing or native rendering.
- Terminal first-frame medians: printf **20.52 ms**, grep **33.81 ms**, pipeline **39.70 ms**, using headless SDL.
- Three measured benchmark samples followed a discarded warm-up. No comparison to another editor or universal resource guarantee is implied.

## Per-feature evidence

| ID | Capability | Local result | Executed mapped tests | Remaining gate |
| --- | --- | --- | ---: | --- |
| F01 | Create documents | LOCAL CHECKS PASS | 4 | Native/platform walkthrough per acceptance plan where applicable. |
| F02 | Open files by path | LOCAL CHECKS PASS | 6 | Native/platform walkthrough per acceptance plan where applicable. |
| F03 | Drag and drop files | PARTIAL: file loading/rendering only | 2 | Native drag-and-drop event delivery is not covered by these component tests. |
| F04 | Save and Save As | LOCAL CHECKS PASS | 5 | Native/platform walkthrough per acceptance plan where applicable. |
| F05 | Read-only viewing | LOCAL CHECKS PASS | 4 | Native/platform walkthrough per acceptance plan where applicable. |
| F06 | Folder workspaces | LOCAL CHECKS PASS | 3 | Native/platform walkthrough per acceptance plan where applicable. |
| F07 | Launch at a location | LOCAL CHECKS PASS | 3 | Native/platform walkthrough per acceptance plan where applicable. |
| F08 | Crash recovery | LOCAL CHECKS PASS | 14 | Native/platform walkthrough per acceptance plan where applicable. |
| F09 | Unicode text editing and movement | LOCAL CHECKS PASS | 25 | Native/platform walkthrough per acceptance plan where applicable. |
| F10 | System clipboard editing | LOCAL CHECKS PASS | 10 | Native clipboard interoperability requires a separate desktop check. |
| F11 | Undo and redo | LOCAL CHECKS PASS | 27 | Native/platform walkthrough per acceptance plan where applicable. |
| F12 | Text selection | LOCAL CHECKS PASS | 30 | Native/platform walkthrough per acceptance plan where applicable. |
| F13 | Multiple cursors and occurrences | LOCAL CHECKS PASS | 18 | Native/platform walkthrough per acceptance plan where applicable. |
| F14 | Vim-style editing | LOCAL CHECKS PASS | 26 | Native/platform walkthrough per acceptance plan where applicable. |
| F15 | Emacs-style editing | LOCAL CHECKS PASS | 6 | Native keyboard event routing still needs a desktop walkthrough. |
| F16 | Incremental text search | LOCAL CHECKS PASS | 63 | Native/platform walkthrough per acceptance plan where applicable. |
| F17 | Regular-expression search | LOCAL CHECKS PASS | 7 | Native/platform walkthrough per acceptance plan where applicable. |
| F18 | Find and replace | LOCAL CHECKS PASS | 11 | Native/platform walkthrough per acceptance plan where applicable. |
| F19 | Go to a location | LOCAL CHECKS PASS | 10 | Native/platform walkthrough per acceptance plan where applicable. |
| F20 | Discoverable command bar | LOCAL CHECKS PASS | 38 | Native/platform walkthrough per acceptance plan where applicable. |
| F21 | Two-pane editing | LOCAL CHECKS PASS | 6 | Native/platform walkthrough per acceptance plan where applicable. |
| F22 | Syntax highlighting | LOCAL CHECKS PASS | 13 | Native/platform walkthrough per acceptance plan where applicable. |
| F23 | Live editor settings | LOCAL CHECKS PASS | 17 | Native/platform walkthrough per acceptance plan where applicable. |
| F24 | Custom keyboard shortcuts | LOCAL CHECKS PASS | 22 | Native/platform walkthrough per acceptance plan where applicable. |
| F25 | Embedded defaults and configuration export | LOCAL CHECKS PASS | 16 | Native/platform walkthrough per acceptance plan where applicable. |
| F26 | Shell command execution | LOCAL CHECKS PASS | 7 | Native/platform walkthrough per acceptance plan where applicable. |
| F27 | Commands directly from the editor | LOCAL CHECKS PASS | 2 | Native/platform walkthrough per acceptance plan where applicable. |
| F28 | Clickable directory browsing | LOCAL CHECKS PASS | 6 | Native/platform walkthrough per acceptance plan where applicable. |
| F29 | Filename completion | LOCAL CHECKS PASS | 8 | Native/platform walkthrough per acceptance plan where applicable. |
| F30 | Create or touch files | LOCAL CHECKS PASS | 3 | Native/platform walkthrough per acceptance plan where applicable. |
| F31 | Clickable output locations | LOCAL CHECKS PASS | 8 | Native/platform walkthrough per acceptance plan where applicable. |
| F32 | Command history | LOCAL CHECKS PASS | 2 | Native/platform walkthrough per acceptance plan where applicable. |
| F33 | Selectable terminal output | LOCAL CHECKS PASS | 9 | Native/platform walkthrough per acceptance plan where applicable. |
| F34 | Command controls | LOCAL CHECKS PASS | 4 | Native/platform walkthrough per acceptance plan where applicable. |
| F35 | Git log and diff browsing | LOCAL CHECKS PASS | 8 | Native/platform walkthrough per acceptance plan where applicable. |
| F36 | Server installation | LOCAL CHECKS PASS | 6 | Live checks need relevant runtimes and network; OS-specific packages need native platforms. |
| F37 | Setup diagnostics | LOCAL CHECKS PASS | 3 | Native/platform walkthrough per acceptance plan where applicable. |
| F38 | Server lifecycle controls | LOCAL CHECKS PASS | 6 | Native/platform walkthrough per acceptance plan where applicable. |
| F39 | Member completion | LOCAL CHECKS PASS | 6 | Native/platform walkthrough per acceptance plan where applicable. |
| F40 | Hover information | LOCAL CHECKS PASS | 6 | Native/platform walkthrough per acceptance plan where applicable. |
| F41 | Definition navigation | LOCAL CHECKS PASS | 4 | Native/platform walkthrough per acceptance plan where applicable. |
| F42 | Project-wide symbol rename | LOCAL CHECKS PASS | 16 | Native/platform walkthrough per acceptance plan where applicable. |
| F43 | Quick fixes | LOCAL CHECKS PASS | 4 | Native/platform walkthrough per acceptance plan where applicable. |
| F44 | Reviewed refactoring | LOCAL CHECKS PASS | 16 | Native/platform walkthrough per acceptance plan where applicable. |
| F45 | Unity project discovery | LOCAL CHECKS PASS | 4 | Real Unity project on Windows/macOS remains a separate acceptance gate. |
| F46 | Built-in indentation | LOCAL CHECKS PASS | 5 | Native/platform walkthrough per acceptance plan where applicable. |
| F47 | External language formatting | LOCAL CHECKS PASS | 14 | Live language examples for every external tool require those tools to be installed. |
| F48 | Formatter discovery and selection | LOCAL CHECKS PASS | 4 | Native/platform walkthrough per acceptance plan where applicable. |
| F49 | Native desktop builds | PARTIAL: local executable only | 0 | This machine can verify its local build only; Windows/Linux/Intel/ARM native packaging need separate hosts. |
| F50 | File-backed document storage | LOCAL CHECKS PASS | 26 | Native/platform walkthrough per acceptance plan where applicable. |
| F51 | Bounded auxiliary storage and quiet idle behavior | LOCAL CHECKS PASS | 6 | Measurements characterize this host; no universal performance threshold or cross-platform result is inferred. |

[Exact test-to-feature mapping](2026-09-15/feature-results.json) · [Core/integration records and log paths](2026-09-15/results.json)

## Outstanding acceptance

1. Enable macOS Computer Use access and perform the native Potyi QA walkthrough, especially drag-and-drop, keyboard modes, menu selection and clipboard.
2. Build and run packaged Windows, Linux, macOS Intel and macOS ARM binaries on their native targets; inspect icons and repeat platform-sensitive scenarios. No release workflow was triggered because the existing workflow also publishes a release.
3. Test C#/Unity, Go, Java and SQL installations with their runtimes, and test hover/navigation against a real Unity project.
4. Run live formatter fixtures for Ruff, gofmt, clang-format, shfmt, StyLua and Taplo once installed.
5. Java archive discovery/download remains an optional unexecuted integration test here; Java runtime initialization is also unverified.

## Test infrastructure notes

The first report parser missed successful tests whose diagnostic output split the test-name/status line. It was fixed using exact single-test summaries and exit codes, without rerunning or changing the application; raw logs remain. Initial resource tests passed their assertions, but macOS blocked the timing tool's system-statistics query in the sandbox. Those logs are retained with `-sandbox-limited` filenames; the measurements were rerun with approved access and passed. These were reporting/environment issues, not Potyi assertion failures.
