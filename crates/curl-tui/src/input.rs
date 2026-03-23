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
/// Normalizes SHIFT modifier for printable characters since terminals
/// inconsistently report SHIFT for chars like '?', '/', uppercase letters, etc.
pub fn resolve_action(key: KeyEvent, keymap: &HashMap<(KeyModifiers, KeyCode), Action>) -> Action {
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }

    // Try exact match first
    if let Some(action) = keymap.get(&(key.modifiers, key.code)) {
        return action.clone();
    }

    // Terminals differ on modifier reporting for characters.
    // Try several normalizations:
    if let KeyCode::Char(c) = key.code {
        // 1. Strip SHIFT for punctuation (e.g., '?' is Shift+/ on US keyboards)
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            let without_shift = key.modifiers - KeyModifiers::SHIFT;
            if let Some(action) = keymap.get(&(without_shift, KeyCode::Char(c))) {
                return action.clone();
            }
        }
        // 2. Try lowercase version (some terminals send uppercase with Ctrl)
        let lower = c.to_ascii_lowercase();
        if lower != c {
            if let Some(action) = keymap.get(&(key.modifiers, KeyCode::Char(lower))) {
                return action.clone();
            }
        }
        // 3. For Ctrl+letter, some terminals add SHIFT; try without
        if key
            .modifiers
            .contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT)
        {
            let just_ctrl = KeyModifiers::CONTROL;
            if let Some(action) = keymap.get(&(just_ctrl, KeyCode::Char(lower))) {
                return action.clone();
            }
        }
    }

    Action::None
}

/// Resolve a key event when in Normal mode but not bound in the keymap.
/// Handles arrow keys, Enter, tab switching, etc.
pub fn resolve_navigation(key: KeyEvent) -> Action {
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Up) => Action::MoveUp,
        (KeyModifiers::NONE, KeyCode::Down) => Action::MoveDown,
        (KeyModifiers::NONE, KeyCode::Enter) => Action::Enter,
        (KeyModifiers::NONE, KeyCode::Left) => Action::PrevTab,
        (KeyModifiers::NONE, KeyCode::Right) => Action::NextTab,
        (KeyModifiers::NONE, KeyCode::Char('a')) => Action::AddItem,
        (KeyModifiers::NONE, KeyCode::Char('d')) => Action::DeleteItem,
        (KeyModifiers::NONE, KeyCode::Char('r')) => Action::Rename,
        (KeyModifiers::NONE, KeyCode::Char('v')) => Action::OpenVariables,
        (KeyModifiers::NONE, KeyCode::Char('s')) => Action::ToggleSecretFlag,
        (KeyModifiers::NONE, KeyCode::Char('j')) => Action::MoveDown,
        (KeyModifiers::NONE, KeyCode::Char('k')) => Action::MoveUp,
        _ => Action::None,
    }
}

/// Resolve a key event when in Editing mode.
/// Characters go to the text field, special keys are editing commands.
pub fn resolve_editing(key: KeyEvent) -> Action {
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Esc) => Action::Cancel, // Exit editing
        (KeyModifiers::NONE, KeyCode::Enter) => Action::Enter, // Confirm / move to next field
        (KeyModifiers::CONTROL, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::F(5)) => {
            Action::SendRequest
        }
        (KeyModifiers::CONTROL, KeyCode::Char('q')) => Action::Quit,
        (KeyModifiers::NONE, KeyCode::Backspace) => Action::Backspace,
        (KeyModifiers::NONE, KeyCode::Delete) => Action::Delete,
        (KeyModifiers::NONE, KeyCode::Left) => Action::CursorLeft,
        (KeyModifiers::NONE, KeyCode::Right) => Action::CursorRight,
        (KeyModifiers::NONE, KeyCode::Home) => Action::Home,
        (KeyModifiers::NONE, KeyCode::End) => Action::End,
        (KeyModifiers::NONE, KeyCode::Tab) => Action::CyclePaneForward,
        // Accept any printable character regardless of modifier combo.
        // On many keyboards, { [ } ] @ ~ etc. require AltGr (reported as ALT|CONTROL)
        // or other modifier combinations. We only exclude pure Ctrl+letter shortcuts.
        (mods, KeyCode::Char(c)) => {
            let is_pure_ctrl = mods.contains(KeyModifiers::CONTROL)
                && !mods.contains(KeyModifiers::ALT)
                && c.is_ascii_alphabetic();
            if is_pure_ctrl {
                // Ctrl+letter shortcut — don't treat as char input
                match c {
                    'q' => Action::Quit,
                    _ => Action::None,
                }
            } else {
                Action::CharInput(c)
            }
        }
        _ => Action::None,
    }
}
