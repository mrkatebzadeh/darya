use crossterm::event::{KeyCode, KeyEvent};

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
        let action = match key_event.code {
            KeyCode::Char('k') | KeyCode::Up => InputAction::MoveUp,
            KeyCode::Char('j') | KeyCode::Down => InputAction::MoveDown,
            KeyCode::Char('h') | KeyCode::Left => InputAction::Collapse,
            KeyCode::Char('l') | KeyCode::Right => InputAction::Expand,
            KeyCode::Char('q') if key_event.modifiers.is_empty() => InputAction::Quit,
            KeyCode::Enter => InputAction::Select,
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
            state.process_key(event(KeyCode::Enter)),
            InputAction::Select
        );
    }
}
