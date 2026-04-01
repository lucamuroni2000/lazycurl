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
        binding("Up / Down", "Navigate items in current pane"),
        binding("Left / Right", "Switch tabs (Headers/Body/Auth/Params)"),
        binding("1 / 2 / 3", "Focus Collections / Request / Response pane"),
        binding("Enter", "Select item or start editing focused field"),
        binding("Esc", "Stop editing / Close overlay"),
        Line::raw(""),
        header("Request Actions"),
        binding("Ctrl+Enter / F5", "Send the current request"),
        binding("Ctrl+S", "Save request to collection"),
        binding("Ctrl+N", "Create a new request or collection"),
        binding("Ctrl+E", "Cycle active environment"),
        binding("Ctrl+Shift+E", "Manage environments"),
        binding("x", "Export request/collection"),
        binding("y", "Copy response body to clipboard"),
        binding("Ctrl+L", "Open request log viewer"),
        Line::raw(""),
        header("Log Viewer (Ctrl+L)"),
        binding("Up / Down", "Navigate log entries"),
        binding("Enter", "Toggle detail pane for selected entry"),
        binding("Esc", "Close detail pane, or close log viewer"),
        binding("/", "Search log entries (highlights matches)"),
        binding("n / N", "Jump to next / previous search match"),
        binding("f", "Filter by method, status, or URL substring"),
        binding("c", "Clear filter"),
        binding("C  (Shift+c)", "Clear search"),
        binding("r", "Re-send: load request into editor"),
        binding("y", "Copy response body to clipboard"),
        binding("Y", "Copy log file path to clipboard"),
        binding("e", "Export current (filtered) view to JSONL file"),
        Line::raw(""),
        header("Item Management"),
        binding("a", "Add new header, param, or variable"),
        binding("d", "Delete selected item"),
        binding("r", "Rename selected item"),
        binding("m", "Open HTTP method picker (in Request pane)"),
        binding("s", "Toggle enabled/disabled on selected item"),
        binding("v", "Open the variables editor overlay"),
        Line::raw(""),
        header("Projects"),
        binding("Ctrl+Right / Left", "Next / previous project"),
        binding("Ctrl+O", "Open project picker"),
        Line::raw(""),
        header("General"),
        binding("F8", "Show or hide secret variable values"),
        binding("F1", "Toggle this help overlay"),
        binding("/", "Search"),
        binding("q", "Quit lazycurl"),
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
