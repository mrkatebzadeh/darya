// Copyright (C) 2026 M.R. Siavash Katebzadeh <mr@katebzadeh.xyz>
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

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

/// Actions exposed by input handling that the rest of the application understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    MoveUp,
    MoveDown,
    Expand,
    Collapse,
    JumpTop,
    JumpBottom,
    Select,
    Delete,
    Open,
    Quit,
    None,
}

/// Stateful processor that understands multi-key sequences like `gg`.
#[derive(Debug, Default)]
pub struct InputState {
    pending_gg: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_key(&mut self, key_event: KeyEvent) -> InputAction {
        if key_event.kind != KeyEventKind::Press && key_event.kind != KeyEventKind::Repeat {
            return InputAction::None;
        }

        let action = match key_event.code {
            KeyCode::Char('k') | KeyCode::Up => InputAction::MoveUp,
            KeyCode::Char('j') | KeyCode::Down => InputAction::MoveDown,
            KeyCode::Char('h') | KeyCode::Left => InputAction::Collapse,
            KeyCode::Char('l') | KeyCode::Right => InputAction::Expand,
            KeyCode::Char('d') => InputAction::Delete,
            KeyCode::Char('o') => InputAction::Open,
            KeyCode::Char('q') if key_event.modifiers.is_empty() => InputAction::Quit,
            KeyCode::Enter | KeyCode::Tab => InputAction::Select,
            KeyCode::Char('g') => {
                if self.pending_gg {
                    InputAction::JumpTop
                } else {
                    self.pending_gg = true;
                    InputAction::None
                }
            }
            KeyCode::Char('G') => InputAction::JumpBottom,
            _ => InputAction::None,
        };

        if key_event.code != KeyCode::Char('g') || action == InputAction::JumpTop {
            self.pending_gg = false;
        }

        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn gg_triggers_jump_top() {
        let mut state = InputState::new();
        assert_eq!(
            state.process_key(event(KeyCode::Char('g'))),
            InputAction::None
        );
        assert_eq!(
            state.process_key(event(KeyCode::Char('g'))),
            InputAction::JumpTop
        );
    }

    #[test]
    fn singly_g_does_not_jump() {
        let mut state = InputState::new();
        assert_eq!(
            state.process_key(event(KeyCode::Char('g'))),
            InputAction::None
        );
        assert_eq!(
            state.process_key(event(KeyCode::Char('h'))),
            InputAction::Collapse
        );
    }

    #[test]
    fn uppercase_g_jumps_bottom() {
        let mut state = InputState::new();
        assert_eq!(
            state.process_key(event(KeyCode::Char('G'))),
            InputAction::JumpBottom
        );
    }

    #[test]
    fn navigation_keys_map_correctly() {
        let mut state = InputState::new();
        assert_eq!(
            state.process_key(event(KeyCode::Char('k'))),
            InputAction::MoveUp
        );
        assert_eq!(
            state.process_key(event(KeyCode::Char('j'))),
            InputAction::MoveDown
        );
        assert_eq!(
            state.process_key(event(KeyCode::Char('h'))),
            InputAction::Collapse
        );
        assert_eq!(
            state.process_key(event(KeyCode::Char('l'))),
            InputAction::Expand
        );
        assert_eq!(
            state.process_key(event(KeyCode::Char('q'))),
            InputAction::Quit
        );
        assert_eq!(
            state.process_key(event(KeyCode::Char('d'))),
            InputAction::Delete
        );
        assert_eq!(
            state.process_key(event(KeyCode::Char('o'))),
            InputAction::Open
        );
        assert_eq!(
            state.process_key(event(KeyCode::Enter)),
            InputAction::Select
        );
        assert_eq!(state.process_key(event(KeyCode::Tab)), InputAction::Select);
    }
}
