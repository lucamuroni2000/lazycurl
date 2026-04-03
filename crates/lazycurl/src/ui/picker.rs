use std::collections::HashMap;

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::key_for;
use crate::app::App;

pub fn draw_collection_picker(frame: &mut Frame, app: &App, kb: &HashMap<String, String>) {
    let filtered = app.picker_collections();
    let height = (filtered.len() + 4).min(20) as u16;
    let area = centered_rect(50, height, frame.area());
    frame.render_widget(Clear, area);

    let title_action = match &app.picker_context {
        crate::app::PickerContext::SaveRequest => "Save to collection",
        crate::app::PickerContext::DuplicateRequest { .. } => "Duplicate to collection",
        crate::app::PickerContext::MoveRequest { .. } => "Move to collection",
    };
    let title = format!(
        " {} \u{2014} {}: select  {}: cancel  {}: new ",
        title_action,
        key_for(kb, "enter"),
        key_for(kb, "cancel"),
        key_for(kb, "new_request"),
    );
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();

    for (i, (_original_idx, collection)) in filtered.iter().enumerate() {
        let is_selected = i == app.picker_cursor;
        let req_count = collection.requests.len();

        let marker = if is_selected { ">" } else { " " };
        let name_style = if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", marker), Style::default().fg(Color::Cyan)),
            Span::styled(collection.name.clone(), name_style),
            Span::styled(
                format!("  ({} requests)", req_count),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn centered_rect(percent_x: u16, lines: u16, area: Rect) -> Rect {
    let height = lines.min(area.height);
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}
