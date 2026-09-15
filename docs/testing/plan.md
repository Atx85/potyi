# Feature test plan

This plan covers the 51 capabilities in [the feature inventory](../features.md).
It is a feature-level acceptance plan backed by named regression tests, integration
checks, real tools and resource measurements. Repeated shortcuts and language
presets belong to their parent feature.

## Execution and evidence

- Run the ordinary suite serially. Use temporary documents and directories; preserve user files.
- Run each opt-in SDL integration test in its own process with the dummy display driver.
- Run available real-language-server and formatter checks; fixtures are not evidence of live upstream compatibility.
- Run resource probes separately from installation/build workloads. Keep raw timing/RSS samples and source fingerprints.
- Check native mouse/keyboard/clipboard/window behavior separately from headless rendering.
- Verify each native OS/architecture package before declaring cross-platform acceptance.
- Never turn an unexecuted check, child fixture, absent prerequisite or GUI permission failure into a pass.
- A failing assertion or violated acceptance criterion is a failure. Missing environment is unverified. A feature can have passing automated coverage and outstanding native acceptance.

The repeatable local runner is `python3 tools/qa/run.py --output docs/testing/RUN_DATE`.
It performs no downloads. `--skip-live` omits real-tool checks; `--core-log` can reuse
a previously completed core log only when the application source has not changed.
The machine-readable [plan](plan.json) maps each feature to its test prefixes.
See [the execution report](report.md) for actual results and outstanding gates.

## Common fixtures and negative checks

Use UTF-8 with accented characters and emoji; LF and CRLF; an optional BOM;
empty files; no final newline; quoted paths with spaces; missing/read-only paths;
both panes dirty; and fresh temporary directories. Test cancellation and undo
for changes, malformed inputs for parsers, and stale replies for asynchronous work.
Performance probes additionally use long lines, large files, dense matches and
bounded output/history. Platform-specific cases include Windows paths and icons.

## Acceptance cases

### F01 — Create documents

**Procedure:** Create unnamed and named files; try an existing destination and a dirty current pane.

**Expected:** New files start empty; existing files and unsaved edits survive rejected requests.

**Automated evidence:** `tests::new_document`, `command_bar::tests::new_command`.

### F02 — Open files by path

**Procedure:** Open relative, absolute, quoted and missing paths; reopen a file already in the other pane.

**Expected:** Correct file opens once; missing files leave both documents intact.

**Automated evidence:** `startup::tests::`, `tests::open_file_detection`, `tests::terminal_failed_open`, `terminal::tests::standalone_text_paths`.

### F03 — Drag and drop files

**Procedure:** Drop a Unicode filename and a BOM/CRLF C# file onto a native window; also drop a missing path.

**Expected:** Correct file opens, malformed/missing input reports an error, and the app remains running.

**Automated evidence:** `renderer::terminal_selection_render_tests::unity_csharp`, `lsp_ui::tests::unity_missing`.

**Additional gate:** Native drag-and-drop event delivery is not covered by these component tests.

### F04 — Save and Save As

**Procedure:** Save, Save As, overwrite rejection and explicit overwrite; repeat on read-only and dirty documents.

**Expected:** Exact bytes saved; original stays intact after Save As; failed saves retain edits/history.

**Automated evidence:** `tests::save_as`, `tests::new_document_creates`, `piece_table::recovery::tests::successful_saves`.

### F05 — Read-only viewing

**Procedure:** Open with view; attempt typing, deletion, formatting, replacement, cut and Save As.

**Expected:** Mutating commands leave file and document unchanged; selection and navigation still work.

**Automated evidence:** `tests::view_mode`, `tests::save_as_protects`, `vim::tests::read_only`, `multi_cursor::tests::occurrence_empty`.

### F06 — Folder workspaces

**Procedure:** Resolve dot, relative and quoted folders; change terminal directory afterward.

**Expected:** Launch root is correct; terminal directory changes persist without changing the window root.

**Automated evidence:** `startup::tests::folder_launches`, `startup::tests::existing_colon`, `terminal::tests::reopening`.

### F07 — Launch at a location

**Procedure:** Launch file:line:column with Unicode and colon-bearing filenames; try invalid paths.

**Expected:** File/location parses correctly; goto reaches a valid UTF-8 position and errors preserve the buffer.

**Automated evidence:** `startup::tests::file_launches`, `tests::terminal_locations`.

### F08 — Crash recovery

**Procedure:** Kill an editing process, restore, truncate the journal, change original on disk, and attempt failed saves.

**Expected:** Last complete text recovers separately; active sessions and externally modified originals are protected.

**Automated evidence:** `piece_table::recovery::tests::`, `tests::recovery_opens`.

### F09 — Unicode text editing and movement

**Procedure:** Edit Unicode at document/line boundaries; navigate short and long lines, tabs and pages.

**Expected:** No split UTF-8 code points; correct cursor/line state; original bytes remain valid.

**Automated evidence:** `tests::editor_`, `tests::cursor_`, `piece_table::line_view::tests::`.

### F10 — System clipboard editing

**Procedure:** Copy/cut forward and backward selections; paste Unicode and mixed newline styles; test empty/NUL input.

**Expected:** Clipboard operations preserve intended text; paste is one undo step; invalid data is handled.

**Automated evidence:** `clipboard::tests::`.

**Additional gate:** Native clipboard interoperability requires a separate desktop check.

### F11 — Undo and redo

**Procedure:** Group typing, undo and redo; mix formatting, replacements and workspace edits; verify redo invalidation.

**Expected:** Expected text and selection restore; unchanged operations preserve history; storage is reused.

**Automated evidence:** `tests::undo_`, `tests::formatting_`, `tests::replace_`, `workspace_edit::tests::`.

### F12 — Text selection

**Procedure:** Select by word, line and page in both directions; click tabs, Unicode and empty lines.

**Expected:** Selection endpoints stay valid and agree with rendered hit targets.

**Automated evidence:** `renderer::cursor_hit_tests::`, `tests::selection_`, `tests::cursor_move_to_clicked`, `piece_table::line_view::tests::`.

### F13 — Multiple cursors and occurrences

**Procedure:** Add occurrences, edit/cut/paste at all cursors, move/extend them and merge overlaps; undo.

**Expected:** Every selected occurrence changes once; overlapping cursors merge; grouped undo restores text.

**Automated evidence:** `multi_cursor::tests::`, `renderer::terminal_selection_render_tests::occurrence_`.

### F14 — Vim-style editing

**Procedure:** Exercise modes, counts, motions, operators, insertion, visual selections, search, dot repeat and read-only.

**Expected:** Document/cursor/history match expected Vim subset; mode transitions render correctly.

**Automated evidence:** `vim::tests::`, `renderer::terminal_selection_render_tests::occurrence_`.

### F15 — Emacs-style editing

**Procedure:** Exercise marks, movement, prefix counts, kill coalescing, yank cycling, search and file/pane commands.

**Expected:** Correct Unicode movement and actions; kill ring obeys bounds; undo and cancellation work.

**Automated evidence:** `emacs::`.

**Additional gate:** Native keyboard event routing still needs a desktop walkthrough.

### F16 — Incremental text search

**Procedure:** Type a live query; navigate forward/backward, change case sensitivity and cancel.

**Expected:** Matches update without editing document; expected direction/case behavior and cursor restoration.

**Automated evidence:** `search::tests::`, `tests::command_bar_drives_live_search_preview`, `vim::tests::cancelling_search`.

### F17 — Regular-expression search

**Procedure:** Search regex metacharacters, Unicode, zero-width and invalid patterns across buffer boundaries.

**Expected:** Correct match byte ranges; invalid patterns are reported; navigation terminates.

**Automated evidence:** `search::tests::regex`, `tests::regex`, `command_bar::tests::parses_quoted`.

### F18 — Find and replace

**Procedure:** Replace current/all, overlapping and dense matches, multiline/Unicode and empty replacement; undo.

**Expected:** Only intended matches change; replacement text is not reprocessed; one undo restores input.

**Automated evidence:** `tests::replace_`, `tests::dense_replace`, `tests::multiline_replacement`, `tests::no_op_replace`.

### F19 — Go to a location

**Procedure:** Goto absolute/relative/gutter labels with columns in all line-number modes; try ambiguous/out-of-range values.

**Expected:** Correct line and column render; invalid destinations preserve cursor/selection.

**Automated evidence:** `tests::goto_`, `command_bar::tests::goto_`, `line_numbers::tests::`.

### F20 — Discoverable command bar

**Procedure:** Browse every command over multiple pages; Tab, Enter, click, scroll, missing arguments and invalid options.

**Expected:** All entries reachable; completion and execution differ correctly; errors explain the next action.

**Automated evidence:** `command_bar::tests::`.

### F21 — Two-pane editing

**Procedure:** Show split, focus either pane, open an already loaded file, and follow links with both panes dirty.

**Expected:** Independent cursors/history; no duplicate document; no unsaved buffer is discarded by terminal links.

**Automated evidence:** `renderer::cursor_hit_tests::fixed_split`, `tests::terminal_file_click`, `tests::open_file_detection`, `tests::formatting_only`.

### F22 — Syntax highlighting

**Procedure:** Load representative source for all 18 definitions; override colors; test long/dense/Unicode lines.

**Expected:** Expected tokens receive colors, unrelated rules are not compiled, and limits preserve text.

**Automated evidence:** `syntax::tests::`.

### F23 — Live editor settings

**Procedure:** Change font size, line-number mode and keyboard mode; test valid and invalid values and small windows.

**Expected:** Settings parse and apply; cursor hit targets and layouts remain correct; invalid values are rejected.

**Automated evidence:** `config::tests::`, `command_bar::tests::font_`, `command_bar::tests::line_number`, `command_bar::tests::keybinding`, `dpi_text::tests::`.

### F24 — Custom keyboard shortcuts

**Procedure:** Load custom conventional bindings, repeat flags and modifier combinations; submit unknown names.

**Expected:** Correct command resolves; extra modifiers/repeats are rejected as configured; defaults still work.

**Automated evidence:** `keybindings::tests::`.

### F25 — Embedded defaults and configuration export

**Procedure:** Start with absent config, extract defaults twice, and retain an edited override.

**Expected:** Embedded settings work; export creates missing files and does not overwrite custom files.

**Automated evidence:** `embedded_config::tests::`, `config::tests::missing_config`, `keybindings::tests::missing_config`, `syntax::tests::`.

### F26 — Shell command execution

**Procedure:** Run echo, a grep pipeline, redirection and a failing command; change directory; stream large output.

**Expected:** Shell semantics, exit status and persistent directory are correct; output remains responsive and bounded.

**Automated evidence:** `terminal::tests::shell_`, `terminal::tests::commands_skip`, `terminal::tests::submit_runs`, `terminal::tests::output_`.

### F27 — Commands directly from the editor

**Procedure:** Run :term ls and shell pipelines directly; repeat while busy and inspect command history.

**Expected:** Same cwd/execution/history as prompt; busy submission leaves the current command/input intact.

**Automated evidence:** `terminal::tests::term_command_shortcut`, `command_bar::tests::parses_term`.

### F28 — Clickable directory browsing

**Procedure:** List empty, large and nested folders, text/binary files, Unicode and spaces; resize and use -1/-lh.

**Expected:** Column widths fit; links retain exact paths; parent entry works; streaming/cancellation remains correct.

**Automated evidence:** `terminal::tests::default_listing`, `terminal::tests::listing_`, `terminal::tests::ls_`, `terminal::tests::folder_click`, `terminal::tests::parent_link`.

### F29 — Filename completion

**Procedure:** Complete filenames at the cursor, cycle both directions, quote special names and change directory.

**Expected:** Correct cached candidates replace only the current argument; directory-only and cache bounds hold.

**Automated evidence:** `terminal::completion::tests::`, `terminal::tests::completion_refreshes`.

### F30 — Create or touch files

**Procedure:** Touch multiple quoted paths, existing files, -c and invalid destinations.

**Expected:** Expected files/timestamps change without content loss; invalid paths produce useful errors.

**Automated evidence:** `terminal::tests::touch_`.

### F31 — Clickable output locations

**Procedure:** Click grep/compiler locations with Unicode byte columns, spaces, wrapping and after cd.

**Expected:** Correct original file and location open; selection dragging does not activate links.

**Automated evidence:** `terminal::locations::tests::`, `tests::terminal_locations`, `terminal::tests::wrapped_compiler`.

### F32 — Command history

**Procedure:** Recall commands with arrows, submit duplicates and exceed history capacity.

**Expected:** Expected order and draft handling; duplicate suppression and capacity bounds hold.

**Automated evidence:** `terminal::tests::history_`, `terminal::tests::term_command_shortcut`.

### F33 — Selectable terminal output

**Procedure:** Select/copy wrapped output by mouse/keyboard and Vim keys; stream and trim during selection.

**Expected:** Selected bytes remain correct; links activate only on clicks; retained selections survive streaming.

**Automated evidence:** `terminal::selection::tests::`, `renderer::terminal_selection_render_tests::terminal_selection_`.

### F34 — Command controls

**Procedure:** Stop a directory scan/process, rerun and clear; dismiss while output is pending.

**Expected:** Cancelled/stale output cannot corrupt later sessions; controls preserve or clear the intended state.

**Automated evidence:** `terminal::tests::large_listing`, `terminal::tests::output_events`, `terminal::tests::term_command_shortcut`, `lsp_setup::tests::background_runner`.

### F35 — Git log and diff browsing

**Procedure:** Run Git log, click a commit, inspect colored diff and go Back while streaming.

**Expected:** Correct commit/repository; saved log, scroll and draft restore without rerunning; text copies unchanged.

**Automated evidence:** `terminal::git::tests::`, `terminal::colors::tests::`, `renderer::terminal_selection_render_tests::git_commit_`.

### F36 — Server installation

**Procedure:** Validate all recipes/platform mappings; install eligible real servers into temporary locations and initialize every profile.

**Expected:** Aliases and native methods resolve; installation/initialization works; failure/cancel preserves previous records.

**Automated evidence:** `lsp_setup::tests::catalogue_`, `lsp_setup::tests::downloads_`, `lsp_setup::tests::archives_`, `lsp_setup::tests::installation_`, `lsp_setup::tests::npm_`, `lsp_setup::tests::live_install`.

**Additional gate:** Live checks need relevant runtimes and network; OS-specific packages need native platforms.

### F37 — Setup diagnostics

**Procedure:** Run doctor with missing tools/prerequisites and project files; dismiss progress while typing.

**Expected:** Useful diagnosis arrives in background; dismissed/stale panels stay closed; document stays editable.

**Automated evidence:** `lsp_ui::tests::setup_ui_doctor`, `lsp_setup::tests::setup_results`, `lsp_setup::tests::managed_configuration`.

### F38 — Server lifecycle controls

**Procedure:** Start/stop/restart, crash or hang a server, cancel initialization, and deliver stale responses.

**Expected:** Bounded waits and clear errors; healthy loaded sessions survive hover timeout; restart recovers.

**Automated evidence:** `lsp::tests::crashed_`, `lsp::tests::hover_timeout`, `lsp::tests::hung_`, `lsp::tests::a_feature_error`, `lsp_ui::tests::start_enables`.

### F39 — Member completion

**Procedure:** Trigger member completion, choose via keyboard/mouse, apply import edits, undo, and invalidate stale results.

**Expected:** Correct insertion and one undo; Unicode ranges valid; stale replies ignored; popup stays within pane.

**Automated evidence:** `lsp::completion::tests::`, `lsp_ui::tests::completion_`, `renderer::completion::tests::`, `lsp::tests::real_servers_complete`.

### F40 — Hover information

**Procedure:** Hover documented symbols, click another word, dismiss, and time out a request.

**Expected:** Correct information/style; latest target wins; timeout retains initialized server and ignores late reply.

**Automated evidence:** `lsp::tests::parses_hover`, `lsp::tests::hover_`, `lsp_ui::tests::hover_`, `lsp::tests::real_clangd`, `lsp::tests::real_rust_analyzer_hover`.

### F41 — Definition navigation

**Procedure:** Jump to a definition and back with unsaved panes, invalid destinations and Unicode locations.

**Expected:** Correct location; existing buffers/history preserved; failure cannot replace clean or dirty work.

**Automated evidence:** `lsp_ui::tests::navigation_`, `lsp_ui::tests::invalid_definition`, `lsp::tests::real_clangd`, `lsp::tests::real_rust_analyzer_hover`.

### F42 — Project-wide symbol rename

**Procedure:** Rename a symbol from a cold server; preview, apply and undo across open/unopened files.

**Expected:** Declaration and references change together; stale/overlapping edits rejected; undo restores files.

**Automated evidence:** `lsp_ui::tests::rename_`, `workspace_edit::tests::`, `lsp::tests::real_rust_analyzer_hover`.

### F43 — Quick fixes

**Procedure:** Request missing-import quick fixes; select, cancel, invalidate, resolve and apply.

**Expected:** Server-supported fix is correctly previewed/applied; unsupported command-only actions remain unavailable.

**Automated evidence:** `lsp::tests::code_actions_`, `lsp::actions::tests::`, `lsp_ui::tests::action_picker`, `lsp::tests::real_rust_analyzer_extract`.

### F44 — Reviewed refactoring

**Procedure:** Extract a module to a file; review creates/renames/deletes and conflicts; apply and undo.

**Expected:** Only reviewed files change; conflicting/stale operations fail atomically where supported; undo works.

**Automated evidence:** `workspace_edit::tests::`, `lsp_ui::tests::action_picker`, `lsp::tests::real_rust_analyzer_extract`.

### F45 — Unity project discovery

**Procedure:** Use generated sln/slnx/csproj fixtures, BOM/CRLF and a missing server; then repeat on a real Unity project.

**Expected:** Correct solution chosen without Library scan; missing dependencies explain issues without quitting.

**Automated evidence:** `lsp::unity::tests::`, `renderer::terminal_selection_render_tests::unity_`, `lsp_ui::tests::unity_missing`.

**Additional gate:** Real Unity project on Windows/macOS remains a separate acceptance gate.

### F46 — Built-in indentation

**Procedure:** Format nested C-style/JSON, tabs/spaces, Unicode, CRLF, comments and raw strings; reject unsupported/huge input; undo.

**Expected:** Only intended indentation changes; literals/newlines preserved; errors leave text/history untouched.

**Automated evidence:** `formatting::builtin::tests::`, `tests::builtin_formatting`.

### F47 — External language formatting

**Procedure:** Validate all eight presets; run installed tools; exercise missing executable, timeout, bad/empty/large output and undo.

**Expected:** Correct tool/arguments selected; failures preserve document; valid output becomes one undo step.

**Automated evidence:** `formatting::tests::`, `tests::formatting_`.

**Additional gate:** Live language examples for every external tool require those tools to be installed.

### F48 — Formatter discovery and selection

**Procedure:** Type :format with/without a name; filter/scroll/select/Tab; show missing status; execute :formatters.

**Expected:** Automatic Enter remains automatic; explicit choices run intended tool; listing command does not accidentally format.

**Automated evidence:** `command_bar::tests::format_`, `command_bar::tests::formatter_`, `command_bar::tests::formatting_`.

### F49 — Native desktop builds

**Procedure:** Build and launch native packages on Windows, Linux, macOS Intel and Apple Silicon; inspect icons.

**Expected:** Correct architecture starts, fonts/icons load and basic file operations work on each OS.

**Automated evidence:** Native build/launch checks.

**Additional gate:** This machine can verify its local build only; Windows/Linux/Intel/ARM native packaging need separate hosts.

### F50 — File-backed document storage

**Procedure:** Open 1 MiB/100 MiB/1 GB fixtures, access initial lines, edit and undo; inspect backing stores/cache growth.

**Expected:** Correct content and lazy access; inserted/formatted text stays file-backed; no whole-file text copy required.

**Automated evidence:** `piece_table::tests::`, `piece_table::line_view::tests::`, `tests::undo_and_redo`, `benchmarks::editor_performance_probe`.

### F51 — Bounded auxiliary storage and quiet idle behavior

**Procedure:** Measure native idle CPU/RSS, recovery on/off and terminal resource probes; overflow bounded caches.

**Expected:** Idle uses event waits; documented bounds hold; measurements and workload limits are recorded explicitly.

**Automated evidence:** `terminal::frame::tests::`, `terminal::tests::output_is_capped`, `terminal::completion::tests::cache_`, `piece_table::recovery::tests::recovery_uses`, `renderer::terminal_selection_render_tests::terminal_performance_probe`, `piece_table::recovery::tests::recovery_memory_probe`.

**Additional gate:** Measurements characterize this host; no universal performance threshold or cross-platform result is inferred.

