// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Legacy plain command output, with bounded partial text and escape state.
use std::io;

pub(super) const MAX_PARTIAL_BYTES: usize = 60 * 1024;

#[derive(Clone, Copy, Default)]
enum State {
    #[default]
    Ground,
    Escape,
    Csi,
    Osc,
    OscEscape,
}

pub(super) struct Capture {
    state: State,
    carriage_return: bool,
    utf8: [u8; 4],
    utf8_len: usize,
    partial: String,
}
impl Default for Capture {
    fn default() -> Self {
        Self {
            state: State::Ground,
            carriage_return: false,
            utf8: [0; 4],
            utf8_len: 0,
            partial: String::with_capacity(MAX_PARTIAL_BYTES),
        }
    }
}
impl Capture {
    pub(super) fn reset(&mut self) {
        self.state = State::Ground;
        self.carriage_return = false;
        self.utf8_len = 0;
        self.partial.clear();
    }
    pub(super) fn tail(&self) -> &str {
        &self.partial
    }
    pub(super) fn process(
        &mut self,
        bytes: &[u8],
        mut emit: impl FnMut(&str, bool) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut position = 0;
        while position < bytes.len() {
            if matches!(self.state, State::Ground) && !self.carriage_return && self.utf8_len == 0 {
                let remaining = MAX_PARTIAL_BYTES - self.partial.len();
                let run = bytes[position..]
                    .iter()
                    .take(remaining.max(1))
                    .take_while(|byte| matches!(**byte, b'\t' | 0x20..=0x7f))
                    .count();
                if run > 0 {
                    if remaining == 0 {
                        emit(&self.partial, true)?;
                        self.partial.clear();
                    } else {
                        // This span contains only ASCII and tabs. Keep the
                        // byte parser for every control and UTF-8 byte.
                        self.partial.push_str(
                            std::str::from_utf8(&bytes[position..position + run])
                                .expect("ASCII span"),
                        );
                        position += run;
                    }
                    continue;
                }
            }
            let byte = bytes[position];
            position += 1;
            // The driver's CAN prefix recovers a hidden boundary after an
            // unfinished CSI/OSC. It never contributes visible plain text.
            if byte == 0x18 {
                self.state = State::Ground;
                continue;
            }
            match self.state {
                State::Ground => match byte {
                    0x1b => self.state = State::Escape,
                    b'\r' => self.carriage_return = true,
                    _ => {
                        if self.carriage_return {
                            if byte != b'\n' {
                                self.byte(b'\n', &mut emit)?;
                            }
                            self.carriage_return = false;
                        }
                        if matches!(byte, b'\n' | b'\t') || byte >= 0x20 {
                            self.byte(byte, &mut emit)?;
                        }
                    }
                },
                State::Escape => {
                    self.state = match byte {
                        b'[' => State::Csi,
                        b']' => State::Osc,
                        _ => State::Ground,
                    };
                }
                State::Csi => {
                    if (0x40..=0x7e).contains(&byte) {
                        self.state = State::Ground;
                    }
                }
                State::Osc => {
                    if byte == 7 {
                        self.state = State::Ground;
                    } else if byte == 0x1b {
                        self.state = State::OscEscape;
                    }
                }
                State::OscEscape => {
                    self.state = if byte == b'\\' {
                        State::Ground
                    } else {
                        State::Osc
                    };
                }
            }
        }
        Ok(())
    }
    pub(super) fn finish(
        &mut self,
        mut emit: impl FnMut(&str, bool) -> io::Result<()>,
    ) -> io::Result<()> {
        if self.carriage_return {
            // Match legacy flush order: final CR is appended before a trailing
            // incomplete UTF-8 sequence is replaced.
            self.character('\n', &mut emit)?;
            self.carriage_return = false;
        }
        if self.utf8_len != 0 {
            self.character('\u{fffd}', &mut emit)?;
            self.utf8_len = 0;
        }
        if !self.partial.is_empty() {
            emit(&self.partial, false)?;
            self.partial.clear();
        }
        Ok(())
    }
    fn byte(
        &mut self,
        byte: u8,
        emit: &mut impl FnMut(&str, bool) -> io::Result<()>,
    ) -> io::Result<()> {
        self.utf8[self.utf8_len] = byte;
        self.utf8_len += 1;
        loop {
            match std::str::from_utf8(&self.utf8[..self.utf8_len]) {
                Ok(text) => {
                    let character = text.chars().next().unwrap();
                    self.utf8_len = 0;
                    self.character(character, emit)?;
                    return Ok(());
                }
                Err(error) => {
                    let Some(length) = error.error_len() else {
                        return Ok(());
                    };
                    self.utf8.copy_within(length..self.utf8_len, 0);
                    self.utf8_len -= length;
                    self.character('\u{fffd}', emit)?;
                    if self.utf8_len == 0 {
                        return Ok(());
                    }
                }
            }
        }
    }
    fn character(
        &mut self,
        character: char,
        emit: &mut impl FnMut(&str, bool) -> io::Result<()>,
    ) -> io::Result<()> {
        if character == '\n' {
            emit(&self.partial, false)?;
            self.partial.clear();
            return Ok(());
        }
        if self.partial.len() + character.len_utf8() > MAX_PARTIAL_BYTES {
            emit(&self.partial, true)?;
            self.partial.clear();
        }
        self.partial.push(character);
        Ok(())
    }
}

#[cfg(test)]
#[path = "literal_output_tests.rs"]
mod tests;
