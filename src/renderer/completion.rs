// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

impl Renderer<'_> {
    pub(crate) fn set_completion(&mut self, display: Option<crate::lsp_ui::Display>) {
        self.completion = display;
        self.completion_bounds = None;
    }

    pub(crate) fn completion_hit_at(&self, x: i32, y: i32) -> Option<usize> {
        let bounds = self.completion_bounds?;
        let display = self.completion.as_ref()?;
        if display.total == 0 {
            return None;
        }
        if !bounds.contains_point(Point::new(x, y)) {
            return None;
        }
        let row = (y - bounds.y() - 4) / (self.command_font.height() + 6);
        if y < bounds.y() + 4 || row < 0 || row as usize >= display.labels.len() {
            return None;
        }
        Some(display.first + row as usize)
    }

    pub(super) fn render_completion(&mut self, table: &mut PieceTable) -> Result<(), String> {
        self.completion_bounds = None;
        let Some(display) = self.completion.clone() else {
            return Ok(());
        };
        if table.cursor.line < self.scroll_line
            || table.cursor.line >= self.scroll_line + self.visible_line_count()
        {
            return Ok(());
        }
        let (left, pane_width) = self.pane_bounds(self.active_pane);
        let text_left = self.text_left(table)?;
        let cursor_x = left
            + self.clipped_column_x(table, table.cursor.line, table.cursor.column, text_left)?;
        let cursor_y = TITLE_BAR_HEIGHT
            + 8
            + (table.cursor.line - self.scroll_line) as i32 * self.font.height();
        let row_height = self.command_font.height() + 6;
        let height = row_height * (display.labels.len() as i32 + 1) + 8;
        let width = 440.min(pane_width - 8).max(1);
        let x = cursor_x.clamp(left + 4, (left + pane_width - width - 4).max(left + 4));
        let below = cursor_y + self.font.height() + 2;
        let y = if below + height <= self.window_height {
            below
        } else {
            (cursor_y - height - 2).max(TITLE_BAR_HEIGHT + 2)
        };
        let bounds = Rect::new(
            x,
            y,
            width as u32,
            height.min(self.window_height - y).max(1) as u32,
        );
        self.completion_bounds = Some(bounds);
        self.canvas.set_clip_rect(bounds);
        self.canvas.set_draw_color(Color::RGB(43, 47, 51));
        self.canvas.fill_rect(bounds).map_err(|e| e.to_string())?;
        std::mem::swap(&mut self.font, &mut self.command_font);
        std::mem::swap(&mut self.raster_font, &mut self.command_raster_font);
        std::mem::swap(&mut self.char_width, &mut self.command_char_width);
        let result = (|| {
            for (row, label) in display.labels.iter().enumerate() {
                let y = y + 4 + row as i32 * row_height;
                if display.total > 0 && row == display.selected {
                    self.canvas.set_draw_color(Color::RGB(57, 85, 111));
                    self.canvas
                        .fill_rect(Rect::new(
                            x + 2,
                            y,
                            (width - 4).max(1) as u32,
                            row_height as u32,
                        ))
                        .map_err(|e| e.to_string())?;
                }
                self.render_dpi_text(
                    label,
                    (x + 8) as f32,
                    (y + 3) as f32,
                    Color::RGB(228, 231, 234),
                )?;
            }
            let hint = if display.total == 0 {
                "Esc: cancel · You can keep typing".into()
            } else {
                format!(
                    "{} / {} · Enter: insert · Esc: close",
                    display.first + display.selected + 1,
                    display.total
                )
            };
            self.render_dpi_text(
                &hint,
                (x + 8) as f32,
                (y + 7 + display.labels.len() as i32 * row_height) as f32,
                Color::RGB(157, 177, 190),
            )
        })();
        std::mem::swap(&mut self.font, &mut self.command_font);
        std::mem::swap(&mut self.raster_font, &mut self.command_raster_font);
        std::mem::swap(&mut self.char_width, &mut self.command_char_width);
        self.canvas.set_clip_rect(None);
        result.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires SDL; run with SDL_VIDEODRIVER=dummy and --test-threads=1"]
    fn completion_mouse_rows_stay_inside_each_pane() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video
            .window("Completion", 900, 500)
            .hidden()
            .build()
            .unwrap();
        let ttf = sdl3::ttf::init().unwrap();
        let font = || {
            ttf.load_font_from_iostream(
                sdl3::iostream::IOStream::from_bytes(crate::FONT_DATA).unwrap(),
                18.0,
            )
            .unwrap()
        };
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let mut renderer = Renderer::new(
            canvas,
            &texture_creator,
            font(),
            font(),
            18.0,
            (font(), font()),
            crate::window::WindowHitTestState::new(900, 1.0),
        )
        .unwrap();
        let mut table = PieceTable::empty().unwrap();
        table.insert(0, "transform.").unwrap();
        table.move_cursor(10).unwrap();
        for split in [false, true] {
            renderer.set_split_mode(split);
            for pane in [0, 1] {
                renderer.set_active_pane(pane);
                renderer.set_completion(Some(crate::lsp_ui::Display {
                    labels: vec!["position".into(), "Translate".into()],
                    selected: 1,
                    first: 4,
                    total: 12,
                }));
                renderer.update_cursor(&table);
                if !split {
                    renderer
                        .render(&mut table, &SearchUi::new(), &CommandBar::new())
                        .unwrap();
                    if pane == 0 {
                        if let Ok(path) = std::env::var("POTYI_COMPLETION_SCREENSHOT") {
                            renderer
                                .canvas
                                .read_pixels(Rect::new(0, 0, 900, 500))
                                .unwrap()
                                .save_bmp(path)
                                .unwrap();
                        }
                    }
                } else {
                    renderer.render_completion(&mut table).unwrap();
                }
                let bounds = renderer.completion_bounds.unwrap();
                let (left, width) = renderer.pane_bounds(pane);
                assert!(bounds.x() >= left && bounds.right() <= left + width);
                assert_eq!(
                    renderer.completion_hit_at(bounds.x() + 5, bounds.y() + 6),
                    Some(4)
                );
                assert_eq!(
                    renderer.completion_hit_at(
                        bounds.x() + 5,
                        bounds.y() + 6 + renderer.command_font.height() + 6
                    ),
                    Some(5)
                );
                assert_eq!(
                    renderer.completion_hit_at(bounds.x() - 1, bounds.y() + 5),
                    None
                );
                assert_eq!(
                    renderer.completion_hit_at(bounds.x() + 5, bounds.bottom() - 2),
                    None
                );
            }
        }
        renderer.set_completion(None);
        assert!(renderer.completion_hit_at(1, 1).is_none());
    }
}
