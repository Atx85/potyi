# Language server support

Read the [web version of this guide](lsp/) on the documentation website.

Pötyi has an optional, initial LSP client for **hover information and
go-to-definition**. It works with separately installed language servers that
speak LSP over stdin/stdout. LSP is disabled by default; it creates no server
processes or worker threads until an enabled command is requested.

## Setup

Run `:extract-config` to create `config/lsp.toml` without overwriting existing
configuration. Set `enabled = true`, then configure an installed server.
Configuration paths, like the other editor settings, are relative to Pötyi's
working directory. This version does not download or install language servers.

## Language server recipes

Keep `enabled = true` at the top of `config/lsp.toml`, before any `[[servers]]` entries. Add the entries for the languages you use. Names must be unique, and the first entry matching a file extension wins. Extensions have no leading dot. Root markers are exact file or directory names, not wildcard patterns.

This guide covers all 18 language families in the bundled syntax configuration, with additional dialect notes. Highlighting and language-server compatibility are separate: clangd has been tested with Pötyi; the other recipes follow upstream setup instructions and still need end-to-end verification. Only hover and definition navigation are available in this editor version, even when a server offers more features.

### Paths, Windows, and checking your setup

Use an absolute executable path when the server is not on Pötyi’s PATH. Installation commands run in your terminal; put only the server executable in `command` and its individual arguments in `args`. Pötyi does not expand `~`, environment variables, or shell commands in these fields. Replace example absolute paths before use.

On Windows, native servers use their `.exe` executable. TOML literal strings make paths easy to write, for example `command = 'C:\Tools\LLVM\bin\clangd.exe'`. For npm-installed servers, do not point to a `.cmd` or `.bat` launcher. Set `command = "node"` (or the absolute `node.exe` path), then put the server’s JavaScript entry file first in `args`, followed by its usual arguments.

To find that script, run `npm root -g`, open the server package’s `package.json` below that directory, and find the `bin` entry matching the command name. Resolve that entry relative to the package directory. For example, a Node launch has the shape `args = ['C:\absolute\package\server.js', '--stdio']`; replace the script path with the actual bin entry. Keep `start` instead of `--stdio` for Bash Language Server. A desktop shortcut may inherit a different PATH from your terminal.

After adding a recipe, save the configuration and run `:lsp-stop`. Open a saved file with a matching extension, place the document cursor on a symbol or documented property, and run `:hover`. Try `:definition` on a symbol whose source exists locally, then `:lsp-back`. Use `:lsp-status` to check configuration and active sessions. Installing a package alone does not verify that it can analyze your project.

### Rust · rust-analyzer

Install a Rust toolchain with rustup, then add the server and standard-library sources. Check the installed server:

```sh
rustup component add rust-analyzer rust-src
rust-analyzer --version
```

```toml
[[servers]]
name = "rust"
command = "rust-analyzer"
args = []
extensions = ["rs"]
language_id = "rust"
root_markers = ["Cargo.toml", ".git"]

[servers.initialization_options]
cachePriming = { enable = false }
checkOnSave = false
```

Open an `.rs` file inside a Cargo project. This is the server entry included in the extracted defaults; edit that entry instead of adding a duplicate. The example disables cache warming and checks on save to reduce background work.

[rust-analyzer installation](https://rust-analyzer.github.io/book/rust_analyzer_binary.html).

### C and C++ · clangd

Install clangd using the upstream instructions: LLVM packages on Windows, Homebrew LLVM on macOS, or your Linux package manager. Use the installed clangd executable path if it is not on PATH. Check:

```sh
clangd --version
```

```toml
[[servers]]
name = "clangd-c"
command = "clangd"
args = ["--background-index=false"]
extensions = ["c", "h"]
language_id = "c"
root_markers = ["compile_commands.json", "CMakeLists.txt", ".git"]
```

```toml
[[servers]]
name = "clangd-cpp"
command = "clangd"
args = ["--background-index=false"]
extensions = ["cc", "cpp", "cxx", "c++", "hh", "hpp", "hxx", "h++"]
language_id = "cpp"
root_markers = ["compile_commands.json", "CMakeLists.txt", ".git"]
```

Provide `compile_commands.json` with your build flags; CMake projects can generate it with `-DCMAKE_EXPORT_COMPILE_COMMANDS=ON`. Follow the upstream project setup instructions for locating the database. The examples treat `.h` as C; move it to the C++ entry for C++ headers. Disabling background indexing reduces background work but can limit navigation across unopened files.

[clangd installation and project setup](https://clangd.llvm.org/installation).

### C# · csharp-ls

Install .NET 10 SDK or newer for the current server, then install the global tool. The project itself may target an older framework.

```sh
dotnet tool install --global csharp-ls
csharp-ls --version
```

```toml
[[servers]]
name = "csharp"
command = "csharp-ls"
args = []
extensions = ["cs", "csx"]
language_id = "csharp"
root_markers = ["global.json", "Directory.Build.props", ".git"]
```

Restore the project dependencies with `dotnet restore`. For a non-Git project, add your exact solution or project filename to `root_markers`, such as `MyApp.sln` or `MyApp.csproj`. With multiple solutions, use `args = ["--solution", "/absolute/path/MyApp.sln"]`. Standalone `.csx` scripts depend on the server’s project support. Keep metadata URI mode disabled; Pötyi opens local files only.

[csharp-ls setup](https://github.com/razzmatazz/csharp-language-server).

### Python · python-lsp-server

Activate your project’s Python environment, install the server there, and check it:

```sh
python -m pip install python-lsp-server
pylsp --help
```

```toml
[[servers]]
name = "python"
command = "pylsp"
args = []
extensions = ["py", "pyw", "pyi"]
language_id = "python"
root_markers = ["pyproject.toml", "setup.py", "setup.cfg", ".git"]
```

Install your project dependencies in the same environment. Launch Pötyi from that activated terminal, or set `command` to the environment’s absolute `bin/pylsp` path (Windows: `Scripts/pylsp.exe`). The base package uses Jedi for hover and definitions; the optional linting extras are unnecessary for these two features.

[python-lsp-server installation](https://github.com/python-lsp/python-lsp-server).

### JavaScript and TypeScript · typescript-language-server

Install Node.js with npm. The upstream server currently specifies TypeScript 6 as its companion package:

```sh
npm install -g typescript-language-server typescript@6
typescript-language-server --version
```

```toml
[[servers]]
name = "javascript"
command = "typescript-language-server"
args = ["--stdio"]
extensions = ["js", "mjs", "cjs"]
language_id = "javascript"
root_markers = ["tsconfig.json", "jsconfig.json", "package.json", ".git"]
```

```toml
[[servers]]
name = "jsx"
command = "typescript-language-server"
args = ["--stdio"]
extensions = ["jsx"]
language_id = "javascriptreact"
root_markers = ["tsconfig.json", "jsconfig.json", "package.json", ".git"]
```

```toml
[[servers]]
name = "typescript"
command = "typescript-language-server"
args = ["--stdio"]
extensions = ["ts", "mts", "cts"]
language_id = "typescript"
root_markers = ["tsconfig.json", "jsconfig.json", "package.json", ".git"]
```

```toml
[[servers]]
name = "tsx"
command = "typescript-language-server"
args = ["--stdio"]
extensions = ["tsx"]
language_id = "typescriptreact"
root_markers = ["tsconfig.json", "jsconfig.json", "package.json", ".git"]
```

Use the entries you need. JSX and TSX require separate language identifiers. Install the project’s npm dependencies and keep its `tsconfig.json` or `jsconfig.json` in place so imports resolve. `.vue` files need the separate integration described below. On Windows, follow the Node launcher instructions above.

[TypeScript language server setup](https://github.com/typescript-language-server/typescript-language-server).

### Go · gopls

Install a current Go toolchain, then install and check gopls:

```sh
go install golang.org/x/tools/gopls@latest
gopls version
```

```toml
[[servers]]
name = "go"
command = "gopls"
args = []
extensions = ["go"]
language_id = "go"
root_markers = ["go.work", "go.mod", ".git"]
```

Make the Go tool installation directory available to Pötyi: `GOBIN` if set, otherwise the `bin` directory beneath `GOPATH`. Use an absolute executable path if needed. Open a file in your Go module or workspace, with dependencies available through `go.mod` or `go.work`.

[gopls installation and workspaces](https://go.dev/gopls/).

### Java · Eclipse JDT Language Server

Install Java 21 or newer and Python 3.9 or newer for the launcher. Download and extract a JDT LS milestone distribution from the upstream instructions. Check `java -version`, then run the extracted `bin/jdtls --help` launcher. Set JAVA_HOME to the JDK before starting Pötyi.

```toml
[[servers]]
name = "java"
command = "/absolute/path/jdtls/bin/jdtls"
args = ["-configuration", "/absolute/path/cache/jdtls/config", "-data", "/absolute/path/cache/jdtls/my-project"]
extensions = ["java"]
language_id = "java"
root_markers = ["pom.xml", "build.gradle", "build.gradle.kts", ".git"]
```

Replace every absolute path. Use a different `-data` directory for each Java project; never share it between concurrent sessions. For multiple projects, launch Pötyi from each project with its own `config/lsp.toml`. On Windows, use `command = "python"` and put the absolute `bin/jdtls` script path first in `args`. Maven/Gradle dependencies must resolve. Definitions inside JARs may use virtual documents that Pötyi cannot open.

[JDT LS downloads and launcher setup](https://github.com/eclipse-jdtls/eclipse.jdt.ls).

### PHP · Intelephense

Install Node.js with npm, then install the standalone server. Check the package is installed:

```sh
npm install -g intelephense
npm list -g intelephense --depth=0
```

```toml
[[servers]]
name = "php"
command = "intelephense"
args = ["--stdio"]
extensions = ["php", "phtml", "php3", "php4", "php5", "php7", "php8", "phps"]
language_id = "php"
root_markers = ["composer.json", ".git"]
```

Install your project’s Composer dependencies so their source files are available. Hover and go-to-definition are available without a paid licence. For legacy file suffixes, also configure Intelephense’s file associations when workspace indexing needs them. On Windows, use the Node launcher instructions above.

[Intelephense installation for other editors](https://github.com/bmewburn/intelephense-docs/blob/master/installation.md).

### Shell · Bash Language Server

Install Node.js 20 or newer with npm, then install and check the server:

```sh
npm install -g bash-language-server
bash-language-server --help
```

```toml
[[servers]]
name = "bash"
command = "bash-language-server"
args = ["start"]
extensions = ["sh", "bash", "bats"]
language_id = "shellscript"
root_markers = [".git"]
```

Try hover on a documented function or go-to-definition on a shell function call. ShellCheck and shfmt are optional upstream integrations; Pötyi does not expose LSP diagnostics or formatting. This recipe targets Bash and compatible shell syntax. Highlighting `.zsh` and `.ksh` does not guarantee this Bash server understands those dialects. Extensionless scripts currently cannot select an LSP entry.

[Bash Language Server installation](https://github.com/bash-lsp/bash-language-server).

### Lua · Lua Language Server

Download the distribution for your operating system from the upstream releases and extract the whole directory. On macOS, `brew install lua-language-server` is another option. Check:

```sh
lua-language-server --version
```

```toml
[[servers]]
name = "lua"
command = "lua-language-server"
args = []
extensions = ["lua"]
language_id = "lua"
root_markers = [".luarc.json", ".luarc.jsonc", ".git"]
```

Set `command` to the extracted `bin/lua-language-server` executable (Windows: `.exe`) if needed. Keep its bundled scripts beside the installation; copying only the executable is insufficient. Configure the project’s Lua version and library paths in `.luarc.json`. Use the separate Luau server for `.luau` files.

[Lua Language Server setup and downloads](https://github.com/LuaLS/lua-language-server/wiki/Getting-Started).

### Luau · luau-lsp

Download and extract the native luau-lsp binary for your platform from the upstream releases. Put it on PATH or use its absolute executable path. Check:

```sh
luau-lsp --help
```

```toml
[[servers]]
name = "luau"
command = "luau-lsp"
args = ["lsp"]
extensions = ["luau"]
language_id = "luau"
root_markers = [".luaurc", ".git"]
```

Use `.luaurc` for language settings and require aliases. The recipe is for ordinary Luau source files. Custom runtimes need their own type definitions and documentation as described upstream. If your Luau project uses `.lua` files, move that extension from the Lua entry to this one.

[luau-lsp client setup and downloads](https://github.com/JohnnyMorganz/luau-lsp/blob/main/editors/README.md).

### HTML, CSS, JSON and JSONC · VS Code language servers

Install Node.js with npm. One package provides the three standalone servers. Check the package after installation:

```sh
npm install -g vscode-langservers-extracted
npm list -g vscode-langservers-extracted --depth=0
```

```toml
[[servers]]
name = "html"
command = "vscode-html-language-server"
args = ["--stdio"]
extensions = ["html", "htm", "xhtml"]
language_id = "html"
root_markers = ["package.json", ".git"]
```

```toml
[[servers]]
name = "css"
command = "vscode-css-language-server"
args = ["--stdio"]
extensions = ["css"]
language_id = "css"
root_markers = ["package.json", ".git"]
```

```toml
[[servers]]
name = "json"
command = "vscode-json-language-server"
args = ["--stdio"]
extensions = ["json"]
language_id = "json"
root_markers = ["package.json", ".git"]
```

```toml
[[servers]]
name = "jsonc"
command = "vscode-json-language-server"
args = ["--stdio"]
extensions = ["jsonc"]
language_id = "jsonc"
root_markers = ["package.json", ".git"]
```

Add only the entries you need. Try hover on an HTML tag or CSS property. JSON hover descriptions depend on a matching JSON Schema; arbitrary JSON keys may have no documentation. JSONC uses its own language identifier. These servers do not replace the JavaScript/TypeScript server for standalone scripts. Definition availability varies by server and document.

[Standalone HTML/CSS/JSON server installation](https://github.com/hrsh7th/vscode-langservers-extracted).

### YAML · YAML Language Server

Install Node.js with npm, then install and check the package:

```sh
npm install -g yaml-language-server
npm list -g yaml-language-server --depth=0
```

```toml
[[servers]]
name = "yaml"
command = "yaml-language-server"
args = ["--stdio"]
extensions = ["yaml", "yml"]
language_id = "yaml"
root_markers = [".git"]
```

Hover is driven by schema descriptions. Associate your file with a schema by adding a comment such as `# yaml-language-server: $schema=./schema.json` at its top, pointing to your actual schema. Relative paths resolve from the YAML file. A plain YAML key without a schema description may return no hover text.

[YAML server installation and schema association](https://github.com/redhat-developer/yaml-language-server).

### TOML · Taplo

With a Rust toolchain installed, build Taplo with its LSP feature enabled, then check that the LSP subcommand exists:

```sh
cargo install taplo-cli --locked --features lsp
taplo lsp --help
```

```toml
[[servers]]
name = "toml"
command = "taplo"
args = ["lsp", "stdio"]
extensions = ["toml"]
language_id = "toml"
root_markers = ["taplo.toml", ".taplo.toml", ".git"]
```

Add Cargo’s executable directory to PATH or use the absolute Taplo path. The npm build does not include the language server. On Linux, building may also require OpenSSL development packages. Configure schema associations through Taplo’s configuration for useful property descriptions; generic TOML keys may have no hover documentation.

[Taplo installation with optional features](https://taplo.tamasfe.dev/cli/installation/cargo.html).

### SQL · sqls

Install Go, then install and check sqls (building its SQLite driver may also require a C compiler):

```sh
go install github.com/sqls-server/sqls@latest
sqls -h
```

```toml
[[servers]]
name = "sql"
command = "sqls"
args = ["-config", "/absolute/path/sqls/config.yml"]
extensions = ["sql"]
language_id = "sql"
root_markers = [".git"]
```

Replace the configuration path with your own sqls YAML file. Follow the upstream database configuration instructions to connect it to your database; schema-aware hover needs that connection. Keep credentials in your local server configuration. Pötyi does not expose sqls query execution, connection switching, or completion. This recipe is primarily useful for hover; do not expect source-file definition navigation for database objects.

[sqls installation and database configuration](https://github.com/sqls-server/sqls).

### Vue: integration status

Pötyi highlights `.vue` files, but the current Vue language server uses custom `tsserver/request` and `tsserver/response` messages to communicate with its TypeScript plugin. Pötyi does not implement that bridge yet, so there is no verified Vue recipe for this client. Do not add `.vue` to the plain JavaScript entry and expect Vue type information. Ordinary `.js`, `.jsx`, `.ts`, and `.tsx` files can use the recipes above. See the [Vue language server integration instructions](https://github.com/vuejs/language-tools/blob/master/packages/language-server/README.md).

## Commands

| Command | Behavior |
| --- | --- |
| `:hover` | Show documentation/type information at the document cursor. |
| `:definition` | Jump to the first definition returned by the server. |
| `:lsp-back` | Return to the previous definition-jump location. |
| `:lsp-status` | Show configuration and background-session count. |
| `:lsp-stop` | Cancel pending work and stop all language-server sessions. |

Commands work through the existing command bar in conventional and Vim modes.
No existing shortcuts are reassigned. Hover information is displayed as plain
text; use Up/Down to scroll and Escape to dismiss it. Markdown returned by a
server is displayed literally. Results are bounded to prevent oversized UI
allocations.

Servers start on the first explicit `:hover` or `:definition` request. The
current unsaved contents of matching documents in both panes are synchronized
before the request. Subsequent requests send a changed range when the server
supports incremental synchronization, or a full update when it requires one.
Typing alone does not trigger synchronization or analysis requests.

There is one session per configured server and project, with at most two
sessions. A configured `.git` marker takes precedence to keep repository files
together; otherwise the nearest matching root marker is used, falling back to
the file's directory. Sessions close documents as panes change and stop when
no matching documents remain open. `:lsp-stop` applies configuration changes
to the next newly started session. Changing `enabled` to false prevents new
requests; use `:lsp-stop` to immediately terminate existing sessions.

## Protecting existing editing behavior

- Pipe I/O, protocol handling and server startup/shutdown run on background
  threads. Explicit requests copy at most 2 MiB per document on the UI thread.
- A response is discarded if either pane's document changes, the requesting
  cursor moves, the file changes, or the command bar is dismissed/edited.
- Definition navigation reuses open buffers, preserving their unsaved text,
  read-only state, selection history and undo history. A new file uses a clean
  pane. If both panes contain unsaved changes, the jump is refused.
- A destination is validated before a new file replaces a pane.
- LSP does not apply server-provided edits, run server-requested commands,
  save documents, change formatting, or alter Vim keybindings.
- Unicode positions are converted between the editor's UTF-8 offsets and
  LSP's UTF-16 positions. File URI escaping handles spaces and Unicode.
- Initialization has a 30-second limit; feature requests have a 15-second
  limit. `:lsp-stop` interrupts waits. An unresponsive server is terminated
  after a brief shutdown attempt; the UI does not wait for it.

## Current limits

This is the first LSP milestone. It does **not** yet display diagnostics,
completion menus, signature help, rename, code actions, semantic highlighting,
or virtual documents such as generated/decompiled definitions. It does not
implement dynamic capability registration or project file watchers.
Open documents must have saved file paths, valid UTF-8, and be at most 2 MiB.
Files outside these limits remain editable using existing Pötyi features.

The editor advertises only the capabilities it implements. Compatibility
depends on the server; servers requiring custom client extensions may need
additional integration. Language servers run as ordinary local programs and
may read project files, index dependencies, or invoke build tooling according
to their configuration. Running them separately keeps analysis off the UI
thread but does not remove their memory/CPU cost.

## Development checks

`cargo test` includes framing/size-limit tests, Unicode and incremental-edit
checks, navigation/data-preservation tests, and (on Unix, with `python3`)
end-to-end tests against a deterministic fixture server. The fixture verifies
unsaved text, full/incremental synchronization, document closing, definition
links, cancellation, server crashes and timeouts.
