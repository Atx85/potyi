// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use crate::command_bar::{CommandBar, ParsedCommand};

fn parse_command(input: &str) -> Result<ParsedCommand, String> {
    let mut bar = CommandBar::new();
    bar.open(input);
    bar.parse()
}

#[test]
fn experimental_terminal_entry_keeps_shell_arguments_and_legacy_entry_separate() {
    assert_eq!(
        parse_command(":term-new").unwrap(),
        ParsedCommand::TermNew { command: None }
    );
    for command in [
        "printf 'a b' | grep a",
        "echo $HOME && pwd",
        "dir C:\\Users",
        "echo \"a&b\"",
    ] {
        assert_eq!(
            parse_command(&format!(":term-new {command}")).unwrap(),
            ParsedCommand::TermNew {
                command: Some(command.into())
            }
        );
        assert_eq!(
            parse_command(&format!(":term {command}")).unwrap(),
            ParsedCommand::Term {
                command: Some(command.into())
            }
        );
    }
    assert!(parse_command(":term-newer").is_err());
}
