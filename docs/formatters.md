# Formatter installation guide

Pötyi includes basic built-in indentation and presets for eight external formatters.
`:format` needs no argument. It uses an installed matching tool when available,
otherwise tries built-in indentation. `:format builtin` selects the built-in
option explicitly. It handles simple delimiter-based indentation for C/C++, C#,
Java, Objective-C, Protobuf, JSON and untitled code; it does not replace a full
language formatter. Untitled code uses C-style rules; save with the correct
extension first for other languages. Unsupported file types or syntax produce an explanation
and leave the document unchanged. See [Code Formatting](../Readme.md#code-formatting)
for the exact scope and limits.

Type `:format ` to see names, file types and installation status. Press Enter
for automatic formatting, or choose with Up/Down then Enter; Tab completes a
name. You can also open the chooser with `:formatters`.

Install only the tools you need;
an existing installation can be reused. Pötyi does not install or update them.
Run the installation commands below in a terminal, outside the editor.

Formatting runs only when you request it. Installing a formatter does not
enable format-on-save or background formatting in Pötyi.

## Choose a formatter

| Provider name | Files handled by the preset |
| --- | --- |
| `rustfmt` | Rust |
| `ruff` | Python, including `.pyi` |
| `gofmt` | Go |
| `clang-format` | C, C++, Objective-C, Java, C#, Protobuf |
| `shfmt` | Shell scripts, including `.bashrc`, `.bash_profile`, `.profile` |
| `stylua` | Lua, Luau |
| `taplo` | TOML |
| `biome` | JavaScript, TypeScript, JSX/TSX, JSON/JSONC, CSS |

Use these lowercase provider names with `:format NAME`, for example
`:format ruff`. These are Pötyi's default file associations; an installed tool
may support additional languages that need a custom configuration entry.

## macOS

If you use Homebrew, run the command for the formatter you want. The package
links include installation details. Rustfmt is installed through Rustup.

| Formatter | Installation command | Source |
| --- | --- | --- |
| rustfmt | `rustup component add rustfmt` | [Rustfmt quick start](https://github.com/rust-lang/rustfmt#quick-start) |
| Ruff | `brew install ruff` | [Homebrew](https://formulae.brew.sh/formula/ruff) |
| gofmt | `brew install go` | [Homebrew](https://formulae.brew.sh/formula/go) |
| clang-format | `brew install clang-format` | [Homebrew](https://formulae.brew.sh/formula/clang-format) |
| shfmt | `brew install shfmt` | [Homebrew](https://formulae.brew.sh/formula/shfmt) |
| StyLua | `brew install stylua` | [Homebrew](https://formulae.brew.sh/formula/stylua) |
| Taplo | `brew install taplo` | [Homebrew](https://formulae.brew.sh/formula/taplo) |
| Biome | `brew install biome` | [Homebrew](https://formulae.brew.sh/formula/biome) |

If Rustup is not installed, follow the [Rust installation guide](https://www.rust-lang.org/tools/install)
first. Gofmt is included with Go, so an existing Go installation may already
provide it. Without Homebrew, use the official installers or macOS binaries
linked in the tool sections below.

## Linux and Windows

### Rustfmt — Rust

Install Rustup using the [Rust installation guide](https://www.rust-lang.org/tools/install),
then run this on either platform:

```sh
rustup component add rustfmt
```

If the project selects a particular Rust toolchain, run the command from that
project's directory to add the component to that toolchain.
See the [Rustfmt quick start](https://github.com/rust-lang/rustfmt#quick-start).

Pötyi's preset passes `--edition 2024`. For another Rust edition, change the
edition value in the existing `rustfmt` entry in `config/formatters.toml`.

### Ruff — Python

Ruff's standalone installer does not require Python. On Linux or macOS:

```sh
curl -LsSf https://astral.sh/ruff/install.sh | sh
```

On Windows, run in PowerShell:

```powershell
irm https://astral.sh/ruff/install.ps1 | iex
```

Follow the installer's instructions for adding its directory to `PATH`.
Other installation methods are listed in the [Ruff installation guide](https://docs.astral.sh/ruff/installation/).

### Gofmt — Go

Install Go using the [official Go installation instructions](https://go.dev/doc/install):
the Windows installer, macOS installer, or Linux archive. Gofmt is part of Go;
there is no separate formatter package to install. Make sure the Go `bin`
directory is on `PATH`, as described in those instructions.

### Clang-format — C and related languages

On Debian, install the [clang-format package](https://packages.debian.org/stable/clang-format):

```sh
sudo apt update
sudo apt install clang-format
```

For other Linux distributions, use their `clang-format` package or a binary
package from the [LLVM releases](https://github.com/llvm/llvm-project/releases).
On Windows, choose the LLVM installer for your processor from that same page.
Locate `clang-format.exe` in the installation's `bin` directory, then add that
directory to `PATH` or configure its full path as described below.

Some Linux packages use a versioned name such as `clang-format-18`. If only
that name is available, set the preset's `command` to the installed name or
its full path.

### Shfmt — shell scripts

Download the executable for your operating system and processor from the
[official shfmt releases](https://github.com/mvdan/sh/releases). Rename it to
`shfmt` on Linux/macOS or `shfmt.exe` on Windows, then follow
[Install a downloaded binary](#install-a-downloaded-binary) below.

If you already use a supported Go toolchain, an alternative is:

```sh
go install mvdan.cc/sh/v3/cmd/shfmt@latest
```

This builds the tool locally. Add Go's tool installation directory to `PATH`.
See the [shfmt quick start](https://github.com/mvdan/sh#quick-start).

### StyLua — Lua and Luau

Download the archive for your operating system and processor from the
[official StyLua releases](https://github.com/JohnnyMorganz/StyLua/releases).
Extract `stylua` or `stylua.exe`, then follow the binary installation steps
below. Official release binaries include Luau support.

If building through Cargo instead, Luau requires the corresponding feature;
see the [StyLua installation instructions](https://github.com/JohnnyMorganz/StyLua#installation).

### Taplo — TOML

Choose your operating system and processor on the
[Taplo binary downloads page](https://taplo.tamasfe.dev/cli/installation/binary.html).
Decompress the downloaded file and name the executable `taplo` or `taplo.exe`.
Follow the binary installation steps below. Pötyi uses the CLI's formatting
command; no language server setup is needed.

### Biome — JavaScript, TypeScript, JSON and CSS

On Windows, install the package documented by Biome using PowerShell:

```powershell
winget install biomejs.biome
```

On Linux, or for a manual Windows/macOS install, download the standalone CLI
for your operating system and processor using the
[Biome manual installation guide](https://biomejs.dev/guides/manual-installation/).
Name it `biome` or `biome.exe`, then follow the steps below. The standalone
binary does not need Node.js. For native Windows Pötyi, choose a Windows binary;
a Linux binary installed inside WSL cannot be used by the native editor.

## Install a downloaded binary

Choose a built executable for your operating system and processor, rather
than a source-code archive. Download names commonly use `x64`, `amd64`, or
`x86_64` for Intel/AMD 64-bit processors, and `arm64` or `aarch64` for ARM64.
Extract archives before using the executable.

On Linux/macOS, place the executable in a permanent directory and make it
executable. For example, after extracting StyLua into the current directory:

```sh
mkdir -p "$HOME/.local/bin"
install -m 755 ./stylua "$HOME/.local/bin/stylua"
```

Add that directory to your `PATH`, or configure the executable's absolute path
in Pötyi. The latter also works when launching the editor from the desktop.

On Windows, put the `.exe` in a permanent folder, such as
`C:\Users\YourName\Tools\formatters`, then add that folder to your user `Path`
in Environment Variables or configure its absolute path. Replace `YourName`
with your actual Windows username. Pötyi requires a native `.exe`; shell aliases
and `.cmd`/`.bat` launchers do not work as formatter commands.

## Check the installation

Run the relevant check in a new terminal:

| Formatter | Check |
| --- | --- |
| rustfmt | `rustfmt --version` |
| Ruff | `ruff --version` |
| gofmt | `gofmt -h` (prints help; there is no version flag) |
| clang-format | `clang-format --version` |
| shfmt | `shfmt --version` |
| StyLua | `stylua --version` |
| Taplo | `taplo --version` |
| Biome | `biome --version` |

After changing `PATH`, reopen Pötyi so it can inherit the updated environment.
Open a file and run `:formatters` to check availability for that file type.
An untitled document shows all providers; missing tools you do not use can
stay missing.

Run `:format` to use the first installed matching provider with a built-in
indentation fallback, or select one explicitly with a command such as
`:format ruff`. Untitled documents use the built-in option unless you select
an external tool. Formatting changes the editor buffer in one undo step;
save when you want to write the result to disk.

## If Pötyi cannot find the tool

An editor launched from the desktop can have a different `PATH` from your
terminal. Find the actual executable's path in the terminal. For example,
on Linux/macOS:

```sh
command -v clang-format
```

On Windows PowerShell:

```powershell
Get-Command clang-format.exe -CommandType Application | Select-Object -ExpandProperty Source
```

Run `:extract-config` in Pötyi to create `config/formatters.toml`. Existing
configuration files are preserved. This directory is relative to Pötyi's
working directory, not automatically the directory of the document being
edited.

Edit the existing provider's `command` to use the path you found. For example,
if LLVM was installed at the following Windows location, change only the
`command` line in its existing entry to:

```toml
command = 'C:\Program Files\LLVM\bin\clang-format.exe'
```

Use the actual absolute path. Pötyi does not expand `~`, `$HOME`, or
`%USERPROFILE%` in configuration. Single-quoted TOML strings keep Windows
backslashes literal. Keep the provider's other fields and do not add a second
entry with the same name. Changes take effect on the next formatting request.

An available executable can still reject a file because of syntax, project
settings, or an incompatible tool version. Read the error, check the tool's
version, and update through the method you used to install it when appropriate.
Pötyi preserves your document on failure. For formatting limits and custom
providers, see [Code Formatting in the README](../Readme.md#code-formatting).
