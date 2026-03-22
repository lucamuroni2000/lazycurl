use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::collections::HashMap;

use crate::app::Action;

/// Maps a string keybinding (from config) to a (modifiers, keycode) pair.
fn parse_binding(binding: &str) -> Option<(KeyModifiers, KeyCode)> {
    let parts: Vec<&str> = binding.split('+').collect();
    let mut modifiers = KeyModifiers::NONE;

    let key_part = if parts.len() == 1 {
        parts[0]
    } else {
        for &part in &parts[..parts.len() - 1] {
            match part.to_lowercase().as_str() {
                "ctrl" => modifiers |= KeyModifiers::CONTROL,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                "alt" => modifiers |= KeyModifiers::ALT,
                _ => {}
            }
        }
        parts[parts.len() - 1]
    };

    let code = match key_part.to_lowercase().as_str() {
        "enter" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "escape" | "esc" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        "/" => KeyCode::Char('/'),
        "?" => KeyCode::Char('?'),
        s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        _ => return None,
    };

    Some((modifiers, code))
}

/// Build a lookup table from (modifiers, keycode) -> Action using config keybindings.
pub fn build_keymap(
    keybindings: &HashMap<String, String>,
) -> HashMap<(KeyModifiers, KeyCode), Action> {
    let mut map = HashMap::new();

    let action_map: Vec<(&str, Action)> = vec![
        ("send_request", Action::SendRequest),
        ("save_request", Action::SaveRequest),
        ("switch_env", Action::SwitchEnvironment),
        ("copy_curl", Action::CopyCurl),
        ("new_request", Action::NewRequest),
        ("cycle_panes", Action::CyclePaneForward),
        ("search", Action::Search),
        ("help", Action::Help),
        ("cancel", Action::Cancel),
        ("toggle_collections", Action::ToggleCollections),
        ("toggle_request", Action::ToggleRequest),
        ("toggle_response", Action::ToggleResponse),
        ("reveal_secrets", Action::RevealSecrets),
    ];

    for (key, action) in action_map {
        if let Some(binding) = keybindings.get(key) {
            if let Some(parsed) = parse_binding(binding) {
                map.insert(parsed, action);
            }
        }
    }

    // Always register Ctrl+Q as quit (not remappable)
    map.insert((KeyModifiers::CONTROL, KeyCode::Char('q')), Action::Quit);
    // F5 as fallback for send (Ctrl+Enter compatibility)
    map.entry((KeyModifiers::NONE, KeyCode::F(5)))
        .or_insert(Action::SendRequest);
    // Shift+Tab for backward cycling
    map.insert(
        (KeyModifiers::SHIFT, KeyCode::BackTab),
        Action::CyclePaneBackward,
    );

    map
}

/// Look up a key event in the keymap.
pub fn resolve_action(key: KeyEvent, keymap: &HashMap<(KeyModifiers, KeyCode), Action>) -> Action {
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }
    keymap
        .get(&(key.modifiers, key.code))
        .cloned()
        .unwrap_or(Action::None)
}
