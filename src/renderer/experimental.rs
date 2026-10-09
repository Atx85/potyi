// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
//! Embedded grid rendering uses the editor's chrome, font, and pane geometry.
use super::*;
use crate::experimental_terminal::{
    links::FileLink,
    pane::{Pane, Ready},
};

impl<'a> Renderer<'a> {
    #[cfg(test)]
    pub(crate) fn terminal_visual_scalar_width(&self, character: char) -> i32 {
        self.logical_text_size(&character.to_string()).0.ceil() as i32
    }

    pub(crate) fn experimental_events(
        &mut self,
        events: &sdl3::EventSubsystem,
    ) -> Result<(), String> {
        if let Err(error) = events.register_custom_event::<Ready>() {
            if error.to_string() != "The same event type can not be registered twice!" {
                return Err(error.to_string());
            }
        }
        let sender = events.event_sender();
        self.experimental_wake = Some(std::sync::Arc::new(move || {
            let _ = sender.push_custom_event(Ready);
        }));
        Ok(())
    }
    pub(crate) fn open_experimental(
        &mut self,
        directory: &std::path::Path,
        command: Option<&str>,
    ) -> Result<(), String> {
        if self.experimental.as_ref().is_none_or(|pane| pane.closed()) {
            let wake = self
                .experimental_wake
                .clone()
                .ok_or("Terminal events are unavailable")?;
            self.experimental = Some(
                Pane::new(directory, 24, 80, wake, self.active_pane).map_err(|e| e.to_string())?,
            );
            self.experimental_cache.clear();
        }
        let pane = self.experimental.as_mut().unwrap();
        pane.reopen();
        pane.pane = self.active_pane;
        if let Some(command) = command {
            pane.launch_command(command, self.split_mode)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub(crate) fn experimental_command_running(&self) -> bool {
        self.experimental
            .as_ref()
            .is_some_and(Pane::command_running)
    }
    pub(crate) fn has_experimental(&self) -> bool {
        self.experimental.is_some()
    }
    pub(crate) fn experimental_enter_directory(
        &mut self,
        directory: &std::path::Path,
    ) -> Result<(), String> {
        self.experimental
            .as_mut()
            .ok_or("Terminal is unavailable")?
            .enter_directory(directory)
            .map_err(|e| e.to_string())
    }
    pub(crate) fn experimental_visible(&self) -> bool {
        self.experimental
            .as_ref()
            .is_some_and(|p| p.visible && (self.split_mode || p.pane == self.active_pane))
    }
    pub(crate) fn experimental_focused(&self) -> bool {
        self.experimental
            .as_ref()
            .is_some_and(|p| p.visible && p.pane == self.active_pane)
    }
    pub(crate) fn hide_experimental(&mut self) {
        self.experimental_cache.clear();
        if let Some(p) = &mut self.experimental {
            p.visible = false;
        }
    }
    pub(crate) fn experimental_pane(&self) -> Option<usize> {
        self.experimental.as_ref().map(|p| p.pane)
    }
    pub(crate) fn poll_experimental(&mut self) -> bool {
        let Some(pane) = self.experimental.as_mut() else {
            return false;
        };
        let was_visible = pane.visible;
        let changed = pane.poll() | pane.poll_copy(&self.canvas.window().subsystem().clipboard());
        if was_visible && !pane.visible {
            self.experimental_cache.clear();
        }
        changed && (was_visible || pane.visible)
    }
    pub(crate) fn experimental_clipboard(
        &mut self,
        paste: bool,
        all: bool,
        clipboard: &sdl3::clipboard::ClipboardUtil,
    ) {
        if let Some(pane) = self.experimental.as_mut() {
            pane.clipboard(paste, all, clipboard);
        }
    }
    pub(crate) fn experimental_clipboard_mode(&mut self) -> Option<bool> {
        self.experimental.as_mut()?.take_clipboard_mode()
    }
    pub(crate) fn experimental_dragging(&self) -> bool {
        self.experimental.as_ref().is_some_and(Pane::dragging)
    }
    #[cfg(test)]
    pub(crate) fn terminal_visual_export(&self, path: &std::path::Path) -> Result<(), String> {
        self.canvas
            .read_pixels(None)
            .map_err(|e| e.to_string())?
            .save_bmp(path)
            .map_err(|e| e.to_string())
    }
    #[cfg(test)]
    pub(crate) fn experimental_cached_glyphs(&self) -> bool {
        self.experimental_cache.has_glyphs()
    }
    #[cfg(test)]
    pub(crate) fn experimental_running(&self) -> bool {
        self.experimental.as_ref().is_some_and(Pane::running)
    }
    #[cfg(test)]
    pub(crate) fn experimental_prompt(&self) -> &str {
        self.experimental
            .as_ref()
            .map(Pane::prompt_text)
            .unwrap_or("")
    }
    #[cfg(test)]
    pub(crate) fn experimental_row_y(&self, index: usize) -> i32 {
        TITLE_BAR_HEIGHT + 8 + index as i32 * self.terminal_line_height() + 4
    }
    #[cfg(test)]
    pub(crate) fn experimental_resource_stats(&self) -> (usize, u64) {
        self.experimental
            .as_ref()
            .map_or((0, 0), Pane::resource_stats)
    }
    #[cfg(test)]
    pub(crate) fn experimental_resource_select_tail(&mut self, bytes: usize) -> Result<(), String> {
        self.experimental
            .as_mut()
            .ok_or("Missing terminal")?
            .resource_select_tail(bytes)
            .map_err(|e| e.to_string())
    }
    #[cfg(test)]
    pub(crate) fn experimental_back_available(&self) -> bool {
        self.experimental.as_ref().is_some_and(Pane::can_go_back)
    }
    #[cfg(test)]
    pub(crate) fn experimental_back_point(&self) -> (f32, f32) {
        let (left, width) = self.pane_bounds(self.experimental_pane().unwrap());
        (
            (left + width - TERMINAL_BAR_MARGIN - TERMINAL_EDITOR_WIDTH - TERMINAL_CLEAR_WIDTH / 2)
                as f32,
            (self.window_height - TERMINAL_BAR_MARGIN - TERMINAL_BAR_HEIGHT / 2) as f32,
        )
    }
    #[cfg(test)]
    pub(crate) fn experimental_output_pending(&self) -> bool {
        self.experimental.as_ref().is_some_and(Pane::output_pending)
    }
    #[cfg(test)]
    pub(crate) fn experimental_output_cursor(&self) -> Option<(u16, u16)> {
        self.experimental.as_ref()?.output_cursor()
    }
    #[cfg(test)]
    pub(crate) fn experimental_text_point(&self, needle: &str) -> Option<(f32, f32)> {
        self.experimental.as_ref()?.text_point(needle)
    }
    #[cfg(test)]
    pub(crate) fn experimental_terminal_text_point(&self, needle: &str) -> Option<(f32, f32)> {
        self.experimental.as_ref()?.terminal_text_point(needle)
    }
    #[cfg(test)]
    pub(crate) fn experimental_path_point(&self, path: &std::path::Path) -> Option<(f32, f32)> {
        self.experimental.as_ref()?.path_point(path)
    }
    #[cfg(test)]
    pub(crate) fn experimental_browser_rows(
        &mut self,
    ) -> Vec<crate::experimental_terminal::browser::Row> {
        self.experimental.as_mut().unwrap().browser_rows()
    }
    #[cfg(test)]
    pub(crate) fn experimental_contents(&self) -> String {
        self.experimental
            .as_ref()
            .map(Pane::contents)
            .unwrap_or_default()
    }
    #[cfg(test)]
    pub(crate) fn experimental_ready(&self) -> bool {
        self.experimental.as_ref().is_some_and(Pane::ready)
    }
    #[cfg(test)]
    pub(crate) fn experimental_directory(&self) -> Option<&std::path::Path> {
        self.experimental.as_ref().map(Pane::directory)
    }
    #[cfg(test)]
    pub(crate) fn experimental_preview(&self) {
        if let Some(path) = std::env::var_os("POTYI_EMBEDDED_PREVIEW") {
            self.canvas
                .read_pixels(None)
                .unwrap()
                .save_bmp(path)
                .unwrap();
        }
    }
    pub(crate) fn experimental_send(&mut self, command: &str) -> Result<(), String> {
        self.experimental
            .as_mut()
            .ok_or("Terminal is unavailable")?
            .send_command(command)
            .map_err(|e| e.to_string())
    }
    pub(crate) fn experimental_pending(&self) -> bool {
        self.experimental.as_ref().is_some_and(Pane::pending)
    }
    pub(crate) fn experimental_request(&mut self) -> Option<FileLink> {
        self.experimental.as_mut().and_then(Pane::request)
    }
    pub(crate) fn experimental_status(&mut self, message: String) {
        if let Some(p) = &mut self.experimental {
            p.set_status(message);
        }
    }
    pub(crate) fn experimental_event(
        &mut self,
        event: &Event,
        mods: sdl3::keyboard::Mod,
        clipboard: &sdl3::clipboard::ClipboardUtil,
        vim: bool,
    ) -> Option<crate::experimental_terminal::pane::Action> {
        let pane = self.experimental.as_mut()?;
        let action = pane.event(event, mods, clipboard, vim);
        if !pane.visible {
            self.experimental_cache.clear();
        }
        action
    }
    pub(crate) fn experimental_hit(&self, x: i32, y: i32, command_bar: &CommandBar) -> bool {
        if !self.experimental_visible() || self.split_divider_hit(x, y) {
            return false;
        }
        let (left, width) = self.pane_bounds(self.experimental_pane().unwrap());
        x >= left
            && x < left + width
            && y >= TITLE_BAR_HEIGHT
            && y < self.window_height - command_bar.reserved_height()
    }
    pub(crate) fn experimental_button(
        &mut self,
        x: i32,
        y: i32,
        command_bar: &CommandBar,
    ) -> Option<crate::experimental_terminal::pane::Action> {
        let (left, width) = self.pane_bounds(self.experimental_pane()?);
        let top = self.window_height
            - command_bar.reserved_height()
            - TERMINAL_BAR_MARGIN
            - TERMINAL_BAR_HEIGHT;
        if y < top || y >= top + TERMINAL_BAR_HEIGHT {
            return None;
        }
        let x = x - left;
        let editor_left = width - TERMINAL_BAR_MARGIN - TERMINAL_EDITOR_WIDTH;
        if x >= editor_left {
            return Some(crate::experimental_terminal::pane::Action::Editor);
        }
        let action_left = editor_left - TERMINAL_CLEAR_WIDTH - TERMINAL_ACTION_WIDTH;
        let cursor = (x < action_left).then(|| self.experimental_prompt_cursor_at(x, width));
        let pane = self.experimental.as_mut()?;
        if x >= editor_left - TERMINAL_CLEAR_WIDTH {
            if pane.can_go_back() {
                pane.go_back();
            } else {
                pane.clear();
            }
        } else if x >= editor_left - TERMINAL_CLEAR_WIDTH - TERMINAL_ACTION_WIDTH {
            pane.stop();
        } else {
            pane.set_prompt_cursor(cursor.unwrap_or(0));
        }
        return Some(crate::experimental_terminal::pane::Action::Handled);
    }
    fn experimental_prompt_cursor_at(&self, x: i32, width: i32) -> usize {
        let pane = self.experimental.as_ref().unwrap();
        let input = pane.prompt_text();
        let cursor = pane.prompt_cursor().min(input.len());
        let text_x = TERMINAL_BAR_MARGIN + 10;
        if x <= text_x {
            return 0;
        }
        let field_width = (width
            - TERMINAL_BAR_MARGIN
            - TERMINAL_EDITOR_WIDTH
            - TERMINAL_CLEAR_WIDTH
            - TERMINAL_ACTION_WIDTH
            - 8
            - text_x)
            .max(1);
        let measure = |text: &str| {
            self.font
                .size_of(text)
                .map(|(width, _)| width as i32)
                .unwrap_or_else(|_| text.chars().count() as i32 * self.char_width)
        };
        let scroll =
            Self::search_query_scroll_x(measure(&input[..cursor]), measure(input), field_width, 4);
        let target = x - text_x + scroll;
        // Prefix widths are monotonic in the embedded monospaced font. Search
        // UTF-8 boundaries instead of rescanning every growing prefix.
        let (mut lower, mut upper) = (0, input.len());
        while lower < upper {
            let mut middle = lower + (upper - lower) / 2;
            while !input.is_char_boundary(middle) {
                middle -= 1;
            }
            let end = middle + input[middle..].chars().next().unwrap().len_utf8();
            let before = measure(&input[..middle]);
            let after = measure(&input[..end]);
            if target < before + (after - before) / 2 {
                upper = middle;
            } else {
                lower = end;
            }
        }
        lower
    }
    pub(crate) fn render_experimental(
        &mut self,
        active: &mut PieceTable,
        inactive: &mut PieceTable,
        terminal: &mut Terminal,
        search: &SearchUi,
        command_bar: &CommandBar,
    ) -> Result<(), String> {
        self.bottom_inset = command_bar.reserved_height();
        self.terminal_reserved_height = self.bottom_inset;
        self.canvas.set_draw_color(Color::RGB(30, 30, 30));
        self.canvas.clear();
        self.render_title_bar()?;
        let pane_index = self.experimental_pane().unwrap();
        let old_visible = self.terminal_visible(terminal) && self.terminal_pane != pane_index;
        if self.split_mode {
            if self.active_pane != pane_index
                && (!old_visible || self.active_pane != self.terminal_pane)
            {
                self.render_document_pane(
                    active,
                    Some(search),
                    true,
                    self.pane_bounds(self.active_pane),
                )?;
            }
            let other = 1 - self.active_pane;
            if other != pane_index && (!old_visible || other != self.terminal_pane) {
                self.swap_view();
                let result =
                    self.render_document_pane(inactive, None, false, self.pane_bounds(other));
                self.swap_view();
                result?;
            }
            if old_visible {
                self.render_terminal_pane(terminal, self.pane_bounds(self.terminal_pane))?;
            }
        }
        let (left, width) = self.pane_bounds(pane_index);
        let height = self.window_height - command_bar.reserved_height();
        // Use the same wrapped hint lines and reserved body space as :term.
        let full_width = self.window_width;
        self.window_width = width;
        let pane = self.experimental.as_ref().unwrap();
        let mut status_lines = pane
            .status()
            .map(|status| self.terminal_status_lines(status))
            .unwrap_or_default();
        status_lines.extend(
            self.terminal_status_lines(
                pane.output_focus_hint()
                    .unwrap_or("Command input · Shift+Up: select output · Esc: editor"),
            ),
        );
        if pane.can_go_back() {
            status_lines
                .extend(self.terminal_status_lines("Commit diff · Back / Alt+Left: return to log"));
        }
        self.window_width = full_width;
        let status_height = if status_lines.is_empty() {
            0
        } else {
            status_lines.len() as i32 * self.font.height().max(1) + 8
        };
        let body = Rect::new(
            left + 4,
            TITLE_BAR_HEIGHT,
            (width - 8).max(1) as u32,
            (height - TITLE_BAR_HEIGHT - TERMINAL_BAR_HEIGHT - TERMINAL_BAR_MARGIN - status_height)
                .max(1) as u32,
        );
        let mut pane = self.experimental.take().unwrap();
        let cell = (self.char_width.max(1), self.terminal_line_height());
        let logical_font = &self.font;
        let mut measure = |character: char| {
            logical_font
                .size_of(&character.to_string())
                .map(|(width, _)| width as i32)
                .unwrap_or(cell.0)
        };
        let result = pane.draw(
            &mut self.canvas,
            self.texture_creator,
            &mut self.raster_font,
            &mut self.experimental_cache,
            cell,
            self.dpi_text.raster_scale(),
            pane_index == self.active_pane && !command_bar.is_active(),
            body,
            self.tab_width,
            self.logical_font_size.to_bits(),
            &mut measure,
        );
        self.experimental = Some(pane);
        result?;
        self.canvas.set_viewport(Rect::new(
            left,
            0,
            width.max(1) as u32,
            height.max(1) as u32,
        ));
        let top = height - TERMINAL_BAR_MARGIN - TERMINAL_BAR_HEIGHT;
        let status_y = top - status_lines.len() as i32 * self.font.height().max(1) - 8;
        for (index, line) in status_lines.iter().enumerate() {
            self.render_dpi_text(
                line,
                (TERMINAL_BAR_MARGIN + 10) as f32,
                (status_y + index as i32 * self.font.height().max(1)) as f32,
                Color::RGB(225, 205, 150),
            )?;
        }
        self.canvas.set_draw_color(Color::RGB(40, 43, 43));
        self.canvas
            .fill_rect(Rect::new(
                TERMINAL_BAR_MARGIN,
                top,
                (width - TERMINAL_BAR_MARGIN * 2).max(1) as u32,
                TERMINAL_BAR_HEIGHT as u32,
            ))
            .map_err(|e| e.to_string())?;
        let editor_left = width - TERMINAL_BAR_MARGIN - TERMINAL_EDITOR_WIDTH;
        let pane = self.experimental.as_ref().unwrap();
        let running = pane.running();
        let can_go_back = pane.can_go_back();
        let output_focused = pane.output_focus_hint().is_some();
        let input = pane.prompt_text().to_owned();
        let cursor = pane.prompt_cursor().min(input.len());
        self.render_terminal_button(
            editor_left - TERMINAL_CLEAR_WIDTH - TERMINAL_ACTION_WIDTH,
            top,
            TERMINAL_ACTION_WIDTH,
            if running { "Stop" } else { "Again" },
            running,
        )?;
        self.render_terminal_button(
            editor_left - TERMINAL_CLEAR_WIDTH,
            top,
            TERMINAL_CLEAR_WIDTH,
            if can_go_back { "Back" } else { "Clear" },
            false,
        )?;
        self.render_terminal_button(editor_left, top, TERMINAL_EDITOR_WIDTH, "Editor", false)?;
        let text_x = TERMINAL_BAR_MARGIN + 10;
        let action_left = editor_left - TERMINAL_CLEAR_WIDTH - TERMINAL_ACTION_WIDTH;
        let field_width = (action_left - 8).saturating_sub(text_x).max(1);
        self.canvas.set_clip_rect(Rect::new(
            text_x,
            top,
            field_width as u32,
            TERMINAL_BAR_HEIGHT as u32,
        ));
        let prefix = input.get(..cursor).unwrap_or(&input);
        let cursor_width = self
            .font
            .size_of(prefix)
            .map(|(width, _)| width as i32)
            .unwrap_or_else(|_| prefix.chars().count() as i32 * self.char_width);
        let input_width = self
            .font
            .size_of(&input)
            .map(|(width, _)| width as i32)
            .unwrap_or_else(|_| input.chars().count() as i32 * self.char_width);
        let scroll_x = Self::search_query_scroll_x(cursor_width, input_width, field_width, 4);
        let text_y = top + (TERMINAL_BAR_HEIGHT - self.font.height()) / 2;
        self.render_dpi_text(
            &input,
            (text_x - scroll_x) as f32,
            text_y as f32,
            if running {
                Color::RGB(145, 145, 145)
            } else {
                Color::RGB(235, 235, 235)
            },
        )?;
        if !running && !output_focused && pane_index == self.active_pane && !command_bar.is_active()
        {
            self.canvas.set_draw_color(Color::RGB(245, 245, 245));
            self.canvas
                .draw_line(
                    Point::new(text_x + cursor_width - scroll_x, top + 9),
                    Point::new(
                        text_x + cursor_width - scroll_x,
                        top + TERMINAL_BAR_HEIGHT - 9,
                    ),
                )
                .map_err(|e| e.to_string())?;
        }
        self.canvas.set_clip_rect(None);
        self.canvas.set_viewport(None);
        self.bottom_inset = command_bar.reserved_height();
        if self.split_mode {
            self.canvas.set_draw_color(Color::RGB(74, 74, 74));
            self.canvas
                .fill_rect(Rect::new(
                    self.split_divider(),
                    TITLE_BAR_HEIGHT,
                    1,
                    (self.window_height - TITLE_BAR_HEIGHT).max(1) as u32,
                ))
                .map_err(|e| e.to_string())?;
            let (left, width) = self.pane_bounds(self.active_pane);
            self.canvas.set_draw_color(Color::RGB(85, 145, 220));
            self.canvas
                .fill_rect(Rect::new(left, TITLE_BAR_HEIGHT, width.max(1) as u32, 2))
                .map_err(|e| e.to_string())?;
        }
        self.render_command_bar(command_bar, search)?;
        if !self.experimental_focused() && !self.terminal_focused(terminal) {
            self.render_completion(active)?;
        }
        #[cfg(test)]
        self.experimental_preview();
        self.canvas.present();
        Ok(())
    }
}
