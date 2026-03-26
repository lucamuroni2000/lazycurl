pub mod collections;
pub mod environment_manager;
pub mod export_picker;
pub mod help;
pub mod layout;
pub mod log_viewer;
pub mod picker;
pub mod project_picker;
pub mod project_tabs;
pub mod request;
pub mod response;
pub mod statusbar;
pub mod variables;

use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let pane_layout = layout::compute_layout(frame.area(), app.pane_visible);

    // Title bar — project tabs + env
    project_tabs::draw(frame, app, pane_layout.title_bar);

    // Panes
    if let Some(area) = pane_layout.collections {
        collections::draw(frame, app, area);
    }
    if let Some(area) = pane_layout.request {
        request::draw(frame, app, area);
    }
    if let Some(area) = pane_layout.response {
        response::draw(frame, app, area);
    }

    // Status bar
    statusbar::draw(frame, app, pane_layout.status_bar);

    // Method picker (rendered relative to request pane)
    if app.show_method_picker {
        if let Some(area) = pane_layout.request {
            request::draw_method_picker(frame, app, area);
        }
    }

    // Overlays (on top of everything)
    if app.show_export_picker {
        export_picker::draw(frame, app);
    }
    if app.show_collection_picker {
        picker::draw_collection_picker(frame, app);
    }
    if app.show_variables {
        variables::draw(frame, app);
    }
    if app.show_env_manager {
        environment_manager::draw(frame, app);
    }
    if app.show_help {
        help::draw(frame);
    }
    if app.show_project_picker {
        project_picker::draw(frame, app);
    }
    if app.show_log_viewer {
        log_viewer::draw(frame, app);
    }
    if app.show_first_launch {
        project_picker::draw_first_launch(frame, app);
    }
}
