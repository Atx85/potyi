# Syntax highlighting

Pötyi includes the following language definitions in the executable. No
language server, formatter, plugin, or separate installation is needed.
The filename extension selects the language automatically, ignoring case.

| Language | Extensions |
| --- | --- |
| C | `.c`, `.h` |
| C++ | `.cc`, `.cpp`, `.cxx`, `.c++`, `.hh`, `.hpp`, `.hxx`, `.h++` |
| C# | `.cs`, `.csx` |
| Python | `.py`, `.pyw`, `.pyi` |
| PHP | `.php`, `.phtml`, `.php3`, `.php4`, `.php5`, `.php7`, `.php8`, `.phps` |
| Rust | `.rs` |
| JavaScript / JSX / Vue | `.js`, `.jsx`, `.mjs`, `.cjs`, `.vue` |
| TypeScript / TSX | `.ts`, `.tsx`, `.mts`, `.cts` |
| Go | `.go` |
| Java | `.java` |
| Shell | `.sh`, `.bash`, `.zsh`, `.ksh`, `.bats` |
| Lua / Luau | `.lua`, `.luau` |
| JSON / JSONC | `.json`, `.jsonc` |
| TOML | `.toml` |
| YAML | `.yaml`, `.yml` |
| HTML | `.html`, `.htm`, `.xhtml` |
| CSS | `.css` |
| SQL | `.sql` |

Definitions color common keywords, strings, comments, numbers, and relevant
language-specific tokens such as PHP variables or C preprocessor directives.
Highlighting and [code formatting](formatters.md) are separate features.

## Resource use and limits

Pötyi compiles regex rules for the selected language when a file's syntax is
loaded. It does not compile unrelated languages' rules during selection or
keep a global cache of all compiled languages. Each open pane holds its own
active syntax definition.

Highlighting runs on visible lines during rendering. There is no background
parser, language server, or whole-document syntax tree. Each line is limited
to the first 16 KiB of text, ending at a complete UTF-8 character, and at most
1,024 candidate matches across its rules. Text without a retained highlight
is drawn normally. These limits affect coloring, not the document contents.

The engine handles each line independently. It does not track multiline
comments, triple-quoted strings, heredocs, or embedded-language boundaries
across lines. String interpolation is colored as part of the string, without
separate expression highlighting. PHP mixed with HTML, JSX/TSX, Vue, and SQL
dialects receive basic lexical highlighting rather than full language parsing.
Files with no recognized extension are displayed as plain text.

## Customize colors or add a language

The [syntax designer](https://atx85.github.io/potyi/designer/) lets you start from a bundled definition,
edit colors and patterns, preview sample code, and download a ready-to-use
TOML file. It runs in your browser and also imports existing definitions.

Run `:extract-config` to create editable definitions under `config/syntax/`.
Existing files are preserved. The directory is relative to the editor's
working directory, as with other Pötyi configuration files.

Edit the existing definition to change its extensions, patterns, or colors,
or add a new TOML file for another language. For example:

```toml
[syntax]
name = "Example"
extensions = ["example"]

[[syntax.rules]]
name = "comment"
pattern = '#.*$'
color = "#6A9955"

[[syntax.rules]]
name = "keyword"
pattern = '\b(begin|end)\b'
color = "#569CD6"
```

Patterns use Rust's `regex` syntax, which does not support look-around or
backreferences. Matches are resolved by their starting position, preferring
the longest at the same position; later overlapping matches are discarded.
For identical spans, the earlier rule wins.

An extracted definition matching the extension takes precedence over the
embedded defaults. Avoid assigning the same extension to multiple custom
files, because their directory order is not guaranteed. Reopen the document
to reload its definition after editing configuration.
