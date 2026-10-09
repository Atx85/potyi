// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use sdl3::keyboard::{Keycode, Mod};

pub(super) fn control(modifiers: Mod) -> bool {
    modifiers.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)
}
pub(super) fn shift(modifiers: Mod) -> bool {
    modifiers.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD)
}
pub(super) fn alt(modifiers: Mod) -> bool {
    modifiers.intersects(Mod::LALTMOD | Mod::RALTMOD)
}
pub(super) fn command(modifiers: Mod) -> bool {
    modifiers.intersects(Mod::LGUIMOD | Mod::RGUIMOD)
}

pub(super) fn alt_graph(modifiers: Mod) -> bool {
    cfg!(windows) && modifiers.intersects(Mod::MODEMOD | Mod::RALTMOD) && control(modifiers)
}

/// Encode keys instead of editing a local input field. Printable text arrives
/// through SDL TextInput, which also preserves composed Unicode input.
pub(super) fn key_bytes(key: Keycode, modifiers: Mod, application_cursor: bool) -> Option<Vec<u8>> {
    use Keycode as K;
    if alt_graph(modifiers) {
        return None;
    }
    if control(modifiers) {
        let control_byte = match key {
            K::A => 1,
            K::B => 2,
            K::C => 3,
            K::D => 4,
            K::E => 5,
            K::F => 6,
            K::G => 7,
            K::H => 8,
            K::I => 9,
            K::J => 10,
            K::K => 11,
            K::L => 12,
            K::M => 13,
            K::N => 14,
            K::O => 15,
            K::P => 16,
            K::Q => 17,
            K::R => 18,
            K::S => 19,
            K::T => 20,
            K::U => 21,
            K::V => 22,
            K::W => 23,
            K::X => 24,
            K::Y => 25,
            K::Z => 26,
            K::LeftBracket => 27,
            K::Backslash => 28,
            K::RightBracket => 29,
            K::Space => 0,
            K::Backspace => 23,
            _ => 255,
        };
        if control_byte != 255 {
            return Some(if alt(modifiers) {
                vec![27, control_byte]
            } else {
                vec![control_byte]
            });
        }
    }
    let modifier = 1
        + u8::from(shift(modifiers))
        + 2 * u8::from(alt(modifiers))
        + 4 * u8::from(control(modifiers));
    let arrow = match key {
        K::Up => 'A',
        K::Down => 'B',
        K::Right => 'C',
        K::Left => 'D',
        K::Home => 'H',
        K::End => 'F',
        _ => '\0',
    };
    if arrow != '\0' {
        return Some(if modifier > 1 {
            format!("\x1b[1;{modifier}{arrow}").into_bytes()
        } else {
            format!("\x1b{}{arrow}", if application_cursor { 'O' } else { '[' }).into_bytes()
        });
    }
    let tilde = match key {
        K::Insert => 2,
        K::Delete => 3,
        K::PageUp => 5,
        K::PageDown => 6,
        K::F5 => 15,
        K::F6 => 17,
        K::F7 => 18,
        K::F8 => 19,
        K::F9 => 20,
        K::F10 => 21,
        K::F11 => 23,
        K::F12 => 24,
        _ => 0,
    };
    if tilde != 0 {
        return Some(if modifier > 1 {
            format!("\x1b[{tilde};{modifier}~").into_bytes()
        } else {
            format!("\x1b[{tilde}~").into_bytes()
        });
    }
    let function = match key {
        K::F1 => 'P',
        K::F2 => 'Q',
        K::F3 => 'R',
        K::F4 => 'S',
        _ => '\0',
    };
    if function != '\0' {
        return Some(if modifier > 1 {
            format!("\x1b[1;{modifier}{function}").into_bytes()
        } else {
            format!("\x1bO{function}").into_bytes()
        });
    }
    let bytes = match key {
        K::Return | K::KpEnter => b"\r".as_slice(),
        K::Backspace => b"\x7f".as_slice(),
        K::Escape => b"\x1b".as_slice(),
        K::Tab if shift(modifiers) => b"\x1b[Z".as_slice(),
        K::Tab => b"\t".as_slice(),
        _ => return None,
    };
    Some(if alt(modifiers) {
        [b"\x1b".as_slice(), bytes].concat()
    } else {
        bytes.to_vec()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_keys_reach_the_shell_including_escape_interrupt_and_application_arrows() {
        assert_eq!(
            key_bytes(Keycode::Escape, Mod::NOMOD, false),
            Some(vec![27])
        );
        assert_eq!(key_bytes(Keycode::C, Mod::LCTRLMOD, false), Some(vec![3]));
        assert_eq!(
            key_bytes(Keycode::Up, Mod::NOMOD, false),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            key_bytes(Keycode::Up, Mod::NOMOD, true),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            key_bytes(Keycode::Left, Mod::LCTRLMOD, true),
            Some(b"\x1b[1;5D".to_vec())
        );
        assert_eq!(
            key_bytes(Keycode::Tab, Mod::LSHIFTMOD, false),
            Some(b"\x1b[Z".to_vec())
        );
        assert_eq!(key_bytes(Keycode::A, Mod::NOMOD, false), None);
    }
}
