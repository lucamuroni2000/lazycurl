use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw(frame: &mut Frame) {
    let area = centered_rect(70, 80, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings — Press Esc to close ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        header("Navigation"),
        binding("Tab / Shift+Tab", "Cycle between panes"),
        binding("Up / Down  (j/k)", "Navigate items in current pane"),
        binding("Left / Right", "Switch tabs (Headers/Body/Auth/Params)"),
        binding("Enter", "Select item or start editing focused field"),
        binding("Esc", "Stop editing / Close overlay"),
        Line::raw(""),
        header("Request Actions"),
        binding("Ctrl+Enter / F5", "Send the current request"),
        binding("Ctrl+S", "Save request to collection"),
        binding("Ctrl+N", "Create a new request"),
        binding("Ctrl+E", "Cycle active environment"),
        binding("Ctrl+Y", "Copy request as curl command"),
        Line::raw(""),
        header("Item Management"),
        binding("a  (Add)", "Add new header, param, or variable"),
        binding("d  (Delete)", "Delete selected header, param, or variable"),
        binding(
            "r  (Rename)",
            "Rename selected collection, request, or variable key",
        ),
        binding("s  (Secret)", "Toggle secret flag on selected variable"),
        binding("v  (Variables)", "Open the variables editor overlay"),
        Line::raw(""),
        header("View & Display"),
        binding(
            "Ctrl+1 / 2 / 3",
            "Toggle Collections / Request / Response pane",
        ),
        binding("F8  (Reveal)", "Show or hide secret variable values"),
        binding("?   (Help)", "Toggle this help overlay"),
        binding("Ctrl+Q", "Quit curl-tui"),
        Line::raw(""),
        header("Text Editing (when a field is focused)"),
        binding("Any character", "Insert at cursor position"),
        binding(
            "Backspace / Delete",
            "Remove character before / after cursor",
        ),
        binding("Home / End", "Jump to start / end of field"),
        binding("Left / Right", "Move cursor within field"),
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
