## License

Pötyi is free and open-source software licensed under the GNU General Public License v3.0 (GPLv3).

Copyright © 2026 Attila Banko.

You are free to use, study, modify, and distribute Pötyi. If you distribute a modified version of Pötyi, you must make the corresponding source code available under the terms of the GPLv3 or any later version.

See [LICENSE](LICENSE) for the full license text.

### Third-Party Assets

Pötyi includes the DejaVu Sans Mono font. The font is distributed under its own license and is not covered by Pötyi's GPLv3 license.

See [`fonts/LICENSE-DejaVuSans.md`](fonts/LICENSE-DejaVuSans.md) for the font's license information.


# Pötyi

A lightweight Rust text editor built with SDL3, designed with low memory and CPU usage in mind.

## Command Bar

Pötyi uses one discoverable command bar for search, replacement, navigation, and file commands. Press `Ctrl+P`, or type `:` on an empty line, to open it. Colons typed in normal content such as `foo: bar` are inserted into the document. Type `::` on an empty line to insert a literal colon there.

Commands and their available options appear above the input. Use the arrow keys and `Tab`, or point and click, to select them. `Escape` closes the bar.

Familiar shortcuts remain available:

- `Ctrl+F` opens `:find `.
- `Ctrl+H` opens `:replace `.
- `Ctrl+Shift+S` (also `Cmd+Shift+S` on macOS) opens `:save-as `.

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
| Select all | `Ctrl+A` (also `Cmd+A` on macOS) |
| Previous / next word | `Ctrl+Left/Right` (also `Option+Left/Right` on macOS) |
| Select by word | Add `Shift` to the word shortcut |
| Page up / down | `Page Up/Down` |
| Select by page | `Shift+Page Up/Down` |
| Select to line start / end | `Shift+Home/End` |

Page movement uses the visible editor height, keeps the preferred column across shorter lines, and stops at the first or last line. Word movement treats Unicode letters, numbers, and underscores as words, with punctuation and whitespace as separate groups. These shortcuts can be changed in `config/keybindings.toml`.

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
:term
:set font-size 20
:set line-numbers dynamic
:set keybindings vim
:split
:extract-config
:save
:quit
```

## Split View

Run `:split` to toggle a fixed 50/50 vertical split. Each pane owns an
independent document, cursor, selection, scroll position, and undo history.
Click a pane to focus it; file and editing commands apply to the focused pane.

Pötyi keeps only one editor instance per file. Opening a file that is
already loaded in the other pane focuses that pane instead of creating a
second copy.

Find and Replace preview their matches while the command is being typed. `Enter` advances Find to the next match; `Shift+Enter` moves backward. Replace changes the current match, while `--all` replaces every leftmost, non-overlapping match as one undo operation.

Search options are `--case-sensitive`, `--ignore-case`, `--regex`, and `--backward`. Replace supports the same matching modes plus `--all`. Quoted arguments and `\n`, `\r`, and `\t` escapes are supported.

`:set font-size points` changes the font immediately. The command bar offers clickable common sizes, and accepts any whole number from 8 through 72. Set `font_size` under `[editor]` in `config/editor.toml` to choose the size used at startup.

`:set line-numbers normal|relative|dynamic` changes the gutter immediately. `normal` shows absolute line numbers, `relative` shows each line's distance from the cursor (including `0` on the cursor line), and `dynamic` combines relative surrounding lines with an absolute number on the cursor line. Set `line_numbers` under `[editor]` in `config/editor.toml` to choose the style used at startup.

`:goto +3` moves three lines down and `:goto -3` moves three lines up, in every line-number mode. An explicit sign always means a relative move unless overridden with `--abs`. Both `+0` and `-0` stay on the current line. Optional columns work with either direction: `:goto +3:2` moves down three lines to column 2.

Without a sign, `:goto line[:column]` matches the number shown in the gutter. In `normal` mode that is an absolute line number. In `relative` and `dynamic` modes it selects the line carrying that label, above or below the cursor. For example, on the last line of a 12-line document in dynamic mode, `:goto 2` selects actual line 10, while `:goto 12` stays on the current line. If multiple lines carry the requested label, the command shows an error with the signed commands to choose a direction. Use `:goto 3 --rel` to force three lines down or `:goto 3 --abs` to force absolute line 3; `:goto +3 --abs` also selects absolute line 3. Absolute destinations must be positive. Missing labels and out-of-range destinations show an error without moving the cursor or clearing the selection.

`:set keybindings vim|conventional` switches keyboard behavior immediately for the current session. Set `keybinding_mode` under `[editor]` in `config/editor.toml` to choose the mode used at startup.

Pötyi embeds its default editor settings, keybindings, and syntax-highlighting definitions in the executable. Run `:extract-config` to create editable copies under `config/`. Existing files are preserved, so the command never overwrites custom configuration. Pötyi uses an extracted file when available and otherwise falls back to the embedded default.

## Command Terminal

Run `:term` to open Pötyi's lightweight command terminal. `Ctrl+\`` switches between the terminal and editor; `Escape`, the `Editor` button, and the `exit` command also return to the editor.

The terminal runs normal non-interactive shell commands in one persistent working directory, so tools such as Git, Cargo, Node, PHP, CMake, and shell scripts can be used. These built-in commands help with file navigation:

```text
cd path
pwd
edit src/main.rs
edit src/main.rs:42:7
view README.md
clear
help
exit
```

`ls` shows text files in green, folders in blue with a trailing `/`, and binary files in amber. Underlined names are clickable: text files open in the editor, and folders change the terminal directory and show its contents. A clickable `../` entry at the start of each folder listing goes up one folder; it is omitted at the filesystem root. Binary and unreadable files are not links. Listing links remember their original paths, including names with spaces. File types are detected from a small content sample.

`edit` opens a file normally, while `view` opens it read-only. Relative paths are resolved from the terminal's current directory. File paths printed in terminal output can be clicked, including compiler-style `path:line:column` locations. If the current file has unsaved edits, terminal file links open in the other pane and preserve those edits; use `:split` to see both files. Links to an already-open file return to that document without reloading it. If both panes have unsaved edits, save one before opening a third file. Terminal messages appear above the command input and wrap to fit the window. Terminal output, including `ls`, also wraps at word boundaries and reflows when the window or font size changes. Very long words and filenames continue on the next row; links remain clickable on every wrapped part. Scrolling follows the displayed rows, while copied output keeps its original line breaks.

The terminal supports command history with the arrow keys, scrolling, paste with `Ctrl+V`, copying its capped output with `Ctrl+Shift+C`, stopping a running command with `Ctrl+C`, clearing with `Ctrl+L`, and clickable `Stop`/`Again`/`Clear`/`Editor` controls.

This is deliberately a command buffer rather than a full terminal emulator. Full-screen or raw interactive programs such as Vim, `top`, and interactive debuggers are not supported. Terminal output is file-backed and capped at 8 MB, and the app sleeps while idle to keep memory and CPU use predictable.

## Requirements

* Rust / Cargo
* CMake
* A C and C++ compiler for the target platform

SDL3, SDL3_ttf, and their font dependencies are built from source and
statically linked by Cargo. They do not need to be installed separately.

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

Mac downloads contain `Potyi.app`; Windows downloads contain `potyi.exe`;
Linux downloads contain `potyi` and optional desktop integration files.
The Mac apps are ad-hoc signed, without Apple notarization. Linux builds
use Ubuntu 22.04 and still require compatible system/display libraries.
Each build uses `Cargo.lock` and the embedded default settings and fonts.

## Windows

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
