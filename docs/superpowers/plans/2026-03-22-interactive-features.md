# Interactive TUI Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make curl-tui actually usable — add text input, collection navigation, tab switching, and the input mode system needed to handle both keybindings and typing.

**Architecture:** Introduce an `InputMode` state machine that determines whether keypresses are routed to action keybindings or to text fields. When a text field is focused, raw characters go into the field's buffer. When in Normal mode, keys map to actions via the keymap. The UI panes are upgraded from static displays to interactive widgets that respond to cursor position, selection state, and text editing.

**Tech Stack:** Ratatui 0.29, crossterm 0.28, existing curl-tui-core types

**Spec:** `docs/superpowers/specs/2026-03-22-curl-tui-design.md`

---

## File Structure

### New files

| File | Responsibility |
|---|---|
| `crates/curl-tui/src/text_input.rs` | Reusable single-line text input widget: cursor position, insert/delete, render with cursor |
| `crates/curl-tui/src/ui/help.rs` | Full-screen help overlay listing all keybindings |

### Modified files

| File | Changes |
|---|---|
| `crates/curl-tui/src/app.rs` | Add `InputMode` enum, text input fields (url, header key/value, body), collection scroll state, focus tracking |
| `crates/curl-tui/src/main.rs` | Route raw character input to text fields when in Editing mode |
| `crates/curl-tui/src/input.rs` | Add new actions: `MoveUp`, `MoveDown`, `Enter`, `DeleteChar`, `NextTab`, `PrevTab`, `StartEditing`, `CharInput(char)` |
| `crates/curl-tui/src/ui/collections.rs` | Interactive tree with arrow-key navigation, highlight selected, Enter to load request |
| `crates/curl-tui/src/ui/request.rs` | Editable URL bar, tab switching, header/param key-value list with add/remove |
| `crates/curl-tui/src/ui/response.rs` | Scrollable response body, proper tab content for Headers/Timing |
| `crates/curl-tui/src/ui/statusbar.rs` | Show current input mode, status messages |
| `crates/curl-tui/src/ui/mod.rs` | Render help overlay when show_help is true |

---

## Task 1: TextInput Widget

**Files:**
- Create: `crates/curl-tui/src/text_input.rs`

- [ ] **Step 1: Create the TextInput struct**

Create `crates/curl-tui/src/text_input.rs`:
```rust
/// A single-line text input with cursor support.
#[derive(Debug, Clone, Default)]
pub struct TextInput {
    /// The current text content
    content: String,
    /// Cursor position (byte index)
    cursor: usize,
}

impl TextInput {
    pub fn new(initial: &str) -> Self {
        let len = initial.len();
        Self {
            content: initial.to_string(),
            cursor: len,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn set_content(&mut self, s: &str) {
        self.content = s.to_string();
        self.cursor = self.content.len();
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn delete_char_before(&mut self) {
        if self.cursor > 0 {
            // Find the previous char boundary
            let prev = self.content[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.content.remove(prev);
            self.cursor = prev;
        }
    }

    pub fn delete_char_after(&mut self) {
        if self.cursor < self.content.len() {
            self.content.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.content[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor += self.content[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.content.len();
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let input = TextInput::default();
        assert_eq!(input.content(), "");
        assert_eq!(input.cursor(), 0);
    }

    #[test]
    fn test_new_with_content() {
        let input = TextInput::new("hello");
        assert_eq!(input.content(), "hello");
        assert_eq!(input.cursor(), 5);
    }

    #[test]
    fn test_insert_char() {
        let mut input = TextInput::default();
        input.insert_char('h');
        input.insert_char('i');
        assert_eq!(input.content(), "hi");
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn test_insert_in_middle() {
        let mut input = TextInput::new("hllo");
        input.cursor = 1;
        input.insert_char('e');
        assert_eq!(input.content(), "hello");
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn test_delete_char_before() {
        let mut input = TextInput::new("hello");
        input.delete_char_before();
        assert_eq!(input.content(), "hell");
        assert_eq!(input.cursor(), 4);
    }

    #[test]
    fn test_delete_char_before_at_start() {
        let mut input = TextInput::new("hello");
        input.cursor = 0;
        input.delete_char_before();
        assert_eq!(input.content(), "hello");
    }

    #[test]
    fn test_delete_char_after() {
        let mut input = TextInput::new("hello");
        input.cursor = 0;
        input.delete_char_after();
        assert_eq!(input.content(), "ello");
    }

    #[test]
    fn test_move_left_right() {
        let mut input = TextInput::new("abc");
        assert_eq!(input.cursor(), 3);
        input.move_left();
        assert_eq!(input.cursor(), 2);
        input.move_left();
        assert_eq!(input.cursor(), 1);
        input.move_right();
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn test_move_home_end() {
        let mut input = TextInput::new("hello");
        input.move_home();
        assert_eq!(input.cursor(), 0);
        input.move_end();
        assert_eq!(input.cursor(), 5);
    }

    #[test]
    fn test_clear() {
        let mut input = TextInput::new("hello");
        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor(), 0);
    }

    #[test]
    fn test_set_content() {
        let mut input = TextInput::default();
        input.set_content("new value");
        assert_eq!(input.content(), "new value");
        assert_eq!(input.cursor(), 9);
    }
}
```

- [ ] **Step 2: Add module to main.rs**

Add `mod text_input;` to `crates/curl-tui/src/main.rs`.

- [ ] **Step 3: Run tests and commit**

Run: `cargo test -p curl-tui`
Expected: All TextInput tests pass.

```bash
git add crates/curl-tui/src/text_input.rs crates/curl-tui/src/main.rs
git commit -m "feat: add TextInput widget with cursor movement and editing"
```

---

## Task 2: Input Mode System & Extended Actions

**Files:**
- Modify: `crates/curl-tui/src/app.rs`
- Modify: `crates/curl-tui/src/input.rs`

- [ ] **Step 1: Add InputMode and editing state to App**

In `crates/curl-tui/src/app.rs`, add the `InputMode` enum and new fields:

```rust
// Add at the top, after existing enums:
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    /// Keybindings are active — keypresses map to actions
    Normal,
    /// Text editing — keypresses go to the focused text field
    Editing,
}

/// Identifies which text field is currently being edited
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditField {
    Url,
    HeaderKey(usize),
    HeaderValue(usize),
    ParamKey(usize),
    ParamValue(usize),
    BodyContent,
}
```

Add new fields to `App`:
```rust
pub struct App {
    // ... existing fields ...
    pub input_mode: InputMode,
    pub edit_field: Option<EditField>,
    pub url_input: crate::text_input::TextInput,
    pub body_input: crate::text_input::TextInput,
    pub header_key_inputs: Vec<crate::text_input::TextInput>,
    pub header_value_inputs: Vec<crate::text_input::TextInput>,
    pub param_key_inputs: Vec<crate::text_input::TextInput>,
    pub param_value_inputs: Vec<crate::text_input::TextInput>,
    pub collection_scroll: usize,
    pub response_scroll: usize,
}
```

In `App::new()`, initialize the new fields:
```rust
input_mode: InputMode::Normal,
edit_field: None,
url_input: crate::text_input::TextInput::new(""),
body_input: crate::text_input::TextInput::new(""),
header_key_inputs: Vec::new(),
header_value_inputs: Vec::new(),
param_key_inputs: Vec::new(),
param_value_inputs: Vec::new(),
collection_scroll: 0,
response_scroll: 0,
```

- [ ] **Step 2: Extend the Action enum**

In `crates/curl-tui/src/app.rs`, extend Action:
```rust
#[derive(Clone)]
pub enum Action {
    // Existing actions...
    Quit,
    Cancel,
    CyclePaneForward,
    CyclePaneBackward,
    SendRequest,
    SaveRequest,
    SwitchEnvironment,
    NewRequest,
    CopyCurl,
    ToggleCollections,
    ToggleRequest,
    ToggleResponse,
    RevealSecrets,
    Help,
    Search,
    // New actions for interactive features
    MoveUp,
    MoveDown,
    Enter,
    NextTab,
    PrevTab,
    DeleteItem,
    AddItem,
    CharInput(char),
    Backspace,
    Delete,
    CursorLeft,
    CursorRight,
    Home,
    End,
    None,
}
```

- [ ] **Step 3: Update input.rs to handle both modes**

In `crates/curl-tui/src/input.rs`, update `build_keymap` to include new normal-mode actions, and add a function for editing-mode key resolution:

Add these to the `action_map` vec in `build_keymap`:
```rust
// These are only active in Normal mode (via keymap)
// Arrow keys, Enter, etc. are handled contextually
```

Add a new public function:
```rust
/// Resolve a key event when in Normal mode but not bound in the keymap.
/// Handles arrow keys, Enter, tab switching, etc.
pub fn resolve_navigation(key: KeyEvent) -> Action {
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Up) => Action::MoveUp,
        (KeyModifiers::NONE, KeyCode::Down) => Action::MoveDown,
        (KeyModifiers::NONE, KeyCode::Enter) => Action::Enter,
        (KeyModifiers::NONE, KeyCode::Left) => Action::PrevTab,
        (KeyModifiers::NONE, KeyCode::Right) => Action::NextTab,
        (KeyModifiers::NONE, KeyCode::Char('a')) => Action::AddItem,
        (KeyModifiers::NONE, KeyCode::Char('d')) => Action::DeleteItem,
        (KeyModifiers::NONE, KeyCode::Char('j')) => Action::MoveDown,
        (KeyModifiers::NONE, KeyCode::Char('k')) => Action::MoveUp,
        _ => Action::None,
    }
}

/// Resolve a key event when in Editing mode.
/// Characters go to the text field, special keys are editing commands.
pub fn resolve_editing(key: KeyEvent) -> Action {
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Esc) => Action::Cancel, // Exit editing
        (KeyModifiers::NONE, KeyCode::Enter) => Action::Enter, // Confirm / move to next field
        (KeyModifiers::CONTROL, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::F(5)) => {
            Action::SendRequest
        }
        (KeyModifiers::CONTROL, KeyCode::Char('q')) => Action::Quit,
        (KeyModifiers::NONE, KeyCode::Backspace) => Action::Backspace,
        (KeyModifiers::NONE, KeyCode::Delete) => Action::Delete,
        (KeyModifiers::NONE, KeyCode::Left) => Action::CursorLeft,
        (KeyModifiers::NONE, KeyCode::Right) => Action::CursorRight,
        (KeyModifiers::NONE, KeyCode::Home) => Action::Home,
        (KeyModifiers::NONE, KeyCode::End) => Action::End,
        (KeyModifiers::NONE, KeyCode::Tab) => Action::CyclePaneForward,
        (KeyModifiers::NONE, KeyCode::Char(c)) => Action::CharInput(c),
        (KeyModifiers::SHIFT, KeyCode::Char(c)) => Action::CharInput(c),
        _ => Action::None,
    }
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build -p curl-tui`
Expected: Compiles (warnings about unused fields are OK at this stage).

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui/src/app.rs crates/curl-tui/src/input.rs
git commit -m "feat: add InputMode system with Normal/Editing modes and extended actions"
```

---

## Task 3: Event Loop Rewrite — Mode-Aware Input Dispatch

**Files:**
- Modify: `crates/curl-tui/src/main.rs`
- Modify: `crates/curl-tui/src/app.rs` — add helper methods for editing

- [ ] **Step 1: Add editing helper methods to App**

In `crates/curl-tui/src/app.rs`, add:
```rust
impl App {
    /// Enter editing mode for a specific field
    pub fn start_editing(&mut self, field: EditField) {
        self.input_mode = InputMode::Editing;
        self.edit_field = Some(field);
    }

    /// Exit editing mode, syncing text input back to request
    pub fn stop_editing(&mut self) {
        if let Some(field) = self.edit_field.take() {
            self.sync_field_to_request(field);
        }
        self.input_mode = InputMode::Normal;
    }

    /// Get a mutable reference to the active TextInput
    pub fn active_text_input(&mut self) -> Option<&mut crate::text_input::TextInput> {
        match self.edit_field? {
            EditField::Url => Some(&mut self.url_input),
            EditField::BodyContent => Some(&mut self.body_input),
            EditField::HeaderKey(i) => self.header_key_inputs.get_mut(i),
            EditField::HeaderValue(i) => self.header_value_inputs.get_mut(i),
            EditField::ParamKey(i) => self.param_key_inputs.get_mut(i),
            EditField::ParamValue(i) => self.param_value_inputs.get_mut(i),
        }
    }

    /// Sync the text input content back to the current request
    fn sync_field_to_request(&mut self, field: EditField) {
        let Some(request) = &mut self.current_request else {
            return;
        };
        match field {
            EditField::Url => {
                request.url = self.url_input.content().to_string();
            }
            EditField::BodyContent => {
                let content = self.body_input.content().to_string();
                if content.is_empty() {
                    request.body = Option::None;
                } else {
                    request.body = Some(curl_tui_core::types::Body::Json { content });
                }
            }
            EditField::HeaderKey(i) => {
                if let Some(header) = request.headers.get_mut(i) {
                    header.key = self.header_key_inputs[i].content().to_string();
                }
            }
            EditField::HeaderValue(i) => {
                if let Some(header) = request.headers.get_mut(i) {
                    header.value = self.header_value_inputs[i].content().to_string();
                }
            }
            EditField::ParamKey(i) => {
                if let Some(param) = request.params.get_mut(i) {
                    param.key = self.param_key_inputs[i].content().to_string();
                }
            }
            EditField::ParamValue(i) => {
                if let Some(param) = request.params.get_mut(i) {
                    param.value = self.param_value_inputs[i].content().to_string();
                }
            }
        }
    }

    /// Load a request's fields into the text inputs
    pub fn load_request_into_inputs(&mut self) {
        if let Some(request) = &self.current_request {
            self.url_input.set_content(&request.url);
            match &request.body {
                Some(curl_tui_core::types::Body::Json { content }) => {
                    self.body_input.set_content(content);
                }
                Some(curl_tui_core::types::Body::Text { content }) => {
                    self.body_input.set_content(content);
                }
                _ => {
                    self.body_input.clear();
                }
            }
            // Populate header inputs
            self.header_key_inputs = request
                .headers
                .iter()
                .map(|h| crate::text_input::TextInput::new(&h.key))
                .collect();
            self.header_value_inputs = request
                .headers
                .iter()
                .map(|h| crate::text_input::TextInput::new(&h.value))
                .collect();
            // Populate param inputs
            self.param_key_inputs = request
                .params
                .iter()
                .map(|p| crate::text_input::TextInput::new(&p.key))
                .collect();
            self.param_value_inputs = request
                .params
                .iter()
                .map(|p| crate::text_input::TextInput::new(&p.value))
                .collect();
        }
    }

    /// Add a new empty header to the current request
    pub fn add_header(&mut self) {
        if let Some(request) = &mut self.current_request {
            request.headers.push(curl_tui_core::types::Header {
                key: String::new(),
                value: String::new(),
                enabled: true,
            });
            self.header_key_inputs
                .push(crate::text_input::TextInput::default());
            self.header_value_inputs
                .push(crate::text_input::TextInput::default());
            let idx = request.headers.len() - 1;
            self.start_editing(EditField::HeaderKey(idx));
        }
    }

    /// Add a new empty param to the current request
    pub fn add_param(&mut self) {
        if let Some(request) = &mut self.current_request {
            request.params.push(curl_tui_core::types::Param {
                key: String::new(),
                value: String::new(),
                enabled: true,
            });
            self.param_key_inputs
                .push(crate::text_input::TextInput::default());
            self.param_value_inputs
                .push(crate::text_input::TextInput::default());
            let idx = request.params.len() - 1;
            self.start_editing(EditField::ParamKey(idx));
        }
    }

    /// Handle Enter in Normal mode based on active pane
    pub fn handle_enter(&mut self) {
        match self.active_pane {
            Pane::Collections => {
                // Load the selected request
                if let Some(col_idx) = self.selected_collection {
                    if let Some(collection) = self.collections.get(col_idx) {
                        if let Some(req_idx) = self.selected_request {
                            if let Some(req) = collection.requests.get(req_idx) {
                                self.current_request = Some(req.clone());
                                self.load_request_into_inputs();
                                self.active_pane = Pane::Request;
                                self.status_message =
                                    Some(format!("Loaded: {}", req.name));
                            }
                        }
                    }
                }
            }
            Pane::Request => {
                // Start editing the URL field
                self.start_editing(EditField::Url);
            }
            _ => {}
        }
    }

    /// Handle MoveUp in Normal mode based on active pane
    pub fn handle_move_up(&mut self) {
        match self.active_pane {
            Pane::Collections => {
                self.move_collection_cursor_up();
                self.adjust_collection_scroll(20); // approximate; UI will clamp
            }
            Pane::Request => {
                // Could move between header rows in future
            }
            Pane::Response => {
                if self.response_scroll > 0 {
                    self.response_scroll -= 1;
                }
            }
        }
    }

    /// Handle MoveDown in Normal mode based on active pane
    pub fn handle_move_down(&mut self) {
        match self.active_pane {
            Pane::Collections => {
                self.move_collection_cursor_down();
                self.adjust_collection_scroll(20); // approximate; UI will clamp
            }
            Pane::Request => {
                // Could move between header rows in future
            }
            Pane::Response => {
                self.response_scroll = self.response_scroll.saturating_add(1);
            }
        }
    }

    /// Calculate the flat index of the current collection cursor position
    fn collection_cursor_flat_index(&self) -> usize {
        let mut idx = 0;
        for (col_idx, col) in self.collections.iter().enumerate() {
            if Some(col_idx) == self.selected_collection && self.selected_request.is_none() {
                return idx;
            }
            idx += 1; // collection row
            for (req_idx, _) in col.requests.iter().enumerate() {
                if Some(col_idx) == self.selected_collection
                    && Some(req_idx) == self.selected_request
                {
                    return idx;
                }
                idx += 1;
            }
        }
        idx
    }

    /// Adjust collection_scroll to keep the cursor visible
    fn adjust_collection_scroll(&mut self, visible_height: usize) {
        let cursor = self.collection_cursor_flat_index();
        if cursor < self.collection_scroll {
            self.collection_scroll = cursor;
        } else if cursor >= self.collection_scroll + visible_height {
            self.collection_scroll = cursor - visible_height + 1;
        }
    }

    /// Move collection cursor up through the flat list of collections and their requests
    fn move_collection_cursor_up(&mut self) {
        if let Some(req_idx) = self.selected_request {
            if req_idx > 0 {
                self.selected_request = Some(req_idx - 1);
            } else {
                // Move back to collection level
                self.selected_request = Option::None;
            }
        } else if let Some(col_idx) = self.selected_collection {
            if col_idx > 0 {
                self.selected_collection = Some(col_idx - 1);
                // Select last request of previous collection
                if let Some(col) = self.collections.get(col_idx - 1) {
                    if !col.requests.is_empty() {
                        self.selected_request = Some(col.requests.len() - 1);
                    }
                }
            }
        } else if !self.collections.is_empty() {
            self.selected_collection = Some(0);
        }
    }

    /// Move collection cursor down through the flat list
    fn move_collection_cursor_down(&mut self) {
        if let Some(col_idx) = self.selected_collection {
            if let Some(collection) = self.collections.get(col_idx) {
                if let Some(req_idx) = self.selected_request {
                    if req_idx + 1 < collection.requests.len() {
                        self.selected_request = Some(req_idx + 1);
                    } else if col_idx + 1 < self.collections.len() {
                        // Move to next collection
                        self.selected_collection = Some(col_idx + 1);
                        self.selected_request = Option::None;
                    }
                } else if !collection.requests.is_empty() {
                    self.selected_request = Some(0);
                } else if col_idx + 1 < self.collections.len() {
                    self.selected_collection = Some(col_idx + 1);
                }
            }
        } else if !self.collections.is_empty() {
            self.selected_collection = Some(0);
        }
    }

    /// Switch to next request tab
    pub fn next_request_tab(&mut self) {
        self.request_tab = match self.request_tab {
            RequestTab::Headers => RequestTab::Body,
            RequestTab::Body => RequestTab::Auth,
            RequestTab::Auth => RequestTab::Params,
            RequestTab::Params => RequestTab::Headers,
        };
    }

    /// Switch to previous request tab
    pub fn prev_request_tab(&mut self) {
        self.request_tab = match self.request_tab {
            RequestTab::Headers => RequestTab::Params,
            RequestTab::Body => RequestTab::Headers,
            RequestTab::Auth => RequestTab::Body,
            RequestTab::Params => RequestTab::Auth,
        };
    }

    /// Switch to next response tab
    pub fn next_response_tab(&mut self) {
        self.response_tab = match self.response_tab {
            ResponseTab::Body => ResponseTab::Headers,
            ResponseTab::Headers => ResponseTab::Timing,
            ResponseTab::Timing => ResponseTab::Body,
        };
    }

    /// Switch to previous response tab
    pub fn prev_response_tab(&mut self) {
        self.response_tab = match self.response_tab {
            ResponseTab::Body => ResponseTab::Timing,
            ResponseTab::Headers => ResponseTab::Body,
            ResponseTab::Timing => ResponseTab::Headers,
        };
    }
}
```

- [ ] **Step 2: Rewrite the event loop in main.rs**

Replace `run_loop` in `crates/curl-tui/src/main.rs`:
```rust
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
                        let cmd =
                            CurlCommandBuilder::new(&req.url).method(req.method).build();
                        let display = cmd.to_display_string(&[]);
                        app.status_message = Some(format!("Copied: {}", display));
                    }
                }
                // Navigation actions (Normal mode)
                Action::MoveUp => app.handle_move_up(),
                Action::MoveDown => app.handle_move_down(),
                Action::Enter => {
                    if app.input_mode == app::InputMode::Editing {
                        // Confirm current field, move to next or exit editing
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
                Action::AddItem => match app.active_pane {
                    app::Pane::Request => match app.request_tab {
                        app::RequestTab::Headers => app.add_header(),
                        app::RequestTab::Params => app.add_param(),
                        _ => {}
                    },
                    _ => {}
                },
                Action::DeleteItem => {
                    // TODO: implement delete for headers/params
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
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
```

- [ ] **Step 3: Call load_request_into_inputs on startup**

In `main.rs`, after creating the app and loading data, add:
```rust
app.load_request_into_inputs();
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build -p curl-tui`

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui/src/main.rs crates/curl-tui/src/app.rs
git commit -m "feat: add mode-aware input dispatch with Normal/Editing routing"
```

---

## Task 4: Interactive Request Pane — URL Bar & Tab Switching

**Files:**
- Modify: `crates/curl-tui/src/ui/request.rs`

- [ ] **Step 1: Rewrite request.rs with an editable URL bar and proper tab content**

Replace `crates/curl-tui/src/ui/request.rs`:
```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;

use crate::app::{App, EditField, InputMode, Pane, RequestTab};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_pane == Pane::Request;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(" Request ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 {
        return;
    }

    // Split inner area: method+url (2 lines) | tabs (1 line) | content (rest)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Method + URL
            Constraint::Length(1), // Tabs
            Constraint::Min(1),   // Tab content
        ])
        .split(inner);

    // Method + URL bar
    if let Some(req) = &app.current_request {
        let url_editing = app.input_mode == InputMode::Editing
            && app.edit_field == Some(EditField::Url);

        let method_span = Span::styled(
            format!(" {} ", req.method),
            Style::default()
                .fg(method_color(req.method))
                .add_modifier(Modifier::BOLD),
        );

        let url_text = if url_editing {
            app.url_input.content()
        } else {
            &req.url
        };

        let url_style = if url_editing {
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
        } else if is_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        let url_display = if url_text.is_empty() && !url_editing {
            "Enter URL...".to_string()
        } else {
            url_text.to_string()
        };

        let url_span = Span::styled(url_display, url_style);
        let line = Line::from(vec![method_span, Span::raw(" "), url_span]);
        frame.render_widget(Paragraph::new(line), chunks[0]);

        // Show cursor when editing URL
        if url_editing {
            let cursor_x = chunks[0].x + req.method.to_string().len() as u16 + 3 + app.url_input.cursor() as u16;
            let cursor_y = chunks[0].y;
            if cursor_x < chunks[0].x + chunks[0].width {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    } else {
        frame.render_widget(
            Paragraph::new("No request selected").style(Style::default().fg(Color::DarkGray)),
            chunks[0],
        );
    }

    // Tabs
    let tab_titles = vec!["Headers", "Body", "Auth", "Params"];
    let selected_tab = match app.request_tab {
        RequestTab::Headers => 0,
        RequestTab::Body => 1,
        RequestTab::Auth => 2,
        RequestTab::Params => 3,
    };
    let tabs = Tabs::new(tab_titles)
        .select(selected_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|");
    frame.render_widget(tabs, chunks[1]);

    // Tab content
    match app.request_tab {
        RequestTab::Headers => draw_headers(frame, app, chunks[2]),
        RequestTab::Body => draw_body(frame, app, chunks[2]),
        RequestTab::Auth => draw_auth(frame, app, chunks[2]),
        RequestTab::Params => draw_params(frame, app, chunks[2]),
    }
}

fn draw_headers(frame: &mut Frame, app: &App, area: Rect) {
    let Some(req) = &app.current_request else {
        return;
    };

    if req.headers.is_empty() {
        let text = Paragraph::new(" No headers. Press 'a' to add one.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, area);
        return;
    }

    let mut lines = Vec::new();
    for (i, header) in req.headers.iter().enumerate() {
        let enabled = if header.enabled { " " } else { "x" };
        let style = if !header.enabled {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("[{}] ", enabled), Style::default().fg(Color::DarkGray)),
            Span::styled(&header.key, Style::default().fg(Color::Yellow)),
            Span::styled(": ", Style::default().fg(Color::DarkGray)),
            Span::styled(&header.value, style),
        ]));
        let _ = i; // used for future selection highlighting
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_body(frame: &mut Frame, app: &App, area: Rect) {
    let body_editing = app.input_mode == InputMode::Editing
        && app.edit_field == Some(EditField::BodyContent);

    let content = if body_editing {
        app.body_input.content().to_string()
    } else if let Some(req) = &app.current_request {
        match &req.body {
            Some(curl_tui_core::types::Body::Json { content }) => content.clone(),
            Some(curl_tui_core::types::Body::Text { content }) => content.clone(),
            _ => String::new(),
        }
    } else {
        String::new()
    };

    let style = if body_editing {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let display = if content.is_empty() && !body_editing {
        " Press Enter to edit body...".to_string()
    } else {
        format!(" {}", content)
    };

    frame.render_widget(Paragraph::new(display).style(style), area);

    if body_editing {
        let cursor_x = area.x + 1 + app.body_input.cursor() as u16;
        let cursor_y = area.y;
        if cursor_x < area.x + area.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

fn draw_auth(frame: &mut Frame, app: &App, area: Rect) {
    let Some(req) = &app.current_request else {
        return;
    };

    let text = match &req.auth {
        Some(curl_tui_core::types::Auth::Bearer { token }) => {
            format!(" Type: Bearer\n Token: {}", token)
        }
        Some(curl_tui_core::types::Auth::Basic { username, password }) => {
            format!(" Type: Basic\n Username: {}\n Password: {}", username, password)
        }
        Some(curl_tui_core::types::Auth::ApiKey {
            key,
            value,
            location,
        }) => format!(" Type: API Key\n Key: {}\n Value: {}\n In: {:?}", key, value, location),
        Some(curl_tui_core::types::Auth::None) | None => " No authentication configured.".to_string(),
    };

    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::White)),
        area,
    );
}

fn draw_params(frame: &mut Frame, app: &App, area: Rect) {
    let Some(req) = &app.current_request else {
        return;
    };

    if req.params.is_empty() {
        let text = Paragraph::new(" No query parameters. Press 'a' to add one.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, area);
        return;
    }

    let mut lines = Vec::new();
    for param in &req.params {
        let enabled = if param.enabled { " " } else { "x" };
        let style = if !param.enabled {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("[{}] ", enabled), Style::default().fg(Color::DarkGray)),
            Span::styled(&param.key, Style::default().fg(Color::Yellow)),
            Span::styled("=", Style::default().fg(Color::DarkGray)),
            Span::styled(&param.value, style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn method_color(method: curl_tui_core::types::Method) -> Color {
    match method {
        curl_tui_core::types::Method::Get => Color::Green,
        curl_tui_core::types::Method::Post => Color::Yellow,
        curl_tui_core::types::Method::Put => Color::Blue,
        curl_tui_core::types::Method::Delete => Color::Red,
        curl_tui_core::types::Method::Patch => Color::Magenta,
        curl_tui_core::types::Method::Head => Color::Cyan,
        curl_tui_core::types::Method::Options => Color::Gray,
    }
}
```

- [ ] **Step 2: Verify it compiles and renders**

Run: `cargo build -p curl-tui`

- [ ] **Step 3: Commit**

```bash
git add crates/curl-tui/src/ui/request.rs
git commit -m "feat: interactive request pane with editable URL bar and tab content"
```

---

## Task 5: Interactive Collections Pane

**Files:**
- Modify: `crates/curl-tui/src/ui/collections.rs`

- [ ] **Step 1: Rewrite collections.rs with navigation highlighting**

Replace `crates/curl-tui/src/ui/collections.rs`:
```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Pane};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_pane == Pane::Collections;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(" Collections ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.collections.is_empty() {
        let text = Paragraph::new(" No collections.\n Press Ctrl+N to create a request.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, inner);
        return;
    }

    let mut lines = Vec::new();
    for (col_idx, collection) in app.collections.iter().enumerate() {
        let is_selected_col = app.selected_collection == Some(col_idx)
            && app.selected_request.is_none();

        let style = if is_selected_col && is_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        };

        lines.push(Line::from(Span::styled(
            format!(" {} {}", if collection.requests.is_empty() { " " } else { ">" }, collection.name),
            style,
        )));

        // Show requests under the collection
        for (req_idx, req) in collection.requests.iter().enumerate() {
            let is_selected_req = app.selected_collection == Some(col_idx)
                && app.selected_request == Some(req_idx);

            let method_style = Style::default().fg(method_color(req.method));
            let name_style = if is_selected_req && is_focused {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };

            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(format!("{:7}", req.method), method_style),
                Span::styled(&req.name, name_style),
            ]));
        }
    }

    // Handle scrolling
    let visible_height = inner.height as usize;
    let start = app.collection_scroll;
    let end = (start + visible_height).min(lines.len());
    let visible_lines: Vec<Line> = lines[start..end].to_vec();

    frame.render_widget(Paragraph::new(visible_lines), inner);
}

fn method_color(method: curl_tui_core::types::Method) -> Color {
    match method {
        curl_tui_core::types::Method::Get => Color::Green,
        curl_tui_core::types::Method::Post => Color::Yellow,
        curl_tui_core::types::Method::Put => Color::Blue,
        curl_tui_core::types::Method::Delete => Color::Red,
        curl_tui_core::types::Method::Patch => Color::Magenta,
        curl_tui_core::types::Method::Head => Color::Cyan,
        curl_tui_core::types::Method::Options => Color::Gray,
    }
}
```

- [ ] **Step 2: Verify and commit**

Run: `cargo build -p curl-tui`

```bash
git add crates/curl-tui/src/ui/collections.rs
git commit -m "feat: interactive collections pane with arrow-key navigation and highlighting"
```

---

## Task 6: Improved Response Pane & Scrolling

**Files:**
- Modify: `crates/curl-tui/src/ui/response.rs`

- [ ] **Step 1: Rewrite response.rs with scrolling and proper tab content**

Replace `crates/curl-tui/src/ui/response.rs`:
```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};
use ratatui::Frame;

use crate::app::{App, Pane, ResponseTab};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_pane == Pane::Response;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(" Response ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 {
        return;
    }

    // Split: status line | tabs | content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    // Status line
    if let Some(resp) = &app.last_response {
        let status_color = match resp.status_code {
            200..=299 => Color::Green,
            300..=399 => Color::Yellow,
            400..=499 => Color::Red,
            500..=599 => Color::Magenta,
            _ => Color::White,
        };
        let status_line = Line::from(vec![
            Span::styled(
                format!(" {} ", resp.status_code),
                Style::default()
                    .fg(Color::Black)
                    .bg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                status_text(resp.status_code),
                Style::default().fg(status_color),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{:.0}ms", resp.timing.total_ms),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(status_line), chunks[0]);
    } else {
        frame.render_widget(
            Paragraph::new(" No response yet").style(Style::default().fg(Color::DarkGray)),
            chunks[0],
        );
    }

    // Tabs
    let tab_titles = vec!["Body", "Headers", "Timing"];
    let selected_tab = match app.response_tab {
        ResponseTab::Body => 0,
        ResponseTab::Headers => 1,
        ResponseTab::Timing => 2,
    };
    let tabs = Tabs::new(tab_titles)
        .select(selected_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|");
    frame.render_widget(tabs, chunks[1]);

    // Tab content
    if let Some(resp) = &app.last_response {
        match app.response_tab {
            ResponseTab::Body => {
                let body_text = &resp.body;
                let paragraph = Paragraph::new(body_text.as_str())
                    .style(Style::default().fg(Color::White))
                    .wrap(Wrap { trim: false })
                    .scroll((app.response_scroll as u16, 0));
                frame.render_widget(paragraph, chunks[2]);
            }
            ResponseTab::Headers => {
                let mut lines = Vec::new();
                for (key, value) in &resp.headers {
                    lines.push(Line::from(vec![
                        Span::styled(key, Style::default().fg(Color::Yellow)),
                        Span::styled(": ", Style::default().fg(Color::DarkGray)),
                        Span::styled(value, Style::default().fg(Color::White)),
                    ]));
                }
                frame.render_widget(Paragraph::new(lines), chunks[2]);
            }
            ResponseTab::Timing => {
                let timing = &resp.timing;
                let lines = vec![
                    Line::from(format!(" DNS Lookup:   {:.1}ms", timing.dns_lookup_ms)),
                    Line::from(format!(" TCP Connect:  {:.1}ms", timing.tcp_connect_ms)),
                    Line::from(format!(" TLS Handshake:{:.1}ms", timing.tls_handshake_ms)),
                    Line::from(format!(" First Byte:   {:.1}ms", timing.transfer_start_ms)),
                    Line::from(format!(" Total:        {:.1}ms", timing.total_ms)),
                ];
                frame.render_widget(
                    Paragraph::new(lines).style(Style::default().fg(Color::White)),
                    chunks[2],
                );
            }
        }
    } else {
        frame.render_widget(
            Paragraph::new(" Send a request to see the response.")
                .style(Style::default().fg(Color::DarkGray)),
            chunks[2],
        );
    }
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "",
    }
}
```

- [ ] **Step 2: Verify and commit**

Run: `cargo build -p curl-tui`

```bash
git add crates/curl-tui/src/ui/response.rs
git commit -m "feat: improved response pane with scrolling, colored status, and timing tab"
```

---

## Task 7: Help Overlay & Updated Status Bar

**Files:**
- Create: `crates/curl-tui/src/ui/help.rs`
- Modify: `crates/curl-tui/src/ui/mod.rs`
- Modify: `crates/curl-tui/src/ui/statusbar.rs`

- [ ] **Step 1: Create help overlay**

Create `crates/curl-tui/src/ui/help.rs`:
```rust
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw(frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings — Press Esc to close ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        header("Navigation"),
        binding("Tab / Shift+Tab", "Cycle panes"),
        binding("Arrow Up/Down", "Navigate items"),
        binding("Arrow Left/Right", "Switch tabs"),
        binding("Enter", "Select / Edit field"),
        binding("Esc", "Cancel / Exit edit mode"),
        Line::raw(""),
        header("Actions"),
        binding("Ctrl+Enter / F5", "Send request"),
        binding("Ctrl+S", "Save request"),
        binding("Ctrl+N", "New request"),
        binding("Ctrl+E", "Switch environment"),
        binding("Ctrl+Y", "Copy as curl"),
        Line::raw(""),
        header("View"),
        binding("Ctrl+1/2/3", "Toggle panes"),
        binding("F8", "Reveal secrets"),
        binding("?", "Toggle this help"),
        binding("Ctrl+Q", "Quit"),
        Line::raw(""),
        header("Editing"),
        binding("Type", "Insert characters"),
        binding("Backspace/Delete", "Remove characters"),
        binding("Home/End", "Jump to start/end"),
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
        Span::styled(
            format!("  {:22}", key),
            Style::default().fg(Color::Yellow),
        ),
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
```

- [ ] **Step 2: Update statusbar to show input mode**

Replace `crates/curl-tui/src/ui/statusbar.rs`:
```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, InputMode};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.input_mode {
        InputMode::Normal => Span::styled(
            " NORMAL ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::Editing => Span::styled(
            " EDITING ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    };

    let status_msg = if let Some(msg) = &app.status_message {
        Span::styled(format!(" {} ", msg), Style::default().fg(Color::White))
    } else {
        Span::raw("")
    };

    let hints = match app.input_mode {
        InputMode::Normal => Span::styled(
            " Tab:pane  Enter:edit  ?:help  Ctrl+Q:quit ",
            Style::default().fg(Color::DarkGray),
        ),
        InputMode::Editing => Span::styled(
            " Esc:done  Ctrl+Enter:send  Tab:next pane ",
            Style::default().fg(Color::DarkGray),
        ),
    };

    let line = Line::from(vec![mode_indicator, Span::raw(" "), status_msg, hints]);

    let status = Paragraph::new(line).style(Style::default().bg(Color::Black));
    frame.render_widget(status, area);
}
```

- [ ] **Step 3: Update ui/mod.rs to render help overlay and register help module**

Replace `crates/curl-tui/src/ui/mod.rs`:
```rust
pub mod collections;
pub mod help;
pub mod layout;
pub mod request;
pub mod response;
pub mod statusbar;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let pane_layout = layout::compute_layout(frame.area(), app.pane_visible);

    // Title bar
    let env_name = app
        .active_environment
        .and_then(|i| app.environments.get(i))
        .map(|e| e.name.as_str())
        .unwrap_or("None");

    let title = Line::from(vec![
        Span::styled(" curl-tui", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(
            format!("[env: {}]", env_name),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  "),
        Span::styled("[v0.1.0]", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().bg(Color::Black)),
        pane_layout.title_bar,
    );

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

    // Help overlay (on top of everything)
    if app.show_help {
        help::draw(frame);
    }
}
```

- [ ] **Step 4: Verify and commit**

Run: `cargo build -p curl-tui`

```bash
git add crates/curl-tui/src/ui/
git commit -m "feat: add help overlay, mode-aware status bar, and improved pane focus indicators"
```

---

## Task 8: Final Cleanup & Verification

- [ ] **Step 1: Run full verification**

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

Fix any issues.

- [ ] **Step 2: Manual test**

Run: `cargo run -p curl-tui`

Verify:
- Tab cycles between panes (border highlights in cyan)
- Arrow Up/Down navigates collections list (if any exist)
- Enter on Request pane enters URL editing mode (cursor visible, status bar shows "EDITING")
- Type a URL (e.g., `https://httpbin.org/get`)
- Esc exits editing mode
- F5 or Ctrl+Enter sends the request
- Response appears in Response pane with colored status code
- Arrow Left/Right switches tabs in Request/Response panes
- ? shows help overlay, Esc closes it
- Ctrl+Q quits cleanly

- [ ] **Step 3: Commit any final fixes**

```bash
git add -A
git commit -m "fix: final cleanup for interactive features"
```

---

## Summary

| Task | What | Key Changes |
|---|---|---|
| 1 | TextInput widget | Reusable single-line input with cursor, 11 tests |
| 2 | Input mode system | Normal/Editing modes, extended Action enum |
| 3 | Event loop rewrite | Mode-aware dispatch, editing helpers |
| 4 | Request pane | Editable URL bar, tab content, method colors |
| 5 | Collections pane | Arrow navigation, selection highlighting |
| 6 | Response pane | Scrolling, status colors, timing tab |
| 7 | Help & status bar | Keybinding overlay, mode indicator |
| 8 | Verification | fmt + clippy + test + manual |
