use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs};
use ratatui::Frame;

use crate::app::{App, InputMode, VarEditTarget, VarTier};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 80, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Variables — Esc: close  Tab: switch tier  a: add  d: delete  s: secret ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 {
        return;
    }

    // Split: tier tabs (1) | content (rest)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    // Tier tabs
    let tier_names = vec!["Global", "Environment", "Collection"];
    let selected_tier = match app.var_tier {
        VarTier::Global => 0,
        VarTier::Environment => 1,
        VarTier::Collection => 2,
    };
    let tabs = Tabs::new(tier_names)
        .select(selected_tier)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|");
    frame.render_widget(tabs, chunks[0]);

    // Tier info line + variable list
    let tier_info = match app.var_tier {
        VarTier::Global => " Global variables (apply to all requests)".to_string(),
        VarTier::Environment => {
            let (name, idx_info) = match app.var_environment_idx() {
                Some(i) => {
                    let name = app
                        .environments()
                        .get(i)
                        .map(|e| e.name.as_str())
                        .unwrap_or("?");
                    let total = app.environments().len();
                    (name.to_string(), format!("{}/{}", i + 1, total))
                }
                None => ("None selected".to_string(), "0/0".to_string()),
            };
            format!(
                " Environment: {} [{}]  (Ctrl+E: switch  Ctrl+Shift+E: manage)",
                name, idx_info
            )
        }
        VarTier::Collection => {
            let (name, idx_info) = match app.var_collection_idx() {
                Some(i) => {
                    let name = app
                        .collections()
                        .get(i)
                        .map(|c| c.name.as_str())
                        .unwrap_or("?");
                    let total = app.collections().len();
                    (name.to_string(), format!("{}/{}", i + 1, total))
                }
                None => ("None selected".to_string(), "0/0".to_string()),
            };
            format!(" Collection: {} [{}]  ([ ] switch)", name, idx_info)
        }
    };

    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(chunks[1]);

    frame.render_widget(
        Paragraph::new(tier_info).style(Style::default().fg(Color::DarkGray)),
        content_chunks[0],
    );

    let keys = app.var_keys();

    if keys.is_empty() {
        let no_env =
            matches!(app.var_tier, VarTier::Environment) && app.var_environment_idx().is_none();
        let no_col =
            matches!(app.var_tier, VarTier::Collection) && app.var_collection_idx().is_none();

        let msg = if no_env {
            " No environment selected. Press Ctrl+Shift+E to manage environments."
        } else if no_col {
            " No collection selected. Save a request first (Ctrl+S)."
        } else {
            " No variables. Press 'a' to add one."
        };

        frame.render_widget(
            Paragraph::new(msg).style(Style::default().fg(Color::DarkGray)),
            content_chunks[1],
        );
        return;
    }

    // Render variable list
    let mut lines = Vec::new();

    // Header
    lines.push(Line::from(vec![
        Span::styled(format!(" {:3} ", ""), Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:<25}", "Key"),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            format!("{:<35}", "Value"),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            "Secret",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::UNDERLINED),
        ),
    ]));

    for (i, key) in keys.iter().enumerate() {
        let is_selected = i == app.var_cursor;
        let var = app.var_get(key);
        let (value, is_secret) = match var {
            Some(v) => (v.value.clone(), v.secret),
            None => continue,
        };

        let display_value = if is_secret && !app.secrets_revealed {
            "••••••".to_string()
        } else {
            value
        };

        let is_editing_key = is_selected
            && app.var_editing == Some(VarEditTarget::Key)
            && app.input_mode == InputMode::Editing;
        let is_editing_value = is_selected
            && app.var_editing == Some(VarEditTarget::Value)
            && app.input_mode == InputMode::Editing;

        let cursor_marker = if is_selected { ">" } else { " " };

        let key_text = if is_editing_key {
            app.var_key_input.content().to_string()
        } else {
            key.clone()
        };

        let value_text = if is_editing_value {
            app.var_value_input.content().to_string()
        } else {
            display_value
        };

        let key_style = if is_editing_key {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let value_style = if is_editing_value {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else if is_secret && !app.secrets_revealed {
            Style::default().fg(Color::Red)
        } else if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let secret_indicator = if is_secret {
            Span::styled(" [secret]", Style::default().fg(Color::Red))
        } else {
            Span::styled(" [plain]", Style::default().fg(Color::DarkGray))
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", cursor_marker),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(format!("{:<25}", key_text), key_style),
            Span::styled(format!("{:<35}", value_text), value_style),
            secret_indicator,
        ]));

        // Show cursor position when editing
        if is_editing_key {
            let cursor_x = content_chunks[1].x + 3 + app.var_key_input.cursor() as u16;
            let cursor_y = content_chunks[1].y + (i + 1) as u16; // +1 for header
            if cursor_y < content_chunks[1].y + content_chunks[1].height
                && cursor_x < content_chunks[1].x + content_chunks[1].width
            {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        } else if is_editing_value {
            let cursor_x = content_chunks[1].x + 3 + 25 + app.var_value_input.cursor() as u16;
            let cursor_y = content_chunks[1].y + (i + 1) as u16;
            if cursor_y < content_chunks[1].y + content_chunks[1].height
                && cursor_x < content_chunks[1].x + content_chunks[1].width
            {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), content_chunks[1]);
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
