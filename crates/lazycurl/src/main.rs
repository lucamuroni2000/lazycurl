mod app;
mod events;
mod input;
pub mod text_input;
mod ui;

use std::collections::HashMap;
use std::io;
use std::time::Duration;

use crossterm::{
    event::{Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{Action, App, EditField};
use lazycurl_core::command::CurlCommandBuilder;
use lazycurl_core::config::{config_dir, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize config directory
    let config_root = config_dir();
    lazycurl_core::init::initialize(&config_root)?;

    // Load config
    let config_path = config_root.join("config.json");
    let config = AppConfig::load_from(&config_path)?;

    // Clean up expired log files
    let logs_path = lazycurl_core::logging::logs_dir();
    let _ = lazycurl_core::logging::cleanup_expired_logs(&logs_path, config.log_retention_days);

    // Parse --debug CLI flag
    let debug_enabled = config.debug_logging || std::env::args().any(|a| a == "--debug");

    // Initialize debug logger if enabled
    if debug_enabled {
        let logs_path = lazycurl_core::logging::logs_dir();
        std::fs::create_dir_all(&logs_path).ok();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let log_file_path = logs_path.join(format!("debug-{}.log", today));
        if let Ok(log_file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
        {
            let _ = simplelog::WriteLogger::init(
                simplelog::LevelFilter::Debug,
                simplelog::ConfigBuilder::new()
                    .set_time_format_rfc3339()
                    .build(),
                log_file,
            );
        }
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(config);

    // Load existing data into default project workspace
    // (For now, create a default project if none exists)
    {
        let projects_dir = config_root.join("projects");
        let projects = lazycurl_core::project::list_projects(&projects_dir).unwrap_or_default();
        if projects.is_empty() {
            // Create a default project
            let default_project = lazycurl_core::types::Project {
                id: uuid::Uuid::new_v4(),
                name: "Default".to_string(),
                active_environment: None,
            };
            if let Ok(dir) = lazycurl_core::project::create_project(&projects_dir, &default_project)
            {
                let slug = dir.file_name().unwrap().to_string_lossy().to_string();
                let mut ws = app::ProjectWorkspace::new(default_project, slug.clone());
                ws.data.collections = lazycurl_core::collection::list_collections(
                    &projects_dir.join(&slug).join("collections"),
                )
                .unwrap_or_default();
                ws.data.environments = lazycurl_core::environment::list_environments(
                    &projects_dir.join(&slug).join("environments"),
                )
                .unwrap_or_default();
                if let Some(env_name) = app.config.active_environment.clone() {
                    ws.data.active_environment =
                        ws.data.environments.iter().position(|e| e.name == env_name);
                }
                app.open_projects.push(ws);
                app.active_project_idx = Some(0);
            }
        } else {
            // Open the first project
            let (project, path) = projects.into_iter().next().unwrap();
            let slug = path.file_name().unwrap().to_string_lossy().to_string();
            let mut ws = app::ProjectWorkspace::new(project, slug.clone());
            ws.data.collections = lazycurl_core::collection::list_collections(
                &projects_dir.join(&slug).join("collections"),
            )
            .unwrap_or_default();
            ws.data.environments = lazycurl_core::environment::list_environments(
                &projects_dir.join(&slug).join("environments"),
            )
            .unwrap_or_default();
            if let Some(env_name) = app.config.active_environment.clone() {
                ws.data.active_environment =
                    ws.data.environments.iter().position(|e| e.name == env_name);
            }
            app.open_projects.push(ws);
            app.active_project_idx = Some(0);
        }
    }

    // Build keymap from config
    let keymap = input::build_keymap(&app.config.keybindings);

    // Load current request fields into text inputs
    app.load_request_into_inputs();

    // Main loop
    let result = run_loop(&mut terminal, &mut app, &keymap).await;

    // Save session state
    app.config.open_projects = app
        .open_projects
        .iter()
        .map(|ws| ws.data.slug.clone())
        .collect();
    app.config.active_project = app
        .active_project_idx
        .and_then(|i| app.open_projects.get(i))
        .map(|ws| ws.data.slug.clone());
    let config_path = config_dir().join("config.json");
    let _ = app.config.save_to(&config_path);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    keymap: &HashMap<(KeyModifiers, KeyCode), Action>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Auto-dismiss status bar messages after 10 seconds
        app.expire_status_message(Duration::from_secs(10));
        // Track timestamp for newly set messages
        if app.status_message.is_some() && app.status_message_at.is_none() {
            app.status_message_at = Some(std::time::Instant::now());
        }

        terminal.draw(|frame| {
            ui::draw(frame, app);
        })?;

        if let Some(Event::Key(key)) = events::poll_event(Duration::from_millis(50))? {
            let action = if app.show_env_manager && app.env_manager_renaming.is_some() {
                // Env manager rename uses editing resolver so letters aren't
                // swallowed by navigation bindings (e.g. 'd' → DeleteItem).
                input::resolve_editing(key)
            } else if app.show_project_picker
                && (app.project_picker_renaming || app.project_picker_confirm_delete.is_some())
            {
                // Project picker rename/delete confirmation uses editing resolver.
                input::resolve_editing(key)
            } else {
                match app.input_mode {
                    app::InputMode::Normal => {
                        // First try keymap, then navigation fallback
                        let mapped = input::resolve_action(key, keymap);
                        if matches!(mapped, Action::None) {
                            input::resolve_navigation(key)
                        } else {
                            mapped
                        }
                    }
                    app::InputMode::Editing => input::resolve_editing(key),
                }
            };

            // Method picker intercepts when open
            if app.show_method_picker {
                match action {
                    Action::Cancel => {
                        app.show_method_picker = false;
                    }
                    Action::MoveUp => {
                        if app.method_picker_cursor > 0 {
                            app.method_picker_cursor -= 1;
                        }
                    }
                    Action::MoveDown => {
                        if app.method_picker_cursor + 1 < lazycurl_core::types::Method::ALL.len() {
                            app.method_picker_cursor += 1;
                        }
                    }
                    Action::Enter => {
                        let method = lazycurl_core::types::Method::ALL[app.method_picker_cursor];
                        app.select_method(method);
                    }
                    Action::Quit => app.should_quit = true,
                    _ => {}
                }
            // Collection picker intercepts when open
            } else if app.show_collection_picker {
                match action {
                    Action::Cancel => {
                        app.show_collection_picker = false;
                        app.status_message = None;
                    }
                    Action::MoveUp => {
                        if app.picker_cursor > 0 {
                            app.picker_cursor -= 1;
                        }
                    }
                    Action::MoveDown => {
                        if app.picker_cursor + 1 < app.collections().len() {
                            app.picker_cursor += 1;
                        }
                    }
                    Action::Enter => {
                        let idx = app.picker_cursor;
                        app.show_collection_picker = false;
                        app.save_request_to_collection(idx);
                    }
                    Action::NewRequest | Action::CharInput('n') => {
                        // 'n' or Ctrl+N — create new collection and save there
                        app.show_collection_picker = false;
                        app.name_input.set_content("My Collection");
                        app.start_editing(EditField::NewCollectionName);
                        app.status_message =
                            Some("Name your collection, then press Enter to save".to_string());
                    }
                    Action::Quit => app.should_quit = true,
                    _ => {}
                }
            // Project picker overlay intercepts when open
            } else if app.show_project_picker {
                handle_project_picker_action(app, &action);
            // Environment manager modal intercepts actions when open
            } else if app.show_env_manager {
                handle_env_manager_action(app, &action);
            // Variables overlay intercepts actions when open
            } else if app.show_variables {
                handle_variables_action(app, &action);
                // Editing actions for variable inputs
                match action {
                    Action::CharInput(c) => {
                        if let Some(input) = app.var_active_input() {
                            input.insert_char(c);
                        }
                    }
                    Action::Backspace => {
                        if let Some(input) = app.var_active_input() {
                            input.delete_char_before();
                        }
                    }
                    Action::Delete => {
                        if let Some(input) = app.var_active_input() {
                            input.delete_char_after();
                        }
                    }
                    Action::CursorLeft => {
                        if let Some(input) = app.var_active_input() {
                            input.move_left();
                        }
                    }
                    Action::CursorRight => {
                        if let Some(input) = app.var_active_input() {
                            input.move_right();
                        }
                    }
                    Action::Home => {
                        if let Some(input) = app.var_active_input() {
                            input.move_home();
                        }
                    }
                    Action::End => {
                        if let Some(input) = app.var_active_input() {
                            input.move_end();
                        }
                    }
                    Action::Quit => app.should_quit = true,
                    _ => {}
                }
            } else if app.confirm_delete {
                // Collection/request delete confirmation
                match action {
                    Action::CharInput('y') => {
                        app.confirm_delete = false;
                        app.delete_selected_in_collections();
                    }
                    Action::Cancel | Action::CharInput(_) => {
                        app.confirm_delete = false;
                        app.status_message = None;
                    }
                    _ => {}
                }
            } else if app.show_log_viewer {
                handle_log_viewer_action(app, &action);
            } else {
                // Normal dispatch
                match action {
                    Action::Quit => app.should_quit = true,
                    Action::Cancel => {
                        if app.input_mode == app::InputMode::Editing {
                            app.stop_editing();
                        } else if app.show_help {
                            app.show_help = false;
                        }
                    }
                    Action::CyclePaneForward => {
                        if app.input_mode == app::InputMode::Editing {
                            app.stop_editing();
                        }
                        app.cycle_pane_forward();
                    }
                    Action::CyclePaneBackward => {
                        if app.input_mode == app::InputMode::Editing {
                            app.stop_editing();
                        }
                        app.cycle_pane_backward();
                    }
                    Action::ToggleCollections => app.toggle_pane(0),
                    Action::ToggleRequest => app.toggle_pane(1),
                    Action::ToggleResponse => app.toggle_pane(2),
                    Action::RevealSecrets => app.secrets_revealed = !app.secrets_revealed,
                    Action::OpenLogViewer => {
                        if !app.show_method_picker
                            && !app.show_collection_picker
                            && !app.show_project_picker
                            && !app.show_env_manager
                            && !app.show_variables
                        {
                            app.open_log_viewer();
                        }
                    }
                    Action::Help => app.show_help = !app.show_help,
                    Action::OpenVariables => {
                        app.show_variables = true;
                        app.var_cursor = 0;
                        app.var_editing = None;
                        if let Some(ws) = app.active_workspace_mut() {
                            ws.data.var_collection_idx =
                                ws.data
                                    .selected_collection
                                    .or(if ws.data.collections.is_empty() {
                                        None
                                    } else {
                                        Some(0)
                                    });
                            ws.data.var_environment_idx =
                                ws.data
                                    .active_environment
                                    .or(if ws.data.environments.is_empty() {
                                        None
                                    } else {
                                        Some(0)
                                    });
                        }
                    }
                    Action::SendRequest => {
                        if app.input_mode == app::InputMode::Editing {
                            app.stop_editing();
                        }
                        app.send_request().await;
                    }
                    Action::SaveRequest => app.save_current_request(),
                    Action::NewRequest => {
                        if app.active_pane == app::Pane::Collections {
                            app.create_new_collection();
                        } else {
                            app.new_request();
                            app.load_request_into_inputs();
                        }
                    }
                    Action::SwitchEnvironment => app.cycle_environment(),
                    Action::CopyCurl => {
                        if let Some(req) = app.current_request() {
                            let cmd = CurlCommandBuilder::new(&req.url).method(req.method).build();
                            let display = cmd.to_display_string(&[]);
                            app.status_message = Some(format!("Copied: {}", display));
                        }
                    }
                    // Navigation actions (Normal mode)
                    Action::MoveUp => app.handle_move_up(),
                    Action::MoveDown => app.handle_move_down(),
                    Action::Enter => {
                        if app.input_mode == app::InputMode::Editing {
                            app.stop_editing();
                        } else {
                            app.handle_enter();
                        }
                    }
                    Action::NextTab => match app.active_pane {
                        app::Pane::Request => app.next_request_tab(),
                        app::Pane::Response => app.next_response_tab(),
                        _ => {}
                    },
                    Action::PrevTab => match app.active_pane {
                        app::Pane::Request => app.prev_request_tab(),
                        app::Pane::Response => app.prev_response_tab(),
                        _ => {}
                    },
                    Action::AddItem => {
                        if app.active_pane == app::Pane::Request {
                            match app.request_tab() {
                                app::RequestTab::Headers => app.add_header(),
                                app::RequestTab::Params => app.add_param(),
                                _ => {}
                            }
                        }
                    }
                    Action::NextProject => app.next_project(),
                    Action::PrevProject => app.prev_project(),
                    Action::OpenProjectPicker => {
                        // Refresh the project list from disk
                        let projects_dir = config_dir().join("projects");
                        app.all_projects = lazycurl_core::project::list_projects(&projects_dir)
                            .unwrap_or_default()
                            .into_iter()
                            .map(|(p, path)| {
                                let slug = path.file_name().unwrap().to_string_lossy().to_string();
                                (p, slug)
                            })
                            .collect();
                        app.project_picker_cursor = 0;
                        app.show_project_picker = true;
                    }
                    Action::CloseProject => {
                        if let Some(idx) = app.active_project_idx {
                            app.close_project(idx);
                        }
                    }
                    Action::DeleteItem => {
                        if app.active_pane == app::Pane::Collections {
                            app.request_collection_delete();
                        }
                        // TODO: implement delete for headers/params in Request pane
                    }
                    Action::CycleMethod => {
                        if app.active_pane == app::Pane::Request {
                            app.open_method_picker();
                        }
                    }
                    Action::Rename => app.handle_rename(),
                    // Copy response body to clipboard (y in Response pane)
                    Action::CharInput('y')
                        if app.active_pane == app::Pane::Response
                            && app.input_mode == app::InputMode::Normal =>
                    {
                        if let Some(resp) = app.last_response() {
                            let text = if resp.body.is_empty() {
                                "[no response body]".to_string()
                            } else {
                                resp.body.clone()
                            };
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                let _ = clipboard.set_text(&text);
                                app.status_message =
                                    Some("Copied response body to clipboard".to_string());
                            }
                        }
                    }
                    // Editing actions
                    Action::CharInput(c) => {
                        if let Some(input) = app.active_text_input() {
                            input.insert_char(c);
                        }
                    }
                    Action::Backspace => {
                        if let Some(input) = app.active_text_input() {
                            input.delete_char_before();
                        }
                    }
                    Action::Delete => {
                        if let Some(input) = app.active_text_input() {
                            input.delete_char_after();
                        }
                    }
                    Action::CursorLeft => {
                        if let Some(input) = app.active_text_input() {
                            input.move_left();
                        }
                    }
                    Action::CursorRight => {
                        if let Some(input) = app.active_text_input() {
                            input.move_right();
                        }
                    }
                    Action::Home => {
                        if let Some(input) = app.active_text_input() {
                            input.move_home();
                        }
                    }
                    Action::End => {
                        if let Some(input) = app.active_text_input() {
                            input.move_end();
                        }
                    }
                    _ => {}
                }
            } // end else (not show_variables)
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_variables_action(app: &mut App, action: &Action) {
    // Variable delete confirmation
    if app.var_confirm_delete {
        match action {
            Action::CharInput('y') => {
                app.var_confirm_delete = false;
                app.var_delete_message = None;
                app.var_delete();
            }
            Action::Cancel | Action::CharInput(_) => {
                app.var_confirm_delete = false;
                app.var_delete_message = None;
            }
            _ => {}
        }
        return;
    }

    match action {
        Action::Cancel => {
            if app.input_mode == app::InputMode::Editing {
                app.var_editing = None;
                app.input_mode = app::InputMode::Normal;
            } else {
                app.show_variables = false;
            }
        }
        Action::MoveUp => app.var_move_up(),
        Action::MoveDown => app.var_move_down(),
        Action::NextTab | Action::CyclePaneForward => app.var_next_tier(),
        Action::PrevTab | Action::CyclePaneBackward => app.var_prev_tier(),
        Action::Enter => {
            if app.input_mode == app::InputMode::Editing {
                // If editing key, move to value; if editing value, confirm
                if app.var_editing == Some(app::VarEditTarget::Key) {
                    app.var_stop_editing();
                    app.var_start_edit_value();
                } else {
                    app.var_stop_editing();
                }
            } else {
                // Start editing value of selected variable
                app.var_start_edit_value();
            }
        }
        Action::Rename => {
            // Edit key of selected variable
            if app.input_mode != app::InputMode::Editing {
                app.var_start_edit_key();
            }
        }
        Action::AddItem => {
            if app.input_mode != app::InputMode::Editing {
                // If on Environment/Collection tier with nothing selected, create one first
                let needs_container = match app.var_tier {
                    app::VarTier::Environment => app.active_environment().is_none(),
                    app::VarTier::Collection => app.selected_collection().is_none(),
                    app::VarTier::Global => false,
                };
                if needs_container {
                    match app.var_tier {
                        app::VarTier::Environment => {
                            app.status_message = Some(
                                "No environment selected. Press Ctrl+Shift+E to manage environments."
                                    .to_string(),
                            );
                        }
                        app::VarTier::Collection => {
                            app.status_message =
                                Some("Select or create a collection first (Ctrl+S)".to_string());
                        }
                        _ => {}
                    }
                } else {
                    app.var_add();
                }
            }
        }
        Action::ManageEnvironments => {
            // Ctrl+Shift+E: open env manager on top of variables overlay
            app.open_env_manager();
        }
        Action::SwitchEnvironment => {
            // Ctrl+E cycles environments even while overlay is open
            app.cycle_environment();
            // Sync var_environment_idx to follow the active env
            if let Some(ws) = app.active_workspace_mut() {
                ws.data.var_environment_idx = ws.data.active_environment;
            }
        }
        Action::DeleteItem => {
            if app.input_mode != app::InputMode::Editing {
                app.var_request_delete();
            }
        }
        Action::ToggleSecretFlag => {
            if app.input_mode != app::InputMode::Editing {
                app.var_toggle_secret();
            }
        }
        Action::RevealSecrets => {
            app.secrets_revealed = !app.secrets_revealed;
        }
        // [ and ] cycle through collections within the current tier
        Action::CharInput('[') => {
            if app.input_mode != app::InputMode::Editing && app.var_tier == app::VarTier::Collection
            {
                app.var_cycle_container_backward();
            }
        }
        Action::CharInput(']') => {
            if app.input_mode != app::InputMode::Editing && app.var_tier == app::VarTier::Collection
            {
                app.var_cycle_container_forward();
            }
        }
        _ => {}
    }
}

fn handle_env_manager_action(app: &mut App, action: &Action) {
    // Delete confirmation state takes priority
    if app.env_manager_confirm_delete.is_some() {
        match action {
            Action::CharInput('y') => {
                app.env_manager_execute_delete();
            }
            Action::Cancel => {
                app.env_manager_confirm_delete = None;
            }
            _ => {}
        }
        return;
    }

    // Renaming state
    if app.env_manager_renaming.is_some() {
        match action {
            Action::Enter => {
                app.env_manager_confirm_rename();
            }
            Action::Cancel => {
                app.env_manager_renaming = None;
            }
            Action::CharInput(c) => {
                app.env_manager_name_input.insert_char(*c);
            }
            Action::Backspace => {
                app.env_manager_name_input.delete_char_before();
            }
            Action::Delete => {
                app.env_manager_name_input.delete_char_after();
            }
            Action::CursorLeft => {
                app.env_manager_name_input.move_left();
            }
            Action::CursorRight => {
                app.env_manager_name_input.move_right();
            }
            Action::Home => {
                app.env_manager_name_input.move_home();
            }
            Action::End => {
                app.env_manager_name_input.move_end();
            }
            _ => {}
        }
        return;
    }

    // Normal modal state
    let env_count = app
        .active_workspace()
        .map(|ws| ws.data.environments.len())
        .unwrap_or(0);

    match action {
        Action::Cancel => {
            // Return to variables overlay if it was open underneath
            app.show_env_manager = false;
        }
        Action::MoveUp => {
            if app.env_manager_cursor > 0 {
                app.env_manager_cursor -= 1;
            }
        }
        Action::MoveDown => {
            if env_count > 0 && app.env_manager_cursor + 1 < env_count {
                app.env_manager_cursor += 1;
            }
        }
        Action::Enter => {
            if env_count > 0 {
                app.env_manager_activate();
            }
        }
        Action::CharInput('n') => {
            app.env_manager_create();
        }
        Action::CharInput('r') | Action::Rename => {
            if env_count > 0 {
                app.env_manager_start_rename();
            }
        }
        Action::CharInput('d') | Action::DeleteItem => {
            if env_count > 0 {
                app.env_manager_request_delete();
            }
        }
        Action::Quit => app.should_quit = true,
        _ => {}
    }
}

fn handle_log_viewer_action(app: &mut App, action: &Action) {
    // Filter editing mode
    if app.log_viewer_editing_filter {
        match action {
            Action::Enter => {
                app.log_viewer_filter = app.log_viewer_filter_input.content().to_string();
                app.log_viewer_editing_filter = false;
                app.input_mode = app::InputMode::Normal;
            }
            Action::Cancel => {
                app.log_viewer_editing_filter = false;
                app.input_mode = app::InputMode::Normal;
            }
            Action::CharInput(c) => app.log_viewer_filter_input.insert_char(*c),
            Action::Backspace => {
                app.log_viewer_filter_input.delete_char_before();
            }
            Action::Delete => {
                app.log_viewer_filter_input.delete_char_after();
            }
            Action::CursorLeft => {
                app.log_viewer_filter_input.move_left();
            }
            Action::CursorRight => {
                app.log_viewer_filter_input.move_right();
            }
            Action::Home => {
                app.log_viewer_filter_input.move_home();
            }
            Action::End => {
                app.log_viewer_filter_input.move_end();
            }
            Action::Quit => app.should_quit = true,
            _ => {}
        }
        return;
    }

    // Search editing mode
    if app.log_viewer_editing_search {
        match action {
            Action::Enter => {
                app.log_viewer_search = app.log_viewer_search_input.content().to_string();
                app.log_viewer_editing_search = false;
                app.input_mode = app::InputMode::Normal;
                // Jump to first matching entry
                if !app.log_viewer_search.is_empty() {
                    let search_lower = app.log_viewer_search.to_lowercase();
                    let filtered = app.filtered_log_entries();
                    if let Some(pos) = filtered
                        .iter()
                        .position(|e| e.request.url.to_lowercase().contains(&search_lower))
                    {
                        app.log_viewer_cursor = pos;
                    }
                }
            }
            Action::Cancel => {
                app.log_viewer_editing_search = false;
                app.input_mode = app::InputMode::Normal;
            }
            Action::CharInput(c) => app.log_viewer_search_input.insert_char(*c),
            Action::Backspace => {
                app.log_viewer_search_input.delete_char_before();
            }
            Action::Delete => {
                app.log_viewer_search_input.delete_char_after();
            }
            Action::CursorLeft => {
                app.log_viewer_search_input.move_left();
            }
            Action::CursorRight => {
                app.log_viewer_search_input.move_right();
            }
            Action::Home => {
                app.log_viewer_search_input.move_home();
            }
            Action::End => {
                app.log_viewer_search_input.move_end();
            }
            Action::Quit => app.should_quit = true,
            _ => {}
        }
        return;
    }

    // Normal log viewer mode
    let entry_count = app.filtered_log_entries().len();

    match action {
        Action::OpenLogViewer | Action::Cancel => {
            if app.log_viewer_detail_focused {
                // Unfocus detail, back to list
                app.log_viewer_detail_focused = false;
            } else if app.log_viewer_show_detail {
                app.log_viewer_show_detail = false;
            } else {
                app.show_log_viewer = false;
            }
        }
        Action::CyclePaneForward | Action::CyclePaneBackward => {
            // Tab toggles focus between list and detail pane
            if app.log_viewer_show_detail {
                app.log_viewer_detail_focused = !app.log_viewer_detail_focused;
            }
        }
        Action::MoveUp => {
            if app.log_viewer_detail_focused {
                if app.log_viewer_detail_scroll > 0 {
                    app.log_viewer_detail_scroll -= 1;
                }
            } else if app.log_viewer_cursor > 0 {
                app.log_viewer_cursor -= 1;
                app.log_viewer_detail_scroll = 0;
            }
        }
        Action::MoveDown => {
            if app.log_viewer_detail_focused {
                let total = app.log_viewer_detail_total_lines();
                if total > 0 && app.log_viewer_detail_scroll + 1 < total {
                    app.log_viewer_detail_scroll += 1;
                }
            } else if entry_count > 0 && app.log_viewer_cursor + 1 < entry_count {
                app.log_viewer_cursor += 1;
                app.log_viewer_detail_scroll = 0;
            }
        }
        Action::Enter => {
            if !app.log_viewer_show_detail {
                // Open detail and focus it
                app.log_viewer_show_detail = true;
                app.log_viewer_detail_focused = true;
                app.log_viewer_detail_scroll = 0;
            } else if !app.log_viewer_detail_focused {
                // Already open but list is focused — focus detail
                app.log_viewer_detail_focused = true;
            } else {
                // Detail is focused — unfocus back to list
                app.log_viewer_detail_focused = false;
            }
        }
        Action::CharInput('f') => {
            app.log_viewer_editing_filter = true;
            app.log_viewer_filter_input
                .set_content(&app.log_viewer_filter);
            app.input_mode = app::InputMode::Editing;
        }
        Action::CharInput('/') | Action::Search => {
            app.log_viewer_editing_search = true;
            app.log_viewer_search_input
                .set_content(&app.log_viewer_search);
            app.input_mode = app::InputMode::Editing;
        }
        Action::CharInput('c') => {
            // Clear filter only
            app.log_viewer_filter.clear();
            app.log_viewer_cursor = 0;
        }
        Action::CharInput('C') => {
            // Clear search only
            app.log_viewer_search.clear();
        }
        Action::CharInput('n') => {
            // Jump to next search match
            if !app.log_viewer_search.is_empty() {
                let search_lower = app.log_viewer_search.to_lowercase();
                let filtered = app.filtered_log_entries();
                let start = app.log_viewer_cursor + 1;
                if let Some(pos) = filtered
                    .iter()
                    .skip(start)
                    .position(|e| e.request.url.to_lowercase().contains(&search_lower))
                {
                    app.log_viewer_cursor = start + pos;
                } else if let Some(pos) = filtered
                    .iter()
                    .position(|e| e.request.url.to_lowercase().contains(&search_lower))
                {
                    // Wrap around
                    app.log_viewer_cursor = pos;
                }
            }
        }
        Action::CharInput('N') => {
            // Jump to previous search match
            if !app.log_viewer_search.is_empty() {
                let search_lower = app.log_viewer_search.to_lowercase();
                let filtered = app.filtered_log_entries();
                if app.log_viewer_cursor > 0 {
                    if let Some(pos) = filtered[..app.log_viewer_cursor]
                        .iter()
                        .rposition(|e| e.request.url.to_lowercase().contains(&search_lower))
                    {
                        app.log_viewer_cursor = pos;
                    } else if let Some(pos) = filtered
                        .iter()
                        .rposition(|e| e.request.url.to_lowercase().contains(&search_lower))
                    {
                        // Wrap around
                        app.log_viewer_cursor = pos;
                    }
                } else {
                    // At 0, wrap to last match
                    if let Some(pos) = filtered
                        .iter()
                        .rposition(|e| e.request.url.to_lowercase().contains(&search_lower))
                    {
                        app.log_viewer_cursor = pos;
                    }
                }
            }
        }
        Action::CharInput('r') | Action::Rename => {
            let filtered = app.filtered_log_entries();
            if let Some(entry) = filtered.get(app.log_viewer_cursor) {
                app.load_log_entry_into_editor(entry.clone());
                app.show_log_viewer = false;
            }
        }
        Action::CharInput('y') => {
            let filtered = app.filtered_log_entries();
            if let Some(entry) = filtered.get(app.log_viewer_cursor) {
                let text = entry
                    .response
                    .as_ref()
                    .and_then(|r| r.body.as_deref())
                    .unwrap_or("[no response body]");
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(text);
                    app.status_message = Some("Copied response body to clipboard".to_string());
                }
            }
        }
        Action::CharInput('Y') => {
            let logs_path = lazycurl_core::logging::logs_dir();
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(logs_path.to_string_lossy().to_string());
                app.status_message = Some(format!("Copied: {}", logs_path.display()));
            }
        }
        Action::CharInput('e') => {
            let filtered = app.filtered_log_entries();
            let now = chrono::Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
            let filename = format!("lazycurl-export-{}.jsonl", now);
            let path = std::env::current_dir().unwrap_or_default().join(&filename);
            if let Ok(mut file) = std::fs::File::create(&path) {
                use std::io::Write;
                for entry in &filtered {
                    if let Ok(line) = serde_json::to_string(entry) {
                        let _ = writeln!(file, "{}", line);
                    }
                }
                app.status_message = Some(format!("Exported to {}", path.display()));
                // Try to open the containing folder in the system file explorer
                if let Some(parent) = path.parent() {
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("explorer").arg(parent).spawn();
                    }
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("open").arg(parent).spawn();
                    }
                    #[cfg(target_os = "linux")]
                    {
                        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
                    }
                }
            }
        }
        Action::Quit => app.should_quit = true,
        _ => {}
    }
}

fn handle_project_picker_action(app: &mut App, action: &Action) {
    // Delete confirmation state takes priority
    if app.project_picker_confirm_delete.is_some() {
        match action {
            Action::CharInput('y') => {
                app.project_picker_execute_delete();
            }
            Action::Cancel => {
                app.project_picker_confirm_delete = None;
            }
            _ => {}
        }
        return;
    }

    // Renaming state
    if app.project_picker_renaming {
        match action {
            Action::Enter => {
                app.project_picker_confirm_rename();
            }
            Action::Cancel => {
                app.project_picker_renaming = false;
            }
            Action::CharInput(c) => {
                app.project_picker_name_input.insert_char(*c);
            }
            Action::Backspace => {
                app.project_picker_name_input.delete_char_before();
            }
            Action::Delete => {
                app.project_picker_name_input.delete_char_after();
            }
            Action::CursorLeft => {
                app.project_picker_name_input.move_left();
            }
            Action::CursorRight => {
                app.project_picker_name_input.move_right();
            }
            Action::Home => {
                app.project_picker_name_input.move_home();
            }
            Action::End => {
                app.project_picker_name_input.move_end();
            }
            _ => {}
        }
        return;
    }

    // Normal modal state
    match action {
        Action::Cancel => {
            app.show_project_picker = false;
        }
        Action::MoveUp => {
            if app.project_picker_cursor > 0 {
                app.project_picker_cursor -= 1;
            }
        }
        Action::MoveDown => {
            if app.project_picker_cursor + 1 < app.all_projects.len() {
                app.project_picker_cursor += 1;
            }
        }
        Action::Enter => {
            if let Some((project, slug)) = app.all_projects.get(app.project_picker_cursor).cloned()
            {
                // Check if already open
                if let Some(idx) = app.open_projects.iter().position(|ws| ws.data.slug == slug) {
                    app.switch_project(idx);
                } else {
                    // Open the project
                    let path = config_dir().join("projects").join(&slug);
                    let mut ws = app::ProjectWorkspace::new(project, slug);
                    ws.data.collections =
                        lazycurl_core::collection::list_collections(&path.join("collections"))
                            .unwrap_or_default();
                    ws.data.environments =
                        lazycurl_core::environment::list_environments(&path.join("environments"))
                            .unwrap_or_default();
                    app.open_projects.push(ws);
                    let idx = app.open_projects.len() - 1;
                    app.switch_project(idx);
                }
                app.show_project_picker = false;
            }
        }
        Action::NewRequest | Action::CharInput('n') => {
            // Create new project
            app.show_project_picker = false;
            app.name_input.set_content("New Project");
            app.start_editing(EditField::NewProjectName);
            app.status_message = Some("Name your project".to_string());
        }
        Action::CharInput('r') | Action::Rename => {
            if !app.all_projects.is_empty() {
                app.project_picker_start_rename();
            }
        }
        Action::CharInput('c') => {
            // Close project (remove from tab bar)
            if let Some((_, slug)) = app.all_projects.get(app.project_picker_cursor) {
                if let Some(idx) = app
                    .open_projects
                    .iter()
                    .position(|ws| ws.data.slug == *slug)
                {
                    app.close_project(idx);
                }
            }
        }
        Action::CharInput('d') | Action::DeleteItem => {
            if !app.all_projects.is_empty() {
                app.project_picker_request_delete();
            }
        }
        Action::Quit => app.should_quit = true,
        _ => {}
    }
}
