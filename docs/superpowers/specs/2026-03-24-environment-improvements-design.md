# Environment Improvements: Restore on Switch + Dedicated Create Command

## Summary

Two related improvements to environment management:

1. **Restore environment on project switch** — keep `project.active_environment` (the persisted name) in sync with the in-memory index so switching projects restores the correct environment.
2. **Dedicated CreateEnvironment action (Ctrl+Shift+E)** — a top-level keybinding to create a new environment without going through the variables overlay first.

## Issue 1: Environment Not Restored on Project Switch

### Root Cause

`cycle_environment()`, `create_new_environment()`, and `delete_active_environment()` update `ws.data.active_environment` (usize index) but never sync the string name back to `ws.data.project.active_environment`. When the user switches projects and returns, the stale name from startup is what gets used.

### Solution

Add a helper method `sync_active_environment_name()` on `ProjectWorkspaceData` in `curl-tui-core` that:
1. Reads `self.active_environment` (the index)
2. Looks up the environment name from `self.environments[idx]`
3. Writes it to `self.project.active_environment`
4. Persists `project.json` to disk via a new `save_project()` utility in `curl-tui-core`

This keeps business logic in `curl-tui-core` per project convention ("All business logic goes in curl-tui-core").

Call this helper from every site that mutates the active environment index:
- `cycle_environment()`
- `create_new_environment()`
- `delete_active_environment()`

No changes needed to `switch_project()` — it already reads from `ProjectWorkspaceData` which has the correct index set at load time.

### Persistence

A `save_project()` function must be added to `curl-tui-core` (alongside existing `create_project`, `load_project`, etc.) to write the `Project` struct back to `projects/<slug>/project.json`. The sync helper calls this after updating the name field, so the change survives app restarts.

### Tests

- Unit test: cycle environment, verify `project.active_environment` name field is updated
- Unit test: create new environment, verify name is synced
- Unit test: delete active environment, verify name is synced (set to None or next env)
- Integration test: full round-trip — cycle env, sync, switch project, switch back, verify index is restored from persisted name

## Issue 2: Dedicated CreateEnvironment Action (Ctrl+Shift+E)

### Changes

| File | Change |
|------|--------|
| `app.rs` | Add `CreateEnvironment` variant to `Action` enum |
| `config.rs` | Add `"create_env" → "ctrl+shift+e"` to default keybindings |
| `input.rs` | Map `"create_env"` config key to `Action::CreateEnvironment` |
| `main.rs` | Handle `Action::CreateEnvironment` in normal dispatch: create env, open variables overlay on Environment tier, enter name editing mode |
| `main.rs` | Update `handle_variables_action` match arm (currently handles `SwitchEnvironment | NewRequest` for env creation) to also handle `CreateEnvironment` |
| `ui/help.rs` | Add Ctrl+Shift+E to help screen |
| `ui/statusbar.rs` | Add Ctrl+Shift+E hint |

### Behavior Flow

1. User presses Ctrl+Shift+E in Normal mode
2. New environment is created and set as active (with name synced via Issue 1 fix)
3. Variables overlay opens with Environment tier selected
4. User enters editing mode for the environment name
5. User can rename, add variables, or Esc to close

### Edge Case

If Ctrl+Shift+E is pressed while the variables overlay is already open, it still works: creates a new environment and switches to the Environment tier. The `handle_variables_action` match arm at ~line 530 of main.rs must be updated to include `Action::CreateEnvironment`.

### Tests

- Unit test: verify Ctrl+Shift+E resolves to `Action::CreateEnvironment`
- Unit test: verify `create_new_environment()` creates, activates, and syncs the environment name (shared with Issue 1 tests)
- UI-level behaviors (overlay opening, editing mode) tested manually per project convention

## Files Modified

- `crates/curl-tui-core/src/types.rs` — `sync_active_environment_name()` on `ProjectWorkspaceData`
- `crates/curl-tui-core/src/project.rs` — new `save_project()` function
- `crates/curl-tui/src/app.rs` — Action enum, calls to sync helper in cycle/create/delete methods
- `crates/curl-tui-core/src/config.rs` — default keybinding
- `crates/curl-tui/src/input.rs` — keymap entry
- `crates/curl-tui/src/main.rs` — action dispatch + handle_variables_action update
- `crates/curl-tui/src/ui/help.rs` — help text
- `crates/curl-tui/src/ui/statusbar.rs` — hints
