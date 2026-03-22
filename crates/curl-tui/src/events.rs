use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;

use crate::app::Action;

/// Poll for terminal events and convert to app actions.
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Map a key event to an app action using default keybindings.
pub fn key_to_action(key: KeyEvent) -> Action {
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }

    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('q')) => Action::Quit,
        (KeyModifiers::NONE, KeyCode::Tab) => Action::CyclePaneForward,
        (KeyModifiers::SHIFT, KeyCode::BackTab) => Action::CyclePaneBackward,
        (KeyModifiers::CONTROL, KeyCode::Enter) => Action::SendRequest,
        (KeyModifiers::NONE, KeyCode::F(5)) => Action::SendRequest,
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => Action::SaveRequest,
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => Action::SwitchEnvironment,
        (KeyModifiers::CONTROL, KeyCode::Char('n')) => Action::NewRequest,
        (KeyModifiers::CONTROL, KeyCode::Char('y')) => Action::CopyCurl,
        (KeyModifiers::CONTROL, KeyCode::Char('1')) => Action::ToggleCollections,
        (KeyModifiers::CONTROL, KeyCode::Char('2')) => Action::ToggleRequest,
        (KeyModifiers::CONTROL, KeyCode::Char('3')) => Action::ToggleResponse,
        (KeyModifiers::NONE, KeyCode::F(8)) => Action::RevealSecrets,
        (KeyModifiers::NONE, KeyCode::Char('?')) => Action::Help,
        (KeyModifiers::NONE, KeyCode::Esc) => Action::Cancel,
        _ => Action::None,
    }
}
