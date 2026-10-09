# Application structure

Potyi shares one file-backed editing model across the graphical interface,
keyboard modes, formatting, language-server actions and crash recovery.

| Location | Responsibility |
| --- | --- |
| `src/main.rs` | Entry point, module declarations, embedded font and existing internal type imports. |
| `src/editor.rs` | Document state, editing commands, cursor/history state, undo/redo, saving and formatting. |
| `src/app/mod.rs` | SDL/window/font setup, event polling, scheduled work, reconciliation and rendering. |
| `src/app/events.rs` | Per-event dispatch boundary: formatter refresh, background delivery, coordinate conversion and input routing. |
| `src/app/background.rs` | Terminal, LSP setup and LSP result handling with their scheduling and document follow-up rules. |
| `src/app/commands.rs` | Command-bar execution and the resulting application actions. |
| `src/app/search.rs` | Search panel actions, replacement and command-search synchronization. |
| `src/app/navigation.rs` | Pane focus, keybinding-mode transitions and terminal file/location navigation. |
| `src/app/input/` | SDL event dispatch and separate keyboard, text, mouse and file-drop handlers. |
| `src/app/input/command_outcome.rs` | Shared command follow-up work, with explicit keyboard, mouse and Emacs cursor/search policies. |
| `src/app/input/contexts.rs` | Smaller borrowed contexts for editing, history and text input. |
| `src/app/input/keyboard.rs` and `src/app/input/keyboard/` | Ordered keyboard dispatcher and focused terminal, Emacs, command-bar, history, Vim and conventional handlers. |
| `src/app/input/mouse.rs` and `src/app/input/mouse/` | Mouse dispatch, drag state and focused command-bar, document, terminal, motion, release and wheel handlers. |
| `src/piece_table.rs` and `src/piece_table/` | File-backed text storage, indexing and recovery. |

The application owns the editors, renderer and controllers. Each input handler
borrows that live state through `InputContext`. Editing, history, text, mouse
and command follow-up operations receive smaller contexts or direct references.
Routing does not clone documents, histories or controllers. Contexts contain
stack-held references, with no event queue or document-sized allocation added
by routing.

Handlers return `Continue` or `Quit`. `Continue` advances to the next event,
including when a modal panel consumes input. `Quit` exits the outer application
loop. Errors retain their existing return paths. `events::dispatch` handles
background terminal/setup/LSP events before SDL coordinate conversion and input dispatch;
recovery warnings, completion reconciliation and rendering remain after the
event batch.

Keyboard priority is explicit in `keyboard.rs`: pane shortcuts, terminal toggle, terminal input,
completion, Emacs handling, search/command shortcuts, command-bar input,
workspace history, conventional Escape, Vim, then conventional commands.
Each keyboard stage returns `KeyFlow::Pass` to try the next handler,
`KeyFlow::Handled` to consume input, or `KeyFlow::Quit` to exit. Stages reborrow
the same live context sequentially. Text events check pane/Emacs suppression,
terminal focus, then Vim suppression before command-bar or document insertion.
Keep these checks in order because they depend on the preceding key event and
the current input target.

Mouse input preserves coordinate-conversion checks, window resize edges, split
dividers and window controls before routing to terminal, completion, command
bar or document interactions. Motion and release preserve drag ownership;
wheel scrolling retains separate fractional accumulators for each target.

Command execution produces `CommandOutcome`. Its shared application handles
pane changes before updating the active editor's renderer state. Keyboard
command-bar editing can reveal an existing search match; mouse execution does
so when the command moves the cursor. Emacs retains its unconditional cursor
refresh. Background LSP outcomes retain their own synchronization and Vim
formatting rules in `background.rs`.

This separation does not add another interface or remove SDL from the input
controllers. The goal is smaller responsibilities with existing behavior.

Use `python3 tools/dev.py run` to build and open the editor,
`python3 tools/dev.py test` for regular tests, and `python3 tools/dev.py check`
for regular tests plus isolated headless SDL checks. On Windows use `py -3`
in place of `python3`. Evidence defaults to the ignored `target/qa/` directory.
The event-routing tests in `src/app/input/tests.rs` exercise synthetic SDL events
through the production dispatcher. Physical keyboard/mouse delivery, OS
clipboard interaction and platform packaging still need native checks.
