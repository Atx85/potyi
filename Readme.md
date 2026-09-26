# Pötyi (Potyi) — lightweight Rust text editor

A free, open-source text and code editor for **macOS, Windows, and Linux**, built with **Rust and SDL3**. Pötyi focuses on low memory use, quiet idle behavior, and practical editing tools.

[Download Pötyi](https://github.com/Atx85/potyi/releases/latest) · [Editor homepage](https://atx85.github.io/potyi/) · [Features](https://atx85.github.io/potyi/features/) · [Language server setup](docs/lsp.md)

- Conventional shortcuts and optional Vim-style Normal, Insert, and Visual modes.
- Split panes, incremental search, regular expressions, and multi-cursor occurrence editing.
- A command terminal with streaming output and clickable file locations.
- Optional LSP hover, definitions, missing-import fixes, symbol rename, and reviewed file refactorings.
- File-backed editing and undo, with no full in-memory copy required to open a large file.

**Pötyi and Potyi are the same editor**; Potyi is the spelling without accents. This README describes the current source tree; packaged releases may lag behind development. See the [benchmark method](tools/benchmarks/README.md) for measured workloads and limits.

## Open a folder from the command line

```sh
potyi .                         # open the current folder
potyi ~/another-project         # open a different folder
potyi "path with spaces"         # quote folder names containing spaces
potyi src/main.rs                # open a file
potyi src/main.rs:42:7           # open a file at a line and column
```

Opening a folder with `potyi .` or dropping a folder onto Pötyi opens a split:
a clickable `ls` listing on the left and an empty editor on the right. Click a
text file to open it in the listing's pane. Ctrl-click or Cmd-click opens it in
the other pane while keeping the listing visible (creating a split if needed). Click
a folder to browse it. A folder drop preserves unsaved documents; an unsaved
right pane stays open instead of being cleared. If a terminal command is still
running, finish or stop it before dropping a folder. `Ctrl+\`` switches between
the terminal and editor. Running `potyi` without an argument opens an empty editor.

For command-line folder launches, the selected folder becomes the workspace root for this app window. Relative
`:new` and `:save-as` paths, unnamed document saves, and `config/` overrides use
that root. Terminal commands initially run there; later `cd` commands change
only the terminal's directory. No project file is created. Language servers
continue to discover their project roots from each file's project markers.
Folder drops change the browser's directory without changing the running app's
workspace root or configuration. Each launch starts a separate app instance.

To install the `potyi` command from a source checkout, run `cargo install --path .`
with the build prerequisites below installed. Ensure Cargo's bin directory
(`~/.cargo/bin` on macOS/Linux, `%USERPROFILE%\.cargo\bin` on Windows) is on your
`PATH`. For a downloaded build, add the directory containing the executable to
`PATH`; on macOS the executable is inside `Potyi.app/Contents/MacOS`.

## Mouse and trackpad

Drag over editor text to select it; hold the pointer beyond a pane's edge to
scroll while selecting. Double-click selects a word, and triple-click selects
a whole line. Continuing to drag extends by words or lines. Shift-click extends
from the current selection's anchor. These work in either pane, including two
views of the same file; Vim uses Visual mode for mouse selections outside Insert mode.
Terminal output also supports word, line, and Shift-click selection.

Drag the divider between split panes to resize them. Both panes keep a usable
minimum width, and the proportion is retained when the window changes size.
Wheel and trackpad scrolling retains fractional movements independently in each
pane; horizontal scrolling and Shift-wheel work in the editor.

## Command Bar

Pötyi uses one discoverable command bar for search, replacement, navigation, and file commands. Press `Ctrl+P`, or type `:` on an empty line, to open it. Colons typed in normal content such as `foo: bar` are inserted into the document. Type `::` on an empty line to insert a literal colon there.

Commands and their available options appear above the input. Use Up/Down or the mouse wheel to browse the full list, including the LSP commands. The list shows nine entries at a time. Click or press `Enter` to accept the highlighted command and run it when its required arguments are present. `Tab` fills in a suggestion without running it. Commands needing arguments leave the input ready for typing. Highlight an option with the arrow keys and press `Enter` to toggle it. `Escape` closes the bar.

The command panel uses compact 14-point text independently of the editor font size.

Familiar shortcuts remain available:

- `Ctrl+F` opens `:find `.
- `Ctrl+H` opens `:replace `.
- `Ctrl+N` (also `Cmd+N` on macOS) opens `:new `.
- `Ctrl+Shift+S` (also `Cmd+Shift+S` on macOS) opens `:save-as `.

Use `:new "path with spaces/file.txt"` to create and open an empty file,
or `:new` to start an unnamed document. Relative paths use the app's working
directory, and the parent folder must already exist. Existing files are never
overwritten. Save any unsaved edits in the focused pane before creating a file.

Use `:save-as "path with spaces/file.txt"` to save the focused document
under a new name. Later saves use that name; the original file, cursor,
selection, and undo history are preserved. Relative paths use the app's
working directory. Existing destinations require `:save-as! path` to
overwrite; files open in the other pane are protected even with `!`.
Files opened for viewing remain read-only. `Escape` cancels the prompt.

In Vim mode, `:saveas path` (or `:sav path`) does the same thing, and
`:saveas! path` allows overwriting. `:w` and `:write` save the current file.
The Save As shortcut also works in Normal, Insert, and Visual modes.

## Navigation and Selection

Quit with `Ctrl+Q` or `Cmd+Q` on macOS. `Escape` closes active panels or leaves Vim modes; it no longer quits the app.

| Action | Default shortcut |
| --- | --- |
| Select word / add next occurrence (conventional mode) | `Ctrl+D` (also `Cmd+D` on macOS) |
| Undo / redo | `Ctrl+Z` / `Ctrl+Y` (also `Cmd+Z` / `Cmd+Shift+Z` on macOS) |
| Select all | `Ctrl+A` (also `Cmd+A` on macOS) |
| Copy / cut / paste | `Ctrl+C/X/V` (also `Cmd+C/X/V` on macOS) |
| Previous / next word | `Ctrl+Left/Right` (also `Option+Left/Right` on macOS) |
| Select by word | Add `Shift` to the word shortcut |
| Page up / down | `Page Up/Down` |
| Select by page | `Shift+Page Up/Down` |
| Select to line start / end | `Shift+Home/End` |

Page movement uses the visible editor height, keeps the preferred column across shorter lines, and stops at the first or last line. Word movement treats Unicode letters, numbers, and underscores as words, with punctuation and whitespace as separate groups. These shortcuts can be changed in `config/keybindings.toml`.

In conventional mode, press `Cmd+D` / `Ctrl+D` to select the word under the cursor, then press again to add the next occurrence. An existing selection can be any literal text, including multiple lines. Matching is case-sensitive, wraps around the file, and skips already selected or overlapping matches. Type or paste to replace all selections together; Backspace, Delete, Enter, and Tab also work at every cursor. Consecutive edits at those cursors form one undo step. Copy joins selections in document order with newlines; paste inserts the same clipboard text at each cursor.

Press `Escape` to keep only the most recent selection or cursor. Arrow keys move every cursor together; Shift+arrows select text at each cursor so you can edit just part of each occurrence. Word, Home/End, and page movements also apply to every cursor, including their selection variants. Cursors or selections that converge are merged. Clicking elsewhere or opening search returns to one cursor. For this workflow from Vim, use `:set keybindings conventional`, make the edits, then open the command bar with `Ctrl+P` and use `:set keybindings vim`. Switching back keeps the edits and drops extra cursors; Vim's `Ctrl+D` half-page motion is unchanged.

## Emacs Keybindings

Use `:set keybindings emacs` for Emacs-style movement, marked selections,
kill/yank editing, search, repeat counts, and file/pane prefix commands.
`Alt+X` opens Pötyi's command bar; `Ctrl+G` cancels; `Ctrl+H` shows the keys.
Set `keybinding_mode = "emacs"` under `[editor]` to use it at startup.
The kill ring is shared between panes and bounded to 32 entries / 256 KiB.
See the [Emacs key reference](docs/emacs.md) for the supported subset.

## Vim-like Keybindings

Set `keybinding_mode = "vim"` under `[editor]` in
`config/editor.toml` to enable the optional Vim-like editing mode.
The default is `"conventional"`, so existing keyboard behavior is
unchanged unless Vim mode is explicitly selected.

The Vim mode supports Normal, Insert, and Visual modes; numeric counts;
`h`, `j`, `k`, `l`, `w`, `b`, `e`, `0`, `^`, `$`, `gg`, and `G` motions;
`i`, `a`, `I`, `A`, `o`, and `O` insertion; and `d`, `c`, and `y`
operators including `dd`, `cc`, and `yy`. `x`, `u`, `Ctrl+R`, `p`, `P`,
`D`, `C`, `Y`, `s`, `S`, `X`, and semantic `.` repeat are also available.
The title bar shows the current Vim mode.

Text objects work with `c` (change), `d` (delete), and `y` (yank), or after
`v` to select them visually. `i` selects the inside; `a` includes the surrounding
pair or adjacent whitespace:

| Object | Inside | Around |
| --- | --- | --- |
| Word | `iw` | `aw` |
| Whitespace-separated WORD | `iW` | `aW` |
| Parentheses | `i(`, `i)`, `ib` | `a(`, `a)`, `ab` |
| Braces | `i{`, `i}`, `iB` | `a{`, `a}`, `aB` |
| Square brackets | `i[`, `i]` | `a[`, `a]` |
| Angle brackets | `i<`, `i>` | `a<`, `a>` |
| Quotes | `i"`, `i'`, i followed by a backtick | `a"`, `a'`, a followed by a backtick |
| Sentence | `is` | `as` |
| Paragraph | `ip` | `ap` |
| HTML/XML tag pair | `it` | `at` |

Examples: `ciw` replaces the word under the cursor, `ci{` replaces a block's
contents, `di(` deletes inside parentheses, and `da"` removes a quoted string
with adjacent spacing. Quotes stay on one line and respect backslash escapes;
bracket pairs can nest and span lines. Counts work before or after the operator:
`2di(` selects the next outer pair, while `d2aw` deletes two words. Inner-word,
inner-sentence, and inner-paragraph counts include intervening whitespace.
`2i"` includes the quotes without the extra whitespace of `a"`.

Changes and their replacement typing undo together. `.` repeats operator-based
object changes at the new cursor. Repeating a block object in Visual mode expands
outward; repeated `it` alternates between contents and enclosing tags. Missing
or unmatched objects leave the document unchanged. Empty quotes and pairs can
be filled with `ci"` and `ci(`.

Object searches run only on demand, using an 8 KiB read buffer rather than
copying the document or maintaining a background parser. Paragraphs use blank
lines as boundaries; sentences use punctuation and paragraph boundaries. Tag
matching is a lightweight, case-insensitive HTML/XML scan, skipping comments,
CDATA, self-closing tags, and standard HTML void elements. Searches stop without
editing if tag nesting exceeds 256 levels or a tag name exceeds 256 bytes.

In Normal or Visual mode, press `Ctrl+W`, then `W` to focus the other pane,
`H` to focus the left pane, or `L` to focus the right pane. You can keep Ctrl
held for the second key. These shortcuts also work from a terminal pane;
press Escape first when editing in Vim Insert mode.

In Normal and Visual modes, `Ctrl+F/B` and `Page Down/Up` move a full page; `Ctrl+D/U` move half a page. Counts repeat full pages or set the number of lines for a half-page movement. In Visual mode these motions extend the selection; `Shift+Page Up/Down` also starts a selection from Normal mode. Insert mode uses the conventional page shortcuts.

Use `v` with `w`, `b`, or `e` to select by word, and `V` for whole-line selection. `ggVG` selects the entire document, including the last line. Whole-line selections also support page motions, `gg`/`G`, and yank/delete/change. In Normal and Visual modes, `Ctrl+F` pages forward; use `/` or `:find` to search.

Search reuses Pötyi's existing search engine: `/` and `?` start forward
and backward searches, `n` and `N` repeat them, and `*` and `#` search for
the word under the cursor. `:` opens the regular Pötyi command bar, so
all existing file, search, replacement, terminal, and settings commands
remain available.

This is intentionally a practical Vim-like subset rather than complete
Vim emulation. Named registers, macros, marks, and Vimscript are not
currently implemented. Yank, delete, and paste use the platform
clipboard as the unnamed register.

Examples:

```text
:find "hello world" --ignore-case
:find "^fn " --regex
:replace old new
:replace old new --all
:goto 120:4
:goto +3
:goto -3
:goto 12 --rel
:goto -6 --rel
:open "path with spaces/file.txt"
:new "notes.txt"
:term
:set font-size 20
:set line-numbers dynamic
:set keybindings vim
:split
:exit
:extract-config
:format
:format rustfmt
:formatters
:save
:quit
```

## Crash recovery

Unsaved documents are journaled to disk with bounded memory overhead. On the
next launch, Potyi offers new sessions. Dismissing the panel stops repeat reminders
without deleting the recovery data. Use `:recover` to list all sessions and
`:recover 1` to open a separate recovered copy, then Save As to choose its
location. Recovery preserves files changed outside Potyi and leaves sessions
open in other windows alone. See [crash recovery](docs/crash-recovery.md) for
storage, behavior and the memory measurements.

## Language Server Support (LSP)

Type `:lsp` and press Enter or Tab to browse all language-server commands.
Select with arrows and Enter or a mouse click. Old command names remain aliases.

Use `:lsp install <server>` to install any supported language server, such as
`:lsp install csharp`, `:lsp install python` or `:lsp install typescript`.
Setup chooses the method for Windows, macOS or Linux, shows progress, and
explains missing prerequisites. `:lsp doctor [server]` checks setup and project
requirements. Existing project settings are preserved. See the
[installation guide](docs/lsp-setup.md) for all 15 packages covering 22 profiles.

Use `:lsp rename new_name` on a function or variable to preview its project-wide references. Click a file or press Enter to review its replacements, then select **Apply changes**. Open buffers stay unsaved; the preview marks unopened files that will be written. Normal Undo reverses the rename across files. Keep affected buffers open while you need their rename undo history. Use `:lsp actions` on an unresolved symbol for missing imports, `using` directives or includes offered by your language server. Use `:lsp refactor` on a type/module or selection for refactorings such as extract/move to file. Click an action or press Enter, review its files, then select **Apply changes**. Available actions depend on your server; command-only extensions are shown as unavailable.

With `:lsp hover` open, click another word to refresh its information without closing the command bar. Scroll over the code or result panel to scroll that area. Enter repeats `:lsp hover` or `:lsp definition` at the current cursor; Escape closes the panel.

Optional LSP support provides `:lsp hover`, `:lsp definition`, `:lsp back`, `:lsp rename new_name`, `:lsp actions`, and `:lsp refactor` through
the command bar, including in Vim mode. It is disabled by default.
`:lsp start` enables LSP for this window and connects using the configured,
installed server. `:lsp restart` reloads settings and reconnects;
`:lsp status` shows progress and the last connection result. `:lsp stop` stops
servers, cancels installation and pauses autocomplete until Start. For automatic LSP in future
windows, use `:extract-config` and set `enabled = true` in `config/lsp.toml`.

When LSP is enabled, servers start when a member-access operator is typed or an LSP command is requested. Current unsaved text is synchronized
on each request, using incremental updates when supported. Server communication
runs in the background; definition jumps preserve unsaved buffers and undo
history. This version supports saved UTF-8 documents up to 2 MiB. Diagnostics provide quick-fix context; diagnostic displays are not implemented. See the [LSP web guide](https://atx85.github.io/potyi/lsp/)
or its [Markdown version](docs/lsp.md)
for setup, behavior, and limitations.

Type `.` (or `::` / `->` when advertised by the server) for member suggestions.
Use Up/Down and Enter or Tab, or click a suggestion; Escape dismisses the popup.
The same implementation is included in Windows, macOS and Linux builds, in
conventional mode and Vim insert mode. Further typing dismisses this initial
trigger-only menu. Completion is one undo step and does not save the file.

Unity C# projects use the bundled `csharp-ls` configuration after the server is
installed. Pötyi detects the Unity root and selects its generated `.sln` without
searching `Library`. See [Unity setup](docs/lsp.md#unity-on-windows-macos-and-linux).
Unity project validation is still pending; Rust and C++ completion have live-server tests.

## Code Formatting

Run `:format` or press `Ctrl+Shift+I` (`Cmd+Shift+I` on macOS) to format
the focused document, including unsaved edits. No argument is required: Pötyi
uses the first installed matching formatter, falling back to built-in indentation
when none is available. Untitled documents use built-in indentation by default.
Untitled code is treated as C-style code; save it with its language's extension
first when that assumption does not apply.
`:format builtin` explicitly selects it; `:format NAME` selects an external tool.
Typing `:format ` shows matching names, file extensions and installation status.
Enter keeps automatic selection; Up/Down then Enter chooses a tool, and Tab
completes its name. `:formatters` opens the chooser; an untitled document shows
all tools. When explicitly selecting an external formatter for an untitled
document, its first configured extension supplies a virtual filename.

Built-in indentation adjusts leading whitespace using braces, brackets and
parentheses for C/C++, C#, Java, Objective-C, Protobuf, JSON and untitled code.
It follows `tab_width` and `insert_spaces`, preserving line endings, blank lines,
trailing whitespace and multiline string/comment contents. It does not wrap
expressions, rearrange braces, or implement language-specific indentation rules
such as unbraced statements and switch labels. Unsupported file types (including
Python), interpolated strings, unmatched closing brackets and unclosed strings
or comments receive an explanation without changes. Use a language formatter
for these cases. The built-in path uses temporary files and bounded buffers
(a 64 KiB maximum line length and 256 nesting levels), with the same document
and output size limits as external formatting. It does not keep a document copy
in memory or require an installed executable.

Formatting is a foreground operation: the editor waits for it to finish.
There is no format-on-save, background worker, watcher, daemon, startup probe,
or idle formatting activity. It never saves the formatted document for you.
Successful changes form one undo step; unchanged output preserves cursor,
selection, dirty state, and redo history. Cursor and selection endpoints retain
their line and column where possible and clamp to shorter lines/documents.
Read-only documents cannot be formatted.
In Vim insert mode, formatting stays separate from the typing before and after
it in undo history. A linewise visual selection becomes a character selection
at the mapped endpoints.

The embedded configuration includes these standalone tool presets:

| Provider | File types |
| --- | --- |
| rustfmt | Rust |
| Ruff | Python |
| gofmt | Go |
| clang-format | C, C++, Objective-C, Java, C#, Protobuf |
| shfmt | Shell scripts |
| StyLua | Lua, Luau |
| Taplo | TOML |
| Biome | JavaScript, TypeScript, JSX/TSX, JSON/JSONC, CSS |

Install the tools you want separately. Pötyi does not download tools or invoke
package managers. Availability checks only inspect executables on `PATH`.
On Windows, use native `.exe` binaries rather than `.cmd`/`.bat` launchers.
See the [formatter installation guide](docs/formatters.md) for macOS, Linux,
and Windows setup, installation checks, and troubleshooting.

Run `:extract-config` to get `config/formatters.toml`. Existing extracted
configuration files are preserved. Formatter configuration is read on each
explicit request, so edits take effect without restarting. The first installed
matching provider in the file wins; reorder entries to change preference.
A selected provider's failure is reported without trying another tool.
The name `builtin` is reserved and cannot be used for a custom provider.

Custom providers use the same stdin/stdout contract:

```toml
[[providers]]
name = "my-formatter"
command = "/absolute/path/to/my-formatter"
args = ["--stdin-filename", "{filepath}", "-"]
extensions = ["my-language"]
filenames = ["Specialfile"]
```

Arguments are passed directly without shell expansion. `{filepath}` expands
to the absolute source filename within an argument. Commands run from the
source file's directory; project configuration discovery follows each tool's
stdin rules. Rustfmt's preset uses edition 2024; change its arguments for a
project requiring another edition. Configure trusted commands that read stdin
and write only formatted source to stdout. Do not configure in-place writes,
directory formatting, watch/server modes, or package-manager launchers.

To bound Pötyi's overhead, formatting accepts at most 2 MiB of input and 4 MiB
of output, stops after three seconds of process runtime, and rejects more than
64 KiB of diagnostics (only the first 1 KiB is retained). Configuration is
limited to 64 KiB and 32 providers. These limits cannot be raised through
formatter configuration. Transfers use fixed-size buffers and temporary files;
the piece table stores formatted text on disk and retains piece references for
undo. Temporary files are removed when the operation ends. Missing tools,
timeouts, failed commands, malformed UTF-8, binary output, and unexpectedly
empty output preserve the document.

The resource policy covers Pötyi's own overhead: bounded text handling, no
idle formatting work, and a limited operation duration. Third-party formatters
manage their own CPU and memory use. Pötyi requests one worker through
`RAYON_NUM_THREADS=1` and `GOMAXPROCS=1` where those settings are supported.

## Split View

Run `:split` to toggle a fixed 50/50 vertical split. Each pane owns an
independent document, cursor, selection, scroll position, and undo history.
Both editor panes use their full width and support horizontal and vertical
scrolling, long-line editing, and automatic scrolling to keep the cursor visible.
Click a pane to focus it; file and editing commands apply to the focused pane.
Keyboard navigation is available in Vim mode (`Ctrl+W`, then `W`, `H`, or `L`)
and Emacs mode (`Ctrl+X`, then `O`), including when a terminal occupies a pane.
Run `:exit` to close the focused split pane and leave the other pane visible.
Its document and unsaved edits stay in memory; `:split` brings the pane back.
You can also type `:exit` directly into a terminal pane. It only closes split
panes; `:quit` remains the command for closing the app.
Run `:term` (or press `Ctrl+\``) to show the terminal in the focused pane.
The other pane stays editable. Click between panes to switch focus; terminal
output continues while you edit. Opening `:term` in the other pane moves the
same terminal there, preserving its output and command history. Closing the
terminal restores that pane's document.

Pötyi keeps only one editor instance per file. Opening a file that is
already loaded in the other pane focuses that pane instead of creating a
second copy.

Find and Replace preview their matches while the command is being typed. `Enter` advances Find to the next match; `Shift+Enter` moves backward. Replace changes the current match, while `--all` replaces every leftmost, non-overlapping match as one undo operation.

Search options are `--case-sensitive`, `--ignore-case`, `--regex`, and `--backward`. Replace supports the same matching modes plus `--all`. Quoted arguments and `\n`, `\r`, and `\t` escapes are supported.

`:set font-size points` changes the font immediately. The command bar offers clickable common sizes, and accepts any whole number from 8 through 72. Set `font_size` under `[editor]` in `config/editor.toml` to choose the size used at startup.

`:set line-numbers normal|relative|dynamic` changes the gutter immediately. `normal` shows absolute line numbers, `relative` shows each line's distance from the cursor (including `0` on the cursor line), and `dynamic` combines relative surrounding lines with an absolute number on the cursor line. Set `line_numbers` under `[editor]` in `config/editor.toml` to choose the style used at startup.

`:goto +3` moves three lines down and `:goto -3` moves three lines up, in every line-number mode. An explicit sign always means a relative move unless overridden with `--abs`. Both `+0` and `-0` stay on the current line. Optional columns work with either direction: `:goto +3:2` moves down three lines to column 2.

Without a sign, `:goto line[:column]` matches the number shown in the gutter. In `normal` mode that is an absolute line number. In `relative` and `dynamic` modes it selects the line carrying that label, above or below the cursor. For example, on the last line of a 12-line document in dynamic mode, `:goto 2` selects actual line 10, while `:goto 12` stays on the current line. If multiple lines carry the requested label, the command shows an error with the signed commands to choose a direction. Use `:goto 3 --rel` to force three lines down or `:goto 3 --abs` to force absolute line 3; `:goto +3 --abs` also selects absolute line 3. Absolute destinations must be positive. Missing labels and out-of-range destinations show an error without moving the cursor or clearing the selection.

`:set keybindings vim|emacs|conventional` switches keyboard behavior immediately for the current session. Set `keybinding_mode` under `[editor]` in `config/editor.toml` to choose the mode used at startup.

Pötyi embeds its default editor settings, keybindings, and syntax-highlighting definitions in the executable. Run `:extract-config` to create editable copies under `config/`. Existing files are preserved, so the command never overwrites custom configuration. Pötyi uses an extracted file when available and otherwise falls back to the embedded default.

Syntax highlighting includes C, C++, C#, Python, PHP, Rust, JavaScript,
TypeScript, Go, Java, shell, Lua, JSON, TOML, YAML, HTML, CSS, and SQL.
See the [syntax highlighting guide](docs/syntax-highlighting.md) for extensions,
customization, and the limits of lightweight, line-based highlighting.
The [browser syntax designer](docs/designer/index.html) provides a visual editor,
sample preview, TOML import, and downloads for these definitions.

## Command Terminal

Git status/log/diff and ordinary `diff` output are highlighted automatically in `:term`: green additions, red removals, blue hunks and amber headers or warnings. Colour metadata is bounded and adds no idle polling. Copying keeps the original plain text. In `git log` output, click an underlined commit hash to open its coloured diff. **Back** (in place of Clear) or **Alt+Left** returns to the saved log, preserving its scroll position and unfinished command input. Back also stops a diff that is still streaming. Standard logs, `--oneline`, `--graph`, and `git -C path log` are supported; shell pipelines and aliases do not create commit links.

Run `:term` to open Pötyi's lightweight command terminal. `Ctrl+\`` switches between the terminal and editor; `Escape` from the command input, the `Editor` button, and the `exit` command also return to the editor.

The terminal runs normal non-interactive shell commands in one persistent working directory, so tools such as Git, Cargo, Node, PHP, CMake, and shell scripts can be used. These built-in commands help with file navigation:

```text
cd path
pwd
ls
touch notes.txt "another file.txt"
edit src/main.rs
edit src/main.rs:42:7
Cargo.lock
./Cargo.lock
view README.md
clear
help
exit
```

On Windows, enter `C:` (or another drive letter) to switch the terminal to that
drive's root and list its contents. `cd C:` is accepted too; `cd C:\work` opens
a specific folder. Drive letters are case-insensitive. This shortcut opens the
root rather than remembering a separate working directory for each drive.
An unavailable drive reports an error and leaves the current directory intact.

`touch` creates empty files or updates existing files' access and modification
times without changing their contents. It works on Windows, macOS, and Linux.
Quote filenames containing spaces; multiple paths are supported. Use `touch -c`
or `touch --no-create` to skip missing files, and `touch -- -notes.txt` for a name
starting with a dash. Relative paths use the terminal's current directory;
parent folders must already exist. Other touch flags are not built in.

Use `:term ls` or `:term git log --oneline` from the editor to open the terminal
and run a command immediately. Everything after `:term` uses the same handling
as the terminal prompt, including quotes, flags, pipelines and redirects. It
uses the terminal's current directory and records the command in its history.
Plain `:term` opens the terminal without running anything.

`ls` uses aligned columns sized to the terminal width when the command runs. Use `ls -1` for one name per line or `ls -lh` for file sizes. Dotfiles and dotfolders are shown by default; use `ls --hide-hidden` to hide them (`-a`/`--all` still work). Files and folders use the same neutral text color unless Git status applies. Folders are identified by a trailing `/`, for example `src/` or `.git/`; files have no trailing slash. Underlined names are clickable: plain-click opens a text file in the terminal's pane; Ctrl-click or Cmd-click opens it in the other pane. Opening the same file in both panes creates two views of its live contents, including unsaved edits. Each view has its own cursor and scroll position; edits, saves, and undo/redo stay synchronized. A pane with unsaved edits must be saved before a click can replace its document. Folders change the terminal directory and show its contents. A clickable `../` entry at the start of each folder listing goes up one folder; it is omitted at the filesystem root. Binary and unreadable files are not links. Listing links remember their original paths, including names with spaces. File types are detected from a small content sample.

`edit` opens a file normally, while `view` opens it read-only. Relative paths are resolved from the terminal's current directory. File paths printed in terminal output can be clicked, including compiler-style `path:line:column` locations. File links use the same plain-click and Ctrl/Cmd-click pane choices as listing links, including protection for unsaved edits. The `edit` and `view` commands prefer the other editor when the terminal is split, and can use an available clean pane when their preferred pane has unsaved edits. Already-open files return to their existing document without reloading it. If both panes have unsaved edits, save one before opening a third file. Terminal messages appear above the command input and wrap to fit the window. Terminal output, including `ls`, also wraps at word boundaries and reflows when the window or font size changes. Very long words and filenames continue on the next row; links remain clickable on every wrapped part. Scrolling follows the displayed rows, while copied output keeps its original line breaks.

`grep` runs through the system shell with its arguments unchanged, so all flags supported by the installed `grep` are available. For example:

```sh
grep -rin --include='*.rs' 'pattern' src
grep -nE 'error|warning' build.log
ls | grep -i 'readme'
```

Inside a Git repository, listing names also show status: modified files are amber, added files green, untracked files cyan, ignored files muted, and conflicts pink. Folders reflect changes in their contents, including red for deletions. Names appear before the background Git check completes; rerun `ls` to refresh colors. Checks are limited to two seconds per listing, 1 MiB of Git output, and 8,192 status paths with at most 1 MiB of path data. If Git is unavailable or a limit is reached, the affected listing keeps its ordinary colors. There are no icons, filesystem watchers, or idle Git polling; submodule contents are not scanned.

On macOS and Linux, external commands use the selected shell with `-c`, inheriting Pötyi's environment and `PATH`. Login profiles are not rerun for every command, avoiding repeated shell setup delays. Pipes, redirection, quoting, and variable expansion still work. If a command specifically needs login-profile setup, request it explicitly, for example `zsh -lc 'your-command'`. Tools installed through a login profile must be on the environment inherited by Pötyi; starting Pötyi from that configured shell provides it.

Pipelines, redirects, and command lists use the system shell, even when they start with a built-in such as `ls` or `cd`. For example, `cd src && grep -n 'main' main.rs` changes directory for that command only; use standalone `cd src` to change the terminal's persistent directory. Quote paths containing shell operators, such as `edit "a&b.txt"`.

Search results with locations are underlined and clickable across the whole result row, including wrapped text. For example, `grep -nH 'fn main' src/main.rs` opens the reported file and line. Single-file output from `grep -n 'fn main' src/main.rs` also works: the terminal remembers the command's filename. Links keep their original directory when you later use `cd`, and dragging selects text without opening it. Standard grep reports lines without columns, so those links land at the start of the line. Output with columns, such as `rg --column 'fn main' src/main.rs`, jumps to the reported column; ripgrep's byte columns are converted correctly for Unicode text. Omitted filenames are inferred only for an unambiguous standalone command with one literal file argument.

Compiler diagnostics are highlighted automatically: errors are red, warnings amber, notes and source locations blue, and help or suggested additions green. Rust's `--> file:line:column` and `::: file:line:column` locations, and panic locations, are underlined links to the exact line and column. Plain-click opens the file in the current pane; Ctrl/Cmd-click opens it in the other pane. Paths with spaces and Unicode work across wrapped lines. Colors are computed only as output arrives, with bounded metadata and no background parser. Copying output preserves the original text.

Pötyi does not bundle `grep`: it must be available on the shell's PATH (including on Windows). GNU and BSD grep have different flag sets. Supply files, a pipe, or input redirection when searching; interactive standard input is unavailable. The terminal strips ANSI colors and control bytes such as NUL, then applies its own Git and diagnostic highlighting; pipe or redirect output when those bytes need to be preserved.

A text-file path on its own, such as `Cargo.lock`, `./Cargo.lock`, or
`"notes with spaces.txt"`, opens that file in the editor. Existing unsaved
documents receive the same protection as with the `edit` command.
Executable files and installed commands still run normally; use `edit PATH`
to explicitly edit a script or a file whose name matches a command.

After `ls`, type a filename prefix and press `Tab` to complete it. Repeated
`Tab` cycles through matching names; `Shift+Tab` cycles backwards. Spaces and
shell punctuation are quoted automatically. Completion replaces the argument
at the cursor and preserves other arguments. `cd` suggests folders only.
Editing the input or moving the cursor starts a fresh completion cycle.

Suggestions come from the latest built-in directory listing, including the
listing shown after `cd` or clicking a folder. Tab does not scan the disk or
run a background search. The cache holds up to 4,096 entries and 256 KiB of
path text; a new listing replaces it. Files changed elsewhere may remain in
the suggestions until the next listing. Shell pipelines such as `ls | grep`
do not populate this cache.

Directory scans and file-type detection run in a background worker and stream
small batches into the terminal. Entries appear in filesystem discovery order;
completion choices are sorted alphabetically. `Ctrl+C` or **Stop** cancels a
listing. External commands also stream output through background readers, with
bounded batches per UI update to keep input and drawing responsive.

The terminal supports command history with the arrow keys in the command input, scrolling, paste with `Ctrl+V` or `Ctrl+Shift+V` (`Cmd+V` on macOS), clearing with `Ctrl+L`, and clickable `Stop`/`Again`/`Clear`/`Editor` controls. Pasting returns focus to the command input. The command bar also accepts `Ctrl+V` / `Cmd+V` to paste into its input.

Terminal output is selectable, read-only text. Drag with the mouse to select; clicking an underlined link without dragging still opens it. `Shift+Up` from the command input starts selecting output; keep holding Shift and use the arrows to extend the selection. `Escape` returns to typing. `F6` also remains available to switch focus. While output is focused, arrows move through displayed rows, `Shift` plus arrows/Home/End extends a selection, `Ctrl`/`Option` plus Left/Right moves by word, and `Ctrl+A` / `Cmd+A` selects all output. `Ctrl+C` / `Cmd+C` copies selected text; `Ctrl+Shift+C` / `Cmd+Shift+C` always copies all retained output. With no selection, `Ctrl+C` stops the running command and `Cmd+C` copies all output. `Escape` clears the selection and returns to the command input.

With Vim enabled, focused output also accepts `h/j/k/l`, `w/b`, `0/$`, `gg/G`, `v` for character selection, `V` for whole-line selection, and `y` to copy the selection to the system clipboard. Output cannot be edited. Selection remains anchored as new output streams in; when the 8 MB limit removes old output, only the retained part of a selection remains available.

Terminal output is consumed in bounded batches, with keyboard and mouse events handled between batches. Streaming redraws run at most about 60 times per second; input can redraw immediately. Textures are reused in a cache with an 8 MB pixel-storage budget and a 512-entry limit, released when returning to the editor. Appending output rewraps only the final displayed row, and trimming complete history lines reuses the retained layout. History stays file-backed instead of adding a second full copy in RAM. Workers wake the UI when output or directory results arrive; a quiet terminal uses the same idle wait as the editor, even while a silent command runs. There is no continuous redraw timer when nothing changes.

This is deliberately a command buffer rather than a full terminal emulator. Full-screen or raw interactive programs such as Vim, `top`, and interactive debuggers are not supported. Terminal output is file-backed and capped at 8 MB, and the app sleeps while idle to keep memory and CPU use predictable.

## Requirements

* Rust / Cargo
* CMake
* A C and C++ compiler for the target platform

SDL3, SDL3_ttf, and their font dependencies are built from source and
statically linked by Cargo. They do not need to be installed separately.

---

## Local development

With the build requirements above and Python 3 installed, run these from the
project folder (on Windows, replace `python3` with `py -3`):

```sh
python3 tools/dev.py run     # Build and open the project in Potyi
python3 tools/dev.py test    # Regular tests, no GUI interaction
python3 tools/dev.py check   # Regular tests plus isolated headless UI checks
```

`run` accepts a file/folder and `--release`, for example
`python3 tools/dev.py run --release "path with spaces/example.rs"`.
`test` accepts a test-name filter, such as `python3 tools/dev.py test recovery`.
`check` saves evidence under `target/qa/` and prints its location; use
`--output "path/to/results"` to choose another directory. It skips live
external-server tests and installation probes. Each command returns a nonzero
exit status if its build or checks fail.

See [Application structure](docs/architecture.md) for where code belongs and
[Feature verification](tools/qa/README.md) for the additional opt-in checks.

---

## Manual GitHub builds

The **Build Potyi** workflow builds release downloads for Linux x64,
Windows x64, and both Apple Silicon and Intel Macs.

Once `.github/workflows/build.yml` is on the repository's default branch,
open **Actions → Build Potyi → Run workflow**, choose the branch to build,
enter a new release tag (for example `v0.1.2`), and start the run. Existing
tags are rejected. After all four builds succeed, the workflow publishes
a GitHub Release for that exact commit and marks it as latest. The download
buttons on the GitHub Pages website pick up its files automatically,
including separate Apple Silicon and Intel Mac downloads. The website
changes must also be deployed through the repository's existing Pages setup.

Release downloads remain available on GitHub; additional copies under
**Artifacts** on the workflow run are retained for 30 days. The workflow
runs only when started manually. Release publication uses the built-in
GitHub token, so no additional credentials are required.

For a Linux-only rebuild, use **Actions → Rebuild Linux → Run workflow**
after `.github/workflows/build-linux.yml` reaches the default branch. Choose
the branch containing your fixes. Leave `release_tag` blank for a downloadable
workflow artifact, or enter an existing tag (for example `v0.1.8`) to replace
only its `potyi-linux-x86_64.tar.gz` download. This builds the selected branch,
not the old tag's source; it does not move the tag or change the other platform
downloads. The workflow checks core behavior, folder drops, and native Wayland
startup before packaging, and caches Linux build files for subsequent runs.

Mac downloads contain `Potyi.app`; Windows downloads contain `potyi.exe`;
Linux downloads contain `potyi` and optional desktop integration files.
The Mac apps are ad-hoc signed, without Apple notarization. Linux builds
use Ubuntu 22.04 and still require compatible system/display libraries.
Each build uses `Cargo.lock` and the embedded default settings and fonts.

## Windows

Windows builds embed `windows/potyi.ico` into `potyi.exe` for Explorer and
shortcuts. MSVC builds need the Windows SDK resource compiler (`rc.exe`),
available with Visual Studio's **Desktop development with C++** workload.
The release workflow checks that every embedded icon image matches the source
icon before packaging the download.

### Install CMake

If CMake is not already installed:

```powershell
winget install Kitware.CMake
```

The project sets `CMAKE_POLICY_VERSION_MINIMUM` to `3.5` automatically through
`.cargo/config.toml`. If Cargo does not load that configuration, set the
variable manually in PowerShell before building:

```powershell
$env:CMAKE_POLICY_VERSION_MINIMUM = "3.5"
```

### Build and run

From the Pötyi project directory:
(use cargo clean before or add in toml
```
[profile.dev]
opt-level = 1
```
)
```powershell
cargo run
```

For a release build:

```powershell
cargo build --release --locked
```

The release executable will be:

```text
target\release\potyi.exe
```

---

## Linux

The Linux build compiles and statically links SDL3 and SDL3_ttf from source.
Install a C/C++ toolchain, CMake, and the development packages for at least
one desktop display backend (X11 or Wayland). SDL maintains the current
[Linux dependency list](https://github.com/libsdl-org/SDL/blob/main/docs/README-linux.md).

For example, the following set has been tested on Debian Bookworm:

```bash
sudo apt update
sudo apt install -y \
  build-essential cmake ninja-build pkg-config \
  libasound2-dev libpulse-dev \
  libx11-dev libxext-dev libxrandr-dev libxcursor-dev \
  libxfixes-dev libxi-dev libxss-dev libxtst-dev \
  libxkbcommon-dev libwayland-dev libdecor-0-dev \
  libgl1-mesa-dev libegl1-mesa-dev \
  libdbus-1-dev libudev-dev
```

### Build Pötyi

From the Pötyi project directory:

```bash
cargo build --release --locked
```

The release executable will be:

```text
target/release/potyi
```

### Run Pötyi

```bash
./target/release/potyi
```

---

## WSL

Pötyi can also be built under WSL using the Linux instructions above.

If your Pötyi source is stored on a Windows drive, WSL exposes Windows drives under `/mnt`.

For example, the Windows `C:` drive is available at:

```text
/mnt/c/
```

Navigate to your own Pötyi project directory before running the Cargo commands.

---

## Notes

### SDL3_ttf

The `build-from-source-static` Cargo feature builds SDL3_ttf together with its
vendored FreeType, HarfBuzz, and SVG dependencies. A clean build is therefore
slower, but later builds reuse Cargo's build cache.

### Project layout

Pötyi uses a **Piece Table** for document storage. This allows large files to be edited without creating a complete in-memory copy of the file.

The project is designed with low memory usage and low CPU overhead as priorities.

Long-line rendering reads only the visible horizontal window and reuses bounded
position caches. See [long-line behavior and measurements](docs/long-lines.md)
for the implementation limits and regression benchmark.


## License

Pötyi is free and open-source software licensed under the GNU General Public License v3.0 (GPLv3).

Copyright © 2026 Attila Banko.

You are free to use, study, modify, and distribute Pötyi. If you distribute a modified version of Pötyi, you must make the corresponding source code available under the terms of the GPLv3 or any later version.

See [LICENSE](LICENSE) for the full license text.

### Third-Party Assets

Pötyi includes the DejaVu Sans Mono font. The font is distributed under its own license and is not covered by Pötyi's GPLv3 license.

See [`fonts/LICENSE-DejaVuSans.md`](fonts/LICENSE-DejaVuSans.md) for the font's license information.
