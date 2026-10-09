# Commit the scaling changes

Run these commands from the repository root. They prepare one commit for the
save fixes, shared document/history, bounded line index, tests and measurements.
These commands have not been executed.

The other LLM also changed several application and storage files. For those
files, select the scaling changes interactively: `y` stages a hunk, `n` skips it,
`s` splits it, and `e` edits it. Select the save fixes, document sharing, line
index/refill changes, history integration and file-state accessor conversions.

The scaling work depends on the existing split-pane/event/mouse-selection work,
including `src/piece_table/mouse_selection.rs`. Coordinate those prerequisite
changes with their owner. Separate commits need compatible intermediate code;
the passing tests reported in the plan cover the combined checkout.

```sh
# Review any changes already staged before adding this work.
git diff --cached --name-only

# New files owned by the scaling work, plus its documentation and evidence.
git add -- \
  COMMIT_SCALING.md \
  docs/scaling-plan.md \
  docs/testing/scaling-2026-10-08/ \
  src/benchmarks/scaling.rs \
  src/editor/document_state.rs \
  src/editor/shared_view_tests.rs \
  src/piece_table/line_index.rs \
  src/piece_table/line_index_tests.rs \
  src/piece_table/line_scan.rs \
  src/piece_table/save_tests.rs \
  src/piece_table/shared_view.rs \
  src/piece_table/view.rs \
  tools/benchmarks/SCALING.md \
  tools/benchmarks/scaling.py \
  tools/benchmarks/test_scaling.py

# Select the relevant changes in existing files; preserve other tasks' changes.
# This also picks the benchmark module declaration and its README link.
git add -p -- \
  src/benchmarks.rs \
  src/editor.rs \
  src/emacs.rs \
  src/lsp_ui.rs \
  src/lsp_ui/completion.rs \
  src/lsp_ui/tests.rs \
  src/multi_cursor.rs \
  src/piece_table.rs \
  src/piece_table/line_view.rs \
  src/piece_table/recovery.rs \
  src/piece_table/recovery/format.rs \
  src/piece_table/recovery/tests.rs \
  src/tests.rs \
  src/vim.rs \
  src/workspace_edit.rs \
  src/workspace_edit/resources.rs \
  src/workspace_edit/tests.rs \
  src/app/ \
  tools/benchmarks/README.md

# Review the complete staged patch, including anything staged earlier.
git diff --cached --check
git diff --cached --stat
git diff --cached

# Commit once the staged changes and their dependencies are ready.
git commit -m "Fix saving and improve shared-document scaling"
```

`git add -p` selects tracked changes. New event modules belong to the other
LLM's prerequisite work; their accessor adaptations must be included when that
work is staged. If splitting the changes, validate a checkout of the staged
files before committing. Exclude `target/`, executable probe copies and
`tools/benchmarks/__pycache__/`.

The existing [detailed command guide](commit-scaling-commands.txt) has additional
hunk-selection notes.
