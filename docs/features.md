# Potyi feature inventory

Reviewed against the source tree on 15 September 2026. Packaged releases may
lag behind this inventory.

This inventory counts **51 features and capabilities across eight areas**.
Related commands, shortcuts, flags and safeguards are grouped into a single
capability. Language presets are counted as coverage within a feature.
This is a practical grouping, not a standardized measure of editor completeness.

| Area | Count |
| --- | ---: |
| Files and workspaces | 8 |
| Editing and keyboard modes | 7 |
| Search and navigation | 4 |
| Interface and configuration | 6 |
| Command terminal and Git | 10 |
| Language servers | 10 |
| Formatting | 3 |
| Platforms and resource use | 3 |
| **Total** | **51** |

## Files and workspaces

1. **Create documents:** start an unnamed document or create a named file with `:new`.
2. **Open files by path:** use `:open`, a launch argument, or a path in the command terminal.
3. **Drag and drop files:** open a file by dropping it onto the editor window.
4. **Save and Save As:** save edits or choose a new filename, with explicit overwrite handling.
5. **Read-only viewing:** use the terminal's `view` command to inspect a file without editing it.
6. **Folder workspaces:** `potyi .` or `potyi folder` sets the window's working root and opens its file listing.
7. **Launch at a location:** `potyi file:line:column` opens a file at a particular position.
8. **Crash recovery:** edits are journaled locally; `:recover` lists sessions and restores a separate copy.

Recovery restores document contents through the last complete journal record.
It does not restore the previous undo history, selections or window layout.
See [crash recovery](crash-recovery.md).

## Editing and keyboard modes

9. **Unicode text editing and movement:** insert and delete text, and move by character, word, line or page.
10. **System clipboard editing:** copy, cut and paste text using familiar platform shortcuts.
11. **Undo and redo:** reverse edits and restore them, including grouped typing and bulk operations.
12. **Text selection:** use the mouse or keyboard to select text, words, lines, pages or the whole document.
13. **Multiple cursors and occurrences:** select subsequent matches, edit them together and move or extend each selection.
14. **Vim-style editing:** Normal, Insert and Visual modes, motions, operators, counts, search, clipboard operations and dot repeat.
15. **Emacs-style editing:** movement, marks, kill/yank history, repeat counts, search, file and pane prefixes, and a key reference.

Conventional shortcuts are the default. Vim and Emacs are practical subsets;
they do not include their parent editors' extension languages or macro systems.
Occurrence-based multiple cursors are available in conventional mode.
See the [editing reference](../Readme.md#navigation-and-selection) and
[Emacs reference](emacs.md).

## Search and navigation

16. **Incremental text search:** preview matches while typing, move forward or backward, and control case sensitivity.
17. **Regular-expression search:** search using patterns rather than only literal text.
18. **Find and replace:** replace the current match or all matching occurrences, with undo.
19. **Go to a location:** jump to a line and optional column using absolute, relative or displayed gutter numbers.

Built-in search is within the current document. Project searches can be run
through external tools such as grep or ripgrep in the command terminal.

## Interface and configuration

20. **Discoverable command bar:** browse commands and options, complete them with Tab, and use keyboard or mouse selection.
21. **Two-pane editing:** a fixed vertical split with independent document, cursor, selection, scroll and undo state in each pane.
22. **Syntax highlighting:** automatic extension-based selection from 18 bundled definitions, with editable colors and patterns.
23. **Live editor settings:** change font size, absolute/relative/dynamic line numbers, and keyboard mode during a session.
24. **Custom keyboard shortcuts:** change conventional bindings in `config/keybindings.toml`.
25. **Embedded defaults and configuration export:** run without separate configuration files, or use `:extract-config` to create editable overrides.

Highlighting is lightweight and line-based. Arbitrary pane layouts, document
tabs and a persistent graphical project tree are not provided by the current
two-pane interface. See [syntax highlighting](syntax-highlighting.md).

## Command terminal and Git

26. **Shell command execution:** run non-interactive commands, pipelines and redirects with streaming output and a persistent terminal directory.
27. **Commands directly from the editor:** `:term command` opens the terminal and runs the command immediately.
28. **Clickable directory browsing:** built-in `ls`, `cd` and `pwd`, including aligned columns, file-type colors and parent-folder links.
29. **Filename completion:** Tab and Shift+Tab cycle names from the latest directory listing, with quoting for spaces and shell punctuation.
30. **Create or touch files:** built-in `touch` creates empty files or updates timestamps across supported operating systems.
31. **Clickable output locations:** open filenames, compiler locations and grep/ripgrep results at their reported lines and columns.
32. **Command history:** recall previous commands from the terminal input.
33. **Selectable terminal output:** select and copy output with the mouse or keyboard, with wrapping, scrolling and a Vim navigation subset.
34. **Command controls:** stop running commands or directory scans, rerun the previous command, and clear output.
35. **Git log and diff browsing:** color Git/diff output, click a commit hash to open its diff, and return to the saved log view.

Git, grep and other external tools must be installed separately. This is a
command terminal: full-screen interactive programs, interactive debuggers and
raw terminal applications are not supported. Retained output is capped at 8 MB.
See the [command terminal reference](../Readme.md#command-terminal).

## Language servers

36. **Server installation:** `:lsp install` selects an operating-system-specific recipe for any of 15 supported packages covering 22 language profiles.
37. **Setup diagnostics:** `:lsp doctor` reports installation, prerequisite and project-configuration issues.
38. **Server lifecycle controls:** start, stop, restart and inspect server status, with background communication and timeout handling.
39. **Member completion:** trigger member suggestions with supported access operators and accept a suggestion from the popup.
40. **Hover information:** request documentation and type information at the cursor.
41. **Definition navigation:** jump to a symbol's definition and return to the previous location.
42. **Project-wide symbol rename:** preview changes by file, apply them and undo the workspace edit.
43. **Quick fixes:** review and apply server-provided fixes such as missing imports, includes or using directives.
44. **Reviewed refactoring:** request server-provided refactorings, including supported file operations, and inspect the changes before applying them.
45. **Unity project discovery:** detect Unity project roots and generated `.sln` or `.slnx` solutions for C# server setup.

These capabilities depend on the server, its prerequisites and the project.
LSP is optional and disabled by default. Current LSP requests support saved
UTF-8 documents up to 2 MiB. Completion is triggered by member-access operators;
general completion while typing is not implemented. Diagnostics supply
quick-fix context; there is no diagnostic display or Problems panel yet.
Unity integration still needs broader testing on real projects.
See [LSP setup](lsp-setup.md) and the [LSP reference](lsp.md).

## Formatting

46. **Built-in indentation:** delimiter-based indentation for supported C-style languages, JSON and untitled code, without installing an executable.
47. **External language formatting:** eight bundled formatter presets, automatic installed-provider selection, and custom provider configuration.
48. **Formatter discovery and selection:** `:format` suggestions and `:formatters` show names, file types and availability; `:format builtin` selects the built-in option.

Formatting uses one undo step and does not automatically save. The built-in
option is an indentation tool with limited syntax support; it does not implement
full language formatting. Untitled code uses C-style rules. Unsupported named
file types, including Python, require a language formatter. External formatter
installation is currently manual. See [formatting](formatters.md).

## Platforms and resource use

49. **Native desktop builds:** Windows, Linux, and macOS on Intel and Apple Silicon, with packaged application icons/integration.
50. **File-backed document storage:** piece-table editing and undo avoid a full in-memory text copy when opening a large file; line data is loaded lazily.
51. **Bounded auxiliary storage and quiet idle behavior:** limited caches, file-backed terminal history and an event-driven UI keep resource use predictable.

Low-memory design does not mean every operation has constant memory or constant
latency. Piece metadata and edit history still grow with work; language servers
and external commands have their own resource use. Large-file editing also
has different limits from LSP and formatting.

## Coverage and companion tools

- **Three keyboard modes:** conventional, Vim-style and Emacs-style.
- **18 syntax definitions:** C, C++, C#, Python, PHP, Rust, JavaScript, TypeScript,
  Go, Java, shell, Lua, JSON, TOML, YAML, HTML, CSS and SQL. Some definitions also
  cover related extensions such as JSX, TSX, JSONC and Luau.
- **15 LSP installation packages covering 22 profiles.**
- **Eight external formatter presets:** rustfmt, Ruff, gofmt, clang-format,
  shfmt, StyLua, Taplo and Biome, plus built-in indentation.
- **Companion syntax designer:** the repository also contains a browser tool
  for editing, previewing, importing and exporting syntax definitions. It is
  separate from the 51 editor capabilities counted above.

This list records implemented scope. It is not a claim that each feature is
equally mature or that the editor has met a 1.0 release checklist.
