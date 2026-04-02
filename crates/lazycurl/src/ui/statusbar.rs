use std::collections::HashMap;

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, EditField, InputMode, Pane, RequestTab};

pub fn draw(frame: &mut Frame, app: &App, area: Rect, keybindings: &HashMap<String, String>) {
    let kb = keybindings;

    let mode_indicator = match app.input_mode {
        InputMode::Normal => Span::styled(
            " NORMAL ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::Editing => Span::styled(
            " EDITING ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    };

    // If editing a name field (collection or request name), show it prominently in the status bar
    let is_name_edit = matches!(
        app.edit_field,
        Some(EditField::NewCollectionName)
            | Some(EditField::CollectionName(_))
            | Some(EditField::RequestName)
            | Some(EditField::NewProjectName)
    );

    if is_name_edit && app.input_mode == InputMode::Editing {
        let label = match app.edit_field {
            Some(EditField::NewCollectionName) => "Collection name",
            Some(EditField::CollectionName(_)) => "Rename collection",
            Some(EditField::RequestName) => "Request name",
            Some(EditField::NewProjectName) => "Project name",
            _ => "Name",
        };

        let line = Line::from(vec![
            mode_indicator,
            Span::raw(" "),
            Span::styled(format!(" {}: ", label), Style::default().fg(Color::Yellow)),
            Span::styled(
                app.name_input.content().to_string(),
                Style::default().fg(Color::White).bg(Color::DarkGray),
            ),
            Span::styled(
                "  Enter:confirm  Esc:cancel ",
                Style::default().fg(Color::DarkGray),
            ),
        ]);

        let status = Paragraph::new(line).style(Style::default().bg(Color::Black));
        frame.render_widget(status, area);

        // Show cursor in the status bar for name editing
        let cursor_x = area.x
            + mode_indicator_len(app.input_mode)
            + 1
            + label.len() as u16
            + 3
            + app.name_input.cursor() as u16;
        if cursor_x < area.x + area.width {
            frame.set_cursor_position((cursor_x, area.y));
        }
        return;
    }

    let status_msg = if let Some(msg) = &app.status_message {
        Span::styled(format!(" {} ", msg), Style::default().fg(Color::White))
    } else {
        Span::raw("")
    };

    let hint_style = Style::default().fg(Color::DarkGray);
    let key_style = Style::default().fg(Color::Yellow);

    let mut hints: Vec<Span> = Vec::new();

    // Log viewer hints (checked early, before other overlays)
    if app.show_log_viewer {
        if app.log_viewer_editing_filter {
            hints.push(Span::styled(" Enter", key_style));
            hints.push(Span::styled(":apply ", hint_style));
            hints.push(Span::styled("Esc", key_style));
            hints.push(Span::styled(":cancel", hint_style));
        } else if app.log_viewer_editing_search {
            hints.push(Span::styled(" Enter", key_style));
            hints.push(Span::styled(":search ", hint_style));
            hints.push(Span::styled("Esc", key_style));
            hints.push(Span::styled(":cancel", hint_style));
        } else if app.log_viewer_detail_focused {
            // Detail pane focused
            hints.push(Span::styled(" Up/Down", key_style));
            hints.push(Span::styled(":scroll ", hint_style));
            hints.extend(hint(
                kb,
                "cycle_pane_forward",
                "list",
                key_style,
                hint_style,
            ));
            hints.extend(hint(kb, "copy", "body", key_style, hint_style));
            hints.extend(hint(kb, "cancel", "back", key_style, hint_style));
        } else {
            // List pane focused
            hints.push(Span::styled(" Up/Down", key_style));
            hints.push(Span::styled(":nav ", hint_style));
            hints.extend(hint(kb, "enter", "detail", key_style, hint_style));
            if app.log_viewer_show_detail {
                hints.extend(hint(
                    kb,
                    "cycle_pane_forward",
                    "detail",
                    key_style,
                    hint_style,
                ));
            }
            hints.extend(hint(
                kb,
                "log_viewer.filter",
                "filter",
                key_style,
                hint_style,
            ));
            hints.extend(hint(kb, "search", "search", key_style, hint_style));
            hints.extend(hint(
                kb,
                "log_viewer.next_match",
                "next",
                key_style,
                hint_style,
            ));
            hints.extend(hint(
                kb,
                "log_viewer.prev_match",
                "prev",
                key_style,
                hint_style,
            ));
            hints.extend(hint(
                kb,
                "log_viewer.clear_filter",
                "clear filter",
                key_style,
                hint_style,
            ));
            hints.extend(hint(
                kb,
                "log_viewer.clear_search",
                "clear search",
                key_style,
                hint_style,
            ));
            hints.extend(hint(kb, "rename", "re-send", key_style, hint_style));
            hints.extend(hint(kb, "copy", "body", key_style, hint_style));
            hints.extend(hint(
                kb,
                "log_viewer.export",
                "export",
                key_style,
                hint_style,
            ));
            hints.extend(hint(kb, "cancel", "close", key_style, hint_style));
        }

        let mut line_spans = vec![mode_indicator, Span::raw(" "), status_msg];
        line_spans.extend(hints);
        let line = Line::from(line_spans);

        let status = Paragraph::new(line).style(Style::default().bg(Color::Black));
        frame.render_widget(status, area);
        return;
    }

    if app.show_export_picker {
        hints.push(Span::styled(" j/k", key_style));
        hints.push(Span::styled(":select ", hint_style));
        if app.export_collection_available {
            hints.push(Span::styled("Tab", key_style));
            hints.push(Span::styled(":scope ", hint_style));
        }
        hints.push(Span::styled("Enter", key_style));
        hints.push(Span::styled(":export ", hint_style));
        hints.extend(hint(kb, "cancel", "cancel", key_style, hint_style));

        let mut line_spans = vec![mode_indicator, Span::raw(" "), status_msg];
        line_spans.extend(hints);
        let line = Line::from(line_spans);
        let status = Paragraph::new(line).style(Style::default().bg(Color::Black));
        frame.render_widget(status, area);
        return;
    }

    if app.input_mode == InputMode::Editing {
        // Editing mode — show editing-specific hints
        hints.push(Span::styled(" Esc", key_style));
        hints.push(Span::styled(":done ", hint_style));
        hints.push(Span::styled("F5", key_style));
        hints.push(Span::styled(":send ", hint_style));
        hints.push(Span::styled("Tab", key_style));
        hints.push(Span::styled(":pane ", hint_style));
    } else {
        // Normal mode — context-sensitive hints

        // Always available
        hints.push(Span::styled(" Tab", key_style));
        hints.push(Span::styled(":pane ", hint_style));

        // Pane-specific hints
        match app.active_pane {
            Pane::Collections => {
                hints.push(Span::styled("Up/Down", key_style));
                hints.push(Span::styled(":navigate ", hint_style));
                hints.extend(hint(kb, "enter", "load", key_style, hint_style));
                hints.extend(hint(
                    kb,
                    "new_request",
                    "new collection",
                    key_style,
                    hint_style,
                ));
                hints.extend(hint(kb, "rename", "rename", key_style, hint_style));
                hints.extend(hint(kb, "delete_item", "delete", key_style, hint_style));
            }
            Pane::Request => {
                hints.push(Span::styled("Left/Right", key_style));
                hints.push(Span::styled(":tab ", hint_style));
                hints.extend(hint(kb, "enter", "edit", key_style, hint_style));

                // Tab-specific hints
                match app.request_tab() {
                    RequestTab::Headers => {
                        hints.push(Span::styled("Up/Down", key_style));
                        hints.push(Span::styled(":navigate ", hint_style));
                        hints.extend(hint(kb, "enter", "edit", key_style, hint_style));
                        hints.extend(hint(kb, "add_item", "add", key_style, hint_style));
                        hints.extend(hint(kb, "delete_item", "delete", key_style, hint_style));
                        hints.extend(hint(kb, "toggle_enabled", "toggle", key_style, hint_style));
                    }
                    RequestTab::Body => {}
                    RequestTab::Auth => {
                        if app.auth_inputs.is_empty() {
                            hints.extend(hint(
                                kb,
                                "enter",
                                "select auth type",
                                key_style,
                                hint_style,
                            ));
                        } else {
                            hints.push(Span::styled("Up/Down", key_style));
                            hints.push(Span::styled(":navigate ", hint_style));
                            hints.extend(hint(kb, "enter", "edit", key_style, hint_style));
                            hints.extend(hint(
                                kb,
                                "change_auth_type",
                                "change type",
                                key_style,
                                hint_style,
                            ));
                            if matches!(
                                app.current_request().and_then(|r| r.auth.as_ref()),
                                Some(lazycurl_core::types::Auth::OAuth2 { .. })
                            ) {
                                hints.push(Span::styled("F5", key_style));
                                hints.push(Span::styled(":get token ", hint_style));
                            }
                        }
                    }
                    RequestTab::Params => {
                        hints.push(Span::styled("Up/Down", key_style));
                        hints.push(Span::styled(":navigate ", hint_style));
                        hints.extend(hint(kb, "enter", "edit", key_style, hint_style));
                        hints.extend(hint(kb, "add_item", "add", key_style, hint_style));
                        hints.extend(hint(kb, "delete_item", "delete", key_style, hint_style));
                        hints.extend(hint(kb, "toggle_enabled", "toggle", key_style, hint_style));
                    }
                }

                hints.extend(hint(kb, "cycle_method", "method", key_style, hint_style));
                hints.extend(hint(kb, "rename", "rename", key_style, hint_style));
                hints.extend(hint(
                    kb,
                    "new_request",
                    "new request",
                    key_style,
                    hint_style,
                ));
            }
            Pane::Response => {
                hints.push(Span::styled("Left/Right", key_style));
                hints.push(Span::styled(":tab ", hint_style));
                hints.push(Span::styled("Up/Down", key_style));
                hints.push(Span::styled(":scroll ", hint_style));
                hints.extend(hint(kb, "copy", "copy body", key_style, hint_style));
            }
        }

        // Separator
        hints.push(Span::styled("| ", hint_style));

        // Global actions (always available)
        hints.extend(hint(
            kb,
            "open_project_picker",
            "projects",
            key_style,
            hint_style,
        ));
        hints.extend(hint(kb, "open_export", "export", key_style, hint_style));
        hints.extend(hint(kb, "send_request", "send", key_style, hint_style));
        hints.extend(hint(kb, "save_request", "save", key_style, hint_style));
        hints.extend(hint(kb, "open_variables", "vars", key_style, hint_style));
        hints.extend(hint(kb, "help", "help", key_style, hint_style));
        hints.extend(hint(kb, "quit", "quit", key_style, hint_style));
    }

    let mut line_spans = vec![mode_indicator, Span::raw(" "), status_msg];
    line_spans.extend(hints);
    let line = Line::from(line_spans);

    let status = Paragraph::new(line).style(Style::default().bg(Color::Black));
    frame.render_widget(status, area);
}

fn hint<'a>(
    kb: &HashMap<String, String>,
    action: &str,
    label: &str,
    key_style: Style,
    hint_style: Style,
) -> Vec<Span<'a>> {
    if let Some(key) = kb.get(action) {
        vec![
            Span::styled(format_key_display(key), key_style),
            Span::styled(format!(":{} ", label), hint_style),
        ]
    } else {
        vec![]
    }
}

/// Format a binding string for display (e.g. "ctrl+s" → "Ctrl+S")
fn format_key_display(binding: &str) -> String {
    binding
        .split('+')
        .map(|part| match part.to_lowercase().as_str() {
            "ctrl" => "Ctrl".to_string(),
            "shift" => "Shift".to_string(),
            "alt" => "Alt".to_string(),
            "enter" => "Enter".to_string(),
            "escape" | "esc" => "Esc".to_string(),
            "backtab" => "Tab".to_string(), // shift+backtab displays as Shift+Tab
            "tab" => "Tab".to_string(),
            s if s.starts_with('f') && s[1..].parse::<u8>().is_ok() => s.to_uppercase(),
            s if s.len() == 1 => s.to_string(), // preserve case for single chars
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn mode_indicator_len(mode: InputMode) -> u16 {
    match mode {
        InputMode::Normal => 8,  // " NORMAL "
        InputMode::Editing => 9, // " EDITING "
    }
}
