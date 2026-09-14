// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


use crate::config::{
    KeybindingMode,
    LineNumberMode,
    MAX_FONT_SIZE,
    MIN_FONT_SIZE,
};
use crate::search::SearchMode;


pub(crate) const COMMAND_BAR_MARGIN: i32 = 8;
pub(crate) const COMMAND_FONT_SIZE: f32 = 14.0;
pub(crate) const COMMAND_INPUT_HEIGHT: i32 = 36;
pub(crate) const COMMAND_SUGGESTION_HEIGHT: i32 = 28;
const INFO_LINE_HEIGHT: i32 = 20;
const INFO_PARAGRAPH_GAP: i32 = 6;
pub(crate) const MAX_VISIBLE_SUGGESTIONS: usize = 9;
pub(crate) const COMMAND_RUN_WIDTH: i32 = 86;


#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ParsedCommand {
    Find {
        query: String,
        mode: SearchMode,
        backward: bool,
    },

    Replace {
        query: String,
        replacement: String,
        mode: SearchMode,
        all: bool,
    },

    Goto {
        line: isize,
        column: Option<usize>,
        mode: GotoMode,
    },

    Open {
        path: String,
    },

    New {
        path: Option<String>,
    },

    SetFontSize {
        points: u16,
    },

    SetLineNumbers {
        mode: LineNumberMode,
    },

    SetKeybindings {
        mode: KeybindingMode,
    },

    Term,
    Split,
    ExtractConfig,
    Format { provider: Option<String> },
    Formatters,
    Hover,
    Definition,
    Rename { name: String },
    Actions { refactor_only: bool },
    LspBack,
    LspStatus,
    LspInstall { server: String },
    LspDoctor { server: Option<String> },
    LspStart,
    LspRestart,
    LspStop,
    Save,
    SaveAs {
        path: Option<String>,
        overwrite: bool,
    },
    Recover { number: Option<usize> },
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GotoMode {
    Automatic,
    Absolute,
    Relative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandBarHit {
    Outside,
    Input,
    Execute,
    Suggestion(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SuggestionAction {
    CompleteCommand(&'static str),
    SetInput(&'static str),
    ToggleOption(&'static str),
    FormatProvider(usize),
    LspRecipe { install: bool, index: usize },
    Review(usize),
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CommandSuggestion<'a> {
    pub label: &'a str,
    pub description: &'static str,
    pub active: bool,
    action: SuggestionAction,
}

#[derive(Clone, Copy)]
struct CommandSpec {
    name: &'static str,
    description: &'static str,
    usage: &'static str,
}

#[derive(Clone, Copy)]
struct OptionSpec {
    name: &'static str,
    description: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandWord {
    text: String,
    quoted: bool,
}

const COMMANDS: &[CommandSpec] = &[


    CommandSpec {
        name: "find",
        description: "Find text in the document",
        usage: ":find text [--ignore-case|--regex|--backward]",
    },
    CommandSpec {
        name: "replace",
        description: "Replace a match or every match",
        usage: ":replace old new [--all|--ignore-case|--regex]",
    },
    CommandSpec {
        name: "goto",
        description: "Gutter label, +N down, or -N up; optional :column",
        usage: ":goto line[:column] [--abs|--rel]",
    },
    CommandSpec {
        name: "open",
        description: "Open a file by path",
        usage: ":open path",
    },
    CommandSpec {
        name: "new",
        description: "Create a file or an unnamed document",
        usage: ":new [path]",
    },
    CommandSpec {
        name: "term",
        description: "Open the command terminal",
        usage: ":term",
    },
    CommandSpec {
        name: "split",
        description: "Toggle the fixed two-pane view",
        usage: ":split",
    },
    CommandSpec {
        name: "set",
        description: "Change an editor setting",
        usage: ":set font-size points | :set line-numbers type | :set keybindings mode",
    },
    CommandSpec {
        name: "save",
        description: "Save the current document",
        usage: ":save",
    },
    CommandSpec {
        name: "format",
        description: "Format the focused document now",
        usage: ":format [provider]",
    },
    CommandSpec {
        name: "formatters",
        description: "Show matching formatters and availability",
        usage: ":formatters",
    },
    CommandSpec {
        name: "save-as",
        description: "Save under a new name; add ! to overwrite",
        usage: ":save-as[!] path",
    },
    CommandSpec {
        name: "saveas",
        description: "Vim Save As (also :sav); add ! to overwrite",
        usage: ":saveas[!] path",
    },
    CommandSpec {
        name: "extract-config",
        description: "Create editable copies of built-in defaults",
        usage: ":extract-config",
    },
    CommandSpec {
        name: "quit",
        description: "Close Pötyi",
        usage: ":quit",
    },
    CommandSpec { name: "recover", description: "List unsaved sessions or open a recovered copy", usage: ":recover [number]" },
    CommandSpec { name: "lsp", description: "Language-server help, navigation, rename and refactoring", usage: ":lsp <command>" },
];

const LSP_COMMANDS: &[CommandSpec] = &[
    CommandSpec { name: "lsp install", description: "Install and configure a language server for this computer", usage: ":lsp install <server>" },
    CommandSpec { name: "lsp doctor", description: "Check language servers and missing prerequisites", usage: ":lsp doctor [server]" },
    CommandSpec { name: "lsp start", description: "Enable LSP for this window and connect to this file's server", usage: ":lsp start" },
    CommandSpec { name: "lsp restart", description: "Reload configuration and reconnect language servers", usage: ":lsp restart" },
    CommandSpec { name: "lsp hover", description: "Show language-server help at the cursor", usage: ":lsp hover" },
    CommandSpec { name: "lsp definition", description: "Go to the definition at the cursor", usage: ":lsp definition" },
    CommandSpec { name: "lsp rename", description: "Preview a symbol rename across the project", usage: ":lsp rename new_name" },
    CommandSpec { name: "lsp actions", description: "Choose a quick fix or refactoring at the cursor or selection", usage: ":lsp actions" },
    CommandSpec { name: "lsp refactor", description: "Choose a refactoring, such as moving a type or module to a file", usage: ":lsp refactor" },
    CommandSpec { name: "lsp back", description: "Return from a definition jump", usage: ":lsp back" },
    CommandSpec { name: "lsp status", description: "Show language-server configuration and sessions", usage: ":lsp status" },
    CommandSpec { name: "lsp stop", description: "Stop all language servers", usage: ":lsp stop" },
];

const FIND_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "--case-sensitive",
        description: "Match letter case (default)",
    },
    OptionSpec {
        name: "--ignore-case",
        description: "Ignore differences in letter case",
    },
    OptionSpec {
        name: "--regex",
        description: "Treat the query as a regular expression",
    },
    OptionSpec {
        name: "--backward",
        description: "Search toward the start of the file",
    },
];

const REPLACE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "--case-sensitive",
        description: "Match letter case (default)",
    },
    OptionSpec {
        name: "--ignore-case",
        description: "Ignore differences in letter case",
    },
    OptionSpec {
        name: "--regex",
        description: "Treat the query as a regular expression",
    },
    OptionSpec {
        name: "--all",
        description: "Replace every non-overlapping match",
    },
];

const GOTO_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "--abs",
        description: "Treat the line as an absolute line number",
    },
    OptionSpec {
        name: "--rel",
        description: "Force an offset: positive down, negative up",
    },
];

const FONT_SIZE_CHOICES: &[OptionSpec] = &[
    OptionSpec {
        name: "12",
        description: "Compact text",
    },
    OptionSpec {
        name: "14",
        description: "Small text",
    },
    OptionSpec {
        name: "16",
        description: "Medium text",
    },
    OptionSpec {
        name: "18",
        description: "Default text size",
    },
    OptionSpec {
        name: "20",
        description: "Large text",
    },
    OptionSpec {
        name: "24",
        description: "Larger text",
    },
    OptionSpec {
        name: "32",
        description: "Extra-large text",
    },
];

const LINE_NUMBER_CHOICES: &[OptionSpec] = &[
    OptionSpec {
        name: "normal",
        description: "Show absolute line numbers",
    },
    OptionSpec {
        name: "relative",
        description: "Show distance from the cursor line",
    },
    OptionSpec {
        name: "dynamic",
        description: "Keep the cursor line absolute",
    },
];

const KEYBINDING_CHOICES: &[OptionSpec] = &[
    OptionSpec {
        name: "conventional",
        description: "Use conventional editor shortcuts",
    },
    OptionSpec {
        name: "vim",
        description: "Use Vim-like modal keybindings",
    },
];


pub(crate) struct CommandBar {
    active: bool,
    input: String,
    cursor: usize,
    selected: usize,
    selection_explicit: bool,
    status: Option<String>,
    formatter_choices: Vec<(String, bool)>,
    review_choices: Vec<String>,
    info_lines: Vec<String>,
    info_documentation: Vec<bool>,
    info_offset: usize,
    epoch: u64,
}

impl CommandBar {
    pub fn new() -> Self {
        Self {
            active: false,
            input: String::new(),
            cursor: 0,
            selected: 0,
            selection_explicit: false,
            status: None,
            formatter_choices: Vec::new(),
            review_choices: Vec::new(),
            info_lines: Vec::new(),
            info_documentation: Vec::new(),
            info_offset: 0,
            epoch: 0,
        }
    }

    pub(crate) fn is_info(&self) -> bool { !self.info_lines.is_empty() }

    pub(crate) fn epoch(&self) -> u64 { self.epoch }

    pub(crate) fn show_info(&mut self, text: &str) {
        self.show_info_styled(text, &[]);
    }

    pub(crate) fn show_hover(&mut self, hover: &crate::lsp::HoverContent) {
        self.show_info_styled(&hover.text, &hover.documentation);
    }

    pub(crate) fn info_is_documentation(&self, index: usize) -> bool {
        self.is_info() && self.info_documentation.get(self.info_offset + index).copied().unwrap_or(false)
    }

    fn show_info_styled(&mut self, text: &str, documentation: &[std::ops::Range<usize>]) {
        self.review_choices.clear();
        self.info_documentation.clear();
        self.info_lines.clear();
        self.info_offset = 0;
        self.selected = 0;
        self.selection_explicit = false;
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            let end = offset + line.len();
            let is_documentation = documentation.iter().any(|range| range.start < end && range.end > offset);
            offset = end;
            let line = line.trim_end_matches(['\r', '\n']).replace('\t', "    ");
            let chars: Vec<_> = line.chars().filter(|c| !c.is_control()).take(4096).collect();
            if chars.iter().all(|c| c.is_whitespace()) {
                if self.info_lines.last().is_some_and(|line| !line.is_empty()) {
                    self.info_lines.push(String::new());
                    self.info_documentation.push(false);
                }
            } else {
                for chunk in chars.chunks(64) {
                    self.info_lines.push(chunk.iter().collect());
                    self.info_documentation.push(is_documentation);
                }
            }
            if self.info_lines.len() >= 256 {
                self.info_lines.truncate(256);
                self.info_documentation.truncate(256);
                break;
            }
        }
        if self.info_lines.last().is_some_and(|line| line.is_empty()) {
            self.info_lines.pop();
            self.info_documentation.pop();
        }
        self.set_status("Up/Down scroll; Escape closes");
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn selected(&self) -> usize {
        self.selected.saturating_sub(self.suggestion_offset())
    }

    fn suggestion_offset(&self) -> usize {
        if self.is_info() { 0 } else { self.selected / MAX_VISIBLE_SUGGESTIONS * MAX_VISIBLE_SUGGESTIONS }
    }

    fn total_suggestion_count(&self) -> usize {
        (0..).take_while(|index| self.raw_suggestion(*index).is_some()).count()
    }

    pub fn navigation_hint(&self) -> Option<String> {
        let total = self.total_suggestion_count();
        (total > MAX_VISIBLE_SUGGESTIONS && !self.is_info()).then(|| format!(
            "{}–{} of {total} · ↑↓/wheel · Enter selects",
            self.suggestion_offset() + 1,
            self.suggestion_offset() + self.suggestion_count(),
        ))
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn set_status(
        &mut self,
        status: impl Into<String>,
    ) {
        self.status = Some(status.into());
    }

    pub(crate) fn show_review(&mut self, choices: Vec<String>) {
        self.info_lines.clear();
        self.info_documentation.clear();
        self.selected = 0;
        self.selection_explicit = false;
        self.review_choices = choices;
        self.set_status("Select a file to review; Apply changes commits; Escape cancels");
    }

    pub(crate) fn review_selection(&self) -> Option<usize> {
        match self.suggestion(self.selected()).map(|s| s.action) {
            Some(SuggestionAction::Review(index)) => Some(index),
            _ => None,
        }
    }

    pub fn show_formatters(&mut self, choices: Vec<(String, bool)>) {
        self.selected = 0;
        self.selection_explicit = false;
        self.status = Some(if choices.is_empty() {
            "No matching formatters configured".to_string()
        } else {
            "Up/Down or wheel to choose a formatter; Enter runs".to_string()
        });
        self.formatter_choices = choices;
    }

    pub fn open(
        &mut self,
        initial: &str,
    ) {
        self.active = true;
        self.formatter_choices = Vec::new();
        self.input.clear();

        if initial.starts_with(':') {
            self.input.push_str(initial);
        } else {
            self.input.push(':');
            self.input.push_str(initial);
        }

        self.cursor = self.input.len();
        self.selected = 0;
        self.selection_explicit = false;
        self.status = None;
        self.info_lines.clear();
        self.review_choices.clear();
        self.epoch = self.epoch.wrapping_add(1);
    }

    pub fn close(&mut self) {
        self.active = false;
        self.formatter_choices = Vec::new();
        self.status = None;
        self.info_lines.clear();
        self.review_choices.clear();
        self.epoch = self.epoch.wrapping_add(1);
    }

    pub fn insert_text(
        &mut self,
        text: &str,
    ) {
        if !self.active || text.is_empty() {
            return;
        }

        if text.chars().any(|character| {
            matches!(character, '\n' | '\r' | '\t')
        }) {
            let visible =
                quote_control_characters(text);

            self.input.insert_str(
                self.cursor,
                &visible,
            );

            self.cursor += visible.len();
        } else {
            self.input.insert_str(
                self.cursor,
                text,
            );

            self.cursor += text.len();
        }
        self.selected = 0;
        self.selection_explicit = false;
        self.status = None;
        self.info_lines.clear();
        self.review_choices.clear();
        self.epoch = self.epoch.wrapping_add(1);
    }

    pub fn backspace(&mut self) {
        if !self.active || self.cursor <= 1 {
            return;
        }

        let previous =
            previous_char_boundary(
                &self.input,
                self.cursor,
            );

        self.input.drain(
            previous..self.cursor
        );

        self.cursor = previous;
        self.selected = 0;
        self.selection_explicit = false;
        self.status = None;
        self.info_lines.clear();
        self.review_choices.clear();
        self.epoch = self.epoch.wrapping_add(1);
    }

    pub fn delete(&mut self) {
        if !self.active
            || self.cursor >= self.input.len()
        {
            return;
        }

        let next =
            next_char_boundary(
                &self.input,
                self.cursor,
            );

        self.input.drain(
            self.cursor..next
        );

        self.selected = 0;
        self.selection_explicit = false;
        self.status = None;
        self.info_lines.clear();
        self.review_choices.clear();
        self.epoch = self.epoch.wrapping_add(1);
    }

    pub fn move_left(&mut self) {
        if self.active && self.cursor > 1 {
            self.cursor =
                previous_char_boundary(
                    &self.input,
                    self.cursor,
                );
        }
    }

    pub fn move_right(&mut self) {
        if self.active
            && self.cursor < self.input.len()
        {
            self.cursor =
                next_char_boundary(
                    &self.input,
                    self.cursor,
                );
        }
    }

    pub fn move_home(&mut self) {
        if self.active {
            self.cursor = 1.min(
                self.input.len()
            );
        }
    }

    pub fn move_end(&mut self) {
        if self.active {
            self.cursor = self.input.len();
        }
    }

    pub fn set_cursor(
        &mut self,
        position: usize,
    ) {
        if !self.active {
            return;
        }

        let mut position =
            position
                .max(1.min(self.input.len()))
                .min(self.input.len());

        while position > 0
            && !self.input
                .is_char_boundary(position)
        {
            position -= 1;
        }

        self.cursor = position;
    }

    pub fn move_selection(
        &mut self,
        direction: isize,
    ) {
        if !self.info_lines.is_empty() {
            self.info_offset = if direction < 0 { self.info_offset.saturating_sub(1) }
                else { (self.info_offset + 1).min(self.info_lines.len().saturating_sub(MAX_VISIBLE_SUGGESTIONS)) };
            return;
        }
        let count = self.total_suggestion_count();

        if count == 0 {
            self.selected = 0;
            self.selection_explicit = false;
            return;
        }

        self.selected =
            if direction < 0 {
                self.selected
                    .checked_sub(1)
                    .unwrap_or(count - 1)
            } else {
                (self.selected + 1) % count
            };
        self.selection_explicit = true;
    }

    pub fn scroll_suggestions(&mut self, direction: isize) {
        if self.is_info() {
            self.move_selection(direction);
            return;
        }
        self.selected = self.selected.saturating_add_signed(direction)
            .min(self.total_suggestion_count().saturating_sub(1));
        self.selection_explicit = true;
    }

    pub fn select_suggestion(
        &mut self,
        index: usize,
    ) -> bool {
        if index < self.suggestion_count() {
            let index = self.suggestion_offset() + index;
            let changed = self.selected != index;
            self.selected = index;
            self.selection_explicit = true;
            changed
        } else {
            false
        }
    }

    pub fn apply_selected(&mut self) -> bool {
        self.apply_suggestion(
            self.selected()
        )
    }

    // Enter accepts the highlighted completion before parsing the command.
    // Commands needing arguments leave the completed usage prompt open.
    pub fn selects_option_on_enter(&self) -> bool {
        self.selection_explicit && matches!(
            self.suggestion(self.selected()).map(|suggestion| suggestion.action),
            Some(SuggestionAction::ToggleOption(_)),
        )
    }

    pub fn prepare_execute(&mut self) -> bool {
        if self.is_info() {
            return matches!(self.parse(), Ok(ParsedCommand::Hover | ParsedCommand::Definition | ParsedCommand::Rename { .. } | ParsedCommand::Actions { .. } | ParsedCommand::LspStart | ParsedCommand::LspRestart | ParsedCommand::LspStatus | ParsedCommand::Recover { number: None }));
        }
        let action = self.suggestion(self.selected()).map(|suggestion| suggestion.action);
        match action {
            Some(SuggestionAction::CompleteCommand(_) | SuggestionAction::SetInput(_)
                | SuggestionAction::FormatProvider(_) | SuggestionAction::LspRecipe { .. }) => {
                self.apply_selected();
                self.parse().is_ok()
            }
            Some(SuggestionAction::ToggleOption(_)) if self.selection_explicit => {
                self.apply_selected();
                false
            }
            _ => true,
        }
    }

    pub fn apply_suggestion(
        &mut self,
        index: usize,
    ) -> bool {
        let Some(suggestion) =
            self.suggestion(index)
        else {
            return false;
        };

        match suggestion.action {
            SuggestionAction::CompleteCommand(
                command
            ) => {
                self.input.clear();
                self.input.push(':');
                self.input.push_str(command);
                self.input.push(' ');
            }

            SuggestionAction::SetInput(input) => {
                self.input.clear();
                self.input.push_str(input);
            }

            SuggestionAction::ToggleOption(
                option
            ) => {
                self.toggle_option(option);
            }

            SuggestionAction::LspRecipe { install, index } => {
                self.input = format!(":lsp {} {}", if install { "install" } else { "doctor" }, crate::lsp_setup::catalog::RECIPES[index].id);
            }

            SuggestionAction::FormatProvider(index) => {
                self.input = format!(":format {}", self.formatter_choices[index].0);
            }

            SuggestionAction::None | SuggestionAction::Review(_) => {
                return false;
            }
        }

        self.cursor = self.input.len();
        self.selected = 0;
        self.selection_explicit = false;
        self.status = None;
        self.info_lines.clear();
        self.review_choices.clear();
        self.epoch = self.epoch.wrapping_add(1);

        self.formatter_choices = Vec::new();

        true
    }

    pub fn suggestion_count(&self) -> usize {
        (0..MAX_VISIBLE_SUGGESTIONS)
            .take_while(|index| {
                self.suggestion(*index)
                    .is_some()
            })
            .count()
    }

    pub fn suggestion(
        &self,
        visible_index: usize,
    ) -> Option<CommandSuggestion<'_>> {
        if visible_index >= MAX_VISIBLE_SUGGESTIONS { return None; }
        self.raw_suggestion(self.suggestion_offset() + visible_index)
    }

    fn raw_suggestion(&self, visible_index: usize) -> Option<CommandSuggestion<'_>> {
        if !self.info_lines.is_empty() {
            return self.info_lines.get(self.info_offset + visible_index).map(|line| CommandSuggestion {
                label: line, description: "", active: false, action: SuggestionAction::None,
            });
        }
        if !self.review_choices.is_empty() {
            return self.review_choices.get(visible_index).map(|label| CommandSuggestion {
                label, description: "", active: false, action: SuggestionAction::Review(visible_index),
            });
        }
        if self.input.trim() == ":formatters" && !self.formatter_choices.is_empty() {
            return self.formatter_choices.get(visible_index).map(|(name, available)| CommandSuggestion {
                label: name,
                description: if *available { "Available; select to format" } else { "Missing; install or configure executable" },
                active: false,
                action: SuggestionAction::FormatProvider(visible_index),
            });
        }
        let body =
            self.input
                .strip_prefix(':')
                .unwrap_or(&self.input)
                .trim_start();

        let first_end =
            body.find(char::is_whitespace)
                .unwrap_or(body.len());

        let command = &body[..first_end];
        let has_arguments = first_end < body.len();

        if !has_arguments {
            return COMMANDS.iter()
                .filter(|spec| {
                    spec.name.starts_with(command)
                })
                .nth(visible_index)
                .map(|spec| {
                    CommandSuggestion {
                        label: spec.name,
                        description:
                            spec.description,
                        active: false,
                        action:
                            SuggestionAction::CompleteCommand(
                                spec.name
                            ),
                    }
                });
        }

        if command == "lsp" {
            return lsp_suggestion(&body[first_end..], visible_index);
        }

        if command == "set" {
            return setting_suggestion(
                body,
                visible_index,
            );
        }

        let command = match command {
            "save-as!" => "save-as",
            "sav" | "savea" | "sav!" | "savea!" | "saveas!" => "saveas",
            "w" | "write" => "save",
            _ => command,
        };

        let options =
            match command {
                "find" => FIND_OPTIONS,
                "replace" => REPLACE_OPTIONS,
                "goto" => GOTO_OPTIONS,
                _ => {
                    return COMMANDS.iter()
                        .find(|spec| {
                            spec.name == command
                        })
                        .filter(|_| {
                            visible_index == 0
                        })
                        .map(|spec| {
                            CommandSuggestion {
                                label: spec.usage,
                                description:
                                    "Press Enter to run",
                                active: false,
                                action:
                                    SuggestionAction::None,
                            }
                        });
                }
            };

        let option_prefix =
            body.split_whitespace()
                .last()
                .filter(|word| {
                    word.starts_with("--")
                })
                .unwrap_or("");

        options.iter()
            .filter(|option| {
                option.name.starts_with(
                    option_prefix
                )
            })
            .nth(visible_index)
            .map(|option| {
                CommandSuggestion {
                    label: option.name,
                    description:
                        option.description,
                    active:
                        has_token(
                            &self.input,
                            option.name,
                        ),
                    action:
                        SuggestionAction::ToggleOption(
                            option.name
                        ),
                }
            })
    }

    pub(crate) fn suggestion_row_height(&self, index: usize) -> i32 {
        if self.is_info() {
            if self.suggestion(index).is_some_and(|line| line.label.is_empty()) {
                INFO_PARAGRAPH_GAP
            } else { INFO_LINE_HEIGHT }
        } else { COMMAND_SUGGESTION_HEIGHT }
    }

    pub(crate) fn suggestion_row_offset(&self, index: usize) -> i32 {
        (0..index.min(self.suggestion_count())).map(|i| self.suggestion_row_height(i)).sum()
    }

    pub(crate) fn suggestions_height(&self) -> i32 {
        self.suggestion_row_offset(self.suggestion_count())
    }

    pub fn panel_height(&self) -> i32 {
        if !self.active {
            return 0;
        }

        COMMAND_INPUT_HEIGHT + self.suggestions_height()
    }

    pub fn reserved_height(&self) -> i32 {
        if self.active {
            self.panel_height()
                + COMMAND_BAR_MARGIN
        } else {
            0
        }
    }

    pub fn hit_test(
        &self,
        window_width: i32,
        window_height: i32,
        x: i32,
        y: i32,
    ) -> CommandBarHit {
        if !self.active {
            return CommandBarHit::Outside;
        }

        let left = COMMAND_BAR_MARGIN;
        let right =
            window_width - COMMAND_BAR_MARGIN;

        let top =
            window_height
                - COMMAND_BAR_MARGIN
                - self.panel_height();

        let suggestion_bottom =
            top + self.suggestions_height();

        let bottom =
            suggestion_bottom
                + COMMAND_INPUT_HEIGHT;

        if x < left
            || x >= right
            || y < top
            || y >= bottom
        {
            return CommandBarHit::Outside;
        }

        if y >= suggestion_bottom {
            return if x
                >= right - COMMAND_RUN_WIDTH
            {
                CommandBarHit::Execute
            } else {
                CommandBarHit::Input
            };
        }

        for index in 0..self.suggestion_count() {
            if y < top + self.suggestion_row_offset(index + 1) {
                return CommandBarHit::Suggestion(index);
            }
        }
        CommandBarHit::Outside
    }

    pub fn parse(&self) -> Result<ParsedCommand, String> {
        parse_command(&self.input)
    }

    fn toggle_option(
        &mut self,
        option: &'static str,
    ) {
        if remove_token(
            &mut self.input,
            option,
        ) {
            return;
        }

        remove_partial_option(
            &mut self.input
        );

        if matches!(
            option,
            "--case-sensitive"
                | "--ignore-case"
                | "--regex"
        ) {
            for other in [
                "--case-sensitive",
                "--ignore-case",
                "--regex",
            ] {
                remove_token(
                    &mut self.input,
                    other,
                );
            }
        }

        if matches!(option, "--abs" | "--rel") {
            remove_token(
                &mut self.input,
                if option == "--abs" {
                    "--rel"
                } else {
                    "--abs"
                },
            );
        }

        if !self.input.chars()
            .next_back()
            .map(char::is_whitespace)
            .unwrap_or(false)
        {
            self.input.push(' ');
        }

        self.input.push_str(option);
        self.input.push(' ');
    }
}

fn lsp_suggestion(tail: &str, index: usize) -> Option<CommandSuggestion<'static>> {
    let tail = tail.trim_start();
    let end = tail.find(char::is_whitespace).unwrap_or(tail.len());
    let subcommand = &tail[..end];
    if end < tail.len() && matches!(subcommand, "install" | "doctor") {
        let query = tail[end..].trim().to_ascii_lowercase();
        if subcommand == "doctor" && query.is_empty() {
            return (index == 0).then_some(CommandSuggestion { label: ":lsp doctor [server]", description: "Check this file, or type a server name", active: false, action: SuggestionAction::None });
        }
        return crate::lsp_setup::catalog::RECIPES.iter().enumerate()
            .filter(|(_, recipe)| recipe.id.starts_with(&query) || recipe.aliases.iter().any(|a| a.starts_with(&query)))
            .nth(index).map(|(index, recipe)| CommandSuggestion {
                label: recipe.id, description: recipe.title, active: false,
                action: SuggestionAction::LspRecipe { install: subcommand == "install", index },
            });
    }
    if end == tail.len() {
        LSP_COMMANDS.iter().filter(|spec| spec.name.strip_prefix("lsp ").unwrap().starts_with(subcommand))
            .nth(index).map(|spec| CommandSuggestion {
                label: spec.name, description: spec.description, active: false,
                action: SuggestionAction::CompleteCommand(spec.name),
            })
    } else {
        LSP_COMMANDS.iter().find(|spec| spec.name.strip_prefix("lsp ") == Some(subcommand))
            .filter(|_| index == 0).map(|spec| CommandSuggestion {
                label: spec.usage, description: "Press Enter to run", active: false,
                action: SuggestionAction::None,
            })
    }
}

fn setting_suggestion(
    body: &str,
    visible_index: usize,
) -> Option<CommandSuggestion<'_>> {
    let setting =
        body.strip_prefix("set")?
            .trim_start();

    let name_end = setting
        .find(char::is_whitespace)
        .unwrap_or(setting.len());

    let name = &setting[..name_end];

    if name_end == setting.len() {
        return [
            CommandSuggestion {
                label: "font-size",
                description:
                    "Set editor text size in points",
                active: false,
                action:
                    SuggestionAction::SetInput(
                        ":set font-size ",
                    ),
            },
            CommandSuggestion {
                label: "line-numbers",
                description:
                    "Choose the line number style",
                active: false,
                action:
                    SuggestionAction::SetInput(
                        ":set line-numbers ",
                    ),
            },
            CommandSuggestion {
                label: "keybindings",
                description:
                    "Choose conventional or Vim-like keys",
                active: false,
                action:
                    SuggestionAction::SetInput(
                        ":set keybindings ",
                    ),
            },
        ]
        .into_iter()
        .filter(|suggestion| {
            suggestion.label.starts_with(name)
        })
        .nth(visible_index);
    }

    let value = setting[name_end..]
        .trim_start();

    let choices = match name {
        "font-size" => FONT_SIZE_CHOICES,
        "line-numbers" => LINE_NUMBER_CHOICES,
        "keybindings" => KEYBINDING_CHOICES,
        _ => return None,
    };

    choices.iter()
        .filter(|choice| {
            choice.name.starts_with(value)
        })
        .nth(visible_index)
        .map(|choice| {
            let input =
                match (name, choice.name) {
                    ("font-size", "12") => ":set font-size 12",
                    ("font-size", "14") => ":set font-size 14",
                    ("font-size", "16") => ":set font-size 16",
                    ("font-size", "18") => ":set font-size 18",
                    ("font-size", "20") => ":set font-size 20",
                    ("font-size", "24") => ":set font-size 24",
                    ("font-size", "32") => ":set font-size 32",
                    ("line-numbers", "normal") =>
                        ":set line-numbers normal",
                    ("line-numbers", "relative") =>
                        ":set line-numbers relative",
                    ("line-numbers", "dynamic") =>
                        ":set line-numbers dynamic",
                    ("keybindings", "conventional") =>
                        ":set keybindings conventional",
                    ("keybindings", "vim") =>
                        ":set keybindings vim",
                    _ => unreachable!(),
                };

            CommandSuggestion {
                label: choice.name,
                description:
                    choice.description,
                active: value == choice.name,
                action:
                    SuggestionAction::SetInput(
                        input,
                    ),
            }
        })
}


pub(crate) fn quote_argument(
    value: &str,
) -> String {
    if !value.is_empty()
        && !value.starts_with("--")
        && !value.chars().any(|character| {
        character.is_whitespace()
            || matches!(
                character,
                '"' | '\\'
            )
        })
    {
        return value.to_owned();
    }

    let mut quoted =
        String::with_capacity(
            value.len() + 2
        );

    quoted.push('"');

    for character in value.chars() {
        match character {
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),

            '"' | '\\' => {
                quoted.push('\\');
                quoted.push(character);
            }

            _ => quoted.push(character),
        }
    }

    quoted.push('"');
    quoted
}

fn parse_command(
    input: &str,
) -> Result<ParsedCommand, String> {
    let mut words = tokenize(input)?;
    if words.first().is_some_and(|word| word.text == "lsp") {
        let subcommand = words.get(1).ok_or("Choose an LSP command: install, doctor, start, restart, hover, definition, rename, actions, refactor, back, status or stop")?;
        let alias = match subcommand.text.as_str() {
            "hover" => "hover", "definition" => "definition", "rename" => "rename",
            "actions" => "actions", "refactor" => "refactor",
            "back" => "lsp-back", "status" => "lsp-status", "stop" => "lsp-stop",
            "start" => "lsp-start", "restart" => "lsp-restart",
            "install" => "lsp-install", "doctor" => "lsp-doctor",
            _ => return Err("Unknown LSP command. Choose install, doctor, start, restart, hover, definition, rename, actions, refactor, back, status or stop".into()),
        };
        words.remove(1);
        words[0].text = alias.into();
    }

    let Some(command_word) = words.first()
    else {
        return Err(
            "Type a command or choose one below"
                .to_string()
        );
    };

    let command = &command_word.text;

    let mut arguments = Vec::new();
    let mut mode = SearchMode::CaseSensitive;
    let mut backward = false;
    let mut all = false;
    let mut search_option_used = false;
    let mut goto_mode = GotoMode::Automatic;

    for word in &words[1..] {
        match (word.quoted, word.text.as_str()) {
            (false, "--case-sensitive") => {
                mode = SearchMode::CaseSensitive;
                search_option_used = true;
            }

            (false, "--ignore-case") => {
                mode = SearchMode::CaseInsensitive;
                search_option_used = true;
            }

            (false, "--regex") => {
                mode = SearchMode::Regex;
                search_option_used = true;
            }

            (false, "--backward") =>
                backward = true,
            (false, "--all") => all = true,
            (false, "--abs") => {
                if goto_mode == GotoMode::Relative {
                    return Err(
                        "--abs and --rel cannot be used together"
                            .to_string()
                    );
                }

                goto_mode = GotoMode::Absolute;
            }
            (false, "--rel") => {
                if goto_mode == GotoMode::Absolute {
                    return Err(
                        "--abs and --rel cannot be used together"
                            .to_string()
                    );
                }

                goto_mode = GotoMode::Relative;
            }

            (false, option)
                if option.starts_with("--") =>
            {
                return Err(
                    format!(
                        "Unknown option: {option}"
                    )
                );
            }

            _ => arguments.push(
                word.text.clone()
            ),
        }
    }

    if command != "goto"
        && goto_mode != GotoMode::Automatic
    {
        return Err(
            "--abs and --rel are only available for goto"
                .to_string()
        );
    }

    match command.as_str() {
        "find" => {
            if all {
                return Err(
                    "--all is only available for replace"
                        .to_string()
                );
            }

            let query = arguments.join(" ");

            if query.is_empty() {
                return Err(
                    "Usage: :find text [options]"
                        .to_string()
                );
            }

            Ok(ParsedCommand::Find {
                query,
                mode,
                backward,
            })
        }

        "replace" => {
            if backward {
                return Err(
                    "--backward is only available for find"
                        .to_string()
                );
            }

            if arguments.len() < 2 {
                return Err(
                    "Usage: :replace old new [options]"
                        .to_string()
                );
            }

            let query = arguments.remove(0);
            let replacement = arguments.join(" ");

            Ok(ParsedCommand::Replace {
                query,
                replacement,
                mode,
                all,
            })
        }

        "goto" => {
            reject_search_options(
                search_option_used,
                backward,
                all,
            )?;

            if arguments.len() != 1 {
                return Err(
                    "Usage: :goto line[:column] [--abs|--rel]"
                        .to_string()
                );
            }

            let mut parts =
                arguments[0].split(':');

            let line_text = parts.next().unwrap_or("");
            // Parsing a signed integer loses an explicit '+' (and '-0').
            // Preserve the user's direction before converting the number.
            if goto_mode == GotoMode::Automatic
                && (line_text.starts_with('+') || line_text.starts_with('-'))
            {
                goto_mode = GotoMode::Relative;
            }
            let line =
                parse_line_number(line_text)?;

            let column =
                match parts.next() {
                    Some(value) =>
                        Some(parse_one_based(
                            value,
                            "column",
                        )?),
                    None => None,
                };

            if parts.next().is_some() {
                return Err(
                    "Usage: :goto line[:column] [--abs|--rel]"
                        .to_string()
                );
            }

            Ok(ParsedCommand::Goto {
                line,
                column,
                mode: goto_mode,
            })
        }

        "new" => {
            reject_search_options(search_option_used, backward, all)?;
            if arguments.len() > 1
                || arguments.first().is_some_and(|path| path.is_empty())
            {
                return Err("Usage: :new [path] (quote paths containing spaces)".to_string());
            }
            Ok(ParsedCommand::New { path: arguments.into_iter().next() })
        }

        "open" => {
            reject_search_options(
                search_option_used,
                backward,
                all,
            )?;

            let path = arguments.join(" ");

            if path.is_empty() {
                return Err(
                    "Usage: :open path"
                        .to_string()
                );
            }

            Ok(ParsedCommand::Open {
                path,
            })
        }

        "set" => {
            reject_search_options(
                search_option_used,
                backward,
                all,
            )?;

            if arguments.len() != 2 {
                return Err(
                    "Usage: :set font-size points | :set line-numbers type | :set keybindings mode"
                        .to_string()
                );
            }

            match arguments[0].as_str() {
                "font-size" => {
                    let points = arguments[1]
                        .parse::<u16>()
                        .map_err(|_| {
                            "Font size must be a whole number"
                                .to_string()
                        })?;

                    if !(MIN_FONT_SIZE..=MAX_FONT_SIZE)
                        .contains(&points)
                    {
                        return Err(format!(
                            "Font size must be between {MIN_FONT_SIZE} and {MAX_FONT_SIZE}"
                        ));
                    }

                    Ok(ParsedCommand::SetFontSize {
                        points,
                    })
                }

                "line-numbers" => {
                    let mode =
                        match arguments[1].as_str() {
                            "normal" =>
                                LineNumberMode::Normal,
                            "relative" =>
                                LineNumberMode::Relative,
                            "dynamic" =>
                                LineNumberMode::Dynamic,
                            _ => {
                                return Err(
                                    "Line number type must be normal, relative, or dynamic"
                                        .to_string()
                                );
                            }
                        };

                    Ok(ParsedCommand::SetLineNumbers {
                        mode,
                    })
                }

                "keybindings" => {
                    let mode =
                        match arguments[1].as_str() {
                            "conventional" =>
                                KeybindingMode::Conventional,
                            "vim" =>
                                KeybindingMode::Vim,
                            _ => {
                                return Err(
                                    "Keybinding mode must be conventional or vim"
                                        .to_string()
                                );
                            }
                        };

                    Ok(ParsedCommand::SetKeybindings {
                        mode,
                    })
                }

                _ => Err(
                    "Usage: :set font-size points | :set line-numbers type | :set keybindings mode"
                        .to_string()
                ),
            }
        }

        "term" => {
            reject_no_arguments(
                command,
                &arguments,
                search_option_used,
                backward,
                all,
            )?;

            Ok(ParsedCommand::Term)
        }

        "split" => {
            reject_no_arguments(
                command,
                &arguments,
                search_option_used,
                backward,
                all,
            )?;

            Ok(ParsedCommand::Split)
        }

        "actions" | "refactor" => {
            reject_search_options(search_option_used, backward, all)?;
            if !arguments.is_empty() { return Err("Usage: :lsp actions or :lsp refactor".into()); }
            Ok(ParsedCommand::Actions { refactor_only: command == "refactor" })
        }
        "rename" => {
            reject_search_options(search_option_used, backward, all)?;
            if arguments.len() != 1 || arguments[0].is_empty() || arguments[0].len() > 256
                || arguments[0].chars().any(|c| c.is_whitespace() || c.is_control()) {
                return Err("Usage: :lsp rename new_name".into());
            }
            Ok(ParsedCommand::Rename { name: arguments.remove(0) })
        }

        "recover" => {
            reject_search_options(search_option_used, backward, all)?;
            if arguments.len() > 1 { return Err("Usage: :recover [number]".into()); }
            let number = arguments.first().map(|value| value.parse::<usize>().ok().filter(|n| *n > 0).ok_or_else(|| "Choose a positive recovery number from :recover".to_string())).transpose()?;
            Ok(ParsedCommand::Recover { number })
        }

        "lsp-install" | "lsp-doctor" => {
            reject_search_options(search_option_used, backward, all)?;
            if arguments.len() > 1 || arguments.first().is_some_and(|s| s.is_empty())
                || (command == "lsp-install" && arguments.is_empty()) {
                return Err("Usage: :lsp install <server> or :lsp doctor [server]".into());
            }
            if command == "lsp-install" {
                Ok(ParsedCommand::LspInstall { server: arguments.remove(0) })
            } else {
                Ok(ParsedCommand::LspDoctor { server: arguments.into_iter().next() })
            }
        }

        "format" => {
            reject_search_options(search_option_used, backward, all)?;
            if arguments.len() > 1 || arguments.first().is_some_and(|name| name.is_empty()) {
                return Err("Usage: :format [provider]".to_string());
            }
            Ok(ParsedCommand::Format { provider: arguments.into_iter().next() })
        }

        "hover" | "definition" | "lsp-back" | "lsp-status" | "lsp-start" | "lsp-restart" | "lsp-stop" => {
            reject_no_arguments(command, &arguments, search_option_used, backward, all)?;
            Ok(match command.as_str() {
                "hover" => ParsedCommand::Hover,
                "definition" => ParsedCommand::Definition,
                "lsp-back" => ParsedCommand::LspBack,
                "lsp-status" => ParsedCommand::LspStatus,
                "lsp-start" => ParsedCommand::LspStart,
                "lsp-restart" => ParsedCommand::LspRestart,
                _ => ParsedCommand::LspStop,
            })
        }

        "formatters" => {
            reject_no_arguments(command, &arguments, search_option_used, backward, all)?;
            Ok(ParsedCommand::Formatters)
        }

        "extract-config" => {
            reject_no_arguments(
                command,
                &arguments,
                search_option_used,
                backward,
                all,
            )?;

            Ok(ParsedCommand::ExtractConfig)
        }

        "save-as" | "saveas" | "sav" | "savea"
        | "save-as!" | "saveas!" | "sav!" | "savea!" => {
            reject_search_options(search_option_used, backward, all)?;
            if arguments.len() > 1
                || arguments.first().is_some_and(|path| path.is_empty())
            {
                return Err(
                    "Usage: :save-as[!] path (quote paths containing spaces)".to_string()
                );
            }
            Ok(ParsedCommand::SaveAs {
                path: arguments.into_iter().next(),
                overwrite: command.ends_with('!'),
            })
        }

        "save" | "w" | "write" => {
            reject_no_arguments(
                command,
                &arguments,
                search_option_used,
                backward,
                all,
            )?;

            Ok(ParsedCommand::Save)
        }

        "quit" => {
            reject_no_arguments(
                command,
                &arguments,
                search_option_used,
                backward,
                all,
            )?;

            Ok(ParsedCommand::Quit)
        }

        _ => Err(
            format!(
                "Unknown command: {command}"
            )
        ),
    }
}

fn reject_search_options(
    search_option_used: bool,
    backward: bool,
    all: bool,
) -> Result<(), String> {
    if search_option_used
        || backward
        || all
    {
        Err(
            "This command does not accept search options"
                .to_string()
        )
    } else {
        Ok(())
    }
}

fn reject_no_arguments(
    command: &str,
    arguments: &[String],
    search_option_used: bool,
    backward: bool,
    all: bool,
) -> Result<(), String> {
    reject_search_options(
        search_option_used,
        backward,
        all,
    )?;

    if arguments.is_empty() {
        Ok(())
    } else {
        let command = match command {
            "hover" => "lsp hover", "definition" => "lsp definition",
            "lsp-back" => "lsp back", "lsp-status" => "lsp status", "lsp-stop" => "lsp stop",
            "lsp-start" => "lsp start", "lsp-restart" => "lsp restart",
            _ => command,
        };
        Err(format!(":{command} does not accept arguments"))
    }
}

fn parse_one_based(
    value: &str,
    name: &str,
) -> Result<usize, String> {
    let number =
        value.parse::<usize>()
            .map_err(|_| {
                format!(
                    "Invalid {name}: {value}"
                )
            })?;

    if number == 0 {
        Err(format!(
            "{name} numbers start at 1"
        ))
    } else {
        Ok(number)
    }
}

fn parse_line_number(
    value: &str,
) -> Result<isize, String> {
    value.parse::<isize>()
        .map_err(|_| {
            format!("Invalid line: {value}")
        })
}

fn tokenize(
    input: &str,
) -> Result<Vec<CommandWord>, String> {
    let body =
        input.strip_prefix(':')
            .unwrap_or(input);

    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut token_started = false;
    let mut token_quoted = false;
    let mut characters = body.chars().peekable();

    while let Some(character) =
        characters.next()
    {
        if let Some(open_quote) = quote {
            if character == open_quote {
                quote = None;
            } else if character == '\\' {
                push_escape(
                    &mut current,
                    &mut characters,
                );
            } else {
                current.push(character);
            }

            continue;
        }

        if matches!(character, '"' | '\'') {
            quote = Some(character);
            token_started = true;
            token_quoted = true;
        } else if character == '\\' {
            token_started = true;
            push_escape(
                &mut current,
                &mut characters,
            );
        } else if character.is_whitespace() {
            if token_started {
                words.push(CommandWord {
                    text: std::mem::take(
                        &mut current
                    ),
                    quoted: token_quoted,
                });

                token_started = false;
                token_quoted = false;
            }
        } else {
            current.push(character);
            token_started = true;
        }
    }

    if quote.is_some() {
        return Err(
            "Close the quoted argument"
                .to_string()
        );
    }

    if token_started {
        words.push(CommandWord {
            text: current,
            quoted: token_quoted,
        });
    }

    Ok(words)
}

fn push_escape<I>(
    output: &mut String,
    characters: &mut std::iter::Peekable<I>,
) where
    I: Iterator<Item = char>,
{
    match characters.peek().copied() {
        Some('n') => {
            characters.next();
            output.push('\n');
        }

        Some('r') => {
            characters.next();
            output.push('\r');
        }

        Some('t') => {
            characters.next();
            output.push('\t');
        }

        Some('\\' | '"' | '\'') => {
            output.push(
                characters.next().unwrap()
            );
        }

        Some(character)
            if character.is_whitespace() =>
        {
            output.push(
                characters.next().unwrap()
            );
        }

        Some(_) | None => output.push('\\'),
    }
}

fn quote_control_characters(
    text: &str,
) -> String {
    let mut visible =
        String::with_capacity(text.len());

    for character in text.chars() {
        match character {
            '\n' => visible.push_str("\\n"),
            '\r' => visible.push_str("\\r"),
            '\t' => visible.push_str("\\t"),
            _ => visible.push(character),
        }
    }

    visible
}

fn has_token(
    input: &str,
    token: &str,
) -> bool {
    find_token_span(input, token)
        .is_some()
}

fn remove_token(
    input: &mut String,
    token: &str,
) -> bool {
    let span =
        find_token_span(input, token);

    let Some((mut start, mut end)) = span
    else {
        return false;
    };

    if end < input.len()
        && input[end..]
            .chars()
            .next()
            .map(char::is_whitespace)
            .unwrap_or(false)
    {
        end += input[end..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0);
    } else if start > 0 {
        let previous =
            previous_char_boundary(
                input,
                start,
            );

        if input[previous..start]
            .chars()
            .next()
            .map(char::is_whitespace)
            .unwrap_or(false)
        {
            start = previous;
        }
    }

    input.drain(start..end);
    true
}

fn find_token_span(
    input: &str,
    token: &str,
) -> Option<(usize, usize)> {
    let mut start = None;

    for (index, character) in
        input.char_indices()
    {
        if character.is_whitespace() {
            if let Some(word_start) =
                start.take()
                && &input[word_start..index]
                    == token
            {
                return Some((
                    word_start,
                    index,
                ));
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }

    start.and_then(|word_start| {
        (&input[word_start..] == token)
            .then_some((
                word_start,
                input.len(),
            ))
    })
}

fn remove_partial_option(
    input: &mut String,
) {
    let Some((start, word)) =
        input.char_indices()
            .rfind(|(_, character)| {
                character.is_whitespace()
            })
            .map(|(index, character)| {
                (
                    index + character.len_utf8(),
                    &input[
                        index + character.len_utf8()..
                    ],
                )
            })
    else {
        return;
    };

    if word.starts_with("--") {
        input.truncate(start);
    }
}

fn previous_char_boundary(
    text: &str,
    position: usize,
) -> usize {
    text[..position]
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_char_boundary(
    text: &str,
    position: usize,
) -> usize {
    text[position..]
        .char_indices()
        .nth(1)
        .map(|(offset, _)| {
            position + offset
        })
        .unwrap_or(text.len())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_commands_require_a_positive_session_number() {
        assert_eq!(parse_command(":recover").unwrap(), ParsedCommand::Recover { number: None });
        assert_eq!(parse_command(":recover 2").unwrap(), ParsedCommand::Recover { number: Some(2) });
        for input in [":recover 0", ":recover -1", ":recover abc", ":recover 1 2", ":recover --all"] { assert!(parse_command(input).is_err(), "{input}"); }
        let mut bar=CommandBar::new(); bar.open(":recov"); assert!(bar.prepare_execute());
        assert_eq!(bar.parse().unwrap(), ParsedCommand::Recover { number: None });
        bar.open(":recover 2"); assert!(bar.prepare_execute());
        assert_eq!(bar.parse().unwrap(), ParsedCommand::Recover { number: Some(2) });
    }

    #[test]
    fn new_command_accepts_optional_quoted_path() {
        assert_eq!(parse_command(":new").unwrap(), ParsedCommand::New { path: None });
        assert_eq!(parse_command(":new \"é notes.txt\"").unwrap(),
            ParsedCommand::New { path: Some("é notes.txt".into()) });
        for input in [":new a b", ":new \"\"", ":new --all", ":new --rel"] {
            assert!(parse_command(input).is_err(), "{input}");
        }
    }

    #[test]
    fn all_commands_are_reachable_by_arrows_and_clicks_across_pages() {
        let mut bar = CommandBar::new();
        bar.open(":");
        for spec in COMMANDS {
            assert_eq!(bar.suggestion(bar.selected()).unwrap().label, spec.name);
            assert!(bar.suggestion_count() <= MAX_VISIBLE_SUGGESTIONS);
            let y = 600 - COMMAND_BAR_MARGIN - bar.panel_height()
                + bar.selected() as i32 * COMMAND_SUGGESTION_HEIGHT + 1;
            assert_eq!(bar.hit_test(800, 600, 30, y), CommandBarHit::Suggestion(bar.selected()));
            bar.move_selection(1);
        }
        assert_eq!(bar.suggestion(bar.selected()).unwrap().label, COMMANDS[0].name);
        bar.move_selection(-1);
        assert_eq!(bar.suggestion(bar.selected()).unwrap().label, "lsp");
        assert!(bar.apply_suggestion(bar.selected()));
        assert_eq!(bar.input(), ":lsp ");
        for spec in LSP_COMMANDS {
            assert_eq!(bar.suggestion(bar.selected()).unwrap().label, spec.name);
            bar.move_selection(1);
        }
        bar.move_selection(-1);
        assert!(bar.apply_suggestion(bar.selected()));
        assert_eq!(bar.parse().unwrap(), ParsedCommand::LspStop);
    }

    #[test]
    fn enter_accepts_highlighted_commands_settings_and_options() {
        let mut bar = CommandBar::new();
        bar.open(":");
        bar.move_selection(-1);
        assert!(!bar.prepare_execute());
        assert_eq!(bar.input(), ":lsp ");
        bar.move_selection(-1);
        assert!(bar.prepare_execute());
        assert_eq!(bar.parse().unwrap(), ParsedCommand::LspStop);
        bar.open(":op");
        assert!(!bar.prepare_execute());
        assert_eq!(bar.input(), ":open ");
        bar.insert_text("notes.txt");
        assert!(bar.prepare_execute());
        assert_eq!(bar.parse().unwrap(), ParsedCommand::Open { path: "notes.txt".into() });
        bar.open(":set ");
        assert!(!bar.prepare_execute());
        assert_eq!(bar.input(), ":set font-size ");
        assert!(bar.prepare_execute());
        assert_eq!(bar.parse().unwrap(), ParsedCommand::SetFontSize { points: 12 });
        bar.open(":find notes ");
        bar.move_selection(1);
        assert!(!bar.prepare_execute());
        assert!(bar.input().contains("--ignore-case"));
        assert!(bar.prepare_execute());
    }

    #[test]
    fn wheel_reaches_lsp_and_editing_resets_the_page() {
        let mut bar = CommandBar::new();
        bar.open(":");
        bar.scroll_suggestions(100);
        assert_eq!(bar.suggestion(bar.selected()).unwrap().label, "lsp");
        assert!(bar.navigation_hint().unwrap().contains(&format!("of {}", COMMANDS.len())));
        bar.insert_text("new");
        assert_eq!(bar.suggestion(0).unwrap().label, "new");
        assert_eq!(bar.selected(), 0);
        assert!(bar.prepare_execute());
        assert_eq!(bar.parse().unwrap(), ParsedCommand::New { path: None });
    }

    #[test]
    fn formatter_choices_beyond_first_page_can_run_with_enter() {
        let mut bar = CommandBar::new();
        bar.open(":formatters");
        bar.show_formatters((0..12).map(|i| (format!("formatter{i}"), true)).collect());
        bar.scroll_suggestions(11);
        assert!(bar.prepare_execute());
        assert_eq!(bar.parse().unwrap(), ParsedCommand::Format { provider: Some("formatter11".into()) });
    }

    #[test]
    fn lsp_commands_are_explicit_and_reject_extra_arguments() {
        for (name, expected) in [
            ("hover", ParsedCommand::Hover), ("definition", ParsedCommand::Definition),
            ("lsp-back", ParsedCommand::LspBack), ("lsp-status", ParsedCommand::LspStatus),
            ("lsp-stop", ParsedCommand::LspStop),
        ] {
            assert_eq!(parse_command(&format!(":{name}")).unwrap(), expected);
            for extra in ["extra", "--all", "--regex", "--rel"] {
                assert!(parse_command(&format!(":{name} {extra}")).is_err());
            }
        }
    }

    #[test]
    fn grouped_lsp_commands_match_aliases_and_validate_arguments() {
        for (subcommand, alias) in [("hover","hover"),("definition","definition"),
            ("actions","actions"),("refactor","refactor"),("back","lsp-back"),
            ("status","lsp-status"),("stop","lsp-stop"),("start","lsp-start"),("restart","lsp-restart")] {
            assert_eq!(parse_command(&format!(":lsp {subcommand}")).unwrap(), parse_command(&format!(":{alias}")).unwrap());
            for extra in ["extra", "--all", "--regex", "--rel"] {
                assert!(parse_command(&format!(":lsp {subcommand} {extra}")).is_err());
            }
        }
        assert_eq!(parse_command(":lsp rename \"greeting_é\"").unwrap(), ParsedCommand::Rename {name:"greeting_é".into()});
        for input in [":lsp",":lsp nope",":lsp rename",":lsp rename a b",":lsp rename \"a b\"",":lsp rename --all"] {
            assert!(parse_command(input).is_err(), "{input}");
        }
    }

    #[test]
    fn lsp_group_supports_prefixes_clicks_and_rename_arguments() {
        let mut bar = CommandBar::new();
        bar.open(":lsp");
        assert!(!bar.prepare_execute());
        assert_eq!(bar.input(), ":lsp ");
        assert_eq!(bar.total_suggestion_count(), LSP_COMMANDS.len());
        for spec in LSP_COMMANDS {
            bar.open(":lsp ");
            let index=LSP_COMMANDS.iter().position(|s| s.name==spec.name).unwrap();
            bar.scroll_suggestions(index as isize);
            let row=bar.selected();
            let y=600-COMMAND_BAR_MARGIN-bar.panel_height()+bar.suggestion_row_offset(row)+1;
            assert_eq!(bar.hit_test(800,600,30,y),CommandBarHit::Suggestion(row));
            bar.select_suggestion(row);
            let ready=bar.prepare_execute();
            assert_eq!(bar.input(),format!(":{} ",spec.name));
            assert_eq!(ready,!matches!(spec.name,"lsp rename" | "lsp install"));
            if spec.name == "lsp install" {
                bar.insert_text("unity");
                assert!(bar.prepare_execute());
                assert_eq!(bar.parse().unwrap(),ParsedCommand::LspInstall {server:"csharp".into()});
            } else if !ready {
                bar.insert_text("new_name");
                assert!(bar.prepare_execute());
                assert_eq!(bar.parse().unwrap(),ParsedCommand::Rename {name:"new_name".into()});
            }
        }
        bar.open(":lsp h");
        assert_eq!(bar.total_suggestion_count(),1);
        assert_eq!(bar.suggestion(0).unwrap().label,"lsp hover");
        assert!(bar.prepare_execute());
        assert_eq!(bar.parse().unwrap(),ParsedCommand::Hover);
        bar.open(":lsp rename preserved_name");
        assert!(bar.prepare_execute());
        assert_eq!(bar.parse().unwrap(),ParsedCommand::Rename {name:"preserved_name".into()});
    }

    #[test]
    fn lsp_setup_commands_validate_arguments_and_complete_all_servers() {
        assert_eq!(parse_command(":lsp doctor").unwrap(), ParsedCommand::LspDoctor { server: None });
        assert_eq!(parse_command(":lsp install csharp-ls").unwrap(), ParsedCommand::LspInstall { server: "csharp-ls".into() });
        for input in [":lsp install", ":lsp install a b", ":lsp doctor a b", ":lsp install --all", ":lsp doctor --rel", ":lsp install \"\""] {
            assert!(parse_command(input).is_err(), "{input}");
        }
        let mut bar = CommandBar::new();
        for (index, recipe) in crate::lsp_setup::catalog::RECIPES.iter().enumerate() {
            bar.open(":lsp install ");
            assert_eq!(bar.total_suggestion_count(), crate::lsp_setup::catalog::RECIPES.len());
            bar.scroll_suggestions(index as isize);
            assert!(bar.apply_suggestion(bar.selected()));
            assert_eq!(bar.parse().unwrap(), ParsedCommand::LspInstall { server: recipe.id.into() });
        }
        bar.open(":lsp doctor ");
        assert!(bar.prepare_execute());
        assert_eq!(bar.parse().unwrap(), ParsedCommand::LspDoctor { server: None });
        bar.open(":lsp install UNI");
        assert!(bar.prepare_execute());
        assert_eq!(bar.parse().unwrap(), ParsedCommand::LspInstall { server: "csharp".into() });
    }

    #[test]
    fn hover_documentation_color_survives_wrapping_and_scrolling() {
        let mut bar = CommandBar::new();
        bar.open(":hover");
        let text = format!("crate_name\n\nfn example()\n\n\n{}", "Documentation é中 ".repeat(70));
        let hover = crate::lsp::hover_content(&serde_json::json!({"contents":{"kind":"plaintext","value":text}}));
        bar.show_hover(&hover);
        assert!(!bar.info_is_documentation(0));
        assert!(!bar.info_is_documentation(2));
        assert!(bar.info_is_documentation(4));
        assert!(bar.info_is_documentation(5));
        for _ in 0..5 { bar.scroll_suggestions(1); }
        assert!(bar.info_is_documentation(0));
        bar.show_info("Waiting for the language server…");
        assert!(!bar.info_is_documentation(0), "status text must not inherit the documentation color");
    }

    #[test]
    fn hover_spacing_collapses_blank_lines_and_keeps_hit_targets_aligned() {
        let mut bar = CommandBar::new();
        bar.open(":hover");
        bar.show_info("\n  \npotyi_lsp_demo\n\nfn greeting() -> &'static str\n\n\nReturns a greeting.\n\n");
        assert_eq!(bar.suggestion_count(), 5);
        assert_eq!(bar.suggestion(0).unwrap().label, "potyi_lsp_demo");
        assert_eq!(bar.suggestion(4).unwrap().label, "Returns a greeting.");
        assert_eq!(bar.suggestions_height(), 3 * INFO_LINE_HEIGHT + 2 * INFO_PARAGRAPH_GAP);
        assert_eq!(bar.panel_height(), COMMAND_INPUT_HEIGHT + 72);
        let top = 600 - COMMAND_BAR_MARGIN - bar.panel_height();
        for index in 0..bar.suggestion_count() {
            let y = top + bar.suggestion_row_offset(index);
            assert_eq!(bar.hit_test(800, 600, 40, y), CommandBarHit::Suggestion(index));
            assert_eq!(bar.hit_test(800, 600, 40, y + bar.suggestion_row_height(index) - 1),
                CommandBarHit::Suggestion(index));
        }
        let input_y = top + bar.suggestions_height();
        assert_eq!(bar.hit_test(800, 600, 40, input_y), CommandBarHit::Input);
        assert_eq!(bar.hit_test(800, 600, 780, input_y), CommandBarHit::Execute);
        assert_eq!(bar.hit_test(800, 600, 40, top - 1), CommandBarHit::Outside);
        bar.open(":");
        assert_eq!(bar.suggestions_height(), bar.suggestion_count() as i32 * COMMAND_SUGGESTION_HEIGHT);
    }

    #[test]
    fn hover_spacing_keeps_indentation_and_scrolled_layout_consistent() {
        let mut bar = CommandBar::new();
        bar.open(":hover");
        bar.show_info(&"    code\n\n\nnext\n".repeat(12));
        assert_eq!(bar.suggestion(0).unwrap().label, "    code");
        bar.scroll_suggestions(3);
        let top = 600 - COMMAND_BAR_MARGIN - bar.panel_height();
        for index in 0..bar.suggestion_count() {
            assert_eq!(bar.hit_test(800, 600, 40, top + bar.suggestion_row_offset(index)),
                CommandBarHit::Suggestion(index));
        }
    }

    #[test]
    fn hover_and_definition_results_can_be_repeated_with_enter() {
        let mut bar = CommandBar::new();
        for command in [":hover", ":definition"] {
            bar.open(command);
            bar.show_info("Result at the previous cursor");
            assert!(bar.prepare_execute());
            assert_eq!(bar.input(), command);
            assert_eq!(bar.hit_test(800, 600, 100, 50), CommandBarHit::Outside);
        }
        for command in [":format", ":save"] {
            bar.open(command);
            bar.show_info("Finished");
            assert!(!bar.prepare_execute());
        }
    }

    #[test]
    fn informational_results_scroll_and_clear_on_input() {
        let mut bar = CommandBar::new();
        bar.open(":hover");
        let epoch = bar.epoch();
        bar.show_info(&(0..20).map(|n| format!("Line {n}")).collect::<Vec<_>>().join("\n"));
        assert_eq!(bar.epoch(), epoch);
        assert_eq!(bar.suggestion(0).unwrap().label, "Line 0");
        bar.move_selection(1);
        assert_eq!(bar.suggestion(0).unwrap().label, "Line 1");
        assert!(!bar.apply_selected());
        bar.insert_text("x");
        assert!(!bar.is_info());
        assert_ne!(bar.epoch(), epoch);
    }

    #[test]
    fn formatting_commands_are_explicit_and_reject_extra_options() {
        assert_eq!(parse_command(":format").unwrap(), ParsedCommand::Format { provider: None });
        assert_eq!(parse_command(":format rustfmt").unwrap(), ParsedCommand::Format { provider: Some("rustfmt".into()) });
        assert_eq!(parse_command(":formatters").unwrap(), ParsedCommand::Formatters);
        for input in [":format one two", ":format --all", ":format --regex", ":format --rel", ":formatters rustfmt"] {
            assert!(parse_command(input).is_err(), "{input}");
        }
    }

    #[test]
    fn formatter_results_are_selectable_without_running_on_selection() {
        let mut bar = CommandBar::new();
        bar.open(":formatters");
        bar.show_formatters(vec![("rustfmt".into(), true), ("ruff".into(), false)]);
        assert_eq!(bar.suggestion_count(), 2);
        assert_eq!(bar.suggestion(0).unwrap().label, "rustfmt");
        assert!(bar.suggestion(1).unwrap().description.starts_with("Missing"));
        assert!(bar.apply_suggestion(0));
        assert_eq!(bar.input(), ":format rustfmt");
        assert!(bar.formatter_choices.is_empty());
        bar.close();
        assert!(bar.formatter_choices.is_empty());
    }

    #[test]
    fn save_as_commands_parse_paths_and_overwrite_intent() {
        for name in ["save-as", "saveas", "sav", "savea"] {
            for overwrite in [false, true] {
                let bang = if overwrite { "!" } else { "" };
                for path in ["new.rs", "folder/é file.txt", r#"C:\new\test.txt"#, "--regex"] {
                    assert_eq!(
                        parse_command(&format!(":{name}{bang} {}", quote_argument(path))).unwrap(),
                        ParsedCommand::SaveAs { path: Some(path.to_string()), overwrite },
                    );
                }
                assert_eq!(
                    parse_command(&format!(":{name}{bang}")).unwrap(),
                    ParsedCommand::SaveAs { path: None, overwrite },
                );
            }
        }
        for input in [":save-as a b", ":sav \"\"", ":saveas a --all", ":saveas a --rel"] {
            assert!(parse_command(input).is_err(), "{input}");
        }
        for input in [":save", ":w", ":write"] {
            assert_eq!(parse_command(input).unwrap(), ParsedCommand::Save);
        }
        let mut bar = CommandBar::new();
        bar.open(":save-");
        assert!(bar.apply_selected());
        assert_eq!(bar.input(), ":save-as ");
        assert_eq!(bar.suggestion(0).unwrap().label, ":save-as[!] path");
        bar.open(":sav! ");
        assert_eq!(bar.suggestion(0).unwrap().label, ":saveas[!] path");
    }

    #[test]
    fn parses_quoted_find_and_visible_options() {
        let mut bar = CommandBar::new();
        bar.open(
            ":find \"hello world\" --ignore-case"
        );

        assert_eq!(
            bar.parse().unwrap(),
            ParsedCommand::Find {
                query: "hello world".to_string(),
                mode: SearchMode::CaseInsensitive,
                backward: false,
            },
        );

        assert!(
            (0..bar.suggestion_count())
                .filter_map(|index| {
                    bar.suggestion(index)
                })
                .any(|suggestion| {
                    suggestion.label
                        == "--ignore-case"
                        && suggestion.active
                })
        );
    }

    #[test]
    fn parses_replace_all_and_goto() {
        let mut bar = CommandBar::new();
        bar.open(
            ":replace \"old value\" new --regex --all"
        );

        assert_eq!(
            bar.parse().unwrap(),
            ParsedCommand::Replace {
                query: "old value".to_string(),
                replacement: "new".to_string(),
                mode: SearchMode::Regex,
                all: true,
            },
        );

        bar.open(":goto 12:4");

        assert_eq!(
            bar.parse().unwrap(),
            ParsedCommand::Goto {
                line: 12,
                column: Some(4),
                mode: GotoMode::Automatic,
            },
        );
    }

    #[test]
    fn goto_parses_line_number_overrides() {
        assert_eq!(
            parse_command(":goto 76 --rel")
                .unwrap(),
            ParsedCommand::Goto {
                line: 76,
                column: None,
                mode: GotoMode::Relative,
            },
        );

        assert_eq!(
            parse_command(":goto -12:4 --rel")
                .unwrap(),
            ParsedCommand::Goto {
                line: -12,
                column: Some(4),
                mode: GotoMode::Relative,
            },
        );

        assert_eq!(
            parse_command(":goto 76 --abs")
                .unwrap(),
            ParsedCommand::Goto {
                line: 76,
                column: None,
                mode: GotoMode::Absolute,
            },
        );

        assert_eq!(
            parse_command(
                ":goto 76 --abs --rel"
            )
            .unwrap_err(),
            "--abs and --rel cannot be used together",
        );
    }

    #[test]
    fn goto_preserves_direction_and_explicit_overrides() {
        for (input, line, column, mode) in [
            (":goto 3", 3, None, GotoMode::Automatic),
            (":goto +3", 3, None, GotoMode::Relative),
            (":goto -3", -3, None, GotoMode::Relative),
            (":goto +0", 0, None, GotoMode::Relative),
            (":goto -0", 0, None, GotoMode::Relative),
            (":goto +3:2", 3, Some(2), GotoMode::Relative),
            (":goto -3:2", -3, Some(2), GotoMode::Relative),
            (":goto 3 --rel", 3, None, GotoMode::Relative),
            (":goto +3 --abs", 3, None, GotoMode::Absolute),
            (":goto --abs +3", 3, None, GotoMode::Absolute),
        ] {
            assert_eq!(
                parse_command(input).unwrap(),
                ParsedCommand::Goto { line, column, mode },
                "{input}",
            );
        }
        for input in [":goto +", ":goto -", ":goto +-3", ":goto +3:0"] {
            assert!(parse_command(input).is_err(), "{input}");
        }
    }

    #[test]
    fn goto_options_are_discoverable_and_exclusive() {
        let mut bar = CommandBar::new();
        bar.open(":goto 76 --r");

        assert_eq!(
            bar.suggestion(0).unwrap().label,
            "--rel",
        );

        assert!(bar.apply_suggestion(0));
        assert_eq!(bar.input(), ":goto 76 --rel ");

        bar.open(":goto 76 --rel --a");
        assert!(bar.apply_suggestion(0));
        assert_eq!(bar.input(), ":goto 76 --abs ");
    }

    #[test]
    fn command_and_option_clicks_are_actionable() {
        let mut bar = CommandBar::new();
        bar.open(":f");

        assert!(bar.apply_suggestion(0));
        assert_eq!(bar.input(), ":find ");

        let regex_index =
            (0..bar.suggestion_count())
                .find(|index| {
                    bar.suggestion(*index)
                        .map(|suggestion| {
                            suggestion.label
                                == "--regex"
                        })
                        .unwrap_or(false)
                })
                .unwrap();

        assert!(
            bar.apply_suggestion(
                regex_index
            )
        );

        assert!(bar.input().contains("--regex"));

        let hit = bar.hit_test(
            800,
            600,
            20,
            600 - COMMAND_BAR_MARGIN - 2,
        );

        assert_eq!(hit, CommandBarHit::Input);

        let execute_hit = bar.hit_test(
            800,
            600,
            800 - COMMAND_BAR_MARGIN - 2,
            600 - COMMAND_BAR_MARGIN - 2,
        );

        assert_eq!(
            execute_hit,
            CommandBarHit::Execute,
        );
    }

    #[test]
    fn utf8_cursor_editing_stays_on_boundaries() {
        let mut bar = CommandBar::new();
        bar.open(":find é🙂");

        bar.move_left();
        bar.backspace();

        assert_eq!(bar.input(), ":find 🙂");
    }

    #[test]
    fn quote_argument_round_trips() {
        let value = "a path/with \"quotes\"";
        let command = format!(
            ":open {}",
            quote_argument(value),
        );

        assert_eq!(
            parse_command(&command).unwrap(),
            ParsedCommand::Open {
                path: value.to_string(),
            },
        );
    }

    #[test]
    fn windows_paths_and_empty_replacements_round_trip() {
        let path = r#"C:\Users\Attila\My File.txt"#;

        assert_eq!(
            parse_command(
                &format!(
                    ":open {}",
                    quote_argument(path),
                )
            )
            .unwrap(),
            ParsedCommand::Open {
                path: path.to_string(),
            },
        );

        assert_eq!(
            parse_command(
                ":replace old \"\" --all"
            )
            .unwrap(),
            ParsedCommand::Replace {
                query: "old".to_string(),
                replacement: String::new(),
                mode: SearchMode::CaseSensitive,
                all: true,
            },
        );
    }

    #[test]
    fn pasted_controls_stay_single_line_and_parse_as_text() {
        let mut bar = CommandBar::new();
        bar.open(":find \"");
        bar.insert_text("one\ntwo\t");
        bar.insert_text("\"");

        assert!(!bar.input().contains('\n'));

        assert_eq!(
            bar.parse().unwrap(),
            ParsedCommand::Find {
                query: "one\ntwo\t".to_string(),
                mode: SearchMode::CaseSensitive,
                backward: false,
            },
        );
    }

    #[test]
    fn quoted_option_name_is_search_text() {
        assert_eq!(
            parse_command(
                ":find \"--regex\""
            )
            .unwrap(),
            ParsedCommand::Find {
                query: "--regex".to_string(),
                mode: SearchMode::CaseSensitive,
                backward: false,
            },
        );
    }

    #[test]
    fn font_size_setting_is_discoverable_and_clickable() {
        let mut bar = CommandBar::new();
        bar.open(":set ");

        assert_eq!(
            bar.suggestion(0)
                .unwrap()
                .label,
            "font-size",
        );

        assert!(bar.apply_suggestion(0));
        assert_eq!(
            bar.input(),
            ":set font-size ",
        );

        let size_index =
            (0..bar.suggestion_count())
                .find(|index| {
                    bar.suggestion(*index)
                        .map(|suggestion| {
                            suggestion.label == "24"
                        })
                        .unwrap_or(false)
                })
                .unwrap();

        assert!(bar.apply_suggestion(
            size_index
        ));

        assert_eq!(
            bar.parse().unwrap(),
            ParsedCommand::SetFontSize {
                points: 24,
            },
        );
    }

    #[test]
    fn font_size_setting_rejects_unsafe_sizes() {
        for input in [
            ":set font-size 0",
            ":set font-size 73",
            ":set font-size huge",
        ] {
            assert!(parse_command(input)
                .is_err());
        }
    }

    #[test]
    fn line_number_setting_is_discoverable_and_clickable() {
        let mut bar = CommandBar::new();
        bar.open(":set ");

        let setting_index =
            (0..bar.suggestion_count())
                .find(|index| {
                    bar.suggestion(*index)
                        .map(|suggestion| {
                            suggestion.label
                                == "line-numbers"
                        })
                        .unwrap_or(false)
                })
                .unwrap();

        assert!(bar.apply_suggestion(
            setting_index
        ));
        assert_eq!(
            bar.input(),
            ":set line-numbers ",
        );

        let relative_index =
            (0..bar.suggestion_count())
                .find(|index| {
                    bar.suggestion(*index)
                        .map(|suggestion| {
                            suggestion.label == "relative"
                        })
                        .unwrap_or(false)
                })
                .unwrap();

        assert!(bar.apply_suggestion(
            relative_index
        ));

        assert_eq!(
            bar.parse().unwrap(),
            ParsedCommand::SetLineNumbers {
                mode: LineNumberMode::Relative,
            },
        );
    }

    #[test]
    fn line_number_setting_parses_all_types() {
        for (name, mode) in [
            ("normal", LineNumberMode::Normal),
            ("relative", LineNumberMode::Relative),
            ("dynamic", LineNumberMode::Dynamic),
        ] {
            assert_eq!(
                parse_command(&format!(
                    ":set line-numbers {name}"
                )).unwrap(),
                ParsedCommand::SetLineNumbers {
                    mode,
                },
            );
        }

        assert!(parse_command(
            ":set line-numbers unknown"
        ).is_err());
    }

    #[test]
    fn keybinding_setting_is_discoverable_and_parses_both_modes() {
        let mut bar = CommandBar::new();
        bar.open(":set keybindings ");

        assert_eq!(
            bar.suggestion(0).unwrap().label,
            "conventional",
        );
        assert_eq!(
            bar.suggestion(1).unwrap().label,
            "vim",
        );

        for (name, mode) in [
            ("conventional", KeybindingMode::Conventional),
            ("vim", KeybindingMode::Vim),
        ] {
            assert_eq!(
                parse_command(&format!(
                    ":set keybindings {name}"
                )).unwrap(),
                ParsedCommand::SetKeybindings {
                    mode,
                },
            );
        }

        assert!(parse_command(
            ":set keybindings emacs"
        ).is_err());
    }

    #[test]
    fn parses_extract_config_without_arguments() {
        assert_eq!(
            parse_command(":extract-config").unwrap(),
            ParsedCommand::ExtractConfig,
        );

        assert!(parse_command(":extract-config overwrite")
            .is_err());
    }

    #[test]
    fn parses_term_without_arguments() {
        assert_eq!(
            parse_command(":term").unwrap(),
            ParsedCommand::Term,
        );

        assert!(parse_command(":term now")
            .is_err());
    }

    #[test]
    fn parses_split_without_arguments() {
        assert_eq!(
            parse_command(":split").unwrap(),
            ParsedCommand::Split,
        );

        assert!(parse_command(":split vertical")
            .is_err());
    }
}
