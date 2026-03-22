mod app;
mod events;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::{
    event::Event,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{Action, App};
use curl_tui_core::config::{config_dir, AppConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Main loop
    let result = run_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| {
            ui::draw(frame, app);
        })?;

        if let Some(Event::Key(key)) = events::poll_event(Duration::from_millis(100))? {
            let action = events::key_to_action(key);
            match action {
                    Action::Quit => {
                        app.should_quit = true;
                    }
                    Action::Cancel => {
                        // Close popups/overlays; if nothing open, do nothing
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
                    // SendRequest, SaveRequest, SwitchEnvironment, NewRequest,
                    // CopyCurl are wired in Tasks 15-17
                    _ => {}
                }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
