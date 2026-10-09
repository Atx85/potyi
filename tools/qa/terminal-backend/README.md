# SDL-free terminal checks

This harness compiles the production managed process, browser, Git, transcript,
layout, selection, diagnostic and helper modules. Build its headless command
helper before running managed-command tests:

```sh
cargo build --locked --manifest-path tools/qa/terminal-backend/Cargo.toml
POTYI_TERM_TEST_CLIENT="$PWD/tools/qa/terminal-backend/target/debug/potyi-terminal-backend-check" \
  cargo test --locked --manifest-path tools/qa/terminal-backend/Cargo.toml -- --test-threads=1
```

On Windows append `.exe` and set `POTYI_TERM_TEST_CLIENT` to the absolute helper
path. An explicit target triple or `CARGO_TARGET_DIR` changes that path. Build
and execute the helper for the same target as the tests. Git fixtures require
Git on PATH. Real process and bridge tests need local PTY/pipe and loopback
access; sandbox permission failures are not implementation test results.

`cargo check --all-targets` validates compilation, not native runtime. The
terminal workflow runs native Linux x64, Windows x64, Intel macOS and Apple
Silicon macOS jobs. Application checks additionally exercise real SDL event
routing with its dummy video driver. Resource measurements use the separate
`tools/qa/terminal-resources.py` runner and a release application test binary.
