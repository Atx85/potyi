# Installing language servers

Open the command bar and type `:lsp install ` to browse every supported server.
Choose with arrows and Enter, or click a suggestion. Language names and server
names are accepted; `:lsp install unity` and `:lsp install csharp-ls` both select C#.

```
:lsp install csharp
:lsp doctor csharp
:lsp install typescript
:lsp doctor
:lsp status
:lsp stop
```

Installation runs in the background. Escape dismisses progress; `:lsp status`
shows live progress and the latest result, and `:lsp stop` cancels setup and stops language servers.
Opening a source file never installs software. A successful installation connects
the current matching file if its setup panel is still open. Otherwise, use
`:lsp start`. `:lsp doctor` checks the current file's language; without a matching
file it lists all managed servers. Add a name to check a particular server.

## Supported installations

The editor chooses native paths and downloads for the operating system and
processor it is running on. Every language profile in the [LSP guide](lsp.md)
has an installation recipe.

| Name | Languages | Installation method | Prerequisites |
| --- | --- | --- | --- |
| `rust` | Rust | rustup stable components: rust-analyzer and rust-src | rustup and stable Rust |
| `clangd` | C, C++ | Official clangd release archive | curl |
| `csharp` | C#, Unity | csharp-ls in a private .NET tool directory | .NET 10 SDK or newer |
| `python` | Python | python-lsp-server in a private Python environment | Python 3.9+, venv and pip |
| `typescript` | JavaScript, JSX, TypeScript, TSX | typescript-language-server and TypeScript 6 through npm | Node.js and npm |
| `go` | Go | gopls through Go | Current Go toolchain |
| `java` | Java | Official Eclipse JDT LS milestone archive | JDK 21+ and curl |
| `php` | PHP | Intelephense through npm | Node.js and npm |
| `bash` | Bash | bash-language-server through npm | Node.js and npm |
| `lua` | Lua | Official LuaLS release archive | curl |
| `luau` | Luau | Official luau-lsp release archive | curl |
| `web` | HTML, CSS, JSON, JSONC | vscode-langservers-extracted through npm | Node.js and npm |
| `yaml` | YAML | yaml-language-server through npm | Node.js and npm |
| `toml` | TOML | Taplo built with its LSP feature through Cargo | Rust/Cargo and native build tools |
| `sql` | SQL | sqls through Go | Current Go; a C compiler may be needed |

Node recipes require at least Node.js 20 and enforce each package's own runtime
requirements. Using the current Node.js LTS is recommended. If a required tool
is missing or too old, setup gives Windows, macOS or Linux installation
instructions. Potyi does not install runtimes, request administrator privileges,
or run your system's package manager. Install the prerequisite and retry.

Release archives cover the platforms offered by their upstream projects. If
there is no matching archive, setup explains how to install the native server
manually. Vue's TypeScript bridge is not supported, so Vue has no installer.

## Where installations go

- Windows: `%LOCALAPPDATA%\Potyi\lsp`
- macOS: `~/Library/Application Support/Potyi/lsp`
- Linux: `$XDG_DATA_HOME/potyi/lsp`, falling back to `~/.local/share/potyi/lsp`

Downloaded packages live in individual installation directories. Only completed
installations become active. Failed or cancelled setup keeps the previous
installation record. Running install again verifies and reuses a working managed
installation; it is not an upgrade command. Broken installations are repaired
in a fresh directory. Older directories are retained so other running windows
can continue using them. Installations shared with other tools are never deleted.

When a compatible native server is already available on the computer, setup
checks and reuses it. Node packages always use a private directory and launch
through Node directly, including on Windows. Rust uses the existing rustup
stable toolchain and adds its components there. Package managers may also use
their normal download/build caches.

## Configuration and project checks

No project configuration file is rewritten. Managed launchers supply defaults
for the installed languages and replace the bundled bare executable names.
Existing arguments, root markers and initialization options remain in place.
Explicit alternative commands and absolute paths take precedence; the first
profile matching an extension still wins. `:lsp doctor <server>` reports the
selected profile and project requirements.

Use `:extract-config` for custom settings and set top-level `enabled = true` in
`config/lsp.toml` for automatic LSP in future windows. For a one-window session,
`:lsp start` is sufficient. See the [manual recipes](lsp.md) for overrides.

Installation checks that launchers run; successful project analysis still needs
project dependencies and settings. Unity needs generated `.sln`/`.csproj` files
and its assemblies. C/C++ needs correct build flags, usually in
`compile_commands.json`. Python's managed server has its own environment, so
configure Jedi's environment or select a project-installed pylsp to resolve
project-only dependencies. Java gets separate writable JDT workspaces per
project and Potyi process. SQL needs your own database connection configuration;
JSON, YAML and TOML benefit from schema associations.

If an installation was interrupted by a power loss, close other Potyi windows
and remove the exact stale `.lock` file named in the error before retrying.
