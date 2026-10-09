# Separate startup timing probe

This ignored test belongs alongside the existing SDL fixtures. It changes neither the default output benchmark nor production startup.

Registration in each **instrumented** comparison checkout:

1. Copy `terminal_startup_probe.rs` to `src/app/input/terminal_startup_probe.rs`.
2. Add these two lines in `src/app/input/tests.rs`, beside the existing resource module:

```rust
#[path = "terminal_startup_probe.rs"]
mod terminal_startup_probe;
```

No Fixture or renderer getter changes are required. Root owns compilation. The exact test name is `app::input::tests::terminal_startup_probe::matched_terminal_startup_probe`; run it alone with `--exact --ignored --nocapture --test-threads=1`.

Required environment:

- `POTYI_TERM_STARTUP_KIND=old` or `new`.
- `POTYI_TERM_STARTUP_ROOT`: absolute private fixture directory, identical contents across comparisons. Use a small directory: automatic listing belongs to initial viewport timing.
- `POTYI_TERM_TEST_CLIENT`: explicit helper built from the same frozen checkout/profile/target, as in the resource runner.
- Headless: `SDL_VIDEODRIVER=dummy`, `SDL_AUDIODRIVER=dummy`.
- Visible: `POTYI_TERM_STARTUP_VISIBLE=1`, with `SDL_VIDEODRIVER` unset or the actual native driver. The existing Fixture window is shown borderless, retaining Pötyi-rendered controls. Dummy-driver visible runs fail clearly. `POTYI_TERM_STARTUP_HOLD_MS` defaults to 2000 when visible, zero otherwise; maximum 10000. The hold follows measured stages.
- Optional `POTYI_TERM_STARTUP_COMMAND`: one literal shell command, reused unchanged for the first and all five warm commands. Default is POSIX `printf` or Windows `echo`, emitting `POTYI_STARTUP_DONE`. A quoted existing 64 KiB producer command may be substituted. Set `POTYI_TERM_STARTUP_EXPECT_LINE=RESOURCE_DONE` for that producer; this checks publication outside timing. Keep six outputs below retained-store eviction if asserting all six lines.

Markers use a separate `POTYI_TERM_STARTUP ` JSON prefix. They record synchronous open-return duration, first observed new Session readiness, initial viewport settlement, explicit Clear, first command and five unique `warm_command_01`…`05` phases. Legacy readiness is labeled `legacy_open_ready` because it owns no persistent Session. New `session_ready` is the first poll observation, not a new backend timestamp. Command durations include final output/layout/render settlement, matching the existing resource fixture's event/frame scheduling.

Fixture SDL/window/fonts construction precedes timing. Visible timing observes a real window and presentation calls, not physical scanout, OS keyboard delivery, full application launch, or necessarily native CPU execution. Record hardware, binary ISA and translation separately. On this ARM laptop, an x86_64 visible window is still a Rosetta execution measurement. The previously unbundled executable could not be addressed by computer-use tools; this opt-in custom window plus trace establishes visibility but does not by itself prove a screenshot or physical input measurement.

The helper owner adds opt-in entry/connect/request-written/reply-read/marker-flushed timing in a **separate bounded diagnostic file**. Never write diagnostics into the PTY transcript or include authentication tokens, nonces, paths or command contents. Helper-entry timing excludes loader time. Use each process's monotonic elapsed durations; do not subtract unrelated `Instant` origins. Parent dispatch versus helper entry requires an explicitly common monotonic clock or a handshake. Run detailed helper tracing separately from final CPU comparisons so tracing I/O does not become an unreported workload difference.

Use at least ten fresh-process startup pairs to assess the previous rare delay, alternating order. Distinguish first-in-process from five warm commands within the same Session; do not call fresh processes truly cold OS-cache launches. Retain each first-command value rather than hiding the first outlier in a pooled warm median. The tiny default command isolates dispatch/completion costs; the 64 KiB mode checks output-inclusive latency.

# Acceptance and identities

- Capture source digest before and after every release build through `terminal-resources.py::source_provenance`; include exact build command, profile, target, source root, test-binary SHA/path and helper SHA/path. Include identical probe source SHA in both instrumented variants. Trace collector and workload/producer SHA must be recorded separately.
- Verify source/binary/helper identities before and after runs; do not compare a baseline binary to a candidate source tree. Keep archived final-V2 evidence immutable. Use separate build targets and pause builds/profilers during measurements.
- Preserve the matched three-pair resource workload: same 64 KiB command, three paced 12 MiB streams, nine resizes, selected 4 KiB and Copy All. Compare corresponding phases only. App and descendant RSS/CPU stay separate; combine simultaneous RSS samples, not separately aggregated medians. Exclude process snapshots crossing phase boundaries; keep raw samples and observer timings.
- Preserve all twelve original terminal files and unrelated LLM changes. No commits, pushes or shared application belong to this probe.
- Run backend, VT dependency and app SDL regression gates on supported OSs. Native macOS ARM backend and Linux ARM container are available locally; x86_64 macOS app execution here is Rosetta. Windows cross-compilation is useful but does not establish native Windows behavior. Native Windows, native Intel macOS, native Linux x86_64 and fish execution remain external gates unless actually run.
- Require the final exact visual/clipboard suite, including accent-only click, same themes/fonts/DPI as the existing proof. Retain diagnostic/editor/Git/Back/selection/Stop lifecycle tests and malformed protocol boundary tests. No optimization may weaken bounded cancellation or stale-completion protection.
- Check the existing 8 MiB transcript budget, capped queues, read/projection/selection caches, and continued eviction across all three streams. Report growth honestly: existing V2 settled RSS did not flatten. Copy All retains different text bytes between original and new; it is not an equal-byte throughput comparison.

Initial aspirational targets on the same machine: warm-command median ≤100 ms, first-command p95 ≤250 ms after readiness, streaming app CPU ≤1.3× original, and no settled-memory or idle-redraw regression. These are investigation targets, not promised savings or test assertions. Report actual distributions and any missed target; a 600-second hang guard is the only settlement timeout in this measurement probe.

# Standalone collector

`review/terminal-startup.py` performs no compilation or resource sampling. Root must first register the identical formatted probe in separate frozen **instrumented** checkouts and produce immutable copies of each release test executable and real Pötyi helper. The raw uninstrumented baseline stays unchanged.

The collector uses the same source-input identity algorithm as the existing resource runner. It requires a manifest with `algorithm`, matching `source_sha256`, `source_before_sha256`, `source_after_sha256`, `source_root`, `binary_path`, `binary_sha256`, `helper_path`, `helper_sha256`, release `profile`, `target`, and exact build `command`. The manifest paths must name the frozen copies being run. It verifies identities before and after each process and records identical probe hashes across variants. Hashing executable files warms their file pages; first-command measurements therefore must not be described as cold OS-cache launches.

Create a JSON configuration with this shape, replacing the three binary/manifest paths with the root-owned final frozen paths:

```json
{
  "variants": [
    {
      "label": "baseline",
      "source_root": "/private/tmp/potyi-terminal-performance-20261009-111723/baseline-instrumented",
      "binary": "/ABSOLUTE/FROZEN/BASELINE/potyi-tests",
      "helper": "/ABSOLUTE/FROZEN/BASELINE/potyi",
      "build_provenance": "/ABSOLUTE/BASELINE-BUILD-PROVENANCE.json"
    },
    {
      "label": "candidate",
      "source_root": "/private/tmp/potyi-terminal-performance-20261009-111723/candidate",
      "binary": "/ABSOLUTE/FROZEN/CANDIDATE/potyi-tests",
      "helper": "/ABSOLUTE/FROZEN/CANDIDATE/potyi",
      "build_provenance": "/ABSOLUTE/CANDIDATE-BUILD-PROVENANCE.json"
    }
  ]
}
```

Run only after builds, correctness tests and resource probes have stopped:

```sh
python3 review/terminal-startup.py \
  --config review/startup-variants.json \
  --output review/startup-small \
  --repetitions 10
```

The output directory must be absent or empty. Each odd repetition runs baseline old/new then candidate old/new; even repetitions reverse that whole order. Each fresh process executes one first command and five identical warm commands in the same Session. The collector retains every process log, structured event, first-command value, pooled warm distribution, per-Session warm median, and per-stage median/min/max/nearest-rank p95. It records failed probes as failures, never successful partial evidence.

For a separate real window check add `--visible --hold-ms 2000`; `--video-driver cocoa` is optional on macOS. Visible runs clear inherited renderer overrides, while headless runs use dummy video/audio and software rendering. Both trace controls (`POTYI_TERM_TRACE` and `POTYI_TERM_TRACE_FILE`) and all other inherited `POTYI_TERM_*` variables are removed. Every case records helper tracing disabled. Detailed helper tracing belongs to a separate experiment.

For output-inclusive command timing, use the same literal producer command for all variants with `--command`, `--expect-line RESOURCE_DONE`, and `--workload-file /ABSOLUTE/producer.py`. External producer SHA is checked before and after each process. Keep six outputs below transcript eviction when asserting six retained marker lines. The collector's tiny default command does not invoke an external Python producer.

Source-only checks completed: Python AST parsing, Rust formatting, and exact source-provenance agreement with `terminal-resources.py` on baseline SHA `ab82a877b99e74df9b88d1c3b4721e38137e0077f7b0312757ea48ab43e73623`. Root compilation is still required. The first compile found the legacy `run_command` arm returns `TerminalAction`; the current formatted probe discards it explicitly so the new arm's unit return matches. No performance measurements were run by this reviewer.

# Located platform gates and caches

All following commands are proposed gates, **not claims of newly passing results**. Run after performance collection, serially. Current changed backend/layout code is imported directly by `tools/qa/terminal-backend/src/lib.rs`; it does not compile Pane or renderer code, so full app SDL tests are also necessary.

Native macOS ARM backend cache exists at `/private/tmp/potyi-terminal-final-arm-target`; its previous helper is a Mach-O arm64 executable. From the final frozen candidate root:

```sh
CARGO_TARGET_DIR=/private/tmp/potyi-terminal-final-arm-target \
  cargo build --offline --locked --target aarch64-apple-darwin \
  --manifest-path tools/qa/terminal-backend/Cargo.toml

POTYI_TERM_TEST_CLIENT=/private/tmp/potyi-terminal-final-arm-target/aarch64-apple-darwin/debug/potyi-terminal-backend-check \
  CARGO_TARGET_DIR=/private/tmp/potyi-terminal-final-arm-target \
  cargo test --offline --locked --target aarch64-apple-darwin \
  --manifest-path tools/qa/terminal-backend/Cargo.toml --lib -- --test-threads=1

CARGO_TARGET_DIR=/private/tmp/potyi-terminal-final-arm-target \
  cargo test --offline --locked --target aarch64-apple-darwin \
  --manifest-path tools/qa/terminal-backend/Cargo.toml -p vt100 --lib -- --test-threads=1
```

These ARM backend gates execute natively on this laptop even though the installed Cargo host is x86_64. The existing full application cache `/private/tmp/potyi-terminal-candidate-target` and release cache `/private/tmp/potyi-term-integration/target` are x86_64 macOS; application SDL execution there is under Rosetta. Full app ARM build/link dependencies must be verified separately before claiming native ARM app execution.

The cached Linux image `potyi-linux-test:latest` (`99a2cdae0c02`) is **arm64 Linux**. Existing caches: `/private/tmp/potyi-terminal-linux-build` for backend (its previous helper is ELF aarch64), `/private/tmp/potyi-terminal-linux-sdl-target` for full app, and Docker volume `potyi-linux-test-registry` for offline crates. There is no persistent terminal test container; prior runs used `--rm`. Reuse the image with explicit readonly candidate source and separate writable build caches:

```sh
docker run --rm --network none \
  --mount type=bind,source=/private/tmp/potyi-terminal-performance-20261009-111723/candidate,target=/workspace,readonly \
  --mount type=bind,source=/private/tmp/potyi-terminal-linux-build,target=/target-backend \
  --mount type=bind,source=/private/tmp/potyi-terminal-linux-sdl-target,target=/target-app \
  --mount type=volume,source=potyi-linux-test-registry,target=/usr/local/cargo/registry \
  potyi-linux-test:latest bash -lc '
set -e
export CARGO_TARGET_DIR=/target-backend
cargo build --offline --locked --manifest-path tools/qa/terminal-backend/Cargo.toml
export POTYI_TERM_TEST_CLIENT=/target-backend/debug/potyi-terminal-backend-check
cargo test --offline --locked --manifest-path tools/qa/terminal-backend/Cargo.toml --lib -- --test-threads=1
cargo test --offline --locked --manifest-path tools/qa/terminal-backend/Cargo.toml -p vt100 --lib -- --test-threads=1
export CARGO_TARGET_DIR=/target-app
cargo build --offline --locked
export POTYI_TERM_TEST_CLIENT=/target-app/debug/potyi
export SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy SDL_RENDER_DRIVER=software
cargo test --offline --locked experimental_terminal -- --include-ignored --test-threads=1
'
```

This is native Linux ARM container execution on ARM hardware, not native Linux x86_64 desktop/GPU validation. Dummy SDL tests do not establish real compositor rendering.

Windows cross-compilation cache exists at `/private/tmp/potyi-terminal-final-windows-target/x86_64-pc-windows-gnu`. From final candidate:

```sh
CARGO_TARGET_DIR=/private/tmp/potyi-terminal-final-windows-target \
  cargo check --offline --locked --target x86_64-pc-windows-gnu \
  --manifest-path tools/qa/terminal-backend/Cargo.toml --tests
```

This compiles backend/library/test cfg branches; it does not run Windows pipes, ConPTY, Jobs, SDL or Pane tests. Full Windows application and native runtime acceptance remain covered by `.github/workflows/terminal-experiment.yml` when actually dispatched/executed. That existing matrix also covers native Linux x86_64 and Intel macOS; local ARM/translated runs do not inherit those passing claims.
