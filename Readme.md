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

`edit` opens a file normally, while `view` opens it read-only. Relative paths are resolved from the terminal's current directory. File paths printed in terminal output can be clicked, including compiler-style `path:line:column` locations. Pötyi asks you to save current edits before replacing the open document.

The terminal supports command history with the arrow keys, scrolling, paste with `Ctrl+V`, copying its capped output with `Ctrl+Shift+C`, stopping a running command with `Ctrl+C`, clearing with `Ctrl+L`, and clickable `Stop`/`Again`/`Clear`/`Editor` controls.

This is deliberately a command buffer rather than a full terminal emulator. Full-screen or raw interactive programs such as Vim, `top`, and interactive debuggers are not supported. Terminal output is file-backed and capped at 8 MB, and the app sleeps while idle to keep memory and CPU use predictable.

## Requirements

* Rust / Cargo
* CMake
* A C and C++ compiler for the target platform

SDL3, SDL3_ttf, and their font dependencies are built from source and
statically linked by Cargo. They do not need to be installed separately.

---

## Windows

### Install CMake

If CMake is not already installed:

```powershell
winget install Kitware.CMake
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
