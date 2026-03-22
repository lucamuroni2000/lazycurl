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

        if let Some(Event::Key(key)) = events::poll_event(Duration::from_millis(100))? {
            {
                let action = input::resolve_action(key, keymap);
                match action {
                    Action::Quit => {
                        app.should_quit = true;
                    }
                    Action::Cancel => {
                        if app.show_help {
                            app.show_help = false;
                        }
                    }
                    Action::CyclePaneForward => app.cycle_pane_forward(),
                    Action::CyclePaneBackward => app.cycle_pane_backward(),
                    Action::ToggleCollections => app.toggle_pane(0),
                    Action::ToggleRequest => app.toggle_pane(1),
                    Action::ToggleResponse => app.toggle_pane(2),
                    Action::RevealSecrets => {
                        app.secrets_revealed = !app.secrets_revealed;
                    }
                    Action::Help => {
                        app.show_help = !app.show_help;
                    }
                    Action::SendRequest => app.send_request().await,
                    Action::SaveRequest => app.save_current_request(),
                    Action::NewRequest => app.new_request(),
                    Action::SwitchEnvironment => app.cycle_environment(),
                    Action::CopyCurl => {
                        if let Some(req) = &app.current_request {
                            let cmd = CurlCommandBuilder::new(&req.url).method(req.method).build();
                            let display = cmd.to_display_string(&[]);
                            app.status_message = Some(format!("Copied: {}", display));
                        }
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
