use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::collections::HashMap;

use crate::app::{Action, InputContext};

/// Context-scoped keymaps: one inner map per InputContext.
pub type ContextKeymaps = HashMap<InputContext, HashMap<(KeyModifiers, KeyCode), Action>>;

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
        // For single characters, preserve original case so "E" and "e" are distinct
        _ if key_part.len() == 1 => KeyCode::Char(key_part.chars().next().unwrap()),
        _ => return None,
    };

    Some((modifiers, code))
}

/// Maps a config key string to an (InputContext, Action) pair.
fn action_for_key(key: &str) -> Option<(InputContext, Action)> {
    use InputContext::*;
    match key {
        // Global actions
        "quit" => Some((Global, Action::Quit)),
        "cancel" => Some((Global, Action::Cancel)),
        "help" => Some((Global, Action::Help)),
        "search" => Some((Global, Action::Search)),
        "send_request" => Some((Global, Action::SendRequest)),
        "save_request" => Some((Global, Action::SaveRequest)),
        "new_request" => Some((Global, Action::NewRequest)),
        "switch_env" => Some((Global, Action::SwitchEnvironment)),
        "manage_envs" => Some((Global, Action::ManageEnvironments)),
        "open_variables" => Some((Global, Action::OpenVariables)),
        "open_export" => Some((Global, Action::OpenExportPicker)),
        "open_log_viewer" => Some((Global, Action::OpenLogViewer)),
        "open_project_picker" => Some((Global, Action::OpenProjectPicker)),
        "reveal_secrets" => Some((Global, Action::RevealSecrets)),
        "focus_url" => Some((Global, Action::FocusUrl)),
        "cycle_method" => Some((Global, Action::CycleMethod)),
        "change_auth_type" => Some((Global, Action::ChangeAuthType)),
        "move_up" => Some((Global, Action::MoveUp)),
        "move_down" => Some((Global, Action::MoveDown)),
        "enter" => Some((Global, Action::Enter)),
        "next_tab" => Some((Global, Action::NextTab)),
        "prev_tab" => Some((Global, Action::PrevTab)),
        "cycle_pane_forward" => Some((Global, Action::CyclePaneForward)),
        "cycle_pane_backward" => Some((Global, Action::CyclePaneBackward)),
        "next_project" => Some((Global, Action::NextProject)),
        "prev_project" => Some((Global, Action::PrevProject)),
        "focus_collections" => Some((Global, Action::FocusCollections)),
        "focus_request" => Some((Global, Action::FocusRequest)),
        "focus_response" => Some((Global, Action::FocusResponse)),
        "add_item" => Some((Global, Action::AddItem)),
        "delete_item" => Some((Global, Action::DeleteItem)),
        "rename" => Some((Global, Action::Rename)),
        "toggle_enabled" => Some((Global, Action::ToggleEnabled)),
        "copy" => Some((Global, Action::Copy)),
        // confirm_yes is handled by raw key bypass in main.rs, not the keymap
        "close_project" => Some((Global, Action::CloseProject)),
        // Log viewer context
        "log_viewer.filter" => Some((LogViewer, Action::LogFilter)),
        "log_viewer.clear_filter" => Some((LogViewer, Action::LogClearFilter)),
        "log_viewer.clear_search" => Some((LogViewer, Action::LogClearSearch)),
        "log_viewer.next_match" => Some((LogViewer, Action::LogNextMatch)),
        "log_viewer.prev_match" => Some((LogViewer, Action::LogPrevMatch)),
        "log_viewer.export" => Some((LogViewer, Action::LogExport)),
        "log_viewer.copy_path" => Some((LogViewer, Action::LogCopyPath)),
        // Variables context
        "variables.cycle_container_fwd" => Some((Variables, Action::CycleContainerForward)),
        "variables.cycle_container_back" => Some((Variables, Action::CycleContainerBackward)),
        _ => None,
    }
}

/// Build context-scoped lookup tables from (modifiers, keycode) -> Action,
/// using the flat config keybindings map.
pub fn build_context_keymaps(keybindings: &HashMap<String, String>) -> ContextKeymaps {
    let mut keymaps: ContextKeymaps = HashMap::new();

    for (key, binding) in keybindings {
        if let Some((context, action)) = action_for_key(key) {
            if let Some(parsed) = parse_binding(binding) {
                keymaps.entry(context).or_default().insert(parsed, action);
            }
        }
    }

    // Universal fallbacks in Global keymap — arrow keys always work regardless of preset
    let global = keymaps.entry(InputContext::Global).or_default();
    global
        .entry((KeyModifiers::NONE, KeyCode::Up))
        .or_insert(Action::MoveUp);
    global
        .entry((KeyModifiers::NONE, KeyCode::Down))
        .or_insert(Action::MoveDown);
    global
        .entry((KeyModifiers::NONE, KeyCode::Left))
        .or_insert(Action::PrevTab);
    global
        .entry((KeyModifiers::NONE, KeyCode::Right))
        .or_insert(Action::NextTab);
    global
        .entry((KeyModifiers::NONE, KeyCode::Enter))
        .or_insert(Action::Enter);
    // F5 as fallback for send (Ctrl+Enter compatibility)
    global
        .entry((KeyModifiers::NONE, KeyCode::F(5)))
        .or_insert(Action::SendRequest);

    keymaps
}

/// Look up a key event in a single keymap with terminal normalization.
/// Normalizes SHIFT modifier for printable characters since terminals
/// inconsistently report SHIFT for chars like '?', '/', uppercase letters, etc.
fn lookup_with_normalization(
    key: KeyEvent,
    keymap: &HashMap<(KeyModifiers, KeyCode), Action>,
) -> Action {
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

        // 4. Try with no modifiers — handles AltGr ({, [, ], } on non-US keyboards).
        //    AltGr is reported as CONTROL|ALT by crossterm.
        //    Skip plain Ctrl+letter to avoid e.g. Ctrl+E matching bare 'e'.
        let is_ctrl_without_alt = key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT);
        if key.modifiers != KeyModifiers::NONE && !is_ctrl_without_alt {
            if let Some(action) = keymap.get(&(KeyModifiers::NONE, KeyCode::Char(c))) {
                return action.clone();
            }
        }

        // 5. Unbound character — pass through so overlays can handle it
        return Action::CharInput(c);
    }

    Action::None
}

/// Look up a key event in the context-scoped keymaps.
/// Checks context-specific keymap first (if not Global), then falls back to Global.
pub fn resolve_action(key: KeyEvent, keymaps: &ContextKeymaps, context: InputContext) -> Action {
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }
    // Try context-specific keymap first (if not Global)
    if context != InputContext::Global {
        if let Some(context_map) = keymaps.get(&context) {
            let result = lookup_with_normalization(key, context_map);
            if !matches!(result, Action::None | Action::CharInput(_)) {
                return result;
            }
        }
    }
    // Fall back to Global keymap
    if let Some(global_map) = keymaps.get(&InputContext::Global) {
        return lookup_with_normalization(key, global_map);
    }
    Action::None
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
