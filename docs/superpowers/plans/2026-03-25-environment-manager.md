# Environment Manager Modal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Ctrl+Shift+E "create environment" action with a dedicated Environment Manager modal for full environment CRUD (create, rename, delete, activate) per project, and simplify the variables overlay to be variables-only.

**Architecture:** A new modal overlay (`environment_manager.rs`) with its own state fields on `App` and a dedicated dispatch function in `main.rs`. The existing `Action::CreateEnvironment` is renamed to `Action::ManageEnvironments`. Environment CRUD is removed from `handle_variables_action`. The `cycle_environment()` method is updated to show a hint instead of auto-creating when the list is empty.

**Tech Stack:** Rust, ratatui, crossterm, serde_json

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/curl-tui/src/app.rs` | Modify | Rename `CreateEnvironment` → `ManageEnvironments`, add env manager state fields and methods |
| `crates/curl-tui-core/src/config.rs` | Modify | Rename `create_env` → `manage_envs` default keybinding |
| `crates/curl-tui/src/input.rs` | Modify | Rename `create_env` → `manage_envs` mapping |
| `crates/curl-tui/src/ui/environment_manager.rs` | Create | Modal rendering |
| `crates/curl-tui/src/ui/mod.rs` | Modify | Add `pub mod environment_manager`, add draw call |
| `crates/curl-tui/src/main.rs` | Modify | Add `handle_env_manager_action`, update dispatch priority, remove env CRUD from variables dispatch, fix `cycle_environment` behavior in overlay |
| `crates/curl-tui/src/ui/variables.rs` | Modify | Remove environment CRUD hints, update empty-state message |
| `crates/curl-tui/src/ui/help.rs` | Modify | Update help text |
| `crates/curl-tui/src/ui/statusbar.rs` | Modify | Update hint text |

---

### Task 1: Rename action and keybinding

**Files:**
- Modify: `crates/curl-tui/src/app.rs:82` (Action enum)
- Modify: `crates/curl-tui-core/src/config.rs:19` (default keybindings)
- Modify: `crates/curl-tui/src/input.rs:67` (action map)
- Modify: `crates/curl-tui/src/ui/help.rs:32` (help text)
- Modify: `crates/curl-tui/src/ui/statusbar.rs:164-165` (hint text)

- [ ] **Step 1: Rename `CreateEnvironment` to `ManageEnvironments` in the Action enum**

In `crates/curl-tui/src/app.rs`, change line 82:

```rust
    // Before:
    CreateEnvironment,
    // After:
    ManageEnvironments,
```

- [ ] **Step 2: Rename config key in `config.rs`**

In `crates/curl-tui-core/src/config.rs`, change line 19:

```rust
    // Before:
    map.insert("create_env".into(), "ctrl+shift+e".into());
    // After:
    map.insert("manage_envs".into(), "ctrl+shift+e".into());
```

- [ ] **Step 3: Rename mapping in `input.rs`**

In `crates/curl-tui/src/input.rs`, change line 67:

```rust
    // Before:
    ("create_env", Action::CreateEnvironment),
    // After:
    ("manage_envs", Action::ManageEnvironments),
```

- [ ] **Step 4: Update help text**

In `crates/curl-tui/src/ui/help.rs`, change line 32:

```rust
    // Before:
    binding("Ctrl+Shift+E", "Create a new environment"),
    // After:
    binding("Ctrl+Shift+E", "Manage environments"),
```

- [ ] **Step 5: Update statusbar hint**

In `crates/curl-tui/src/ui/statusbar.rs`, change line 165:

```rust
    // Before:
    hints.push(Span::styled(":new env ", hint_style));
    // After:
    hints.push(Span::styled(":envs ", hint_style));
```

- [ ] **Step 6: Update all references in `main.rs`**

In `crates/curl-tui/src/main.rs`, rename all `Action::CreateEnvironment` to `Action::ManageEnvironments`. There are two occurrences:

Line 360 (normal dispatch):
```rust
    // Before:
    Action::CreateEnvironment => {
    // After:
    Action::ManageEnvironments => {
```

Line 544 (handle_variables_action):
```rust
    // Before:
    Action::CreateEnvironment => {
    // After:
    Action::ManageEnvironments => {
```

- [ ] **Step 7: Verify it compiles**

Run: `cargo check --workspace`
Expected: Compiles without errors

- [ ] **Step 8: Commit**

```bash
git add crates/curl-tui/src/app.rs crates/curl-tui-core/src/config.rs crates/curl-tui/src/input.rs crates/curl-tui/src/ui/help.rs crates/curl-tui/src/ui/statusbar.rs crates/curl-tui/src/main.rs
git commit -m "refactor: rename CreateEnvironment to ManageEnvironments"
```

---

### Task 2: Add env manager state fields and open/close methods

**Files:**
- Modify: `crates/curl-tui/src/app.rs` (App struct fields + methods)

- [ ] **Step 1: Add state fields to `App`**

In `crates/curl-tui/src/app.rs`, find the `App` struct and add these fields alongside the other overlay state (near `show_variables`, `show_project_picker`, etc.):

```rust
    // Environment Manager state
    pub show_env_manager: bool,
    pub env_manager_cursor: usize,
    pub env_manager_renaming: Option<usize>,
    pub env_manager_confirm_delete: Option<usize>,
    pub env_manager_name_input: TextInput,
```

Note: `TextInput` is already imported — it's used for `name_input`, `url_input`, etc.

- [ ] **Step 2: Initialize the new fields in `App::new()`**

Find the `App::new()` constructor and add the initializers:

```rust
    show_env_manager: false,
    env_manager_cursor: 0,
    env_manager_renaming: None,
    env_manager_confirm_delete: None,
    env_manager_name_input: TextInput::new(),
```

- [ ] **Step 3: Add `open_env_manager()` method**

Add this method to the `App` impl block, near the other environment methods (around line 1460):

```rust
    /// Open the environment manager modal, reset state.
    pub fn open_env_manager(&mut self) {
        self.show_env_manager = true;
        self.env_manager_cursor = 0;
        self.env_manager_renaming = None;
        self.env_manager_confirm_delete = None;
    }
```

- [ ] **Step 4: Add `env_manager_create()` method**

This is the modal-specific create that does NOT auto-activate (unlike `create_new_environment()`):

```rust
    /// Create a new environment from the env manager modal (does not auto-activate).
    pub fn env_manager_create(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        let env = Environment {
            id: uuid::Uuid::new_v4(),
            name: "New Environment".to_string(),
            variables: std::collections::HashMap::new(),
        };
        let env_dir = config_dir()
            .join("projects")
            .join(&ws.data.slug)
            .join("environments");
        match curl_tui_core::environment::save_environment(&env_dir, &env) {
            Ok(_) => {
                ws.data.environments.push(env);
                let idx = ws.data.environments.len() - 1;
                self.env_manager_cursor = idx;
                self.env_manager_name_input.set_content("New Environment");
                self.env_manager_renaming = Some(idx);
            }
            Err(e) => {
                self.status_message = Some(format!("Error creating environment: {}", e));
            }
        }
    }
```

- [ ] **Step 5: Add `env_manager_activate()` method**

```rust
    /// Activate the environment at the cursor and close the modal.
    pub fn env_manager_activate(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        if let Some(env) = ws.data.environments.get(self.env_manager_cursor) {
            let name = env.name.clone();
            ws.data.active_environment = Some(self.env_manager_cursor);
            // Also update var_environment_idx so variables overlay shows the right env
            ws.data.var_environment_idx = Some(self.env_manager_cursor);
            let _ = ws;
            self.persist_active_environment();
            self.show_env_manager = false;
            self.status_message = Some(format!("Environment: {}", name));
        }
    }
```

- [ ] **Step 6: Add `env_manager_start_rename()` method**

```rust
    /// Enter rename mode for the environment at the cursor.
    pub fn env_manager_start_rename(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        if let Some(env) = ws.data.environments.get(self.env_manager_cursor) {
            self.env_manager_name_input.set_content(&env.name);
            self.env_manager_renaming = Some(self.env_manager_cursor);
        }
    }
```

- [ ] **Step 7: Add `env_manager_confirm_rename()` method**

```rust
    /// Commit the rename: delete old file, save new, update name in memory.
    pub fn env_manager_confirm_rename(&mut self) {
        let Some(rename_idx) = self.env_manager_renaming else {
            return;
        };
        let new_name = self.env_manager_name_input.content().to_string();
        if new_name.is_empty() {
            self.env_manager_renaming = None;
            return;
        }
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        if let Some(env) = ws.data.environments.get_mut(rename_idx) {
            let old_slug = curl_tui_core::collection::slugify(&env.name);
            let env_dir = config_dir()
                .join("projects")
                .join(&ws.data.slug)
                .join("environments");
            // Delete old file
            let old_path = env_dir.join(format!("{}.json", old_slug));
            if old_path.exists() {
                let _ = std::fs::remove_file(&old_path);
            }
            // Update name and save new file
            env.name = new_name;
            let _ = curl_tui_core::environment::save_environment(&env_dir, env);
        }
        self.env_manager_renaming = None;
        // If the renamed env is the active one, update project.json
        if self
            .active_workspace()
            .and_then(|ws| ws.data.active_environment)
            == Some(rename_idx)
        {
            self.persist_active_environment();
        }
    }
```

- [ ] **Step 8: Add `env_manager_request_delete()` and `env_manager_confirm_delete()` methods**

```rust
    /// Show delete confirmation for the environment at the cursor.
    pub fn env_manager_request_delete(&mut self) {
        let env_count = self
            .active_workspace()
            .map(|ws| ws.data.environments.len())
            .unwrap_or(0);
        if self.env_manager_cursor < env_count {
            self.env_manager_confirm_delete = Some(self.env_manager_cursor);
        }
    }

    /// Execute the confirmed deletion.
    pub fn env_manager_execute_delete(&mut self) {
        let Some(delete_idx) = self.env_manager_confirm_delete.take() else {
            return;
        };
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        let Some(env) = ws.data.environments.get(delete_idx) else {
            return;
        };

        // Delete the file from disk
        let slug_str = curl_tui_core::collection::slugify(&env.name);
        let path = config_dir()
            .join("projects")
            .join(&ws.data.slug)
            .join("environments")
            .join(format!("{}.json", slug_str));
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }

        let name = env.name.clone();
        ws.data.environments.remove(delete_idx);

        // Adjust active_environment index
        if ws.data.environments.is_empty() {
            ws.data.active_environment = None;
            ws.data.var_environment_idx = None;
        } else {
            let max = ws.data.environments.len() - 1;
            ws.data.active_environment = ws.data.active_environment.map(|i| {
                if i == delete_idx {
                    return max.min(delete_idx); // deleted the active one
                }
                if i > delete_idx {
                    i - 1 // shift down
                } else {
                    i
                }
            });
            ws.data.var_environment_idx = ws.data.active_environment;
        }

        // Clamp cursor
        if ws.data.environments.is_empty() {
            self.env_manager_cursor = 0;
        } else {
            let max = ws.data.environments.len() - 1;
            self.env_manager_cursor = self.env_manager_cursor.min(max);
        }

        let _ = ws;
        self.persist_active_environment();
        self.status_message = Some(format!("Deleted environment '{}'", name));
    }
```

- [ ] **Step 9: Verify it compiles**

Run: `cargo check -p curl-tui`
Expected: Compiles (fields are added but not yet used in rendering/dispatch — that's fine)

- [ ] **Step 10: Commit**

```bash
git add crates/curl-tui/src/app.rs
git commit -m "feat: add env manager state fields and CRUD methods"
```

---

### Task 3: Create the environment manager modal renderer

**Files:**
- Create: `crates/curl-tui/src/ui/environment_manager.rs`
- Modify: `crates/curl-tui/src/ui/mod.rs`

- [ ] **Step 1: Create `environment_manager.rs`**

Create `crates/curl-tui/src/ui/environment_manager.rs` with this content:

```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_width = (area.width * 50 / 100).max(40).min(area.width);
    // Height: 3 (border + padding) + env count + 1 (confirm line), capped at 60%
    let env_count = app
        .active_workspace()
        .map(|ws| ws.data.environments.len())
        .unwrap_or(0);
    let content_lines = if env_count == 0 { 1 } else { env_count };
    let confirm_line = if app.env_manager_confirm_delete.is_some() {
        1
    } else {
        0
    };
    let popup_height = ((content_lines + confirm_line + 2) as u16)
        .max(5)
        .min(area.height * 60 / 100)
        .min(area.height);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let project_name = app
        .active_workspace()
        .map(|ws| ws.data.project.name.as_str())
        .unwrap_or("No project");

    let title = format!(
        " Environments ({}) — n:new  r:rename  d:delete  Enter:activate  Esc:close ",
        project_name
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if env_count == 0 {
        let msg = Paragraph::new(" No environments. Press n to create one.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    }

    let active_idx = app
        .active_workspace()
        .and_then(|ws| ws.data.active_environment);

    let environments: Vec<&curl_tui_core::types::Environment> = app
        .active_workspace()
        .map(|ws| ws.data.environments.iter().collect())
        .unwrap_or_default();

    // If there's a confirm delete, reserve the last line
    let list_area = if app.env_manager_confirm_delete.is_some() && inner.height > 1 {
        Rect::new(inner.x, inner.y, inner.width, inner.height - 1)
    } else {
        inner
    };

    let items: Vec<ListItem> = environments
        .iter()
        .enumerate()
        .map(|(i, env)| {
            let is_cursor = i == app.env_manager_cursor;
            let is_active = active_idx == Some(i);
            let is_renaming = app.env_manager_renaming == Some(i);

            let cursor_marker = if is_cursor { "> " } else { "  " };
            let active_marker = if is_active { "[*] " } else { "    " };

            let name_span = if is_renaming {
                // Show the text input content when renaming
                Span::styled(
                    app.env_manager_name_input.content(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::UNDERLINED),
                )
            } else {
                let style = if is_cursor {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Span::styled(&env.name, style)
            };

            ListItem::new(Line::from(vec![
                Span::raw(cursor_marker),
                Span::styled(active_marker, Style::default().fg(Color::Green)),
                name_span,
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);

    // Show cursor position when renaming
    if let Some(rename_idx) = app.env_manager_renaming {
        if rename_idx < environments.len() {
            // cursor_marker (2) + active_marker (4) + input cursor
            let cursor_x = inner.x + 2 + 4 + app.env_manager_name_input.cursor() as u16;
            let cursor_y = inner.y + rename_idx as u16;
            if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    // Delete confirmation line
    if let Some(delete_idx) = app.env_manager_confirm_delete {
        if let Some(env) = environments.get(delete_idx) {
            let confirm_area = Rect::new(
                inner.x,
                inner.y + inner.height - 1,
                inner.width,
                1,
            );
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(
                    format!(" Delete '{}'? ", env.name),
                    Style::default().fg(Color::Red),
                ),
                Span::styled("y", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(" to confirm, ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
                Span::styled(" to cancel", Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(msg, confirm_area);
        }
    }
}
```

- [ ] **Step 2: Add module declaration in `mod.rs`**

In `crates/curl-tui/src/ui/mod.rs`, add after `pub mod collections;` (line 1):

```rust
pub mod environment_manager;
```

- [ ] **Step 3: Add draw call in `mod.rs`**

In `crates/curl-tui/src/ui/mod.rs`, in the `draw` function, add the env manager draw call. It must render AFTER variables but BEFORE help (so help overlays everything). Add after the `if app.show_variables` block (line 42):

```rust
    if app.show_env_manager {
        environment_manager::draw(frame, app);
    }
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p curl-tui`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui/src/ui/environment_manager.rs crates/curl-tui/src/ui/mod.rs
git commit -m "feat: add environment manager modal renderer"
```

---

### Task 4: Add dispatch for the environment manager modal

**Files:**
- Modify: `crates/curl-tui/src/main.rs`

- [ ] **Step 1: Add `handle_env_manager_action` function**

Add this function in `crates/curl-tui/src/main.rs`, after `handle_variables_action` (after line 581):

```rust
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
        Action::CharInput('r') => {
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
```

- [ ] **Step 2: Wire env manager dispatch into the main event loop**

In `crates/curl-tui/src/main.rs`, find the dispatch priority chain. Currently it goes:

```
if app.show_first_launch { ... }
else if app.show_collection_picker { ... }
else if app.show_project_picker { ... }
else if app.show_variables { ... }
else if app.input_mode == InputMode::Editing { ... }
else { /* normal dispatch */ }
```

Add the env manager check BEFORE `show_variables` (highest priority after other modals). Find the line `} else if app.show_variables {` (around line 251) and insert before it:

```rust
            } else if app.show_env_manager {
                handle_env_manager_action(app, &action);
```

- [ ] **Step 3: Update `Action::ManageEnvironments` in normal dispatch**

Find the existing `Action::ManageEnvironments` arm in normal dispatch (was `Action::CreateEnvironment`, around line 360). Replace the entire block:

```rust
                    Action::ManageEnvironments => {
                        app.open_env_manager();
                    }
```

This replaces the old behavior that created an env + opened the variables overlay.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p curl-tui`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui/src/main.rs
git commit -m "feat: add env manager dispatch and wire into event loop"
```

---

### Task 5: Remove environment CRUD from variables overlay

**Files:**
- Modify: `crates/curl-tui/src/main.rs` (handle_variables_action)
- Modify: `crates/curl-tui/src/ui/variables.rs` (hints and empty-state message)
- Modify: `crates/curl-tui/src/app.rs` (cycle_environment empty-list behavior)

- [ ] **Step 1: Remove env creation arms from `handle_variables_action`**

In `crates/curl-tui/src/main.rs`, in `handle_variables_action` (starting around line 478):

**Remove** the entire `Action::SwitchEnvironment | Action::NewRequest` arm (lines 537-543):

```rust
        // DELETE THIS BLOCK:
        Action::SwitchEnvironment | Action::NewRequest => {
            // Ctrl+E or Ctrl+N in the variables overlay: create a new environment
            if app.var_tier == app::VarTier::Environment {
                app.show_variables = false;
                app.create_new_environment();
            }
        }
```

**Remove** the entire `Action::ManageEnvironments` arm (was `Action::CreateEnvironment`, lines 544-548):

```rust
        // DELETE THIS BLOCK:
        Action::ManageEnvironments => {
            // Ctrl+Shift+E in the variables overlay: create and stay in overlay
            app.create_new_environment();
            app.var_tier = app::VarTier::Environment;
        }
```

**Replace** with a single arm that lets Ctrl+Shift+E open the env manager from within the overlay:

```rust
        Action::ManageEnvironments => {
            // Ctrl+Shift+E: close variables overlay, open env manager
            app.show_variables = false;
            app.open_env_manager();
        }
```

- [ ] **Step 2: Update the `Action::AddItem` arm for environments**

In `handle_variables_action`, find the `Action::AddItem` arm (around line 512). The `needs_container` branch currently creates an environment when on the Environment tier. Change it to show a hint instead:

```rust
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
```

- [ ] **Step 3: Remove environment deletion from `Action::DeleteItem` arm**

In `handle_variables_action`, find the `Action::DeleteItem` arm (around line 549). Currently it deletes the environment when the variable list is empty. Change it to only delete variables:

```rust
        Action::DeleteItem => {
            if app.input_mode != app::InputMode::Editing {
                app.var_delete();
            }
        }
```

- [ ] **Step 4: Remove `[`/`]` environment cycling from variables overlay**

In `handle_variables_action`, find the `Action::CharInput('[')` and `Action::CharInput(']')` arms (around lines 569-578). These currently cycle through environments/collections. Remove the environment cycling by making them only work for collections:

```rust
        Action::CharInput('[') => {
            if app.input_mode != app::InputMode::Editing
                && app.var_tier == app::VarTier::Collection
            {
                app.var_cycle_container_backward();
            }
        }
        Action::CharInput(']') => {
            if app.input_mode != app::InputMode::Editing
                && app.var_tier == app::VarTier::Collection
            {
                app.var_cycle_container_forward();
            }
        }
```

- [ ] **Step 5: Add `SwitchEnvironment` passthrough in variables overlay**

Ctrl+E should still cycle environments while the overlay is open. Add a new arm to `handle_variables_action`:

```rust
        Action::SwitchEnvironment => {
            // Ctrl+E cycles environments even while overlay is open
            app.cycle_environment();
            // Update var_environment_idx to match the new active environment
            if let Some(ws) = app.active_workspace() {
                let idx = ws.data.active_environment;
                // Need mutable access
                drop(ws);
                if let Some(ws) = app.active_workspace_mut() {
                    ws.data.var_environment_idx = ws.data.active_environment;
                }
            }
        }
```

```rust
        Action::SwitchEnvironment => {
            // Ctrl+E cycles environments even while overlay is open
            app.cycle_environment();
            // Sync var_environment_idx to follow the active env
            if let Some(ws) = app.active_workspace_mut() {
                ws.data.var_environment_idx = ws.data.active_environment;
            }
        }
```

- [ ] **Step 6: Update `cycle_environment()` to not auto-create**

In `crates/curl-tui/src/app.rs`, find `cycle_environment()` (around line 1462). Change the empty-list behavior from auto-creating to showing a hint:

```rust
    pub fn cycle_environment(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        if ws.data.environments.is_empty() {
            self.status_message =
                Some("No environments. Press Ctrl+Shift+E to manage environments.".to_string());
            return;
        }
        // Cycle: None -> 0 -> 1 -> ... -> N-1 -> None -> 0 -> ...
        ws.data.active_environment = match ws.data.active_environment {
            None => Some(0),
            Some(i) if i + 1 < ws.data.environments.len() => Some(i + 1),
            Some(_) => None, // wrap back to "no environment"
        };
        match &ws.data.active_environment {
            Some(i) => {
                if let Some(env) = ws.data.environments.get(*i) {
                    self.status_message = Some(format!("Environment: {}", env.name));
                }
            }
            None => {
                self.status_message = Some("Environment: None".to_string());
            }
        }
        self.persist_active_environment();
    }
```

- [ ] **Step 7: Update variables overlay UI hints**

In `crates/curl-tui/src/ui/variables.rs`, find the Environment tier info line (around line 65-68). Change:

```rust
        // Before:
        format!(
            " Environment: {} [{}]  ([ ] switch  Ctrl+E: new  d: delete)",
            name, idx_info
        )
        // After:
        format!(
            " Environment: {} [{}]  (Ctrl+E: switch  Ctrl+Shift+E: manage)",
            name, idx_info
        )
```

- [ ] **Step 8: Update the empty-state message**

In `crates/curl-tui/src/ui/variables.rs`, find the empty-state message for environments (around line 105-106). Change:

```rust
        // Before:
        let msg = if no_env {
            " No environment selected. Press Ctrl+E to create one, or 'a' to add."
        // After:
        let msg = if no_env {
            " No environment selected. Press Ctrl+Shift+E to manage environments."
```

- [ ] **Step 9: Verify it compiles**

Run: `cargo check -p curl-tui`
Expected: Compiles

- [ ] **Step 10: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 11: Commit**

```bash
git add crates/curl-tui/src/main.rs crates/curl-tui/src/app.rs crates/curl-tui/src/ui/variables.rs
git commit -m "feat: remove env CRUD from variables overlay, fix cycle_environment"
```

---

### Task 6: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 2: Run fmt check**

Run: `cargo fmt --all --check`
Expected: No formatting issues

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings

- [ ] **Step 4: Fix any issues found in steps 1-3**

If any tests fail or lints fire, fix and re-run.

- [ ] **Step 5: Final commit (if any fixes were needed)**

```bash
git add -A
git commit -m "chore: fix lint/fmt issues from environment manager implementation"
```
