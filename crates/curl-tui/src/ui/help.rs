use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw(frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings — Press Esc to close ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        header("Navigation"),
        binding("Tab / Shift+Tab", "Cycle panes"),
        binding("Arrow Up/Down", "Navigate items"),
        binding("Arrow Left/Right", "Switch tabs"),
        binding("Enter", "Select / Edit field"),
        binding("Esc", "Cancel / Exit edit mode"),
        Line::raw(""),
        header("Actions"),
        binding("Ctrl+Enter / F5", "Send request"),
        binding("Ctrl+S", "Save request"),
        binding("Ctrl+N", "New request"),
        binding("Ctrl+E", "Switch environment"),
        binding("Ctrl+Y", "Copy as curl"),
        Line::raw(""),
        header("View"),
        binding("Ctrl+1/2/3", "Toggle panes"),
        binding("F8", "Reveal secrets"),
        binding("?", "Toggle this help"),
        binding("Ctrl+Q", "Quit"),
        Line::raw(""),
        header("Editing"),
        binding("Type", "Insert characters"),
        binding("Backspace/Delete", "Remove characters"),
        binding("Home/End", "Jump to start/end"),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
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
        Span::styled(format!("  {:22}", key), Style::default().fg(Color::Yellow)),
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
