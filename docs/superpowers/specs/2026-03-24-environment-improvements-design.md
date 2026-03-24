# Environment Improvements: Restore on Switch + Dedicated Create Command

## Summary

Two related improvements to environment management:

1. **Restore environment on project switch** — keep `project.active_environment` (the persisted name) in sync with the in-memory index so switching projects restores the correct environment.
2. **Dedicated CreateEnvironment action (Ctrl+N)** — a top-level keybinding to create a new environment without going through the variables overlay first.

## Issue 1: Environment Not Restored on Project Switch

### Root Cause

`cycle_environment()` and `create_new_environment()` update `ws.data.active_environment` (usize index) but never sync the string name back to `ws.data.project.active_environment`. When the user switches projects and returns, the stale name from startup is what gets used.

### Solution

Add a helper method `sync_active_environment_name()` on `App` that:
1. Reads `ws.data.active_environment` (the index)
2. Looks up the environment name from `ws.data.environments[idx]`
3. Writes it to `ws.data.project.active_environment`

Call this helper from every site that mutates the active environment index:
- `cycle_environment()`
- `create_new_environment()`
- Any future method that changes the active environment

No changes needed to `switch_project()` — it already reads from `ProjectWorkspaceData` which has the correct index set at load time.

### Tests

- Unit test: cycle environment, verify `project.active_environment` name field is updated
- Unit test: create new environment, verify name is synced
- Integration test: simulate project switch, verify environment index matches last selection

## Issue 2: Dedicated CreateEnvironment Action (Ctrl+N)

### Changes

| File | Change |
|------|--------|
| `app.rs` | Add `CreateEnvironment` variant to `Action` enum |
| `config.rs` | Add `"create_env" → "ctrl+n"` to default keybindings |
| `input.rs` | Map `"create_env"` config key to `Action::CreateEnvironment` |
| `main.rs` | Handle `Action::CreateEnvironment`: create env, open variables overlay on Environment tier, enter name editing mode |
| `ui/help.rs` | Add Ctrl+N to help screen |
| `ui/statusbar.rs` | Add Ctrl+N hint |

### Behavior Flow

1. User presses Ctrl+N in Normal mode
2. New environment is created and set as active
3. Variables overlay opens with Environment tier selected
4. User enters editing mode for the environment name
5. User can rename, add variables, or Esc to close

### Edge Case

If Ctrl+N is pressed while the variables overlay is already open, it still works: creates a new environment and switches to the Environment tier (same as current Ctrl+E behavior inside the overlay).

### Tests

- Unit test: verify Ctrl+N resolves to `Action::CreateEnvironment`
- Unit test: verify `create_new_environment()` creates, activates, and syncs the environment name
- UI-level behaviors (overlay opening, editing mode) tested manually per project convention

## Files Modified

- `crates/curl-tui/src/app.rs` — Action enum, sync helper, create_new_environment updates
- `crates/curl-tui-core/src/config.rs` — default keybinding
- `crates/curl-tui/src/input.rs` — keymap entry
- `crates/curl-tui/src/main.rs` — action dispatch
- `crates/curl-tui/src/ui/help.rs` — help text
- `crates/curl-tui/src/ui/statusbar.rs` — hints
