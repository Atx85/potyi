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

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

static FONT_DATA: &[u8] = include_bytes!("../fonts/DejaVuSansMono.ttf");
const APP_TITLE: &str = concat!("Pötyi  ", env!("POTYI_DISPLAY_VERSION"));

mod app;
#[cfg(test)]
mod benchmarks;
mod clipboard;
mod command_bar;
mod config;
mod dpi_text;
mod editor;
mod emacs;
mod embedded_config;
mod experimental_terminal;
mod formatting;
mod keybindings;
mod line_numbers;
mod lsp;
mod lsp_setup;
mod lsp_ui;
mod multi_cursor;
mod piece_table;
mod renderer;
mod search;
mod search_ui;
mod startup;
mod syntax;
mod syntax_core;
mod terminal;
mod terminal_layout;
mod terminal_text_cache;
mod vim;
mod window;
mod workspace_edit;

// Shared internal types retained at the crate root for existing feature modules.
use app::commands::CommandOutcome;
#[cfg(test)]
use app::navigation::apply_keybinding_mode;
use app::navigation::parse_location;
use editor::{CursorState, Editor, HistoryEntry, HistoryKind, TextChange, file_is_open_in};
#[cfg(test)]
use piece_table::PieceTable;
use piece_table::PieceTableSnapshot;
#[cfg(test)]
use sdl3::keyboard::{Keycode, Mod};
use search::Searcher;
#[cfg(test)]
use vim::VimController;

fn main() -> Result<(), String> {
    let mut arguments = std::env::args();
    arguments.next();
    let first = arguments.next();
    if first.as_deref() == Some("--term-request") {
        return experimental_terminal::bridge::client(arguments.collect())
            .map_err(|e| e.to_string());
    }
    if first.as_deref() == Some("--term-new") {
        return experimental_terminal::run(arguments.next());
    }
    app::run()
}

#[cfg(test)]
mod tests;
