// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Copy)]
enum Prefix { Vim, EmacsTerminal }

#[derive(Default)]
pub(crate) struct PaneKeys {
    pending: Option<Prefix>,
    consumed_key: Option<Keycode>,
    suppress_text: bool,
}

impl PaneKeys {
    pub fn cancel(&mut self) { *self = Self::default(); }
    pub fn consume_text(&mut self) -> bool { std::mem::take(&mut self.suppress_text) }
    fn consume(&mut self, key: Keycode) {
        self.consumed_key = Some(key);
        self.suppress_text = true;
    }
}

pub(super) fn handle(context: InputContext<'_, '_>, key: Keycode, mods: Mod, repeat: bool) -> KeyResult {
    let InputContext { pane_keys, editor, other_editor, renderer, terminal, vim, other_vim,
        command_bar, search_ui, lsp_ui, active_pane, split_mode, dirty, .. } = context;
    pane_keys.suppress_text = false;
    if command_bar.is_active() {
        pane_keys.cancel();
        return Ok(None);
    }
    // Modifier key presses between the two chords must not cancel the prefix.
    if matches!(key, Keycode::LCtrl | Keycode::RCtrl | Keycode::LShift | Keycode::RShift
        | Keycode::LAlt | Keycode::RAlt | Keycode::LGui | Keycode::RGui) {
        return Ok(pane_keys.pending.map(|_| EventFlow::Continue));
    }
    if repeat && pane_keys.consumed_key == Some(key) {
        pane_keys.consume(key);
        return Ok(Some(EventFlow::Continue));
    }
    pane_keys.consumed_key = None;
    let ctrl = ctrl_pressed(mods);
    let plain_or_ctrl = !mods.intersects(Mod::LALTMOD | Mod::RALTMOD | Mod::LGUIMOD
        | Mod::RGUIMOD | Mod::LSHIFTMOD | Mod::RSHIFTMOD);
    if let Some(prefix) = pane_keys.pending.take() {
        pane_keys.consume(key);
        let target = match prefix {
            Prefix::Vim if editor.config.keybinding_mode == KeybindingMode::Vim && plain_or_ctrl => {
                match key {
                    Keycode::W => Some(1 - *active_pane),
                    Keycode::H | Keycode::Left => Some(0),
                    Keycode::L | Keycode::Right => Some(1),
                    _ => None,
                }
            }
            Prefix::EmacsTerminal if editor.config.keybinding_mode == KeybindingMode::Emacs
                && plain_or_ctrl && !ctrl && key == Keycode::O => Some(1 - *active_pane),
            _ => None,
        };
        if *split_mode && let Some(target) = target {
            search_ui.close();
            lsp_ui.dismiss_completion();
            focus_pane(target, active_pane, editor, other_editor, vim, other_vim, renderer);
            *dirty = true;
        }
        // Escape, Ctrl+G and unsupported second keys cancel without editing.
        return Ok(Some(EventFlow::Continue));
    }
    if ctrl && plain_or_ctrl && !repeat {
        pane_keys.pending = match editor.config.keybinding_mode {
            KeybindingMode::Vim if key == Keycode::W
                && (renderer.terminal_focused(terminal) || vim.mode() != vim::VimMode::Insert) => {
                vim.cancel_pending();
                Some(Prefix::Vim)
            }
            // Editor Emacs prefixes retain their existing full command set.
            KeybindingMode::Emacs if key == Keycode::X && renderer.terminal_focused(terminal) => {
                Some(Prefix::EmacsTerminal)
            }
            _ => None,
        };
        if pane_keys.pending.is_some() {
            pane_keys.consume(key);
            return Ok(Some(EventFlow::Continue));
        }
    }
    Ok(None)
}
