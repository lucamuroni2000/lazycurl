pub mod collections;
pub mod environment_manager;
pub mod export_picker;
pub mod help;
pub mod import_overlay;
pub mod layout;
pub mod log_viewer;
pub mod picker;
pub mod project_picker;
pub mod project_tabs;
pub mod request;
pub mod response;
pub mod statusbar;
pub mod variables;

use std::collections::HashMap;

use ratatui::Frame;

use crate::app::App;

/// Format a binding string for display (e.g. "ctrl+s" → "Ctrl+S").
pub fn format_key_display(binding: &str) -> String {
    binding
        .split('+')
        .map(|part| {
            // Check for uppercase single char BEFORE lowercasing
            if part.len() == 1 {
                let c = part.chars().next().unwrap();
                if c.is_ascii_uppercase() {
                    return c.to_string();
                }
                return part.to_string();
            }
            match part.to_lowercase().as_str() {
                "ctrl" => "Ctrl".to_string(),
                "shift" => "Shift".to_string(),
                "alt" => "Alt".to_string(),
                "enter" => "Enter".to_string(),
                "escape" | "esc" => "Esc".to_string(),
                "backtab" => "Tab".to_string(),
                "tab" => "Tab".to_string(),
                "space" => "Space".to_string(),
                s if s.starts_with('f') && s[1..].parse::<u8>().is_ok() => s.to_uppercase(),
                _ => part.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// Look up a keybinding and return its display name, or "?" if not found.
pub fn key_for(kb: &HashMap<String, String>, action: &str) -> String {
    kb.get(action)
        .map(|k| format_key_display(k))
        .unwrap_or_else(|| "?".to_string())
}

/// Look up two keybindings and return them as "X/Y".
pub fn key_pair_for(kb: &HashMap<String, String>, action1: &str, action2: &str) -> String {
    format!("{}/{}", key_for(kb, action1), key_for(kb, action2))
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    // All non-help rendering uses an immutable borrow of app + keybindings.
    // Help overlay needs &mut App (to clamp cursor), so it's rendered last
    // after the immutable borrows are dropped.
    {
        let kb = &app.config.keybindings;
        let pane_layout = layout::compute_layout(frame.area(), app.pane_visible);

        // Title bar — project tabs + env
        project_tabs::draw(frame, app, pane_layout.title_bar);

        // Panes
        if let Some(area) = pane_layout.collections {
            collections::draw(frame, app, area);
        }
        if let Some(area) = pane_layout.request {
            request::draw(frame, app, area, kb);
        }
        if let Some(area) = pane_layout.response {
            response::draw(frame, app, area);
        }

        // Status bar
        statusbar::draw(frame, app, pane_layout.status_bar, kb);

        // Method picker (rendered relative to request pane)
        if app.show_method_picker {
            if let Some(area) = pane_layout.request {
                request::draw_method_picker(frame, app, area);
            }
        }

        // Auth type picker (rendered relative to request pane)
        if app.show_auth_picker {
            if let Some(area) = pane_layout.request {
                request::draw_auth_picker(frame, app, area);
            }
        }

        // Body type picker (rendered relative to request pane)
        if app.show_body_type_picker {
            if let Some(area) = pane_layout.request {
                request::draw_body_type_picker(frame, app, area);
            }
        }

        // Overlays (on top of everything)
        if app.show_export_picker {
            export_picker::draw(frame, app, kb);
        }
        if app.show_import_overlay {
            import_overlay::draw(frame, app, kb);
        }
        if app.show_collection_picker {
            picker::draw_collection_picker(frame, app, kb);
        }
        if app.show_variables {
            variables::draw(frame, app, kb);
        }
        if app.show_env_manager {
            environment_manager::draw(frame, app, kb);
        }
        if app.show_project_picker {
            project_picker::draw(frame, app, kb);
        }
        if app.show_log_viewer {
            log_viewer::draw(frame, app);
        }
        if app.show_first_launch {
            project_picker::draw_first_launch(frame, app);
        }
    }

    // Help overlay rendered last — needs &mut App to clamp cursor
    if app.show_help {
        help::draw(frame, app);
    }
}
