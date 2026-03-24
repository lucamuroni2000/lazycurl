# Environment Manager Modal — Design Spec

## Goal

Replace the current Ctrl+Shift+E "create environment" action with a dedicated Environment Manager modal that shows all environments for the active project. Users can create, delete, rename, and activate environments from this modal. All environment CRUD is removed from the variables overlay, which becomes variables-only.

## Key Decisions

- Ctrl+E remains the quick-cycle shortcut for switching environments (works everywhere: normal mode, variables overlay)
- Ctrl+Shift+E opens the Environment Manager modal
- The variables overlay no longer handles environment creation, deletion, or switching via `[`/`]`
- Global variables are not environments — they remain in the variables overlay's Global tier, unaffected
- Environments are per-project — the modal shows only the active project's environments
- Delete requires confirmation (`y` to confirm, Esc to cancel)

## Modal UI

Centered overlay, ~50% width, height scales to content (max ~60% terminal height). Follows the project picker modal pattern.

```
┌─ Environments (My API Project) ─ n:new  r:rename  d:delete  Enter:activate  Esc:close ─┐
│                                                                                          │
│  > [*] Development                                                                       │
│        Production                                                                        │
│        Staging                                                                            │
│                                                                                          │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

- `>` cursor marker on selected row
- `[*]` marks the currently active environment
- Inline rename: name becomes an editable `TextInput` on the selected row
- Delete confirmation: line below the list reads `Delete 'Production'? y to confirm, Esc to cancel`
- Empty state: "No environments. Press n to create one."

## State

New fields on `App`:

| Field | Type | Purpose |
|-------|------|---------|
| `show_env_manager` | `bool` | Modal visibility |
| `env_manager_cursor` | `usize` | Selected row index |
| `env_manager_renaming` | `Option<usize>` | Index being renamed, None if not renaming |
| `env_manager_confirm_delete` | `Option<usize>` | Index pending delete confirmation, None if no prompt |
| `env_manager_name_input` | `TextInput` | Text input for inline rename |

## Dispatch

When `show_env_manager` is true, input routes to `handle_env_manager_action` in `main.rs`. Priority order: `show_env_manager` > `show_variables` > normal dispatch.

### Key bindings within the modal

| Key | State | Effect |
|-----|-------|--------|
| Up/Down | Normal | Move cursor |
| Enter | Normal | Activate selected environment, close modal |
| n | Normal | Create new environment, enter rename mode on it |
| r | Normal | Enter rename mode on cursor row |
| d | Normal | Show delete confirmation for cursor row |
| y | Confirming delete | Execute delete, clear confirmation |
| Esc | Confirming delete | Cancel confirmation |
| Enter | Renaming | Commit rename to disk |
| Esc | Renaming | Cancel rename, restore original name |
| Esc | Normal | Close modal |

## CRUD Operations

### Create (n)

1. New `Environment` appended to `ws.data.environments` with name "New Environment"
2. Saved to disk via `save_environment`
3. Cursor moves to the new row, enters rename mode (name input pre-populated with "New Environment")
4. Does not auto-activate — user must press Enter to activate

Note: This is different from the existing `create_new_environment()` method which auto-activates and enters `EditField::EnvironmentName` editing mode. The modal needs its own create helper that does not auto-activate or enter the main editing flow — it only appends, saves to disk, and sets `env_manager_renaming`.

### Activate (Enter)

The modal list order matches `ws.data.environments` order — `cursor_index` maps directly to the vector index.

1. `ws.data.active_environment = Some(cursor_index)`
2. `persist_active_environment()` called to sync name to `project.json`
3. Modal closes
4. Status message: `"Environment: <name>"`

### Rename (r)

1. `env_manager_renaming = Some(cursor_index)`, name input populated with current name
2. User edits inline, Enter to confirm
3. On confirm: old environment file deleted, new file saved with updated name
4. If renamed env is the active one, `persist_active_environment()` called to update `project.json`
5. Esc cancels, restoring original name

### Delete (d → y)

1. `env_manager_confirm_delete = Some(cursor_index)`, confirmation text shown
2. `y` confirms: file deleted from disk, removed from `ws.data.environments`, indices adjusted
3. If deleted env was active, `active_environment` set to None, `persist_active_environment()` called
4. Cursor clamped to list bounds
5. Esc cancels confirmation

## Changes to Variables Overlay

The Environment tier becomes view/edit variables only. Remove:

- `Action::SwitchEnvironment | Action::NewRequest` arm that creates environments
- `Action::CreateEnvironment` arm
- Environment deletion via `Action::DeleteItem` when variable list is empty
- `[` / `]` cycling through environments

Keep:

- All variable CRUD (add, delete, edit key/value, toggle secret)
- Tab cycling between Global / Environment / Collection tiers
- Display of active environment's variables
- "No environment selected" message, updated to: "No environment selected. Press Ctrl+Shift+E to manage environments."

Ctrl+E in the variables overlay still cycles the active environment (quick-switch), updating which environment's variables are displayed.

**Behavior change:** Currently, Ctrl+E (`Action::SwitchEnvironment`) inside the variables overlay creates a new environment instead of cycling. This must be fixed to actually cycle environments, matching its behavior in normal mode. When the environment list is empty and Ctrl+E is pressed (in any context), it should be a no-op with a status hint: "No environments. Press Ctrl+Shift+E to manage environments." (Previously it auto-created an environment.)

## Action and Keybinding Rename

| Before | After |
|--------|-------|
| `Action::CreateEnvironment` | `Action::ManageEnvironments` |
| Config key `create_env` | `manage_envs` |
| Default binding `ctrl+shift+e` | unchanged |
| Help text "Create a new environment" | "Manage environments" |
| Statusbar hint `:new env` | `:envs` |

`Action::SwitchEnvironment` and `Ctrl+E` remain unchanged.

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/curl-tui/src/ui/environment_manager.rs` | Create | Modal rendering |
| `crates/curl-tui/src/ui/mod.rs` | Modify | Add `pub mod environment_manager` |
| `crates/curl-tui/src/app.rs` | Modify | Add state fields, rename action variant, add env manager methods |
| `crates/curl-tui/src/main.rs` | Modify | Add `handle_env_manager_action`, update dispatch priority, remove env CRUD from variables dispatch |
| `crates/curl-tui/src/input.rs` | Modify | Rename `create_env` → `manage_envs` mapping |
| `crates/curl-tui-core/src/config.rs` | Modify | Rename `create_env` → `manage_envs` default keybinding |
| `crates/curl-tui/src/ui/help.rs` | Modify | Update help text |
| `crates/curl-tui/src/ui/statusbar.rs` | Modify | Update hint text |
| `crates/curl-tui/src/ui/variables.rs` | Modify | Remove environment CRUD hints, update empty-state message |
