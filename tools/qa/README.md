# Feature verification

The [acceptance plan](../../docs/testing/plan.md) covers all 51 inventory entries.
The [execution report](../../docs/testing/report.md) distinguishes passing local
checks from missing native, platform and dependency coverage.

Run the repeatable core, headless SDL and installed-tool checks from the project root:

```sh
python3 tools/qa/run.py --output docs/testing/NEW_RUN
```

This creates logs and `results.json`. It builds the test executable, runs the
ordinary suite serially, then runs opt-in SDL and available real-tool tests in
separate processes. It does not download servers, install runtimes, publish a
release or operate the user's desktop. `--skip-live` omits the installed-tool
checks. Native clipboard and keyboard routing still need the manual cases.

Additional acceptance groups are separate because they need network access,
resource measurements, particular runtimes or a native desktop:

```sh
# Run one eligible installer at a time. This uses temporary server directories
# and may use normal package-manager caches or existing Rust components.
POTYI_LSP_SMOKE=python cargo test --locked lsp_setup::tests::live_install_and_initialize -- --exact --ignored --nocapture --test-threads=1

# Repeated measurements and one separate native idle app window.
python3 tools/benchmarks/run.py --samples 3 --idle-samples 1 --output docs/testing/NEW_RUN/benchmarks.json
```

Installer recipe names are `rust`, `clangd`, `csharp`, `python`, `typescript`,
`go`, `java`, `php`, `bash`, `lua`, `luau`, `web`, `yaml`, `toml` and `sql`.
Put a supported Node runtime on PATH when testing Node recipes. Missing runtimes
must be recorded as unverified, not passed. Real-project feature checks remain
separate from initialization checks.

The additional opt-in probes are:

- `benchmarks::recovery_latency::typing_latency_probe`, run through
  `python3 tools/qa/recovery_latency.py --output docs/testing/NEW_RECOVERY_RUN`.
  This compares recovery on/off in fresh release-test processes for empty,
  1 MiB, 100 MiB and 1 GiB files with three repetitions. It retains first-edit,
  later edit and Save timing samples, plus macOS peak RSS. Timings cover edit
  handlers, not native keyboard/display latency. Use `--report-only` with an
  existing output directory to regenerate its summary without rerunning tests.
- `piece_table::recovery::tests::recovery_memory_probe`, separately with
  `POTYI_RECOVERY_MEMORY=off` and `POTYI_RECOVERY_MEMORY=on`.
- `formatting::tests::resource_probe`.
- `renderer::terminal_selection_render_tests::long_line_navigation_probe`.

Build with `cargo test --release --locked --no-run --message-format=json` and
take the test executable from the compiler-artifact record. On macOS, invoke
that executable directly under `/usr/bin/time -l` with the exact probe name,
`--exact --ignored --nocapture --test-threads=1`, and `SDL_VIDEODRIVER=dummy`.
Measuring Cargo itself would include build/launcher memory. Resource-statistics
queries may need access outside a restricted sandbox. Keep blocked attempts
distinct from failed test assertions.

The dated evidence directory also contains `environment.json`, `resources.json`,
`installers.json` and `formatters.json` for the separate checks. With all those
records present, regenerate the feature mapping and report with:

```sh
python3 tools/qa/report.py docs/testing/2026-09-15
```

The report generator is tailored to the recorded acceptance run and its six
probe types. It is not a CI gate for arbitrary future test counts. A future
release test run must update the plan and outstanding environment checks rather
than inheriting the historical acceptance claims.
