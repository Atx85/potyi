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
pub(crate) const COMMAND_INPUT_HEIGHT: i32 = 42;
pub(crate) const COMMAND_SUGGESTION_HEIGHT: i32 = 32;
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
        line: usize,
        column: Option<usize>,
    },

    Open {
        path: String,
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
    Save,
    Quit,
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
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CommandSuggestion {
    pub label: &'static str,
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
        description: "Move to a line and optional column",
        usage: ":goto line[:column]",
    },
    CommandSpec {
        name: "open",
        description: "Open a file by path",
        usage: ":open path",
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
        name: "extract-config",
        description: "Create editable copies of built-in defaults",
        usage: ":extract-config",
    },
    CommandSpec {
        name: "quit",
        description: "Close Pötyi",
        usage: ":quit",
    },
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
    status: Option<String>,
}

impl CommandBar {
    pub fn new() -> Self {
        Self {
            active: false,
            input: String::new(),
            cursor: 0,
            selected: 0,
            status: None,
        }
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
        self.selected
            .min(
                self.suggestion_count()
                    .saturating_sub(1)
            )
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

    pub fn open(
        &mut self,
        initial: &str,
    ) {
        self.active = true;
        self.input.clear();

        if initial.starts_with(':') {
            self.input.push_str(initial);
        } else {
            self.input.push(':');
            self.input.push_str(initial);
        }

        self.cursor = self.input.len();
        self.selected = 0;
        self.status = None;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.status = None;
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
        self.status = None;
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
        self.status = None;
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
        self.status = None;
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
        let count = self.suggestion_count();

        if count == 0 {
            self.selected = 0;
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
    }

    pub fn select_suggestion(
        &mut self,
        index: usize,
    ) -> bool {
        if index < self.suggestion_count() {
            let changed = self.selected != index;
            self.selected = index;
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

            SuggestionAction::None => {
                return false;
            }
        }

        self.cursor = self.input.len();
        self.selected = 0;
        self.status = None;

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
    ) -> Option<CommandSuggestion> {
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

        if command == "set" {
            return setting_suggestion(
                body,
                visible_index,
            );
        }

        let options =
            match command {
                "find" => FIND_OPTIONS,
                "replace" => REPLACE_OPTIONS,
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

    pub fn panel_height(&self) -> i32 {
        if !self.active {
            return 0;
        }

        COMMAND_INPUT_HEIGHT
            + self.suggestion_count() as i32
                * COMMAND_SUGGESTION_HEIGHT
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
            top
                + self.suggestion_count() as i32
                    * COMMAND_SUGGESTION_HEIGHT;

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

        let index =
            ((y - top)
                / COMMAND_SUGGESTION_HEIGHT)
                as usize;

        if index < self.suggestion_count() {
            CommandBarHit::Suggestion(index)
        } else {
            CommandBarHit::Outside
        }
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

fn setting_suggestion(
    body: &str,
    visible_index: usize,
) -> Option<CommandSuggestion> {
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
    let words = tokenize(input)?;

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
                    "Usage: :goto line[:column]"
                        .to_string()
                );
            }

            let mut parts =
                arguments[0].split(':');

            let line =
                parse_one_based(
                    parts.next().unwrap_or(""),
                    "line",
                )?;

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
                    "Usage: :goto line[:column]"
                        .to_string()
                );
            }

            Ok(ParsedCommand::Goto {
                line,
                column,
            })
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

        "save" => {
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
        Err(format!(
            ":{command} does not accept arguments"
        ))
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
            },
        );
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
