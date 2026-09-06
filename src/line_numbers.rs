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


use sdl3::pixels::Color;
use sdl3::render::{Canvas, FRect};
use sdl3::ttf::Font;
use sdl3::video::Window;

use crate::config::LineNumberMode;

pub struct LineNumbers;

impl LineNumbers {
    fn display_number(
        mode: LineNumberMode,
        line: usize,
        cursor_line: usize,
    ) -> usize {
        match mode {
            LineNumberMode::Normal => line + 1,
            LineNumberMode::Relative =>
                line.abs_diff(cursor_line),
            LineNumberMode::Dynamic
                if line == cursor_line => line + 1,
            LineNumberMode::Dynamic =>
                line.abs_diff(cursor_line),
        }
    }

    pub fn width<'a>(
        font: &Font<'a>,
        line_count: usize,
    ) -> Result<i32, String> {
        let last_line =
            line_count.max(1).to_string();

        let (text_width, _) =
            font.size_of(&last_line)
                .map_err(|error| error.to_string())?;

        // Space on both sides of the line numbers.
        Ok(text_width as i32 + 20)
    }

    pub fn render<'a>(
        canvas: &mut Canvas<Window>,
        logical_font: &Font<'a>,
        raster_font: &Font<'a>,
        first_line: usize,
        visible_lines: usize,
        line_count: usize,
        line_height: i32,
        mode: LineNumberMode,
        cursor_line: usize,
    ) -> Result<(), String> {
        let gutter_width =
            Self::width(
                logical_font,
                line_count,
            )?;

        let texture_creator =
            canvas.texture_creator();

        for row in 0..visible_lines {
            let line =
                first_line + row;

            if line >= line_count {
                break;
            }

            let text =
                Self::display_number(
                    mode,
                    line,
                    cursor_line,
                ).to_string();

            let (logical_width, logical_height) =
                logical_font
                    .size_of(&text)
                    .map_err(|error| error.to_string())?;

            let surface =
                raster_font.render(&text)
                    .blended(
                        Color::RGB(
                            120,
                            120,
                            120,
                        ),
                    )
                    .map_err(|error| error.to_string())?;

            let texture =
                texture_creator
                    .create_texture_from_surface(
                        &surface,
                    )
                    .map_err(|error| error.to_string())?;

            let x =
                gutter_width
                    - 10
                    - logical_width as i32;

            let y =
                crate::window::TITLE_BAR_HEIGHT
        + 8 + row as i32 * line_height;

            canvas
                .copy(
                    &texture,
                    None,
                    FRect::new(
                        x as f32,
                        y as f32,
                        logical_width as f32,
                        logical_height as f32,
                    ),
                )
                .map_err(|error| error.to_string())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_numbers_are_absolute() {
        assert_eq!(
            LineNumbers::display_number(
                LineNumberMode::Normal,
                7,
                4,
            ),
            8,
        );
    }

    #[test]
    fn relative_numbers_are_cursor_distances() {
        assert_eq!(
            LineNumbers::display_number(
                LineNumberMode::Relative,
                7,
                4,
            ),
            3,
        );
        assert_eq!(
            LineNumbers::display_number(
                LineNumberMode::Relative,
                4,
                4,
            ),
            0,
        );
    }

    #[test]
    fn dynamic_numbers_keep_cursor_line_absolute() {
        assert_eq!(
            LineNumbers::display_number(
                LineNumberMode::Dynamic,
                7,
                4,
            ),
            3,
        );
        assert_eq!(
            LineNumbers::display_number(
                LineNumberMode::Dynamic,
                4,
                4,
            ),
            5,
        );
    }
}
