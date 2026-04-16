use std::collections::HashMap;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::format_key_display;
use crate::app::App;

/// A single help entry: description text (for search filtering) + rendered line.
struct Entry {
    desc: String,
    line: Line<'static>,
}

/// A section of related entries with a header.
struct Section {
    header: Line<'static>,
    entries: Vec<Entry>,
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 80, frame.area());
    frame.render_widget(Clear, area);

    // Build title with search state (always visible, like log viewer)
    let search_display = if app.help_editing_search {
        format!("Search: {}_", app.help_search_input.content())
    } else if app.help_search.is_empty() {
        "Search: (none)".to_string()
    } else {
        format!("Search: {}", app.help_search)
    };
    let title = format!(" Keybindings \u{2014} {} ", search_display);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let kb = &app.config.keybindings;
    let search = &app.help_search;

    // Build all sections and collect filtered entries
    let sections = build_sections(kb);
    let matcher = SkimMatcherV2::default();

    // Collect entries that match the search, tracking which flat line indices are "entry" lines
    let mut lines: Vec<Line> = Vec::new();
    let mut entry_indices: Vec<usize> = Vec::new(); // line indices that are selectable entries

    for section in &sections {
        if search.is_empty() {
            lines.push(section.header.clone());
            for entry in &section.entries {
                entry_indices.push(lines.len());
                lines.push(entry.line.clone());
            }
            lines.push(Line::raw(""));
        } else {
            // Fuzzy match entries, collect with scores for sorting
            let mut scored: Vec<(i64, &Entry)> = section
                .entries
                .iter()
                .filter_map(|e| matcher.fuzzy_match(&e.desc, search).map(|score| (score, e)))
                .collect();
            if !scored.is_empty() {
                scored.sort_by_key(|b| std::cmp::Reverse(b.0)); // highest score first
                lines.push(section.header.clone());
                for (_score, entry) in scored {
                    entry_indices.push(lines.len());
                    lines.push(entry.line.clone());
                }
                lines.push(Line::raw(""));
            }
        }
    }

    let entry_count = entry_indices.len();

    if entry_count == 0 && !search.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No matching commands found.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Clamp cursor to valid range
    if entry_count > 0 {
        if app.help_cursor >= entry_count {
            app.help_cursor = entry_count - 1;
        }
    } else {
        app.help_cursor = 0;
    }

    // Highlight the selected entry line
    if entry_count > 0 {
        let line_idx = entry_indices[app.help_cursor];
        if let Some(line) = lines.get_mut(line_idx) {
            *line = Line::from(
                line.spans
                    .iter()
                    .map(|span| Span::styled(span.content.clone(), span.style.bg(Color::DarkGray)))
                    .collect::<Vec<_>>(),
            );
        }
    }

    // Auto-scroll to keep cursor visible
    let visible_height = inner.height as usize;
    let cursor_line = if entry_count > 0 {
        entry_indices[app.help_cursor]
    } else {
        0
    };
    let scroll = if visible_height == 0 {
        0
    } else if cursor_line >= visible_height {
        // Keep cursor near the bottom of the viewport
        cursor_line - visible_height + 1
    } else {
        0
    };

    let paragraph = Paragraph::new(lines).scroll((scroll as u16, 0));
    frame.render_widget(paragraph, inner);
}

fn build_sections(kb: &HashMap<String, String>) -> Vec<Section> {
    let mut sections = Vec::new();

    // Navigation
    {
        let mut entries = Vec::new();
        push_pair(
            &mut entries,
            kb,
            "cycle_pane_forward",
            "cycle_pane_backward",
            "Cycle between panes",
        );
        push_pair(
            &mut entries,
            kb,
            "move_up",
            "move_down",
            "Navigate items in current pane",
        );
        push_pair(
            &mut entries,
            kb,
            "next_tab",
            "prev_tab",
            "Switch tabs (Headers/Body/Auth/Params)",
        );
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
                let desc = "Focus Collections / Request / Response pane";
                entries.push(Entry {
                    desc: desc.to_string(),
                    line: Line::from(vec![
                        Span::styled(
                            format!("  {:26}", display),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(desc.to_string(), Style::default().fg(Color::White)),
                    ]),
                });
            }
        }
        push_single(
            &mut entries,
            kb,
            "enter",
            "Select item or start editing focused field",
        );
        push_single(&mut entries, kb, "cancel", "Stop editing / Close overlay");
        sections.push(Section {
            header: header("Navigation"),
            entries,
        });
    }

    // Request Actions
    {
        let mut entries = Vec::new();
        push_single(&mut entries, kb, "send_request", "Send the current request");
        push_single(
            &mut entries,
            kb,
            "save_request",
            "Save request to collection",
        );
        push_single(
            &mut entries,
            kb,
            "new_request",
            "Create a new request or collection",
        );
        push_single(&mut entries, kb, "switch_env", "Cycle active environment");
        push_single(&mut entries, kb, "manage_envs", "Manage environments");
        push_single(&mut entries, kb, "open_export", "Export request/collection");
        push_single(
            &mut entries,
            kb,
            "open_import",
            "Import (Curl/Postman/OpenAPI)",
        );
        push_single(&mut entries, kb, "copy", "Copy response body to clipboard");
        push_single(
            &mut entries,
            kb,
            "open_log_viewer",
            "Open request log viewer",
        );
        sections.push(Section {
            header: header("Request Actions"),
            entries,
        });
    }

    // Log Viewer
    {
        let mut entries = Vec::new();
        push_hardcoded(&mut entries, "Up / Down", "Navigate log entries");
        push_single(
            &mut entries,
            kb,
            "enter",
            "Toggle detail pane for selected entry",
        );
        push_single(
            &mut entries,
            kb,
            "cancel",
            "Close detail pane, or close log viewer",
        );
        push_single(
            &mut entries,
            kb,
            "search",
            "Search log entries (highlights matches)",
        );
        push_pair(
            &mut entries,
            kb,
            "log_viewer.next_match",
            "log_viewer.prev_match",
            "Jump to next / previous search match",
        );
        push_single(
            &mut entries,
            kb,
            "log_viewer.filter",
            "Filter by method, status, or URL substring",
        );
        push_single(&mut entries, kb, "log_viewer.clear_filter", "Clear filter");
        push_single(&mut entries, kb, "log_viewer.clear_search", "Clear search");
        push_single(
            &mut entries,
            kb,
            "rename",
            "Re-send: load request into editor",
        );
        push_single(&mut entries, kb, "copy", "Copy response body to clipboard");
        push_single(
            &mut entries,
            kb,
            "log_viewer.copy_path",
            "Copy log file path to clipboard",
        );
        push_single(
            &mut entries,
            kb,
            "log_viewer.export",
            "Export current (filtered) view to JSONL file",
        );
        sections.push(Section {
            header: header("Log Viewer"),
            entries,
        });
    }

    // Item Management
    {
        let mut entries = Vec::new();
        push_single(
            &mut entries,
            kb,
            "add_item",
            "Add new header, param, or variable",
        );
        push_single(&mut entries, kb, "delete_item", "Delete selected item");
        push_single(&mut entries, kb, "rename", "Rename selected item");
        push_single(
            &mut entries,
            kb,
            "duplicate_item",
            "Duplicate selected collection or request",
        );
        push_single(
            &mut entries,
            kb,
            "move_request",
            "Move request to another collection",
        );
        push_single(
            &mut entries,
            kb,
            "toggle_collapse",
            "Expand/collapse collection in sidebar",
        );
        push_single(
            &mut entries,
            kb,
            "cycle_method",
            "Open HTTP method picker (in Request pane)",
        );
        push_single(
            &mut entries,
            kb,
            "cycle_body_type",
            "Open body type picker (in Body tab)",
        );
        push_single(
            &mut entries,
            kb,
            "toggle_auto_headers",
            "Show/hide auto-generated headers (in Headers tab)",
        );
        push_single(
            &mut entries,
            kb,
            "toggle_enabled",
            "Toggle enabled/disabled on selected item",
        );
        push_single(
            &mut entries,
            kb,
            "open_variables",
            "Open the variables editor overlay",
        );
        sections.push(Section {
            header: header("Item Management"),
            entries,
        });
    }

    // Projects
    {
        let mut entries = Vec::new();
        push_pair(
            &mut entries,
            kb,
            "next_project",
            "prev_project",
            "Next / previous project",
        );
        push_single(
            &mut entries,
            kb,
            "open_project_picker",
            "Open project picker",
        );
        sections.push(Section {
            header: header("Projects"),
            entries,
        });
    }

    // General
    {
        let mut entries = Vec::new();
        push_single(
            &mut entries,
            kb,
            "open_config",
            "Open config file in editor",
        );
        push_single(
            &mut entries,
            kb,
            "reveal_secrets",
            "Show or hide secret variable values",
        );
        push_single(&mut entries, kb, "help", "Toggle this help overlay");
        push_single(
            &mut entries,
            kb,
            "search",
            "Search within this help overlay",
        );
        push_single(&mut entries, kb, "quit", "Quit lazycurl");
        sections.push(Section {
            header: header("General"),
            entries,
        });
    }

    // Text Editing
    {
        let mut entries = Vec::new();
        push_hardcoded(&mut entries, "Any character", "Insert at cursor position");
        push_hardcoded(
            &mut entries,
            "Backspace / Delete",
            "Remove character before / after cursor",
        );
        push_hardcoded(&mut entries, "Home / End", "Jump to start / end of field");
        push_hardcoded(&mut entries, "Left / Right", "Move cursor within field");
        sections.push(Section {
            header: header("Text Editing (when a field is focused)"),
            entries,
        });
    }

    sections
}

fn push_single(entries: &mut Vec<Entry>, kb: &HashMap<String, String>, action: &str, desc: &str) {
    if let Some(key) = kb.get(action) {
        entries.push(Entry {
            desc: desc.to_string(),
            line: Line::from(vec![
                Span::styled(
                    format!("  {:26}", format_key_display(key)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(desc.to_string(), Style::default().fg(Color::White)),
            ]),
        });
    }
}

fn push_pair(
    entries: &mut Vec<Entry>,
    kb: &HashMap<String, String>,
    action1: &str,
    action2: &str,
    desc: &str,
) {
    let key1 = kb.get(action1);
    let key2 = kb.get(action2);
    if let (Some(k1), Some(k2)) = (key1, key2) {
        let display = format!("{} / {}", format_key_display(k1), format_key_display(k2));
        entries.push(Entry {
            desc: desc.to_string(),
            line: Line::from(vec![
                Span::styled(
                    format!("  {:26}", display),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(desc.to_string(), Style::default().fg(Color::White)),
            ]),
        });
    }
}

fn push_hardcoded(entries: &mut Vec<Entry>, key: &str, desc: &str) {
    entries.push(Entry {
        desc: desc.to_string(),
        line: Line::from(vec![
            Span::styled(format!("  {:26}", key), Style::default().fg(Color::Yellow)),
            Span::styled(desc.to_string(), Style::default().fg(Color::White)),
        ]),
    });
}

fn header(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {}", text),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
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
