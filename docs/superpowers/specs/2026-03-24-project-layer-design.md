# Project Layer Design Spec

## Context

curl-tui currently stores all collections and environments in flat directories under a single config root. As users accumulate many collections for different APIs/services, there is no way to group them. This spec introduces **Projects** as a top-level organizational unit, allowing users to separate their work into independent workspaces — each with its own collections, environments, and history.

## Data Model

### New `Project` type (`types.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_environment: Option<String>,  // environment slug, persisted per-project
}
```

### `HistoryEntry` extension (`types.rs`)

Add two optional fields (backward-compatible — use both `#[serde(default)]` for deserialization of old entries AND `#[serde(skip_serializing_if = "Option::is_none")]` to match the existing pattern in the struct):

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub project_id: Option<Uuid>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub project_name: Option<String>,
```

### `AppConfig` additions (`config.rs`)

```rust
pub open_projects: Vec<String>,              // slugs of open projects
pub active_project: Option<String>,          // slug of active project
pub restore_session: SessionRestore,         // Auto (default) or Prompt
```

`SessionRestore` enum:
- `Auto` — restore all previously open projects on startup (default)
- `Prompt` — show a "restore session?" dialog if multiple projects were open

The existing `active_environment` field in `AppConfig` is kept for migration but becomes per-project state going forward. Per-project `active_environment` is persisted in `project.json` (add an `active_environment: Option<String>` field to `Project`).

## Disk Structure

```
~/.config/curl-tui/                          # (or %APPDATA%/curl-tui/ on Windows)
├── config.json                              # Global vars, keybindings, session state
├── .gitignore
├── history.jsonl                            # Global aggregated history
└── projects/
    ├── my-api/
    │   ├── project.json                     # { id, name }
    │   ├── collections/                     # Same format as current collection files
    │   │   └── users-api.json
    │   ├── environments/                    # Same format as current environment files
    │   │   ├── dev.json
    │   │   └── prod.json
    │   └── history.jsonl                    # Per-project history
    └── another-project/
        ├── project.json
        ├── collections/
        ├── environments/
        └── history.jsonl
```

Each project directory is self-contained. Projects are identified by their slug (filesystem-safe name derived from `collection::slugify`).

## Variable Hierarchy

Unchanged: **Collection > Environment > Global**

- Collection and environment variables are now project-scoped (stored inside the project directory)
- Global variables remain in `config.json` and apply across all projects

## History

Dual-write strategy:
- Every request appends to **both** the project's `history.jsonl` and the global `history.jsonl`
- Global entries include `project_id` and `project_name` for filtering
- Per-project entries omit these fields (redundant in context)

## New Core Module: `project.rs`

CRUD operations following the same pattern as `collection.rs`:

- `create_project(projects_dir, project)` — creates `<slug>/project.json` + subdirs
- `load_project(project_dir)` — reads `project.json`
- `list_projects(projects_dir)` — scans subdirectories, returns `Vec<(Project, PathBuf)>`
- `delete_project(project_dir)` — removes entire project directory
- `rename_project(projects_dir, project, old_slug)` — renames directory on disk
- `project_dir(projects_dir, project)` — returns `projects/<slug>/`

Reuses `collection::slugify` for slug generation.

**Slug collision handling:** `create_project` must check for existing directories with the same slug. If `projects/<slug>/` already exists and belongs to a different project (different UUID in `project.json`), append a numeric suffix: `<slug>-2`, `<slug>-3`, etc. — mirroring the pattern in `collection.rs:find_path_for`. `rename_project` must also update `config.json` session state (`open_projects`, `active_project`) to reflect the new slug.

**Name validation:** Reject empty names and names that produce empty slugs (e.g., purely non-ASCII names). Return a clear error so the UI can prompt the user to choose a different name.

## App State Refactoring

### `ProjectWorkspace` struct

The data portion lives in `curl-tui-core` (e.g., `types.rs`) per the convention that all business logic goes in core. The TUI-only fields (`request_tab`, `response_tab`, `collection_scroll`, `response_scroll`) are stored alongside in `App` or wrapped in a thin TUI struct that embeds the core workspace.

Extracts all per-project state from `App`:

```rust
pub struct ProjectWorkspace {
    pub project: Project,
    pub slug: String,
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub active_environment: Option<usize>,
    pub selected_collection: Option<usize>,
    pub selected_request: Option<usize>,
    pub current_request: Option<Request>,
    pub last_response: Option<CurlResponse>,
    pub request_tab: RequestTab,
    pub response_tab: ResponseTab,
    pub collection_scroll: usize,
    pub response_scroll: usize,
    pub var_collection_idx: Option<usize>,
    pub var_environment_idx: Option<usize>,
}
```

### `App` struct changes

Replace flat `collections`/`environments` fields with:

```rust
pub open_projects: Vec<ProjectWorkspace>,
pub active_project_idx: Option<usize>,
```

Add helpers:
- `active_workspace(&self) -> Option<&ProjectWorkspace>`
- `active_workspace_mut(&mut self) -> Option<&mut ProjectWorkspace>`

All existing methods that reference `self.collections`, `self.environments`, `self.active_environment`, `self.selected_collection`, `self.selected_request`, `self.current_request`, `self.last_response` are refactored to go through `active_workspace()`.

Text inputs remain on `App` (not per-workspace) since only one is visible at a time. On project switch, content is synced to/from the workspace.

### New overlay state on `App`

```rust
pub show_project_picker: bool,
pub project_picker_cursor: usize,
pub all_projects: Vec<(Project, String)>,    // (project, slug) for all on disk
pub show_first_launch: bool,
```

### Project switching flow

1. Flush text inputs → active workspace's `current_request`
2. Set `active_project_idx` to new index
3. Call `load_request_into_inputs()` from new workspace
4. Persist session state to `config.json`

## UI Design

### Title bar becomes project tab bar

The title bar row is repurposed to show project tabs + the active environment indicator:

```
┌────────────────────────────────────────────────────────┐
│ [ My API ] [ Another ] [ Third ]  ◀ ▶  [+]  env: Dev  │
├──────────┬─────────────────────────────────────────────┤
│ Collec-  │  Request / Response panes                   │
│ tions    │                                             │
├──────────┴─────────────────────────────────────────────┤
│ NORMAL │ Tab:pane  F6:project  Ctrl+O:open  ...         │
└────────────────────────────────────────────────────────┘
```

- Active tab: bold/highlighted (e.g., cyan background)
- Inactive tabs: dimmer style
- `[+]` button: visual cue for Ctrl+O
- `env: Dev` or `env: None`: right-aligned environment indicator
- When tabs overflow the available width, `◀ ▶` arrows appear and Left/Right scrolls the visible tabs

No extra row is consumed — the layout stays at 3 vertical sections (title, main, statusbar).

### Project picker overlay (Ctrl+O)

Centered modal listing ALL projects on disk (similar to existing collection picker):

- Up/Down to navigate
- Enter to open/switch to selected project
- Ctrl+N to create a new project
- `d` to close an open project (remove from tab bar, not delete from disk)
- Delete key to permanently delete a project (with confirmation)
- Already-open projects marked with a visual indicator

### First launch modal

Shown when `projects/` is empty and no legacy data exists to migrate:

> "Welcome to curl-tui! Enter a name for your first project:"

Text input with Enter to confirm, creating the project and opening it immediately.

### Statusbar updates

Add project navigation hints in Normal mode:
- `F6:next project`
- `Ctrl+O:projects`

## Keybindings

### New actions

```rust
NextProject,         // F6 (secondary: Ctrl+Tab)
PrevProject,         // F7 (secondary: Ctrl+Shift+Tab)
OpenProjectPicker,   // Ctrl+O
CloseProject,        // from project picker
```

### Default keybinding additions in `config.rs`

```rust
"next_project"    -> "f6"       // also accept ctrl+tab where supported
"prev_project"    -> "f7"       // also accept ctrl+shift+tab where supported
"open_project"    -> "ctrl+o"
```

**Note:** `Ctrl+Tab` is intercepted by most terminal emulators for their own tab switching. `F6`/`F7` are the reliable primary bindings. `Ctrl+Tab`/`Ctrl+Shift+Tab` are registered as secondary bindings for terminals that do pass them through.

### Input dispatch priority (`main.rs`)

```
if show_first_launch { ... }
else if show_project_picker { ... }
else if show_collection_picker { ... }
else if show_variables { ... }
else { normal dispatch }
```

## Migration Strategy

Automatic one-time migration when legacy flat data is detected:

**Trigger:** `projects/` dir is empty AND root `collections/` or `environments/` contain `.json` files.

**Steps:**
1. Create `projects/default/` with `project.json` (`name: "Default"`)
2. Move files from root `collections/` → `projects/default/collections/`
3. Move files from root `environments/` → `projects/default/environments/`
4. Copy root `history.jsonl` → `projects/default/history.jsonl` (keep original as global)
5. Update `config.json`: `open_projects: ["default"]`, `active_project: "default"`
6. Migrate `active_environment` from config to project workspace state

Lives in `curl-tui-core` as a dedicated `migrate_flat_to_project(root)` function (in `init.rs` or a new `migration.rs`).

**Atomicity:** Use a copy-then-delete strategy. Files are copied to the project directory first, then originals are deleted only after all copies succeed. A marker file `projects/.migration-complete` is written last. On startup, if the marker is absent but `projects/default/` exists, re-run the migration to handle partial failures.

### Edge Cases

- **Last project closed:** The UI shows the project picker overlay automatically (user must open or create a project to continue working).
- **Project directory deleted externally:** On next access, `load_project` fails gracefully — the project is removed from `open_projects` and a status message notifies the user.
- **`all_projects` staleness:** The `all_projects` list is refreshed every time the project picker opens (re-scans `projects/` directory), not cached from startup.

## Implementation Phases

1. **Core types & project CRUD** — `Project` in types.rs, new `project.rs` module with tests
2. **Config changes** — session fields in `AppConfig`, `HistoryEntry` extension
3. **History dual-write** — new `append_entry_dual` in history.rs
4. **Migration logic** — detect + migrate flat structure, with tempdir tests
5. **App state refactor** — `ProjectWorkspace`, refactor `App` to use `Vec<ProjectWorkspace>`
6. **Project switching & session** — switch/open/close methods, session persistence
7. **UI: tab bar & picker** — title bar tabs, project picker overlay, first-launch modal
8. **Keybindings & input** — new actions, dispatch chain updates
9. **Polish** — edge cases, help overlay updates, restore-session prompt mode

## Existing Functions to Reuse

- `collection::slugify` (`crates/curl-tui-core/src/collection.rs`) — for project slug generation
- `collection::save_collection` / `list_collections` — unchanged, just called with project-scoped paths
- `environment::save_environment` / `list_environments` — unchanged, same approach
- `secret::redact_secrets` (`crates/curl-tui-core/src/secret.rs`) — for history redaction
- `AppConfig::save_to` / `load_from` (`crates/curl-tui-core/src/config.rs`) — for session persistence

## Verification

1. `cargo test --workspace` — all existing tests pass (new config fields have defaults)
2. New unit tests in `project.rs` — CRUD operations with tempdir
3. Migration test — create flat structure in tempdir, run migration, verify project structure
4. Integration test — create project, add collections/environments, switch projects, verify isolation
5. Manual TUI testing:
   - Fresh launch → first-launch modal appears
   - Create project → tab appears in title bar
   - Ctrl+O → picker shows all projects
   - Ctrl+Tab → cycles between open projects
   - Close project → tab removed, data preserved on disk
   - Restart → session restored (open projects remembered)
