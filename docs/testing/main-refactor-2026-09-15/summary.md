# Main-module refactor

`src/main.rs` decreased from **4,505 to 76 lines**. Document state and editing
live in `editor.rs`; application setup and frame scheduling live in `app/mod.rs`.
Command execution, search actions and navigation have separate modules. Keyboard,
text, mouse and file-drop events have separate handlers under `app/input/`.
See the [architecture map](../../architecture.md).

The application still owns the same state. Input handlers receive stack-held
references, with no added document/history copies, background queue or new
interface. The journal, saving rules, keyboard bindings and startup behavior
are unchanged.

## Verification

- Baseline: **543 regular tests passed** before editing.
- Editor extraction: **543 regular tests passed**.
- Action extraction: **543 regular tests passed**.
- Final tree: **543 regular tests and 22 isolated headless SDL tests passed**.
- The updated release executable built successfully.
- Four new routing tests cover conventional input/undo/redo, terminal and command
  priority, Vim text suppression and history, Emacs prefixes/cancellation, pane
  clicks, invalid coordinates, file-drop reuse/failure and quit propagation.
- A source comparison checked **63 moved function bodies**, plus setup and frame
  logic. It also checked that the four extracted input handlers contain only
  the intended borrow adaptations and replacement of loop jumps with explicit
  return values. Whitespace, comments and formatting commas were ignored; this
  is a refactoring audit, not a formal proof of equivalence.
- `git diff --check` passed.

The new conventional routing test initially assumed Ctrl+Shift+Z for redo.
The existing configuration uses Ctrl+Y; the test was corrected to that binding.
No production shortcut was changed to satisfy the test.

## Limits

Tests ran locally on macOS with the x86_64 toolchain. The headless tests dispatch
SDL event values through production handlers but do not verify physical mouse
or keyboard delivery, OS clipboard interoperability or native window dragging.
Computer Use permission was unavailable in this session, so those native checks
remain unverified. Windows/Linux packaging and live external-server tests were
not rerun for this refactor. Existing missing-platform checks still apply.

## Evidence

[Baseline](baseline.log) · [Editor extraction](editor-extraction.log) ·
[Action extraction](actions-extraction.log) · [Final QA](results.json) ·
[Move audit](move-audit.json) · [Release build](release-build.log)
