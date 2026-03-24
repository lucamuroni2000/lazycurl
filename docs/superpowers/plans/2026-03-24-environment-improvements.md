# Environment Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix environment persistence across project switches and add a dedicated Ctrl+Shift+E keybinding for creating environments.

**Architecture:** Two changes: (1) a `save_project()` function in `curl-tui-core` plus a `sync_active_environment_name()` method on `ProjectWorkspaceData` that keeps `project.active_environment` (the persisted name string) in sync with the in-memory index whenever the active environment changes, and (2) a new `CreateEnvironment` action wired to Ctrl+Shift+E that creates an environment and opens the variables overlay.

**Tech Stack:** Rust, serde_json, tempfile (tests)

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/curl-tui-core/src/project.rs` | Modify | Add `save_project()` function |
| `crates/curl-tui-core/src/types.rs` | Modify | Add `sync_active_environment_name()` method on `ProjectWorkspaceData` |
| `crates/curl-tui-core/tests/integration_test.rs` | Modify | Add environment sync round-trip integration test |
| `crates/curl-tui/src/app.rs` | Modify | Add `CreateEnvironment` action variant, wire sync calls into `cycle_environment()`, `create_new_environment()`, `delete_active_environment()` |
| `crates/curl-tui-core/src/config.rs` | Modify | Add `create_env` default keybinding |
| `crates/curl-tui/src/input.rs` | Modify | Map `create_env` config key to `Action::CreateEnvironment` |
| `crates/curl-tui/src/main.rs` | Modify | Handle `Action::CreateEnvironment` in normal dispatch and `handle_variables_action` |
| `crates/curl-tui/src/ui/help.rs` | Modify | Add Ctrl+Shift+E to help screen |
| `crates/curl-tui/src/ui/statusbar.rs` | Modify | Add Ctrl+Shift+E hint |

---

### Task 1: Add `save_project()` to core

**Files:**
- Modify: `crates/curl-tui-core/src/project.rs:164-286` (tests section)

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block at the bottom of `project.rs`:

```rust
#[test]
fn test_save_project_persists_active_environment() {
    let tmp = tempfile::tempdir().unwrap();
    let projects_dir = tmp.path().join("projects");

    let mut project = make_project("Save Test");
    let dir = create_project(&projects_dir, &project).unwrap();

    // Set active_environment and save
    project.active_environment = Some("Production".to_string());
    save_project(&dir, &project).unwrap();

    // Reload and verify
    let loaded = load_project(&dir).unwrap();
    assert_eq!(loaded.active_environment, Some("Production".to_string()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p curl-tui-core -- project::tests::test_save_project_persists_active_environment`
Expected: FAIL — `save_project` does not exist

- [ ] **Step 3: Write minimal implementation**

Add this function in `crates/curl-tui-core/src/project.rs` after `load_project()` (after line 81):

```rust
/// Persist a `Project` struct back to its `project.json` in the given project directory.
pub fn save_project(project_dir: &Path, project: &Project) -> Result<(), Box<dyn std::error::Error>> {
    let content = serde_json::to_string_pretty(project)?;
    std::fs::write(project_dir.join("project.json"), content)?;
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p curl-tui-core -- project::tests::test_save_project_persists_active_environment`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui-core/src/project.rs
git commit -m "feat: add save_project() to persist project.json changes"
```

---

### Task 2: Add `sync_active_environment_name()` to `ProjectWorkspaceData`

**Files:**
- Modify: `crates/curl-tui-core/src/types.rs:205-230` (impl block for `ProjectWorkspaceData`)

- [ ] **Step 1: Write the failing tests**

`types.rs` already has a `#[cfg(test)] mod tests` block starting at line 236. Append the following helper and tests inside that existing block (before the closing `}`), after the last test (`test_history_entry_backward_compat` at line 453):

```rust
    fn make_workspace_with_envs() -> ProjectWorkspaceData {
        let project = Project {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            active_environment: None,
        };
        let mut ws = ProjectWorkspaceData::new(project, "test".to_string());
        ws.environments = vec![
            Environment {
                id: uuid::Uuid::new_v4(),
                name: "Development".to_string(),
                variables: HashMap::new(),
            },
            Environment {
                id: uuid::Uuid::new_v4(),
                name: "Production".to_string(),
                variables: HashMap::new(),
            },
        ];
        ws
    }

    #[test]
    fn test_sync_sets_name_from_index() {
        let mut ws = make_workspace_with_envs();
        ws.active_environment = Some(1);
        ws.sync_active_environment_name();
        assert_eq!(ws.project.active_environment, Some("Production".to_string()));
    }

    #[test]
    fn test_sync_clears_name_when_none() {
        let mut ws = make_workspace_with_envs();
        ws.active_environment = None;
        ws.sync_active_environment_name();
        assert_eq!(ws.project.active_environment, None);
    }

    #[test]
    fn test_sync_clears_name_when_index_out_of_bounds() {
        let mut ws = make_workspace_with_envs();
        ws.active_environment = Some(99);
        ws.sync_active_environment_name();
        assert_eq!(ws.project.active_environment, None);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core -- types::tests`
Expected: FAIL — `sync_active_environment_name` does not exist

- [ ] **Step 3: Write minimal implementation**

Add this method to the `impl ProjectWorkspaceData` block in `types.rs` (after the `new()` method, around line 229):

```rust
/// Sync the `project.active_environment` name field from the current index.
/// Call this after any mutation of `self.active_environment`.
pub fn sync_active_environment_name(&mut self) {
    self.project.active_environment = self
        .active_environment
        .and_then(|i| self.environments.get(i))
        .map(|env| env.name.clone());
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p curl-tui-core -- types::tests`
Expected: PASS (all 3 tests)

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui-core/src/types.rs
git commit -m "feat: add sync_active_environment_name() to ProjectWorkspaceData"
```

---

### Task 3: Wire sync + persist into environment mutation methods

**Files:**
- Modify: `crates/curl-tui/src/app.rs:1451-1554` (the three environment methods)

The three methods that mutate `active_environment` are `cycle_environment()` (line 1451), `create_new_environment()` (line 1479), and `delete_active_environment()` (line 1513). Each needs to call `sync_active_environment_name()` and then `save_project()` after changing the index.

- [ ] **Step 1: Add a `persist_active_environment()` helper to `App`**

Add this private method to `App` in `app.rs` (near the environment methods, e.g., before `cycle_environment()`):

```rust
/// Sync the active environment name to project.json after any environment index change.
fn persist_active_environment(&mut self) {
    let Some(ws) = self.active_workspace_mut() else {
        return;
    };
    ws.data.sync_active_environment_name();
    let project_dir = config_dir()
        .join("projects")
        .join(&ws.data.slug);
    let _ = curl_tui_core::project::save_project(&project_dir, &ws.data.project);
}
```

This is a fire-and-forget helper — errors are silently ignored (consistent with how other disk writes in `app.rs` are handled, e.g., `delete_active_environment` ignores `remove_file` errors).

- [ ] **Step 2: Add sync call to `cycle_environment()`**

At `app.rs:1476` (just before the closing `}` of `cycle_environment()`), add:

```rust
        self.persist_active_environment();
```

The full method becomes:

```rust
pub fn cycle_environment(&mut self) {
    let Some(ws) = self.active_workspace_mut() else {
        return;
    };
    if ws.data.environments.is_empty() {
        let _ = ws;
        self.create_new_environment();
        return;
    }
    ws.data.active_environment = match ws.data.active_environment {
        None => Some(0),
        Some(i) if i + 1 < ws.data.environments.len() => Some(i + 1),
        Some(_) => None,
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

- [ ] **Step 3: Add sync call to `create_new_environment()`**

At `app.rs:1505` (inside the `Ok` branch, after `self.status_message = ...`), add:

```rust
                self.persist_active_environment();
```

- [ ] **Step 4: Add sync call to `delete_active_environment()`**

At `app.rs:1553` (just before the closing `self.status_message = ...` line, or right after it), add:

```rust
        self.persist_active_environment();
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build -p curl-tui`
Expected: Compiles without errors

- [ ] **Step 6: Commit**

```bash
git add crates/curl-tui/src/app.rs
git commit -m "feat: sync active environment name to project.json on every change"
```

---

### Task 4: Integration test for environment sync round-trip

**Files:**
- Modify: `crates/curl-tui-core/tests/integration_test.rs`

- [ ] **Step 1: Write the integration test**

Add at the bottom of `integration_test.rs`:

```rust
#[test]
fn test_environment_sync_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let projects_dir = tmp.path().join("projects");

    // Create project with two environments
    let project = curl_tui_core::types::Project {
        id: uuid::Uuid::new_v4(),
        name: "Env Sync Test".to_string(),
        active_environment: None,
    };
    let dir = curl_tui_core::project::create_project(&projects_dir, &project).unwrap();

    let env1 = curl_tui_core::types::Environment {
        id: uuid::Uuid::new_v4(),
        name: "Development".to_string(),
        variables: std::collections::HashMap::new(),
    };
    let env2 = curl_tui_core::types::Environment {
        id: uuid::Uuid::new_v4(),
        name: "Production".to_string(),
        variables: std::collections::HashMap::new(),
    };
    curl_tui_core::environment::save_environment(&dir.join("environments"), &env1).unwrap();
    curl_tui_core::environment::save_environment(&dir.join("environments"), &env2).unwrap();

    // Build workspace, select Production (index 1), sync
    let mut ws = curl_tui_core::types::ProjectWorkspaceData::new(project, "env-sync-test".to_string());
    ws.environments = vec![env1, env2];
    ws.active_environment = Some(1);
    ws.sync_active_environment_name();

    assert_eq!(ws.project.active_environment, Some("Production".to_string()));

    // Persist and reload
    curl_tui_core::project::save_project(&dir, &ws.project).unwrap();
    let reloaded = curl_tui_core::project::load_project(&dir).unwrap();
    assert_eq!(reloaded.active_environment, Some("Production".to_string()));

    // Simulate restoring index from name (what switch_project does at load time)
    let envs = curl_tui_core::environment::list_environments(&dir.join("environments")).unwrap();
    let restored_idx = reloaded.active_environment.as_ref().and_then(|name| {
        envs.iter().position(|e| &e.name == name)
    });
    assert!(restored_idx.is_some());
    assert_eq!(envs[restored_idx.unwrap()].name, "Production");
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p curl-tui-core -- test_environment_sync_round_trip`
Expected: PASS (all building blocks were implemented in Tasks 1-2)

- [ ] **Step 3: Commit**

```bash
git add crates/curl-tui-core/tests/integration_test.rs
git commit -m "test: add environment sync round-trip integration test"
```

---

### Task 5: Add `CreateEnvironment` action variant and keybinding

**Files:**
- Modify: `crates/curl-tui/src/app.rs:73-115` (Action enum)
- Modify: `crates/curl-tui-core/src/config.rs:14-32` (default keybindings)
- Modify: `crates/curl-tui/src/input.rs:63-80` (action_map in build_keymap)

- [ ] **Step 1: Add `CreateEnvironment` variant to the Action enum**

In `crates/curl-tui/src/app.rs`, add `CreateEnvironment` after `SwitchEnvironment` (line 81):

```rust
    SwitchEnvironment,
    CreateEnvironment,
    NewRequest,
```

- [ ] **Step 2: Add default keybinding in config.rs**

In `crates/curl-tui-core/src/config.rs`, add after the `switch_env` entry (after line 18):

```rust
    map.insert("create_env".into(), "ctrl+shift+e".into());
```

- [ ] **Step 3: Add keymap entry in input.rs**

In `crates/curl-tui/src/input.rs`, add after the `switch_env` entry (line 66):

```rust
        ("create_env", Action::CreateEnvironment),
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build --workspace`
Expected: Compiles (the `CreateEnvironment` action is not yet handled anywhere but that's fine — the `_ => {}` catch-all in dispatch covers it)

- [ ] **Step 5: Verify keybinding resolves correctly**

The existing `input.rs` test infrastructure can be used to verify. Add a test in `input.rs` tests (or verify manually):

The `parse_binding("ctrl+shift+e")` should produce `(CONTROL | SHIFT, Char('e'))`, and `resolve_action` with that event should return `CreateEnvironment` because:
1. Exact match: `(CONTROL | SHIFT, Char('e'))` → `CreateEnvironment` ✓

No new test needed — the existing `parse_binding` tests cover this pattern. Verify manually with:

Run: `cargo test -p curl-tui -- input`
Expected: PASS (existing tests still pass)

- [ ] **Step 6: Commit**

```bash
git add crates/curl-tui/src/app.rs crates/curl-tui-core/src/config.rs crates/curl-tui/src/input.rs
git commit -m "feat: add CreateEnvironment action with Ctrl+Shift+E keybinding"
```

---

### Task 6: Handle `CreateEnvironment` in dispatch

**Files:**
- Modify: `crates/curl-tui/src/main.rs` (normal dispatch + `handle_variables_action`)

- [ ] **Step 1: Handle in normal dispatch**

Find the match arm for `Action::SwitchEnvironment` in the normal dispatch section of `main.rs` (this is the main event loop, not `handle_variables_action`). It calls `app.cycle_environment()`. Add a new arm for `CreateEnvironment` nearby:

```rust
            Action::CreateEnvironment => {
                app.create_new_environment();
                app.show_variables = true;
                app.var_tier = app::VarTier::Environment;
            }
```

**Note:** `create_new_environment()` already calls `self.start_editing(EditField::EnvironmentName(idx))` which enters Editing mode for the name field. Setting `show_variables = true` afterward opens the variables overlay with the name field already in editing mode — this is the intended UX (user sees the overlay and can immediately type a name).

- [ ] **Step 2: Handle in `handle_variables_action`**

In `crates/curl-tui/src/main.rs:530`, the existing match arm for `SwitchEnvironment | NewRequest` sets `show_variables = false` before creating the environment. For `CreateEnvironment`, the overlay should stay open instead. Update the match arm to handle `CreateEnvironment` separately:

Change:
```rust
        Action::SwitchEnvironment | Action::NewRequest => {
            // Ctrl+E or Ctrl+N in the variables overlay: create a new environment
            if app.var_tier == app::VarTier::Environment {
                app.show_variables = false;
                app.create_new_environment();
            }
        }
```

To:
```rust
        Action::SwitchEnvironment | Action::NewRequest => {
            // Ctrl+E or Ctrl+N in the variables overlay: create a new environment
            if app.var_tier == app::VarTier::Environment {
                app.show_variables = false;
                app.create_new_environment();
            }
        }
        Action::CreateEnvironment => {
            // Ctrl+Shift+E in the variables overlay: create and stay in overlay
            app.create_new_environment();
            app.var_tier = app::VarTier::Environment;
        }
```

This keeps the overlay open when `CreateEnvironment` is triggered from within the overlay, matching the normal dispatch behavior.

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p curl-tui`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add crates/curl-tui/src/main.rs
git commit -m "feat: handle CreateEnvironment action in normal and variables dispatch"
```

---

### Task 7: Update help screen and statusbar

**Files:**
- Modify: `crates/curl-tui/src/ui/help.rs:31` (add Ctrl+Shift+E line)
- Modify: `crates/curl-tui/src/ui/statusbar.rs:162-163` (add hint)

- [ ] **Step 1: Update help screen**

In `crates/curl-tui/src/ui/help.rs`, after line 31 (`binding("Ctrl+E", "Cycle active environment")`), add:

```rust
        binding("Ctrl+Shift+E", "Create a new environment"),
```

- [ ] **Step 2: Update statusbar**

In `crates/curl-tui/src/ui/statusbar.rs`, after the `v:vars` hint (around line 163), add:

```rust
        hints.push(Span::styled("Ctrl+Shift+E", key_style));
        hints.push(Span::styled(":new env ", hint_style));
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p curl-tui`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add crates/curl-tui/src/ui/help.rs crates/curl-tui/src/ui/statusbar.rs
git commit -m "docs: add Ctrl+Shift+E to help screen and statusbar hints"
```

---

### Task 8: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass (existing + new)

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
git commit -m "chore: fix lint/fmt issues from environment improvements"
```
