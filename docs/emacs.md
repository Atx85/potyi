# Emacs keybinding mode

Enable it for the current session with `:set keybindings emacs`, or set
`keybinding_mode = "emacs"` in `[editor]` in `config/editor.toml` for startup.
Conventional and Vim modes remain available. Both panes use the selected mode.

Type text normally. `C-` means Ctrl; `M-` means Alt. Escape followed by the key
also supplies Meta, useful where Alt/Option produces accented characters.
`M-x` opens Pötyi's command bar, using Pötyi command names such as `term ls`,
`open`, `find`, and `set keybindings vim`. `C-h` displays the key reference.

| Action | Keys |
| --- | --- |
| Character / line movement | `C-f`, `C-b`, `C-n`, `C-p`, or arrows |
| End / beginning of a word | `M-f`, `M-b` |
| Beginning / end of line | `C-a`, `C-e`, Home, End |
| Beginning / end of document | `M-<`, `M->` |
| Page down / up | `C-v`, `M-v`, Page Down, Page Up |
| Set selection mark | `C-Space`, then move |
| Exchange cursor and mark | `C-x C-x` |
| Select whole document | `C-x h` |
| Cancel a prefix or selection | `C-g` |
| Delete forward / backward | `C-d`, Delete / Backspace |
| Newline / open line without advancing | Enter, `C-j`, `C-m` / `C-o` |
| Cut / copy selected region | `C-w`, `M-w` |
| Kill to end of line; at line end kill newline | `C-k` |
| Kill next / previous word | `M-d`, `M-Backspace` |
| Paste (“yank”) / cycle earlier kills | `C-y`, then `M-y` |
| Undo | `C-/`, `C-_`, `C-x u` |
| Redo | `C-Shift-/` |
| Search forward / backward | `C-s`, `C-r` |
| Open / save / Save As | `C-x C-f`, `C-x C-s`, `C-x C-w` |
| Show two panes / switch pane / show only current pane | `C-x 2` or `C-x 3` / `C-x o` / `C-x 1` |
| Quit when both documents are saved | `C-x C-c` |

Movement extends an active mark. Typing replaces the selected region. Consecutive
kills accumulate in one kill-ring entry; backward kills prepend their text.
The kill ring is shared by the two documents and retains at most 32 entries and
256 KiB of text. A larger kill still goes to the system clipboard but clears the
in-memory ring; it can be pasted with `C-y`, without retaining a second large
copy for cycling. `M-y` is available immediately after a yank or another yank-pop.
A different edit, movement, or pane switch ends that sequence.

`C-u` supplies a repeat count of four; another `C-u` multiplies it by four.
`M-digits` or digits after `C-u` set a count, with a maximum magnitude of 1000.
`M--` supplies a negative direction for movement and word/line kills. Counts
apply to movements, deletion, newline/tab insertion, ordinary typing, undo and
redo. An explicit positive count with `C-k` kills that many line endings.
Repeated text insertion is capped at 1 MiB and is one undo step.

Search uses the existing incremental search engine. Type the query, use `C-s`
or `C-r` to move between matches, and Enter to accept. `C-g` or Escape cancels
and returns to the starting cursor position. In the command bar, `C-a/e/b/f`
move, `C-d` deletes, `C-k` cuts the remaining input, `C-y` pastes, and `C-n/p`
select suggestions. Existing terminal prompt shortcuts remain available.

This is a practical editing subset, with bindings based on the
[GNU Emacs reference](https://www.gnu.org/software/emacs/refcards/pdf/refcard.pdf).
It uses Pötyi's existing two-pane layout, file handling, undo history and command
bar. Lisp extensions, keyboard macros, arbitrary buffer/window layouts,
syntax-aware expression navigation, and the full Emacs help system are outside
this mode. The default keybinding mode is still conventional.
