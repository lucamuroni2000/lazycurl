use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();

    let active_idx = app.active_project_idx;
    let tab_scroll = app.project_tab_scroll;

    // Calculate how many tabs fit
    let env_indicator_width: usize = 15;
    let available = area.width.saturating_sub(env_indicator_width as u16) as usize;

    let tabs: Vec<(usize, &str)> = app
        .open_projects
        .iter()
        .enumerate()
        .map(|(i, ws)| (i, ws.data.project.name.as_str()))
        .collect();

    let mut used_width = 0usize;
    let visible_start = tab_scroll;
    let mut visible_end = tabs.len();
    let show_left_arrow = visible_start > 0;

    if show_left_arrow {
        spans.push(Span::styled(" < ", Style::default().fg(Color::DarkGray)));
        used_width += 3;
    }

    for (i, (_idx, name)) in tabs.iter().enumerate().skip(visible_start) {
        let tab_width = name.len() + 6;
        if used_width + tab_width > available {
            visible_end = i;
            break;
        }
        let is_active = Some(*_idx) == active_idx;
        if is_active {
            spans.push(Span::styled(
                format!(" [ {} ] ", name),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" [ {} ] ", name),
                Style::default().fg(Color::Gray),
            ));
        }
        used_width += tab_width;
    }

    let show_right_arrow = visible_end < tabs.len();
    if show_right_arrow {
        spans.push(Span::styled(" > ", Style::default().fg(Color::DarkGray)));
    }

    // [+] button
    spans.push(Span::styled(" [+] ", Style::default().fg(Color::DarkGray)));

    // Right-align: environment indicator
    let env_name = app
        .active_workspace()
        .and_then(|ws| {
            ws.data
                .active_environment
                .and_then(|i| ws.data.environments.get(i))
        })
        .map(|e| e.name.as_str())
        .unwrap_or("None");

    let total_used: usize = spans.iter().map(|s| s.content.len()).sum();
    let env_text = format!("env: {}", env_name);
    let remaining = (area.width as usize)
        .saturating_sub(total_used)
        .saturating_sub(env_text.len());
    if remaining > 0 {
        spans.push(Span::raw(" ".repeat(remaining)));
    }
    spans.push(Span::styled(env_text, Style::default().fg(Color::Yellow)));

    let line = Line::from(spans);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Black)),
        area,
    );
}
