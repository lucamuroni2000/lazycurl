use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Computed pane areas based on visibility flags.
pub struct PaneLayout {
    pub title_bar: Rect,
    pub collections: Option<Rect>,
    pub request: Option<Rect>,
    pub response: Option<Rect>,
    pub status_bar: Rect,
}

/// Compute the layout based on which panes are visible.
pub fn compute_layout(area: Rect, visible: [bool; 3]) -> PaneLayout {
    // Top: title bar (1 line), Bottom: status bar (1 line), Middle: panes
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

    let title_bar = vertical[0];
    let main_area = vertical[1];
    let status_bar = vertical[2];

    let [col_vis, req_vis, res_vis] = visible;

    // If only one pane visible, it gets the full area
    let visible_count = visible.iter().filter(|&&v| v).count();
    if visible_count == 0 {
        return PaneLayout {
            title_bar,
            collections: None,
            request: None,
            response: None,
            status_bar,
        };
    }

    // Split: left (collections) | right (request + response)
    let (left_area, right_area) = if col_vis && (req_vis || res_vis) {
        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(main_area);
        (Some(h[0]), Some(h[1]))
    } else if col_vis {
        (Some(main_area), None)
    } else {
        (None, Some(main_area))
    };

    // Split right area into request (top) and response (bottom)
    let (req_area, res_area) = match right_area {
        Some(right) if req_vis && res_vis => {
            let v = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(right);
            (Some(v[0]), Some(v[1]))
        }
        Some(right) if req_vis => (Some(right), None),
        Some(right) if res_vis => (None, Some(right)),
        _ => (None, None),
    };

    PaneLayout {
        title_bar,
        collections: if col_vis { left_area } else { None },
        request: req_area,
        response: res_area,
        status_bar,
    }
}
