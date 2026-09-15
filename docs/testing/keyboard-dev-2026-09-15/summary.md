# Keyboard handlers and development commands

The keyboard dispatcher shrank from **943 to 63 lines**. Seven focused modules
handle shortcuts, terminal input, Emacs, the command bar, workspace history, Vim
and conventional commands. Dispatch priority remains explicit; the conventional
handler is the final fallback. Stages borrow existing state without adding
queues or copying documents or histories.

The new `tools/dev.py` launcher provides:

- `python3 tools/dev.py run`: build and open the editor (optional path and `--release`).
- `python3 tools/dev.py test`: regular tests (optional name filter).
- `python3 tools/dev.py check`: regular tests and isolated headless SDL checks.

On Windows, use `py -3` instead of `python3`. Check evidence defaults to the
ignored `target/qa/` directory. `--output` selects another location.

Verification: **543 regular tests and 23 isolated headless checks passed**;
the release executable built successfully. The real `test` and `check` commands
were exercised. Launch-boundary checks also verified literal paths containing
spaces/shell characters, repository working directory, failure codes and
interruption handling. Those boundary checks used mocked subprocess launches;
no native GUI interaction was performed.

A comparison of nine moved logic blocks found only the intended stage-result
adaptations. The new routing check covers terminal-toggle repetition, terminal
priority and conventional/Vim/Emacs Ctrl+F behavior. Its first assertion wrongly
expected an empty `:find ` prompt to parse as a complete command; that test
expectation was corrected, with no production shortcut change.

Windows/Linux execution and physical keyboard/mouse/clipboard interaction remain
unverified locally. Live external-server tests and installation probes were
excluded from these checks.

[Final QA](results.json) · [Baseline](baseline.log) · [Fast command](dev-test.log) ·
[Move audit](move-audit.json) · [Launcher checks](launcher-checks.json) ·
[Release build](release-build.log) · [Initial test-assumption failure](initial-priority-test.log)
