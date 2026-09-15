# Application structure

Potyi shares one file-backed editing model across the graphical interface,
keyboard modes, formatting, language-server actions and crash recovery.

| Location | Responsibility |
| --- | --- |
| `src/main.rs` | Entry point, module declarations, embedded font and existing internal type imports. |
| `src/editor.rs` | Document state, editing commands, cursor/history state, undo/redo, saving and formatting. |
| `src/app/mod.rs` | SDL/window/font setup, event polling, background results and frame scheduling. |
| `src/app/commands.rs` | Command-bar execution and the resulting application actions. |
| `src/app/search.rs` | Search panel actions, replacement and command-search synchronization. |
| `src/app/navigation.rs` | Pane focus, keybinding-mode transitions and terminal file/location navigation. |
| `src/app/input/` | SDL event dispatch and separate keyboard, text, mouse and file-drop handlers. |
| `src/app/input/keyboard.rs` and `src/app/input/keyboard/` | Ordered keyboard dispatcher and focused terminal, Emacs, command-bar, history, Vim and conventional handlers. |
| `src/piece_table.rs` and `src/piece_table/` | File-backed text storage, indexing and recovery. |

The application owns the editors, renderer and controllers. Each input handler
borrows that live state through `InputContext`; it does not clone documents,
histories or controllers. The context is a fixed set of stack-held references,
with no event queue or document-sized allocation added by routing.

Handlers return `Continue` or `Quit`. `Continue` advances to the next event,
including when a modal panel consumes input. `Quit` exits the outer application
loop. Errors retain their existing return paths. Background terminal/setup/LSP
events are processed before SDL coordinate conversion and input dispatch;
recovery warnings, completion reconciliation and rendering remain after the
event batch.

Keyboard priority is explicit in `keyboard.rs`: terminal toggle, terminal input,
completion, Emacs handling, search/command shortcuts, command-bar input,
workspace history, conventional Escape, Vim, then conventional commands.
Each stage returns `None` to try the next handler, or `Some(Continue)` to consume
input; `Some(Quit)` exits. Stages reborrow the same live context sequentially.
Text events separately respect suppression from Vim/Emacs key events. Mouse
input preserves coordinate-conversion checks and window controls before
interacting with the document. Avoid changing these orders as a side effect of
reorganizing code.

This separation does not add another interface or remove SDL from the input
controllers. The goal is smaller responsibilities with existing behavior.

Use `python3 tools/dev.py run` to build and open the editor,
`python3 tools/dev.py test` for regular tests, and `python3 tools/dev.py check`
for regular tests plus isolated headless SDL checks. On Windows use `py -3`
in place of `python3`. Evidence defaults to the ignored `target/qa/` directory.
The event-routing tests in `src/app/input/tests.rs` exercise synthetic SDL events
through the production dispatcher. Physical keyboard/mouse delivery, OS
clipboard interaction and platform packaging still need native checks.
