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

use app::{Action, App};
use curl_tui_core::command::CurlCommandBuilder;
use curl_tui_core::config::{config_dir, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize config directory
    let config_root = config_dir();
    curl_tui_core::init::initialize(&config_root)?;

    // Load config
    let config_path = config_root.join("config.json");
    let config = AppConfig::load_from(&config_path)?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(config);

    // Load existing data
    app.collections = curl_tui_core::collection::list_collections(&config_root.join("collections"))
        .unwrap_or_default();
    app.environments =
        curl_tui_core::environment::list_environments(&config_root.join("environments"))
            .unwrap_or_default();

    // Set active environment from config
    if let Some(env_name) = app.config.active_environment.clone() {
        app.active_environment = app.environments.iter().position(|e| e.name == env_name);
    }

    // Build keymap from config
    let keymap = input::build_keymap(&app.config.keybindings);

    // Load current request fields into text inputs
    app.load_request_into_inputs();

    // Main loop
    let result = run_loop(&mut terminal, &mut app, &keymap).await;

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
        terminal.draw(|frame| {
            ui::draw(frame, app);
        })?;

        if let Some(Event::Key(key)) = events::poll_event(Duration::from_millis(50))? {
            let action = match app.input_mode {
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
            };

            // Variables overlay intercepts actions when open
            if app.show_variables {
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
                    Action::Help => app.show_help = !app.show_help,
                    Action::OpenVariables => {
                        app.show_variables = true;
                        app.var_cursor = 0;
                        app.var_editing = None;
                    }
                    Action::SendRequest => {
                        if app.input_mode == app::InputMode::Editing {
                            app.stop_editing();
                        }
                        app.send_request().await;
                    }
                    Action::SaveRequest => app.save_current_request(),
                    Action::NewRequest => {
                        app.new_request();
                        app.load_request_into_inputs();
                    }
                    Action::SwitchEnvironment => app.cycle_environment(),
                    Action::CopyCurl => {
                        if let Some(req) = &app.current_request {
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
                            match app.request_tab {
                                app::RequestTab::Headers => app.add_header(),
                                app::RequestTab::Params => app.add_param(),
                                _ => {}
                            }
                        }
                    }
                    Action::DeleteItem => {
                        // TODO: implement delete for headers/params
                    }
                    Action::Rename => app.handle_rename(),
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
                app.var_add();
            }
        }
        Action::DeleteItem => {
            if app.input_mode != app::InputMode::Editing {
                app.var_delete();
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
        _ => {}
    }
}
