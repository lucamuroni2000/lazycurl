use std::collections::HashMap;

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, keybindings: &HashMap<String, String>) {
    let area = centered_rect(70, 80, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings — Press Esc to close ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let kb = keybindings;
    let mut lines: Vec<Line> = Vec::new();

    // Navigation
    lines.push(header("Navigation"));
    lines.extend(binding_pair_line(
        kb,
        "cycle_pane_forward",
        "cycle_pane_backward",
        "Cycle between panes",
    ));
    lines.extend(binding_pair_line(
        kb,
        "move_up",
        "move_down",
        "Navigate items in current pane",
    ));
    lines.extend(binding_pair_line(
        kb,
        "next_tab",
        "prev_tab",
        "Switch tabs (Headers/Body/Auth/Params)",
    ));
    // Focus pane triple combo
    {
        let k1 = kb.get("focus_collections");
        let k2 = kb.get("focus_request");
        let k3 = kb.get("focus_response");
        if let (Some(k1), Some(k2), Some(k3)) = (k1, k2, k3) {
            let display = format!(
                "{} / {} / {}",
                format_key_display(k1),
                format_key_display(k2),
                format_key_display(k3)
            );
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:26}", display),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    "Focus Collections / Request / Response pane".to_string(),
                    Style::default().fg(Color::White),
                ),
            ]));
        }
    }
    lines.extend(binding_line(
        kb,
        "enter",
        "Select item or start editing focused field",
    ));
    lines.extend(binding_line(kb, "cancel", "Stop editing / Close overlay"));
    lines.push(Line::raw(""));

    // Request Actions
    lines.push(header("Request Actions"));
    lines.extend(binding_line(kb, "send_request", "Send the current request"));
    lines.extend(binding_line(
        kb,
        "save_request",
        "Save request to collection",
    ));
    lines.extend(binding_line(
        kb,
        "new_request",
        "Create a new request or collection",
    ));
    lines.extend(binding_line(kb, "switch_env", "Cycle active environment"));
    lines.extend(binding_line(kb, "manage_envs", "Manage environments"));
    lines.extend(binding_line(kb, "open_export", "Export request/collection"));
    lines.extend(binding_line(kb, "copy", "Copy response body to clipboard"));
    lines.extend(binding_line(
        kb,
        "open_log_viewer",
        "Open request log viewer",
    ));
    lines.push(Line::raw(""));

    // Log Viewer
    lines.push(header("Log Viewer"));
    lines.push(binding("Up / Down", "Navigate log entries"));
    lines.extend(binding_line(
        kb,
        "enter",
        "Toggle detail pane for selected entry",
    ));
    lines.extend(binding_line(
        kb,
        "cancel",
        "Close detail pane, or close log viewer",
    ));
    lines.extend(binding_line(
        kb,
        "search",
        "Search log entries (highlights matches)",
    ));
    lines.extend(binding_pair_line(
        kb,
        "log_viewer.next_match",
        "log_viewer.prev_match",
        "Jump to next / previous search match",
    ));
    lines.extend(binding_line(
        kb,
        "log_viewer.filter",
        "Filter by method, status, or URL substring",
    ));
    lines.extend(binding_line(kb, "log_viewer.clear_filter", "Clear filter"));
    lines.extend(binding_line(kb, "log_viewer.clear_search", "Clear search"));
    lines.extend(binding_line(
        kb,
        "rename",
        "Re-send: load request into editor",
    ));
    lines.extend(binding_line(kb, "copy", "Copy response body to clipboard"));
    lines.extend(binding_line(
        kb,
        "log_viewer.copy_path",
        "Copy log file path to clipboard",
    ));
    lines.extend(binding_line(
        kb,
        "log_viewer.export",
        "Export current (filtered) view to JSONL file",
    ));
    lines.push(Line::raw(""));

    // Item Management
    lines.push(header("Item Management"));
    lines.extend(binding_line(
        kb,
        "add_item",
        "Add new header, param, or variable",
    ));
    lines.extend(binding_line(kb, "delete_item", "Delete selected item"));
    lines.extend(binding_line(kb, "rename", "Rename selected item"));
    lines.extend(binding_line(
        kb,
        "cycle_method",
        "Open HTTP method picker (in Request pane)",
    ));
    lines.extend(binding_line(
        kb,
        "toggle_enabled",
        "Toggle enabled/disabled on selected item",
    ));
    lines.extend(binding_line(
        kb,
        "open_variables",
        "Open the variables editor overlay",
    ));
    lines.push(Line::raw(""));

    // Projects
    lines.push(header("Projects"));
    lines.extend(binding_pair_line(
        kb,
        "next_project",
        "prev_project",
        "Next / previous project",
    ));
    lines.extend(binding_line(
        kb,
        "open_project_picker",
        "Open project picker",
    ));
    lines.push(Line::raw(""));

    // General
    lines.push(header("General"));
    lines.extend(binding_line(
        kb,
        "reveal_secrets",
        "Show or hide secret variable values",
    ));
    lines.extend(binding_line(kb, "help", "Toggle this help overlay"));
    lines.extend(binding_line(kb, "search", "Search"));
    lines.extend(binding_line(kb, "quit", "Quit lazycurl"));
    lines.push(Line::raw(""));

    // Text Editing (hardcoded — these keys never change)
    lines.push(header("Text Editing (when a field is focused)"));
    lines.push(binding("Any character", "Insert at cursor position"));
    lines.push(binding(
        "Backspace / Delete",
        "Remove character before / after cursor",
    ));
    lines.push(binding("Home / End", "Jump to start / end of field"));
    lines.push(binding("Left / Right", "Move cursor within field"));

    frame.render_widget(Paragraph::new(lines), inner);
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
            "backtab" => "Tab".to_string(),
            "tab" => "Tab".to_string(),
            s if s.starts_with('f') && s[1..].parse::<u8>().is_ok() => s.to_uppercase(),
            s if s.len() == 1 && s.chars().next().unwrap().is_ascii_uppercase() => {
                format!("Shift+{}", s)
            }
            s if s.len() == 1 => s.to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn binding_line(kb: &HashMap<String, String>, action: &str, desc: &str) -> Option<Line<'static>> {
    kb.get(action).map(|key| {
        Line::from(vec![
            Span::styled(
                format!("  {:26}", format_key_display(key)),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(desc.to_string(), Style::default().fg(Color::White)),
        ])
    })
}

fn binding_pair_line(
    kb: &HashMap<String, String>,
    action1: &str,
    action2: &str,
    desc: &str,
) -> Option<Line<'static>> {
    let key1 = kb.get(action1)?;
    let key2 = kb.get(action2)?;
    let display = format!(
        "{} / {}",
        format_key_display(key1),
        format_key_display(key2)
    );
    Some(Line::from(vec![
        Span::styled(
            format!("  {:26}", display),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(desc.to_string(), Style::default().fg(Color::White)),
    ]))
}

fn header(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {}", text),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

fn binding(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:26}", key), Style::default().fg(Color::Yellow)),
        Span::styled(desc.to_string(), Style::default().fg(Color::White)),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}
