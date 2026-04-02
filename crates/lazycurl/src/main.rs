mod app;
mod events;
mod handlers;
mod input;
pub mod text_input;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;
use input::ContextKeymaps;
use lazycurl_core::config::{config_dir, AppConfig};
use lazycurl_core::export::{self, ExportFormat};
use lazycurl_core::types::{Auth, Body};
use lazycurl_core::variable::FileVariableResolver;

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
                ws.data.restore_active_environment();
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
            ws.data.restore_active_environment();
            app.open_projects.push(ws);
            app.active_project_idx = Some(0);
        }
    }

    // Build context-scoped keymaps from config
    let keymaps = input::build_context_keymaps(&app.config.keybindings);

    // Load current request fields into text inputs
    app.load_request_into_inputs();

    // Main loop
    let result = run_loop(&mut terminal, &mut app, &keymaps).await;

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
    keymaps: &ContextKeymaps,
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
            // Confirmation dialogs bypass the keymap entirely: 'y'/'Y' confirms,
            // Esc cancels, everything else is ignored. This avoids conflicts where
            // 'y' is also bound to Copy in the global keymap.
            let any_confirm = app.confirm_delete
                || app.var_confirm_delete
                || app.env_manager_confirm_delete.is_some()
                || app.project_picker_confirm_delete.is_some();

            let action = if any_confirm {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => app::Action::ConfirmYes,
                    KeyCode::Esc => app::Action::Cancel,
                    _ => app::Action::None,
                }
            } else if app.show_env_manager && app.env_manager_renaming.is_some() {
                // Env manager rename uses editing resolver so letters aren't
                // swallowed by navigation bindings (e.g. 'd' → DeleteItem).
                input::resolve_editing(key)
            } else if app.show_project_picker && app.project_picker_renaming {
                // Project picker rename uses editing resolver.
                input::resolve_editing(key)
            } else {
                match app.input_mode {
                    app::InputMode::Normal => {
                        let ctx = app.active_input_context();
                        input::resolve_action(key, keymaps, ctx)
                    }
                    app::InputMode::Editing => input::resolve_editing(key),
                }
            };

            // Delete confirmations take priority
            if app.confirm_delete {
                handlers::confirmations::handle_collection_delete(app, &action);
            } else if app.var_confirm_delete {
                handlers::confirmations::handle_variable_delete(app, &action);
            } else if app.env_manager_confirm_delete.is_some() {
                handlers::confirmations::handle_env_delete(app, &action);
            } else if app.project_picker_confirm_delete.is_some() {
                handlers::confirmations::handle_project_delete(app, &action);
            // Modal handlers
            } else if app.show_method_picker {
                handlers::pickers::handle_method_picker(app, &action);
            } else if app.show_auth_picker {
                handlers::pickers::handle_auth_picker(app, &action);
            } else if app.show_export_picker {
                handlers::pickers::handle_export_picker(app, &action);
            } else if app.show_collection_picker {
                handlers::pickers::handle_collection_picker(app, &action);
            } else if app.show_project_picker {
                handlers::pickers::handle_project_picker(app, &action);
            } else if app.show_env_manager {
                handlers::pickers::handle_env_manager(app, &action);
            } else if app.show_variables {
                handlers::variables::handle(app, &action);
            } else if app.show_log_viewer {
                handlers::log_viewer::handle(app, &action);
            } else {
                handlers::main_dispatch::handle(app, &action).await;
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

pub(crate) fn open_in_file_explorer(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        #[cfg(target_os = "windows")]
        {
            if !is_explorer_open_for(parent) {
                let _ = std::process::Command::new("explorer").arg(parent).spawn();
            }
        }
        #[cfg(target_os = "macos")]
        {
            // `open` on macOS reuses an existing Finder window for the same folder
            let _ = std::process::Command::new("open").arg(parent).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }
}

#[cfg(target_os = "windows")]
fn is_explorer_open_for(dir: &std::path::Path) -> bool {
    // Query open Explorer windows via the Shell.Application COM object.
    // The PowerShell snippet lists the LocationURL of every open window;
    // we check if any of them matches the target directory.
    let dir_str = dir.to_string_lossy().replace('\\', "/");
    let script =
        "(New-Object -ComObject Shell.Application).Windows() | ForEach-Object { $_.LocationURL }";
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // LocationURL is a file:/// URI, e.g. "file:///C:/Users/foo/bar"
            let target = format!("file:///{}", dir_str.trim_start_matches('/'));
            stdout.lines().any(|line| {
                let decoded = urldecode(line.trim());
                decoded.eq_ignore_ascii_case(&target)
            })
        }
        Err(_) => false,
    }
}

#[cfg(target_os = "windows")]
fn urldecode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(h), Some(l)) = (hi, lo) {
                let hex = [h, l];
                if let Ok(val) = u8::from_str_radix(&String::from_utf8_lossy(&hex), 16) {
                    result.push(val as char);
                    continue;
                }
            }
            result.push(b as char);
        } else {
            result.push(b as char);
        }
    }
    result
}

pub(crate) fn execute_export(app: &mut App, format: ExportFormat) {
    match format {
        ExportFormat::Curl => {
            if let Some(req) = app.current_request().cloned() {
                let (resolved_req, secrets) = match resolve_request(app, &req) {
                    Ok(r) => r,
                    Err(e) => {
                        app.status_message = Some(format!("Variable error: {}", e));
                        return;
                    }
                };
                let curl_str = export::export_curl(&resolved_req, &secrets);
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&curl_str);
                }
                app.status_message = Some("Copied curl command to clipboard".to_string());
            }
        }
        ExportFormat::PostmanV21 | ExportFormat::OpenApi3 => {
            let (json, name) = if app.export_scope_is_collection {
                let collection = app
                    .active_workspace()
                    .and_then(|ws| {
                        ws.data
                            .selected_collection
                            .and_then(|idx| ws.data.collections.get(idx))
                    })
                    .cloned();
                if let Some(col) = collection {
                    let json = match format {
                        ExportFormat::PostmanV21 => export::export_postman_collection(&col),
                        ExportFormat::OpenApi3 => export::export_openapi_collection(&col),
                        _ => unreachable!(),
                    };
                    (json, col.name)
                } else {
                    app.status_message = Some("No collection selected".to_string());
                    return;
                }
            } else {
                if let Some(req) = app.current_request().cloned() {
                    let json = match format {
                        ExportFormat::PostmanV21 => export::export_postman_request(&req),
                        ExportFormat::OpenApi3 => export::export_openapi_request(&req),
                        _ => unreachable!(),
                    };
                    (json, req.name)
                } else {
                    app.status_message = Some("No request to export".to_string());
                    return;
                }
            };

            let dir = export::exports_dir();
            if let Err(e) = std::fs::create_dir_all(&dir) {
                app.status_message = Some(format!("Failed to create exports dir: {}", e));
                return;
            }

            let filename = export::export_filename(&name, format);
            let path = dir.join(&filename);

            match serde_json::to_string_pretty(&json) {
                Ok(content) => {
                    if let Err(e) = std::fs::write(&path, content) {
                        app.status_message = Some(format!("Export failed: {}", e));
                        return;
                    }
                    app.status_message = Some(format!("Exported to {}", filename));
                    open_in_file_explorer(&path);
                }
                Err(e) => {
                    app.status_message = Some(format!("Serialization failed: {}", e));
                }
            }
        }
    }
}

/// Build a FileVariableResolver from the app's current variable stores
/// and resolve all fields of a Request, returning the resolved request
/// and a list of secret values for redaction.
fn resolve_request(
    app: &App,
    req: &lazycurl_core::types::Request,
) -> Result<(lazycurl_core::types::Request, Vec<String>), lazycurl_core::variable::ResolveError> {
    let ws = app.active_workspace();
    let global_vars = app.config.variables.clone();
    let env_vars = ws.and_then(|ws| {
        ws.data
            .active_environment
            .and_then(|i| ws.data.environments.get(i))
            .map(|e| e.variables.clone())
    });
    let col_vars = ws.and_then(|ws| {
        ws.data
            .selected_collection
            .and_then(|i| ws.data.collections.get(i))
            .map(|c| c.variables.clone())
    });

    let resolver = FileVariableResolver::new(global_vars, env_vars, col_vars);
    let mut secrets = Vec::new();

    // Resolve URL
    let (resolved_url, s) = resolver.resolve(&req.url)?;
    secrets.extend(s);

    // Resolve headers
    let mut resolved_headers = Vec::new();
    for header in &req.headers {
        if header.enabled {
            let (resolved_val, s) = resolver.resolve(&header.value)?;
            secrets.extend(s);
            resolved_headers.push(lazycurl_core::types::Header {
                key: header.key.clone(),
                value: resolved_val,
                enabled: true,
            });
        }
    }

    // Resolve params
    let mut resolved_params = Vec::new();
    for param in &req.params {
        if param.enabled {
            let (resolved_val, s) = resolver.resolve(&param.value)?;
            secrets.extend(s);
            resolved_params.push(lazycurl_core::types::Param {
                key: param.key.clone(),
                value: resolved_val,
                enabled: true,
            });
        }
    }

    // Resolve body
    let resolved_body = match &req.body {
        Some(Body::Json { content }) => {
            let (resolved, s) = resolver.resolve(content)?;
            secrets.extend(s);
            Some(Body::Json { content: resolved })
        }
        Some(Body::Text { content }) => {
            let (resolved, s) = resolver.resolve(content)?;
            secrets.extend(s);
            Some(Body::Text { content: resolved })
        }
        Some(Body::Form { fields }) => {
            let mut resolved_fields = Vec::new();
            for field in fields {
                if field.enabled {
                    let (resolved_val, s) = resolver.resolve(&field.value)?;
                    secrets.extend(s);
                    resolved_fields.push(lazycurl_core::types::FormField {
                        key: field.key.clone(),
                        value: resolved_val,
                        enabled: true,
                    });
                }
            }
            Some(Body::Form {
                fields: resolved_fields,
            })
        }
        other => other.clone(),
    };

    // Resolve auth
    let resolved_auth = match &req.auth {
        Some(Auth::Bearer { token }) => {
            let (resolved, s) = resolver.resolve(token)?;
            secrets.extend(s);
            Some(Auth::Bearer { token: resolved })
        }
        Some(Auth::Basic { username, password }) => {
            let (resolved_user, s) = resolver.resolve(username)?;
            secrets.extend(s);
            let (resolved_pass, s) = resolver.resolve(password)?;
            secrets.extend(s);
            Some(Auth::Basic {
                username: resolved_user,
                password: resolved_pass,
            })
        }
        Some(Auth::ApiKey {
            key,
            value,
            location,
        }) => {
            let (resolved_val, s) = resolver.resolve(value)?;
            secrets.extend(s);
            Some(Auth::ApiKey {
                key: key.clone(),
                value: resolved_val,
                location: location.clone(),
            })
        }
        other => other.clone(),
    };

    let resolved_req = lazycurl_core::types::Request {
        id: req.id,
        name: req.name.clone(),
        method: req.method,
        url: resolved_url,
        headers: resolved_headers,
        params: resolved_params,
        body: resolved_body,
        auth: resolved_auth,
    };

    Ok((resolved_req, secrets))
}
