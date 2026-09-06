# Pötyi — Project Plan

## Goal

Build a lightweight programming text editor for Windows 11 using:

* Rust
* SDL3
* As few external dependencies as possible
* Minimal memory usage
* Simple, understandable architecture

The editor should eventually be capable of editing programming source files comfortably while remaining small and efficient.

---

# Core Design Principles

## 1. Correctness Before Optimization

Implement simple, correct versions first.

Do not introduce complicated optimizations until they are justified by measurement.

> Do not sacrifice correctness for premature optimization.

---

## 2. Keep Responsibilities Separate

The project should have clear boundaries between:

```text
Input
  ↓
Editor Commands
  ↓
Editor State
  ↓
Text Model
  ↓
Rendering
```

SDL3 should primarily handle:

* Window creation
* Input events
* Rendering
* OS/window integration

The text model must not depend on SDL3.

---

## 3. Avoid Rewriting Architecture Mid-Stage

Once a subsystem's design is agreed upon, implementation should follow that design.

If a better idea appears later:

1. Finish the current implementation.
2. Test it.
3. Document the improvement.
4. Change the architecture deliberately.

Do not repeatedly replace working code with a different approach during implementation.

---

# Text Storage

## Piece Table

The document will use a piece table.

```text
Original Buffer
      ↓
Original file contents

Add Buffer
      ↓
All newly inserted text

Piece Table
      ↓
References ranges in either buffer
```

A piece contains metadata:

```text
source
offset
length
```

Pieces do not contain copies of the text.

### Reasons

* Efficient insertion
* Efficient deletion
* Good foundation for undo/redo
* Avoid copying the entire document
* Low memory overhead
* Interesting data structure to implement ourselves

---

# Memory Goals

Minimal memory usage is a primary design goal.

Avoid unnecessary:

* String copies
* Temporary allocations
* Duplicate text buffers
* Per-character heap allocations
* Whole-document copies

Prefer:

* `Vec<u8>` where appropriate
* Offsets
* Indices
* Byte ranges
* Compact structs
* Reused allocations
* File-backed storage where practical

Memory usage should eventually be **measured**, not guessed.

---

# Project Architecture

The project may initially use a small number of modules rather than forcing everything into one file.

Target architecture:

```text
src/
├── main.rs
├── editor.rs
├── document.rs
├── piece_table.rs
├── keybindings.rs
├── input.rs
├── rendering.rs
└── tests.rs
```

Modules should only be introduced when they provide a clear responsibility.

Do not create modules merely for the sake of splitting files.

---

# Document Model

A document represents one open file.

Conceptually:

```text
Document
├── PieceTable
├── file path
├── modified state
└── cursor/editor-related state as appropriate
```

The document owns the text model.

The renderer does not own document text.

---

# File Handling

File handling will be designed around the concept of a **document being opened**.

There should be one central operation conceptually equivalent to:

```text
open_file(path)
```

Different ways of opening a file should eventually call the same operation.

For example:

```text
Command / menu
      ↓
open_file(path)
      ↑
      │
SDL file drop
      │
      ↑
future file dialog
```

This prevents separate implementations of "open a file".

---

# File Drop

SDL3 window file dropping will be supported.

The intended flow is:

```text
Windows Explorer
       │
       │ drag file
       ▼
    SDL Window
       │
       ▼
SDL Drop Event
       │
       ▼
Extract path
       │
       ▼
open_file(path)
       │
       ▼
Read file
       │
       ▼
Create PieceTable
       │
       ▼
Display document
```

### Important rule

File dropping is an **input mechanism**, not part of the document implementation.

The editor should not have a special "dropped file" document type.

A dropped file is simply another request to:

```text
open_file(path)
```

---

# File Drop Requirements

The file-drop implementation should:

* Detect SDL drop events
* Obtain the dropped path
* Validate that it can be opened
* Load the file
* Create a new document
* Display it
* Handle empty files
* Handle invalid paths
* Handle directories appropriately
* Handle read failures
* Avoid crashing on malformed input

Later, multiple dropped files may be supported.

For the initial implementation, one dropped file at a time is sufficient.

---

# File Paths

File paths should be represented using Rust's path types rather than manually manipulating path strings.

Prefer:

```rust
Path
PathBuf
```

over manually parsing Windows paths.

This keeps Windows-specific path handling out of the editor logic wherever possible.

---

# Editor State

The editor state should contain information such as:

```text
Current Document
Cursor
Selection
Viewport
```

The cursor should represent a position/offset.

It should not contain a copy of the text.

---

# Keybindings

Keybindings are part of the editor input system.

They should translate keyboard input into editor commands.

Conceptually:

```text
SDL Keyboard Event
        ↓
Key Binding Resolver
        ↓
Command
        ↓
Editor
```

Example:

```text
Left
    ↓
MoveLeft

Ctrl+S
    ↓
Save

Escape
    ↓
Quit
```

Keybindings should not directly modify the piece table.

---

# Commands

Editor operations should eventually be represented as commands.

Examples:

```text
MoveLeft
MoveRight
MoveUp
MoveDown

InsertText
Backspace
Delete

Save
Open

SelectLeft
SelectRight

Quit
```

This gives keyboard input, menus, and future UI elements a common interface.

---

# Development Stages

## Stage 0 — Environment

* [x] Install Rust
* [x] Create Cargo project
* [x] Establish basic project
* [ ] Confirm `cargo run`
* [ ] Confirm `cargo test`

---

# Stage 1 — Piece Table

Implement the basic piece table.

No SDL functionality should be required for the text model.

### Required structures

* [ ] `Piece`
* [ ] `PieceSource`
* [ ] `PieceTable`

### Required functionality

* [ ] Original buffer
* [ ] Add buffer
* [ ] Piece list
* [ ] Read/reconstruct document
* [ ] Insert
* [ ] Delete

### Required cases

* [ ] Insert at beginning
* [ ] Insert in middle
* [ ] Insert at end
* [ ] Delete at beginning
* [ ] Delete in middle
* [ ] Delete at end
* [ ] Delete across pieces
* [ ] Delete entire document
* [ ] Insert after delete
* [ ] Multiple alternating edits

### Tests

* [ ] Empty document
* [ ] Single character
* [ ] Multiple characters
* [ ] Large input
* [ ] Edge positions
* [ ] Invalid ranges

The piece table must be correct before editor functionality is built around it.

---

# Stage 2 — SDL3 Foundation

Add SDL3.

First goal:

```text
┌──────────────────────────────┐
│                              │
│                              │
│                              │
│                              │
└──────────────────────────────┘
```

* [ ] Create window
* [ ] Create renderer
* [ ] Create event loop
* [ ] Handle quit
* [ ] Basic frame loop
* [ ] Confirm application runs correctly

SDL3 remains an input/presentation layer.

---

# Stage 3 — Text Rendering

Implement basic text rendering.

Investigate whether a small built-in bitmap font can be used before adding a font dependency.

* [ ] Render characters
* [ ] Render lines
* [ ] Monospace layout
* [ ] Cursor rendering
* [ ] Scrolling
* [ ] Viewport handling

Rendering must read from the document model rather than owning a second copy of the document.

---

# Stage 4 — Editor Input

Connect keyboard input to the document.

Initial commands:

* [ ] Character insertion
* [ ] Backspace
* [ ] Delete
* [ ] Enter
* [ ] Left
* [ ] Right
* [ ] Up
* [ ] Down
* [ ] Home
* [ ] End

The flow should be:

```text
SDL event
   ↓
Input
   ↓
Keybinding
   ↓
Command
   ↓
Editor
   ↓
PieceTable
```

---

# Stage 5 — Keybindings

Implement the configurable keybinding system.

* [x] Basic keybinding structure
* [x] Command enum
* [x] Key lookup
* [x] Modifier handling
* [x] Keybinding tests
* [ ] External keybinding configuration
* [ ] Configuration loading
* [ ] Configuration validation

Keybindings should remain independent of the piece table.

---

# Stage 6 — Opening Files

Implement the central file-opening operation.

Conceptually:

```text
open_file(path)
```

Responsibilities:

1. Validate the path.
2. Open/read the file.
3. Create the original buffer.
4. Create a piece table.
5. Create the document.
6. Make it the active document.
7. Reset appropriate editor state.

* [ ] `open_file`
* [ ] Read normal files
* [ ] Empty files
* [ ] Large files
* [ ] Invalid paths
* [ ] Read errors
* [ ] Track current file path
* [ ] Track modified state

Do not create separate document-loading logic for each input method.

---

# Stage 7 — File Drop

Add Windows Explorer drag-and-drop through SDL3.

* [ ] Handle SDL file-drop event
* [ ] Extract dropped path
* [ ] Convert to `PathBuf`
* [ ] Call `open_file(path)`
* [ ] Display dropped document
* [ ] Handle invalid drops
* [ ] Handle directories
* [ ] Handle read errors
* [ ] Test multiple drops

The architecture must remain:

```text
SDL Drop Event
      ↓
PathBuf
      ↓
open_file(path)
```

No duplicate file-loading implementation.

---

# Stage 8 — Saving

Implement:

* [ ] Save
* [ ] Save As
* [ ] Write document contents
* [ ] Update modified state
* [ ] Handle write errors
* [ ] Handle empty documents
* [ ] Preserve file path

Saving should reconstruct the document from the piece table only when necessary.

---

# Stage 9 — Selection

Implement:

* [ ] Shift + arrows
* [ ] Mouse selection
* [ ] Select all
* [ ] Delete selection
* [ ] Replace selection

Selection should contain positions/ranges.

It should not contain copied document text.

---

# Stage 10 — Clipboard

* [ ] Copy
* [ ] Cut
* [ ] Paste

Clipboard handling should remain separate from the piece table.

---

# Stage 11 — Undo / Redo

Undo should be based on editor operations rather than complete document copies.

Examples:

```text
Insert
Delete
Replace
```

Each operation should contain only the information necessary to reverse it.

* [ ] Operation representation
* [ ] Undo stack
* [ ] Redo stack
* [ ] Insert undo
* [ ] Delete undo
* [ ] Replace undo
* [ ] Selection restoration

---

# Stage 12 — Syntax Highlighting

Initially implement a lightweight lexer ourselves.

First language:

```text
Rust
```

Potential future languages:

```text
C
C++
C#
Python
```

Initial categories:

* [ ] Keywords
* [ ] Identifiers
* [ ] Numbers
* [ ] Strings
* [ ] Comments
* [ ] Operators
* [ ] Punctuation

Avoid a parser dependency unless it becomes necessary.

---

# Stage 13 — Multiple Files

Each open file becomes its own document.

* [ ] Document collection
* [ ] Tabs
* [ ] Switch documents
* [ ] Modified indicator
* [ ] Close document
* [ ] Multiple dropped files
* [ ] File switching

Architecture:

```text
Editor
 │
 ├── Document
 │    └── PieceTable
 │
 ├── Document
 │    └── PieceTable
 │
 └── Document
      └── PieceTable
```

---

# Stage 14 — Search

* [ ] Find
* [ ] Find next
* [ ] Find previous
* [ ] Replace
* [ ] Replace all

Avoid regex initially.

Add regex only if there is a compelling reason.

---

# Stage 15 — Editor Features

Potential features:

* [ ] Line numbers
* [ ] Status bar
* [ ] File tree
* [ ] Go to line
* [ ] Matching brackets
* [ ] Auto indentation
* [ ] Comment/uncomment
* [ ] Configurable keybindings
* [ ] Themes
* [ ] Settings

Features should only be added when they do not unnecessarily complicate the core architecture.

---

# Dependency Policy

Start with the smallest practical dependency set.

Current intended dependency:

```text
SDL3
```

Avoid libraries simply because they make implementation easier.

Before adding a dependency:

1. Do we actually need it?
2. Could we reasonably implement it ourselves?
3. What memory/runtime cost does it add?
4. Does it complicate the build?
5. Does it lock us into an architecture?

---

# Performance / Memory Rules

Prefer:

```text
offsets
indices
byte ranges
small structs
contiguous allocations
```

Avoid:

```text
String per character
allocation per token
copying the whole document after edits
duplicating source text
```

Measure before optimizing.

Important:

> Do not sacrifice correctness for premature optimization.

---

# Testing Philosophy

Every important subsystem should have tests.

Especially:

```text
PieceTable
KeyBindings
Document
Editor commands
File handling
```

Tests should cover:

```text
empty document

single character

insert at start
insert at middle
insert at end

delete at start
delete at middle
delete at end

delete entire document

insert after delete

multiple alternating edits

large input

invalid input
```

File handling should additionally test:

```text
normal file
empty file
missing file
directory
unreadable file
large file
```

---

# Current Status

## Current Stage

```text
Stage 5 — Keybindings
```

The keybinding system has been implemented and is currently being tested.

The latest `cargo test` run has revealed test/compile issues that need to be cleaned up before moving forward.

Current issues include:

* `tests.rs` contains Markdown code fences (` ```rust ` / ` ``` `) that must not be present in Rust source.
* `Command` needs the appropriate comparison/debug derives required by its tests.
* `KeyBindings` needs `Debug` for tests using `unwrap_err()`.
* There are currently unused-import/unused-variable warnings in `main.rs`.

These are **cleanup issues**, not a reason to redesign the keybinding architecture.

---

# Immediate Next Task

Fix the current Stage 5 test/build issues.

Then run:

```text
cargo test
```

The goal is:

```text
all tests passing
```

Only after the keybinding tests are clean should we move to:

```text
Stage 6 — Opening Files
```

---

# Next Major Feature

The next architectural feature will be:

```text
Centralized file opening
```

The important design decision is:

```text
open_file(path)
```

will be the single path into the document-loading system.

Eventually:

```text
Ctrl+O / Menu
      │
      ▼
 open_file(path)
      ▲
      │
 SDL file drop
      │
      ▲
 future file dialog
```

This is intentional.

We do **not** want separate implementations for:

```text
"open via keyboard"
"open via file drop"
"open via menu"
"open via file dialog"
```

They should all converge on the same document-opening operation.

---

# Current Architecture Direction

The editor is evolving toward:

```text
                 ┌─────────────────┐
                 │      SDL3       │
                 │                 │
                 │ Keyboard        │
                 │ Mouse           │
                 │ File Drop       │
                 │ Rendering       │
                 └────────┬────────┘
                          │
                          ▼
                 ┌─────────────────┐
                 │     Input       │
                 │ / Keybindings   │
                 └────────┬────────┘
                          │
                          ▼
                 ┌─────────────────┐
                 │     Editor      │
                 │                 │
                 │ Cursor          │
                 │ Selection       │
                 │ Viewport        │
                 │ Documents       │
                 └────────┬────────┘
                          │
                          ▼
                 ┌─────────────────┐
                 │    Document     │
                 │                 │
                 │ Path            │
                 │ Modified state  │
                 │ Piece Table     │
                 └────────┬────────┘
                          │
                          ▼
                 ┌─────────────────┐
                 │   Piece Table   │
                 │                 │
                 │ Original Buffer │
                 │ Add Buffer      │
                 │ Pieces          │
                 └─────────────────┘
```

The important boundary is:

```text
SDL3
  ↓
Editor/Input
  ↓
Document
  ↓
PieceTable
```

The piece table should know nothing about SDL3, keyboard events, windows, rendering, or file-drop events.

---

# Project Rule

Keep the project understandable.

If an optimization or abstraction makes the code dramatically more complicated:

**stop and discuss it before implementing it.**

The goal is not:

> The world's fastest text editor.

The goal is:

> A genuinely lightweight programming editor that we understand completely.

> Minimal memory.

> Minimal dependencies.

> Simple architecture.

> Fast enough.

> Correct first.
