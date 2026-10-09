# Event handling refactor plan

Status: implementation, automated verification and independent review complete.
Native verification remains outstanding.

## Implementation record — 2026-10-08

| Stage | Result |
| --- | --- |
| Baseline | 605 regular tests and 50 isolated checks passed before implementation. |
| Command outcomes | Shared application added in `src/app/input/command_outcome.rs`; keyboard, mouse and Emacs retain explicit follow-up policies. All checks passed, including a new command-path regression test. |
| Mouse responsibilities | Press routing, command-bar, document, terminal, motion, release and wheel operations separated under `src/app/input/mouse/`. All checks passed. |
| Routing and dependencies | Keyboard stages return `KeyFlow::Pass`, `Handled` or `Quit`. Editing, history, text and mouse operations use smaller borrowed contexts; conventional Escape receives direct references. All checks passed. |
| Background delivery | `src/app/events.rs` provides one event boundary; `src/app/background.rs` handles terminal and LSP delivery. The new mixed-event regression test passes. |
| macOS automated verification | 606 regular tests and 52 isolated checks passed. `cargo check --locked --bin potyi` and `cargo build --locked --bin potyi` passed. |
| Linux container verification | Debian 12 arm64: 610 regular tests and 31 isolated input checks passed. Headless Wayland startup and 29 input checks passed; two clipboard checks have the same headless failure before and after the refactor. |
| Review follow-up verification | Added successful-LSP delivery test passes on macOS dummy, Linux dummy and headless Wayland. Production application sources are unchanged since the full QA run; this follow-up adds only test coverage. |
| Independent review | Complete. The reviewer compared the task-specific changes with the saved baseline, reviewed the added LSP regression and checked final evidence/documentation. No actionable bugs found in this refactor. |
| Native verification | Not complete; platform and permission limitations are recorded below. |

Automated evidence: [macOS QA results](../target/qa/event-refactor-final-20261008/results.json)
and [Linux results](../target/qa/event-refactor-linux-20261008/results.json),
plus the [review follow-up](../target/qa/event-refactor-review-followup-20261008/results.json).
Application source hashes remained unchanged throughout the final run. The QA
report records 17 deferred resource, fixture or live-tool checks under the
workflow's opt-in policy.

The new regressions cover keyboard/mouse command outcomes and mixed background
delivery with text input, stale LSP results, terminal frame scheduling, and quit.
A review follow-up also covers successful completion edits, shared-pane
synchronization, Vim undo grouping and definition replies that change pane focus.
It uses the existing mock server; the definition destination is supplied as a
synthetic reply retaining the actual request ID and queued through SDL.
Background events are queued through SDL; text is injected directly because
SDL's Rust queue API does not support pushing synthetic `TextInput` events.

Concurrent scaling work introduced an opt-in benchmark requiring explicit case
parameters. `tools/qa/run.py` now classifies that benchmark with the other
separate resource measurements so normal QA does not run it without a case.

### Pre-existing behavior noted during review

The reviewer confirmed a shared-view autocomplete guard issue in the original
handlers: text insertion starts a request before the other view is synchronized,
so its recorded revision can become stale when synchronization follows. This
refactor preserves that ordering. The successful-delivery regression starts its
request with synchronized views to test background delivery independently of
this existing issue. Fixing the trigger/guard behavior is a separate change.

### Native verification status — 2026-10-08

A separate macOS test bundle and disposable text files are prepared under
`/private/tmp/potyi-event-native-20261008/`. Computer Use returned
“permissions are not granted” before the walkthrough; no native UI result is
claimed. App access can be limited to the test app through Computer Use app
[approvals](https://learn.chatgpt.com/docs/computer-use#permissions-and-approvals);
macOS system permissions are separate. Desktop actions remain
paused while the user considers that access.

The user has no additional Windows or Linux desktop available. Linux container
checks therefore provide build, rendering and synthetic routing evidence, not
physical input or OS clipboard verification. Two hidden-window Wayland
clipboard checks fail with both the pre-refactor and current handlers;
this comparison is retained in the Linux evidence. The first container snapshot
omitted mock-server and syntax fixtures. Adding those existing repository
fixtures resolved the nine regular-test failures without changing source code.

### Remaining native verification

- Exercise physical key/text pairing, repeat and suppression in conventional,
  Vim and Emacs modes, including terminal and command-bar focus.
- Verify OS clipboard copy, cut and paste in the editor and terminal.
- Exercise selection dragging, split resizing and cancellation on focus loss.
- Verify window controls and coordinate targeting at supported display scales.
- Repeat the relevant walkthrough on macOS, Windows and Linux.

The implementation and verification below preserve the scope of the original
plan. The starting structure is retained as context for reviewing the changes.

## Objective

Make event handling easier to change and review by centralizing repeated
follow-up work, separating mouse responsibilities, and narrowing handler
dependencies. Deliver the refactor in small stages that preserve existing
behavior.

## Starting structure

- `src/app/mod.rs` owns initialization, event polling, terminal and LSP event
  handling, scheduled work, reconciliation, and rendering.
- `src/app/input/mod.rs` dispatches SDL input and window events, cancels pending
  interactions, and synchronizes shared pane views before and after dispatch.
- `src/app/input/keyboard.rs` defines the order in which keyboard handlers run.
  Each stage currently returns `Result<Option<EventFlow>, String>`.
- `src/app/input/mouse.rs` combines window controls, split resizing, terminal
  interactions, completion, command-bar execution, document selection, and
  scrolling.
- Keyboard command-bar, Emacs, and mouse handlers repeat parts of the work
  required after command execution: pane changes, renderer updates, cache
  invalidation, cursor visibility, and redraw requests.
- `src/app/input/tests.rs` provides synthetic SDL event tests through the
  production input dispatcher.

## Behavior to preserve

- Keep keyboard priority explicit: pane shortcuts, terminal toggle, terminal
  input, completion, Emacs handling, search/command shortcuts, command-bar
  input, history, conventional Escape, Vim, then conventional commands.
- Preserve paired key/text suppression, repeat handling, prefixes, cancellation,
  and undo grouping in every keybinding mode.
- Preserve mouse hit-test priority, coordinate-conversion checks, drag ownership,
  fractional scrolling, and autoscroll cancellation.
- Preserve pane focus and shared-document synchronization. Pane focus currently
  swaps active editor and Vim state; follow-up work must address the correct
  editor after a switch.
- Process custom terminal and LSP events before input coordinate conversion for
  those events, without reordering the event batch.
- Preserve bounded event processing, terminal frame scheduling, idle waits, and
  recovery timing.
- Keep live state borrowed without cloning documents or histories. Preserve
  existing error propagation and user-visible error reporting.

## Implementation stages

### 1. Establish the baseline

Run the regular tests and isolated headless SDL checks using the existing QA
workflow. Review coverage for keyboard priority, key/text suppression, undo
groups, pane focus, command execution, and mouse interactions.

Add focused regression coverage where upcoming extractions expose meaningful
gaps, particularly command outcomes reached through different input paths and
background events that change documents or focus.

**Acceptance:** existing checks pass, and behavior relied on by the first
extraction is covered. Record any pre-existing failures separately.

### 2. Centralize command outcome application

Start with `CommandOutcome` in `src/app/commands.rs`. Extract shared application
of quit, pane closure, split changes, focus changes, path updates, document
reloads, keybinding changes, and renderer invalidation.

Migrate keyboard command-bar execution and mouse command execution first, then
the applicable Emacs paths. Preserve the ordering of these operations. Make
caller-specific cursor and search visibility behavior explicit rather than
assuming the existing paths are identical.

Keep execution of the command separate from applying its follow-up work.

**Acceptance:** equivalent commands retain their behavior across input paths;
shared follow-up operations have one implementation, with deliberate
caller-specific behavior visible at each call site.

### 3. Split mouse handling by responsibility

Retain a small dispatcher whose order is easy to inspect. Extract focused
operations for window controls, split dragging, terminal input/output,
completion, command-bar interactions, document selection, and wheel scrolling.

Keep drag state, capture ownership, cancellation, and scheduled autoscroll
coherent. Reuse the command outcome application from stage 2.

**Acceptance:** the dispatcher clearly expresses mouse priority, and selection,
pane resizing, terminal links, wheel accumulation, and focus-loss cancellation
continue to pass their regression checks.

### 4. Clarify routing results and narrow dependencies

Replace the keyboard routing convention based on `Option<EventFlow>` with an
explicit result such as `Pass`, `Handled`, and `Quit`, retaining `Result` for
errors. Preserve the current sequential handler order.

Gradually introduce smaller borrowed contexts or direct parameters for focused
operations. Define them around actual dependencies, such as document editing,
pane navigation, or terminal interaction, rather than passing the full
`InputContext` everywhere.

Document the key/text suppression contract and keep mode-specific suppression
rules intact. Apply this work incrementally to avoid coupling it to a redesign
of pane ownership or controllers.

**Acceptance:** pass-through and consumption are explicit, handlers receive
only the state their responsibilities require, and key/text and repeat behavior
remain unchanged.

### 5. Extract background-event handling

Move terminal, LSP setup, and LSP result handling into focused application
functions or modules. Keep one clear dispatch boundary for custom events and
ordinary SDL input.

Leave the outer loop responsible for polling and waiting, dispatching events,
running scheduled work, reconciling state, and rendering. Preserve per-event
follow-up work and shared-pane synchronization at their current boundaries.

Retain the existing event-batch limit, terminal frame schedule, mouse deadlines,
and recovery prompt timing.

**Acceptance:** the main loop exposes its lifecycle clearly; background results
still update the correct documents and panes, and idle behavior and input
responsiveness remain consistent.

### 6. Complete verification and documentation

Run the regular suite and isolated headless SDL routing checks after each
meaningful stage. Add tests for behavior affected by each change rather than
tests that mirror the extracted implementation.

Finish with native checks for physical keyboard/text pairing, clipboard
interaction, dragging, window controls, and display scaling. Synthetic SDL
events establish routing behavior but do not establish native event delivery.

Update `docs/architecture.md` to describe the final routing order, handler
boundaries, command outcome application, and background-event lifecycle.

**Acceptance:** relevant automated checks pass, native results and any remaining
verification gaps are recorded, and architecture documentation matches the
implementation.

## Delivery order

Implement the stages as small, reviewable changes in the order above. Stage 2
is the first production-code change after establishing the baseline. Keep
behavior changes separate from structural changes so each stage can be assessed
and reverted independently.

## Existing verification commands

```sh
python3 tools/dev.py test
python3 tools/dev.py check
```

Use `py -3` instead of `python3` on Windows. The `check` workflow includes the
regular tests and isolated headless SDL checks; evidence defaults to
`target/qa/`.
