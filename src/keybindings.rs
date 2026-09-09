// Pötyi - Lightweight text editor
// Copyright (C) 2026  Attila Banko
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


use sdl3::keyboard::{Keycode, Mod};
use serde::Deserialize;
use std::fs;
use std::io;

use crate::embedded_config::KEYBINDINGS;

/// Commands understood by the editor.
///
/// The keybinding system only translates keyboard input into commands.
/// It does not execute the commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveWordLeft,
    MoveWordRight,
    PageUp,
    PageDown,

    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    SelectAll,
    SelectWordLeft,
    SelectWordRight,
    SelectPageUp,
    SelectPageDown,
    SelectHome,
    SelectEnd,

    Home,
    End,

    Delete,
    Backspace,
    Newline,
    InsertTab,

    Quit,

    Save,
    SaveAs,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
}

/// A single keyboard binding used by the editor.
#[derive(Clone, Copy, Debug)]
pub struct KeyBinding {
    pub key: Keycode,
    pub modifiers: Mod,
    pub command: Command,
    pub repeatable: bool,
}

/// Collection of keyboard bindings.
#[derive(Debug)]
pub struct KeyBindings {
    bindings: Vec<KeyBinding>,
}

/// Representation of one binding as it appears in TOML.
///
/// This is deliberately separate from `KeyBinding` because SDL's
/// `Keycode` and `Mod` types are runtime types, not configuration types.
#[derive(Debug, Deserialize)]
struct ConfigBinding {
    key: String,

    #[serde(default)]
    modifiers: Vec<String>,

    command: String,

    #[serde(default)]
    repeatable: bool,
}

/// Root TOML configuration.
#[derive(Debug, Deserialize)]
struct KeyBindingsConfig {
    bindings: Vec<ConfigBinding>,
}

impl KeyBindings {
    /// Create the built-in default bindings.
    pub fn default() -> Self {
        Self::from_config(KEYBINDINGS)
            .expect("embedded keybindings must be valid")
    }

    /// Load keybindings from a TOML configuration file.
    ///
    /// If the file does not exist, the embedded defaults are used.
    ///
    /// If the file exists but cannot be read or parsed, an error is returned.
    pub fn load(path: &str) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(contents) => Self::from_config(&contents),

            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(Self::default())
            }

            Err(error) => Err(error),
        }
    }

    /// Translate an SDL keyboard event into an editor command.
    ///
    /// `repeat` is SDL's repeat flag. A binding marked as non-repeatable
    /// will only fire for the initial KeyDown event.
    pub fn command_for(
        &self,
        key: Keycode,
        keymod: Mod,
        repeat: bool,
    ) -> Option<Command> {
        self.bindings
            .iter()
            .find(|binding| {
                binding.key == key
                    && Self::modifiers_match(
                        binding.modifiers,
                        keymod,
                    )
                    && (!repeat || binding.repeatable)
            })
            .map(|binding| binding.command)
    }

    /// Check whether the requested modifier state matches the actual
    /// keyboard modifier state.
    ///
    /// Left and right variants of Shift, Ctrl, Alt and GUI are treated
    /// as the same modifier group.
    ///
    /// Extra modifiers do not match.
    fn modifiers_match(required: Mod, actual: Mod) -> bool {
        Self::modifier_group(
            required,
            Mod::LSHIFTMOD | Mod::RSHIFTMOD,
        ) == Self::modifier_group(
            actual,
            Mod::LSHIFTMOD | Mod::RSHIFTMOD,
        )
        &&
        Self::modifier_group(
            required,
            Mod::LCTRLMOD | Mod::RCTRLMOD,
        ) == Self::modifier_group(
            actual,
            Mod::LCTRLMOD | Mod::RCTRLMOD,
        )
        &&
        Self::modifier_group(
            required,
            Mod::LALTMOD | Mod::RALTMOD,
        ) == Self::modifier_group(
            actual,
            Mod::LALTMOD | Mod::RALTMOD,
        )
        &&
        Self::modifier_group(
            required,
            Mod::LGUIMOD | Mod::RGUIMOD,
        ) == Self::modifier_group(
            actual,
            Mod::LGUIMOD | Mod::RGUIMOD,
        )
    }

    /// Returns whether a modifier group is active.
    fn modifier_group(modifiers: Mod, group: Mod) -> bool {
        modifiers.intersects(group)
    }

    /// Parse the TOML configuration and convert it into runtime bindings.
    fn from_config(contents: &str) -> io::Result<Self> {
        let config: KeyBindingsConfig =
            toml::from_str(contents).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid keybindings.toml: {}", error),
                )
            })?;

        let mut bindings = Vec::with_capacity(config.bindings.len());

        for (index, config_binding) in config.bindings.iter().enumerate() {
            let key =
                Self::parse_keycode(&config_binding.key)
                    .map_err(|error| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "binding {}: {}",
                                index + 1,
                                error
                            ),
                        )
                    })?;

            let modifiers =
                Self::parse_modifiers(&config_binding.modifiers)
                    .map_err(|error| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "binding {}: {}",
                                index + 1,
                                error
                            ),
                        )
                    })?;

            let command =
                Self::parse_command(&config_binding.command)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "binding {}: unknown command '{}'",
                                index + 1,
                                config_binding.command
                            ),
                        )
                    })?;

            bindings.push(KeyBinding {
                key,
                modifiers,
                command,
                repeatable: config_binding.repeatable,
            });
        }

        Ok(Self { bindings })
    }

    /// Convert TOML modifier names into SDL modifier flags.
    fn parse_modifiers(modifiers: &[String]) -> Result<Mod, String> {
        let mut result = Mod::NOMOD;

        for modifier in modifiers {
            match modifier.to_ascii_lowercase().as_str() {
                "shift" => {
                    result |= Mod::LSHIFTMOD;
                }

                "ctrl" | "control" => {
                    result |= Mod::LCTRLMOD;
                }

                "alt" => {
                    result |= Mod::LALTMOD;
                }

                "gui" | "meta" | "super" | "win" => {
                    result |= Mod::LGUIMOD;
                }

                "" => {
                    return Err(
                        "empty modifier name".to_string()
                    );
                }

                _ => {
                    return Err(format!(
                        "unknown modifier '{}'",
                        modifier
                    ));
                }
            }
        }

        Ok(result)
    }

    /// Convert a TOML key name into an SDL Keycode.
    fn parse_keycode(name: &str) -> Result<Keycode, String> {
        match name.to_ascii_lowercase().as_str() {
            "left" => Ok(Keycode::Left),
            "right" => Ok(Keycode::Right),
            "up" => Ok(Keycode::Up),
            "down" => Ok(Keycode::Down),
            "tab" => Ok(Keycode::Tab),

            "home" => Ok(Keycode::Home),
            "end" => Ok(Keycode::End),
            "pageup" | "page_up" => Ok(Keycode::PageUp),
            "pagedown" | "page_down" => Ok(Keycode::PageDown),

            "delete" | "del" => Ok(Keycode::Delete),
            "backspace" => Ok(Keycode::Backspace),

            "return" | "enter" => Ok(Keycode::Return),
            "kpenter" | "keypadenter" => Ok(Keycode::KpEnter),

            "escape" | "esc" => Ok(Keycode::Escape),

            "a" => Ok(Keycode::A),
            "b" => Ok(Keycode::B),
            "c" => Ok(Keycode::C),
            "d" => Ok(Keycode::D),
            "e" => Ok(Keycode::E),
            "f" => Ok(Keycode::F),
            "g" => Ok(Keycode::G),
            "h" => Ok(Keycode::H),
            "i" => Ok(Keycode::I),
            "j" => Ok(Keycode::J),
            "k" => Ok(Keycode::K),
            "l" => Ok(Keycode::L),
            "m" => Ok(Keycode::M),
            "n" => Ok(Keycode::N),
            "o" => Ok(Keycode::O),
            "p" => Ok(Keycode::P),
            "q" => Ok(Keycode::Q),
            "r" => Ok(Keycode::R),
            "s" => Ok(Keycode::S),
            "t" => Ok(Keycode::T),
            "u" => Ok(Keycode::U),
            "v" => Ok(Keycode::V),
            "w" => Ok(Keycode::W),
            "x" => Ok(Keycode::X),
            "y" => Ok(Keycode::Y),
            "z" => Ok(Keycode::Z),

            _ => Err(format!(
                "unknown key '{}'",
                name
            )),
        }
    }

    /// Convert a TOML command name into an editor command.
    ///
    /// Both PascalCase and snake_case are accepted so configuration
    /// remains convenient to edit.
    fn parse_command(name: &str) -> Option<Command> {
        match name.to_ascii_lowercase().as_str() {
            "moveleft" | "move_left" => Some(Command::MoveLeft),
            "moveright" | "move_right" => Some(Command::MoveRight),
            "moveup" | "move_up" => Some(Command::MoveUp),
            "movedown" | "move_down" => Some(Command::MoveDown),
            "movewordleft" | "move_word_left" => Some(Command::MoveWordLeft),
            "movewordright" | "move_word_right" => Some(Command::MoveWordRight),
            "pageup" | "page_up" => Some(Command::PageUp),
            "pagedown" | "page_down" => Some(Command::PageDown),

            "selectleft" | "select_left" => Some(Command::SelectLeft),
            "selectright" | "select_right" => Some(Command::SelectRight),
            "selectup" | "select_up" => Some(Command::SelectUp),
            "selectdown" | "select_down" => Some(Command::SelectDown),
            "selectall" | "select_all" => Some(Command::SelectAll),
            "selectwordleft" | "select_word_left" => Some(Command::SelectWordLeft),
            "selectwordright" | "select_word_right" => Some(Command::SelectWordRight),
            "selectpageup" | "select_page_up" => Some(Command::SelectPageUp),
            "selectpagedown" | "select_page_down" => Some(Command::SelectPageDown),
            "selecthome" | "select_home" => Some(Command::SelectHome),
            "selectend" | "select_end" => Some(Command::SelectEnd),

            "home" => Some(Command::Home),
            "end" => Some(Command::End),

            "delete" => Some(Command::Delete),
            "backspace" => Some(Command::Backspace),
            "newline" => Some(Command::Newline),

            "inserttab" => Some(Command::InsertTab),
            "quit" => Some(Command::Quit),

            "save" => Some(Command::Save),
            "saveas" => Some(Command::SaveAs),
            "undo" => Some(Command::Undo),
            "redo" => Some(Command::Redo),
            "copy" => Some(Command::Copy),
            "cut" => Some(Command::Cut),
            "paste" => Some(Command::Paste),

            _ => None,
        }
    }

}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_as_shortcuts_are_distinct_and_do_not_repeat() {
        let bindings = KeyBindings::default();
        for modifiers in [
            Mod::LCTRLMOD | Mod::LSHIFTMOD,
            Mod::RCTRLMOD | Mod::RSHIFTMOD,
            Mod::LGUIMOD | Mod::LSHIFTMOD,
        ] {
            assert_eq!(bindings.command_for(Keycode::S, modifiers, false), Some(Command::SaveAs));
            assert_eq!(bindings.command_for(Keycode::S, modifiers, true), None);
        }
        assert_eq!(bindings.command_for(Keycode::S, Mod::LCTRLMOD, false), Some(Command::Save));
    }

    #[test]
    fn parses_basic_binding() {
        let toml = r#"
            [[bindings]]
            key = "Left"
            command = "MoveLeft"
            repeatable = true
        "#;

        let bindings = KeyBindings::from_config(toml)
            .expect("configuration should parse");

        assert_eq!(
            bindings.command_for(
                Keycode::Left,
                Mod::NOMOD,
                false
            ),
            Some(Command::MoveLeft)
        );
    }

    #[test]
    fn parses_modifier_binding() {
        let toml = r#"
            [[bindings]]
            key = "S"
            modifiers = ["Ctrl"]
            command = "Save"
            repeatable = false
        "#;

        let bindings = KeyBindings::from_config(toml)
            .expect("configuration should parse");

        assert_eq!(
            bindings.command_for(
                Keycode::S,
                Mod::LCTRLMOD,
                false
            ),
            Some(Command::Save)
        );
    }

    #[test]
    fn right_ctrl_matches_left_ctrl_binding() {
        let toml = r#"
            [[bindings]]
            key = "S"
            modifiers = ["Ctrl"]
            command = "Save"
            repeatable = false
        "#;

        let bindings = KeyBindings::from_config(toml)
            .expect("configuration should parse");

        assert_eq!(
            bindings.command_for(
                Keycode::S,
                Mod::RCTRLMOD,
                false
            ),
            Some(Command::Save)
        );
    }

    #[test]
    fn shift_binding_works() {
        let toml = r#"
            [[bindings]]
            key = "Left"
            modifiers = ["Shift"]
            command = "SelectLeft"
            repeatable = true
        "#;

        let bindings = KeyBindings::from_config(toml)
            .expect("configuration should parse");

        assert_eq!(
            bindings.command_for(
                Keycode::Left,
                Mod::LSHIFTMOD,
                false
            ),
            Some(Command::SelectLeft)
        );

        assert_eq!(
            bindings.command_for(
                Keycode::Left,
                Mod::RSHIFTMOD,
                false
            ),
            Some(Command::SelectLeft)
        );
    }

    #[test]
    fn extra_modifier_does_not_match() {
        let toml = r#"
            [[bindings]]
            key = "S"
            modifiers = ["Ctrl"]
            command = "Save"
            repeatable = false
        "#;

        let bindings = KeyBindings::from_config(toml)
            .expect("configuration should parse");

        assert_eq!(
            bindings.command_for(
                Keycode::S,
                Mod::LCTRLMOD | Mod::LSHIFTMOD,
                false
            ),
            None
        );
    }

    #[test]
    fn repeatable_binding_accepts_repeat() {
        let toml = r#"
            [[bindings]]
            key = "Left"
            command = "MoveLeft"
            repeatable = true
        "#;

        let bindings = KeyBindings::from_config(toml)
            .expect("configuration should parse");

        assert_eq!(
            bindings.command_for(
                Keycode::Left,
                Mod::NOMOD,
                true
            ),
            Some(Command::MoveLeft)
        );
    }

    #[test]
    fn non_repeatable_binding_rejects_repeat() {
        let toml = r#"
            [[bindings]]
            key = "Escape"
            command = "Quit"
            repeatable = false
        "#;

        let bindings = KeyBindings::from_config(toml)
            .expect("configuration should parse");

        assert_eq!(
            bindings.command_for(
                Keycode::Escape,
                Mod::NOMOD,
                true
            ),
            None
        );
    }

    #[test]
    fn missing_modifiers_defaults_to_no_modifiers() {
        let toml = r#"
            [[bindings]]
            key = "Escape"
            command = "Quit"
        "#;

        let bindings = KeyBindings::from_config(toml)
            .expect("configuration should parse");

        assert_eq!(
            bindings.command_for(
                Keycode::Escape,
                Mod::NOMOD,
                false
            ),
            Some(Command::Quit)
        );
    }

    #[test]
    fn accepts_snake_case_commands() {
        let toml = r#"
            [[bindings]]
            key = "Left"
            command = "move_left"
        "#;

        let bindings = KeyBindings::from_config(toml)
            .expect("configuration should parse");

        assert_eq!(
            bindings.command_for(
                Keycode::Left,
                Mod::NOMOD,
                false
            ),
            Some(Command::MoveLeft)
        );
    }

    #[test]
    fn rejects_unknown_key() {
        let toml = r#"
            [[bindings]]
            key = "DefinitelyNotAKey"
            command = "MoveLeft"
        "#;

        let result = KeyBindings::from_config(toml);

        assert!(result.is_err());

        let error = result.unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unknown key")
        );
    }

    #[test]
    fn rejects_unknown_modifier() {
        let toml = r#"
            [[bindings]]
            key = "S"
            modifiers = ["SuperCoolModifier"]
            command = "Save"
        "#;

        let result = KeyBindings::from_config(toml);

        assert!(result.is_err());

        let error = result.unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unknown modifier")
        );
    }

    #[test]
    fn rejects_unknown_command() {
        let toml = r#"
            [[bindings]]
            key = "S"
            command = "DestroyComputer"
        "#;

        let result = KeyBindings::from_config(toml);

        assert!(result.is_err());

        let error = result.unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unknown command")
        );
    }

    #[test]
    fn parses_multiple_bindings() {
        let toml = r#"
            [[bindings]]
            key = "Left"
            command = "MoveLeft"
            repeatable = true

            [[bindings]]
            key = "Right"
            command = "MoveRight"
            repeatable = true

            [[bindings]]
            key = "S"
            modifiers = ["Ctrl"]
            command = "Save"

            [[bindings]]
            key = "Escape"
            command = "Quit"
        "#;

        let bindings = KeyBindings::from_config(toml)
            .expect("configuration should parse");

        assert_eq!(
            bindings.command_for(
                Keycode::Left,
                Mod::NOMOD,
                false
            ),
            Some(Command::MoveLeft)
        );

        assert_eq!(
            bindings.command_for(
                Keycode::Right,
                Mod::NOMOD,
                false
            ),
            Some(Command::MoveRight)
        );

        assert_eq!(
            bindings.command_for(
                Keycode::S,
                Mod::LCTRLMOD,
                false
            ),
            Some(Command::Save)
        );

        assert_eq!(
            bindings.command_for(
                Keycode::Escape,
                Mod::NOMOD,
                false
            ),
            Some(Command::Quit)
        );
    }

    #[test]
    fn default_bindings_work() {
        let bindings = KeyBindings::default();

        assert_eq!(
            bindings.command_for(
                Keycode::Left,
                Mod::NOMOD,
                false
            ),
            Some(Command::MoveLeft)
        );

        assert_eq!(
            bindings.command_for(
                Keycode::Right,
                Mod::NOMOD,
                false
            ),
            Some(Command::MoveRight)
        );

        assert_eq!(
            bindings.command_for(
                Keycode::S,
                Mod::LCTRLMOD,
                false
            ),
            Some(Command::Save)
        );

        assert_eq!(
            bindings.command_for(
                Keycode::Escape,
                Mod::NOMOD,
                false
            ),
            None
        );
        for modifiers in [Mod::LCTRLMOD, Mod::RCTRLMOD, Mod::LGUIMOD, Mod::RGUIMOD] {
            assert_eq!(bindings.command_for(Keycode::Q, modifiers, false), Some(Command::Quit));
            assert_eq!(bindings.command_for(Keycode::Q, modifiers, true), None);
        }
        assert_eq!(bindings.command_for(Keycode::Q, Mod::NOMOD, false), None);
    }

    #[test]
    fn missing_config_uses_embedded_bindings() {
        let path = std::env::temp_dir()
            .join(format!(
                "potyi-missing-keybindings-{}.toml",
                std::process::id(),
            ));

        let bindings = KeyBindings::load(
            path.to_str().unwrap()
        )
        .unwrap();

        assert_eq!(
            bindings.command_for(
                Keycode::S,
                Mod::LCTRLMOD,
                false
            ),
            Some(Command::Save)
        );
    }

    #[test]
    fn default_navigation_bindings_and_selection_variants() {
        let bindings = KeyBindings::default();
        for (key, modifiers, command) in [
            (Keycode::Left, Mod::RCTRLMOD, Command::MoveWordLeft),
            (Keycode::Right, Mod::RCTRLMOD, Command::MoveWordRight),
            (Keycode::Left, Mod::RALTMOD, Command::MoveWordLeft),
            (Keycode::Right, Mod::RALTMOD, Command::MoveWordRight),
            (Keycode::Left, Mod::RCTRLMOD | Mod::RSHIFTMOD, Command::SelectWordLeft),
            (Keycode::Right, Mod::RCTRLMOD | Mod::RSHIFTMOD, Command::SelectWordRight),
            (Keycode::Left, Mod::RALTMOD | Mod::RSHIFTMOD, Command::SelectWordLeft),
            (Keycode::Right, Mod::RALTMOD | Mod::RSHIFTMOD, Command::SelectWordRight),
            (Keycode::PageUp, Mod::NOMOD, Command::PageUp),
            (Keycode::PageDown, Mod::NOMOD, Command::PageDown),
            (Keycode::PageUp, Mod::RSHIFTMOD, Command::SelectPageUp),
            (Keycode::PageDown, Mod::RSHIFTMOD, Command::SelectPageDown),
            (Keycode::Home, Mod::RSHIFTMOD, Command::SelectHome),
            (Keycode::End, Mod::RSHIFTMOD, Command::SelectEnd),
        ] {
            assert_eq!(bindings.command_for(key, modifiers, false), Some(command));
            assert_eq!(bindings.command_for(key, modifiers, true), Some(command));
        }
        for modifiers in [Mod::RCTRLMOD, Mod::RGUIMOD] {
            assert_eq!(bindings.command_for(Keycode::A, modifiers, false), Some(Command::SelectAll));
            assert_eq!(bindings.command_for(Keycode::A, modifiers, true), None);
        }
    }

    #[test]
    fn parses_custom_page_and_word_selection_commands() {
        let bindings = KeyBindings::from_config(r#"
            [[bindings]]
            key = "page_up"
            modifiers = ["Shift"]
            command = "select_page_up"
            [[bindings]]
            key = "Right"
            modifiers = ["Ctrl", "Shift"]
            command = "select_word_right"
        "#).unwrap();
        assert_eq!(bindings.command_for(Keycode::PageUp, Mod::LSHIFTMOD, false), Some(Command::SelectPageUp));
        assert_eq!(bindings.command_for(Keycode::Right, Mod::LCTRLMOD | Mod::LSHIFTMOD, false), Some(Command::SelectWordRight));
    }
}
