# Project Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "Project" top-level organizational unit that groups collections, environments, and history into independent workspaces with tab-based project switching.

**Architecture:** Each project is a self-contained directory under `projects/` containing its own `collections/`, `environments/`, `history.jsonl`, and `project.json` metadata. The `App` struct is refactored to hold a `Vec<ProjectWorkspace>` where each workspace encapsulates per-project state. A project tab bar in the title bar row enables switching. Migration auto-converts existing flat data into a "Default" project.

**Tech Stack:** Rust, serde/serde_json, uuid, ratatui, crossterm, thiserror, tempfile (tests)

**Spec:** `docs/superpowers/specs/2026-03-24-project-layer-design.md`

---

## File Map

### New files
| File | Responsibility |
|---|---|
| `crates/curl-tui-core/src/project.rs` | Project CRUD: create, load, list, delete, rename with slug collision handling |
| `crates/curl-tui-core/src/migration.rs` | One-time migration from flat structure to project-based structure |
| `crates/curl-tui/src/ui/project_tabs.rs` | Render project tab bar in the title bar row |
| `crates/curl-tui/src/ui/project_picker.rs` | Render the project picker overlay (Ctrl+O) |

### Modified files
| File | Changes |
|---|---|
| `crates/curl-tui-core/src/types.rs` | Add `Project` struct, extend `HistoryEntry` with project fields |
| `crates/curl-tui-core/src/config.rs` | Add `open_projects`, `active_project`, `restore_session` to `AppConfig` |
| `crates/curl-tui-core/src/history.rs` | Add `append_entry_dual` for dual-write to project + global history |
| `crates/curl-tui-core/src/init.rs` | Create `projects/` dir, call migration when needed |
| `crates/curl-tui-core/src/lib.rs` | Add `pub mod project;` and `pub mod migration;` |
| `crates/curl-tui/src/app.rs` | Extract `ProjectWorkspace`, refactor `App` to `Vec<ProjectWorkspace>`, add project switching methods |
| `crates/curl-tui/src/main.rs` | Update startup to load projects, add project picker dispatch, session persistence |
| `crates/curl-tui/src/input.rs` | Add `NextProject`, `PrevProject`, `OpenProjectPicker` actions and keybindings |
| `crates/curl-tui/src/ui/mod.rs` | Wire in project tab bar rendering and project picker overlay |
| `crates/curl-tui/src/ui/layout.rs` | No structural change (title bar row is reused for tabs) |
| `crates/curl-tui/src/ui/statusbar.rs` | Add project-related hints |
| `crates/curl-tui/src/ui/help.rs` | Add project keybindings to help overlay |
| `crates/curl-tui-core/tests/integration_test.rs` | Add project-level integration tests |

---

## Task 1: Add `Project` type to `types.rs`

**Files:**
- Modify: `crates/curl-tui-core/src/types.rs`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block at the bottom of `types.rs`:

```rust
#[test]
fn test_project_roundtrip() {
    let project = Project {
        id: uuid::Uuid::new_v4(),
        name: "My API".to_string(),
        active_environment: Some("dev".to_string()),
    };
    let json = serde_json::to_string(&project).unwrap();
    let deserialized: Project = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "My API");
    assert_eq!(deserialized.active_environment, Some("dev".to_string()));
}

#[test]
fn test_project_without_active_env() {
    let json = r#"{"id":"00000000-0000-0000-0000-000000000000","name":"Test"}"#;
    let project: Project = serde_json::from_str(json).unwrap();
    assert!(project.active_environment.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p curl-tui-core -- types::tests::test_project_roundtrip`
Expected: FAIL — `Project` type not defined

- [ ] **Step 3: Write minimal implementation**

Add above `HistoryEntry` in `types.rs` (around line 131, after `Environment`):

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: uuid::Uuid,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_environment: Option<String>,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p curl-tui-core -- types::tests::test_project`
Expected: 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui-core/src/types.rs
git commit -m "feat: add Project type to types.rs"
```

---

## Task 2: Extend `HistoryEntry` with project fields

**Files:**
- Modify: `crates/curl-tui-core/src/types.rs`

- [ ] **Step 1: Write the failing test**

Add to tests in `types.rs`:

```rust
#[test]
fn test_history_entry_with_project_fields() {
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        collection_id: None,
        request_name: "Test".to_string(),
        method: Method::Get,
        url: "https://example.com".to_string(),
        status_code: Some(200),
        duration_ms: Some(100),
        environment: None,
        project_id: Some(uuid::Uuid::new_v4()),
        project_name: Some("My API".to_string()),
    };
    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: HistoryEntry = serde_json::from_str(&json).unwrap();
    assert!(deserialized.project_id.is_some());
    assert_eq!(deserialized.project_name, Some("My API".to_string()));
}

#[test]
fn test_history_entry_backward_compat() {
    // Old entries without project fields should deserialize fine
    let json = r#"{"id":"00000000-0000-0000-0000-000000000000","timestamp":"2026-01-01T00:00:00Z","request_name":"Test","method":"GET","url":"https://example.com"}"#;
    let entry: HistoryEntry = serde_json::from_str(json).unwrap();
    assert!(entry.project_id.is_none());
    assert!(entry.project_name.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p curl-tui-core -- types::tests::test_history_entry_with_project`
Expected: FAIL — `project_id` field not found

- [ ] **Step 3: Add fields to `HistoryEntry`**

Add these two fields at the end of the `HistoryEntry` struct (before the closing `}`):

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
```

Also add `#[serde(default)]` to the existing `Option` fields in `HistoryEntry` (`collection_id`, `status_code`, `duration_ms`, `environment`) that currently only have `skip_serializing_if` — this is needed for backward-compatible deserialization of entries that omit these fields.

Update every existing place that constructs a `HistoryEntry` to include `project_id: None, project_name: None`. This includes:
- `crates/curl-tui-core/src/history.rs` test helper `make_entry` (line 57)
- `crates/curl-tui/src/app.rs` in `send_request` method (around line 445)

- [ ] **Step 4: Run ALL tests to verify nothing breaks**

Run: `cargo test --workspace`
Expected: All tests PASS (existing tests compile with the new fields)

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui-core/src/types.rs crates/curl-tui-core/src/history.rs crates/curl-tui/src/app.rs
git commit -m "feat: extend HistoryEntry with project_id and project_name fields"
```

---

## Task 3: Add session fields to `AppConfig`

**Files:**
- Modify: `crates/curl-tui-core/src/config.rs`

- [ ] **Step 1: Write the failing test**

Add to tests in `config.rs`:

```rust
#[test]
fn test_config_session_fields_default() {
    let config = AppConfig::default();
    assert!(config.open_projects.is_empty());
    assert!(config.active_project.is_none());
    assert_eq!(config.restore_session, SessionRestore::Auto);
}

#[test]
fn test_config_session_fields_roundtrip() {
    let json = r#"{"open_projects":["my-api","other"],"active_project":"my-api","restore_session":"prompt"}"#;
    let config = AppConfig::load_from_str(json).unwrap();
    assert_eq!(config.open_projects, vec!["my-api", "other"]);
    assert_eq!(config.active_project, Some("my-api".to_string()));
    assert_eq!(config.restore_session, SessionRestore::Prompt);
}

#[test]
fn test_config_backward_compat_no_session_fields() {
    let json = r#"{"default_timeout": 30}"#;
    let config = AppConfig::load_from_str(json).unwrap();
    assert!(config.open_projects.is_empty());
    assert!(config.active_project.is_none());
    assert_eq!(config.restore_session, SessionRestore::Auto);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p curl-tui-core -- config::tests::test_config_session`
Expected: FAIL — `open_projects` field not found

- [ ] **Step 3: Write implementation**

Add the `SessionRestore` enum above `AppConfig` in `config.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionRestore {
    Auto,
    Prompt,
}

impl Default for SessionRestore {
    fn default() -> Self {
        SessionRestore::Auto
    }
}
```

Add these fields to `AppConfig`:

```rust
    #[serde(default)]
    pub open_projects: Vec<String>,
    #[serde(default)]
    pub active_project: Option<String>,
    #[serde(default)]
    pub restore_session: SessionRestore,
```

Update `Default` impl for `AppConfig` to include:

```rust
    open_projects: Vec::new(),
    active_project: None,
    restore_session: SessionRestore::default(),
```

- [ ] **Step 4: Run ALL tests**

Run: `cargo test --workspace`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui-core/src/config.rs
git commit -m "feat: add session persistence fields to AppConfig"
```

---

## Task 4: Create `project.rs` module — CRUD operations

**Files:**
- Create: `crates/curl-tui-core/src/project.rs`
- Modify: `crates/curl-tui-core/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/curl-tui-core/src/project.rs` with the test module first:

```rust
use crate::collection::slugify;
use crate::types::Project;
use std::path::{Path, PathBuf};

// Functions will go here

#[cfg(test)]
mod tests {
    use super::*;

    fn make_project(name: &str) -> Project {
        Project {
            id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            active_environment: None,
        }
    }

    #[test]
    fn test_create_and_load_project() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");
        let project = make_project("My API");
        let dir = create_project(&projects_dir, &project).unwrap();
        assert!(dir.join("project.json").exists());
        assert!(dir.join("collections").is_dir());
        assert!(dir.join("environments").is_dir());
        let loaded = load_project(&dir).unwrap();
        assert_eq!(loaded.name, "My API");
        assert_eq!(loaded.id, project.id);
    }

    #[test]
    fn test_list_projects() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");
        create_project(&projects_dir, &make_project("First")).unwrap();
        create_project(&projects_dir, &make_project("Second")).unwrap();
        let list = list_projects(&projects_dir).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_delete_project() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");
        let project = make_project("Delete Me");
        let dir = create_project(&projects_dir, &project).unwrap();
        assert!(dir.exists());
        delete_project(&dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn test_slug_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");
        let p1 = make_project("Test");
        let p2 = make_project("Test");
        let dir1 = create_project(&projects_dir, &p1).unwrap();
        let dir2 = create_project(&projects_dir, &p2).unwrap();
        assert_ne!(dir1, dir2);
        assert!(dir2.to_str().unwrap().contains("test-2"));
    }

    #[test]
    fn test_empty_name_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");
        let project = make_project("");
        assert!(create_project(&projects_dir, &project).is_err());
    }

    #[test]
    fn test_project_dir_helper() {
        let projects_dir = Path::new("/tmp/projects");
        let project = make_project("My Cool API");
        let dir = project_dir(projects_dir, &project);
        assert_eq!(dir, PathBuf::from("/tmp/projects/my-cool-api"));
    }

    #[test]
    fn test_rename_project() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");
        let mut project = make_project("Old Name");
        let old_dir = create_project(&projects_dir, &project).unwrap();
        assert!(old_dir.exists());
        project.name = "New Name".to_string();
        let new_dir = rename_project(&projects_dir, &project, "old-name").unwrap();
        assert!(!old_dir.exists());
        assert!(new_dir.exists());
        assert!(new_dir.join("project.json").exists());
        let loaded = load_project(&new_dir).unwrap();
        assert_eq!(loaded.name, "New Name");
    }
}
```

- [ ] **Step 2: Add `pub mod project;` to `lib.rs`**

Add `pub mod project;` to `crates/curl-tui-core/src/lib.rs`.

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core -- project::tests`
Expected: FAIL — functions not defined

- [ ] **Step 4: Implement the functions**

Add above the `#[cfg(test)]` block in `project.rs`:

```rust
use crate::collection::slugify;
use crate::types::Project;
use std::path::{Path, PathBuf};

/// Get the directory path for a project based on its name slug.
pub fn project_dir(projects_dir: &Path, project: &Project) -> PathBuf {
    projects_dir.join(slugify(&project.name))
}

/// Find a non-conflicting directory for a new project.
fn find_dir_for(projects_dir: &Path, project: &Project) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let slug = slugify(&project.name);
    if slug.is_empty() {
        return Err("Project name must produce a non-empty slug".into());
    }

    // Check if directory with this project's id already exists
    if let Ok(entries) = std::fs::read_dir(projects_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let pj = path.join("project.json");
                if pj.exists() {
                    if let Ok(content) = std::fs::read_to_string(&pj) {
                        if let Ok(existing) = serde_json::from_str::<Project>(&content) {
                            if existing.id == project.id {
                                return Ok(path);
                            }
                        }
                    }
                }
            }
        }
    }

    // Try base slug
    let candidate = projects_dir.join(&slug);
    if !candidate.exists() {
        return Ok(candidate);
    }

    // Numeric suffix
    let mut counter = 2;
    loop {
        let candidate = projects_dir.join(format!("{}-{}", slug, counter));
        if !candidate.exists() {
            return Ok(candidate);
        }
        counter += 1;
    }
}

/// Create a new project directory with project.json, collections/, environments/ subdirs.
/// Returns the path to the created project directory.
pub fn create_project(
    projects_dir: &Path,
    project: &Project,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let slug = slugify(&project.name);
    if slug.is_empty() {
        return Err("Project name must produce a non-empty slug".into());
    }
    std::fs::create_dir_all(projects_dir)?;
    let dir = find_dir_for(projects_dir, project)?;
    std::fs::create_dir_all(&dir)?;
    std::fs::create_dir_all(dir.join("collections"))?;
    std::fs::create_dir_all(dir.join("environments"))?;
    let content = serde_json::to_string_pretty(project)?;
    std::fs::write(dir.join("project.json"), content)?;
    Ok(dir)
}

/// Load a project from its directory (reads project.json).
pub fn load_project(project_dir: &Path) -> Result<Project, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(project_dir.join("project.json"))?;
    let project: Project = serde_json::from_str(&content)?;
    Ok(project)
}

/// List all projects in the projects directory.
/// Returns (Project, PathBuf) tuples where PathBuf is the project directory.
pub fn list_projects(
    projects_dir: &Path,
) -> Result<Vec<(Project, PathBuf)>, Box<dyn std::error::Error>> {
    let mut projects = Vec::new();
    if !projects_dir.exists() {
        return Ok(projects);
    }
    for entry in std::fs::read_dir(projects_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            match load_project(&path) {
                Ok(project) => projects.push((project, path)),
                Err(_) => continue,
            }
        }
    }
    Ok(projects)
}

/// Delete a project directory and all its contents.
pub fn delete_project(project_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::remove_dir_all(project_dir)?;
    Ok(())
}

/// Rename a project: update project.json and rename the directory.
/// Returns the new (slug, PathBuf) so the caller can update config session state.
pub fn rename_project(
    projects_dir: &Path,
    project: &Project,
    old_slug: &str,
) -> Result<(String, PathBuf), Box<dyn std::error::Error>> {
    let new_slug = slugify(&project.name);
    if new_slug.is_empty() {
        return Err("Project name must produce a non-empty slug".into());
    }
    let old_dir = projects_dir.join(old_slug);

    // Update project.json first
    let content = serde_json::to_string_pretty(project)?;
    std::fs::write(old_dir.join("project.json"), content)?;

    // Rename directory if slug changed, handling collisions
    if old_slug != new_slug {
        let mut target_slug = new_slug.clone();
        let mut target_dir = projects_dir.join(&target_slug);
        let mut counter = 2;
        while target_dir.exists() {
            target_slug = format!("{}-{}", new_slug, counter);
            target_dir = projects_dir.join(&target_slug);
            counter += 1;
        }
        std::fs::rename(&old_dir, &target_dir)?;
        Ok((target_slug, target_dir))
    } else {
        Ok((old_slug.to_string(), old_dir))
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p curl-tui-core -- project::tests`
Expected: All 7 tests PASS

- [ ] **Step 6: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All PASS

- [ ] **Step 7: Commit**

```bash
git add crates/curl-tui-core/src/project.rs crates/curl-tui-core/src/lib.rs
git commit -m "feat: add project.rs module with CRUD operations"
```

---

## Task 5: Add `append_entry_dual` to `history.rs`

**Files:**
- Modify: `crates/curl-tui-core/src/history.rs`

- [ ] **Step 1: Write the failing test**

Add to tests in `history.rs`:

```rust
#[test]
fn test_append_entry_dual() {
    let tmp = tempfile::tempdir().unwrap();
    let global_path = tmp.path().join("history.jsonl");
    let project_path = tmp.path().join("project").join("history.jsonl");

    let entry = make_entry("Dual Write");
    let secrets = vec![];
    append_entry_dual(&global_path, &project_path, &entry, &secrets).unwrap();

    assert!(global_path.exists());
    assert!(project_path.exists());
    let global = std::fs::read_to_string(&global_path).unwrap();
    let project = std::fs::read_to_string(&project_path).unwrap();
    assert!(global.contains("Dual Write"));
    assert!(project.contains("Dual Write"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p curl-tui-core -- history::tests::test_append_entry_dual`
Expected: FAIL — function not defined

- [ ] **Step 3: Implement**

Add to `history.rs`:

```rust
/// Append a redacted history entry to both the global and project-scoped history files.
pub fn append_entry_dual(
    global_path: &Path,
    project_path: &Path,
    entry: &HistoryEntry,
    secrets: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    append_entry_redacted(global_path, entry, secrets)?;
    append_entry_redacted(project_path, entry, secrets)?;
    Ok(())
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p curl-tui-core -- history::tests`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui-core/src/history.rs
git commit -m "feat: add dual-write history function for project + global"
```

---

## Task 6: Create `migration.rs` — flat-to-project migration

**Files:**
- Create: `crates/curl-tui-core/src/migration.rs`
- Modify: `crates/curl-tui-core/src/lib.rs`
- Modify: `crates/curl-tui-core/src/init.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/curl-tui-core/src/migration.rs`:

```rust
use crate::types::Project;
use std::path::Path;

// Implementation will go here

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_migration_true() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("collections")).unwrap();
        std::fs::create_dir_all(root.join("projects")).unwrap();
        std::fs::write(root.join("collections/test.json"), r#"{"id":"00000000-0000-0000-0000-000000000001","name":"Test","requests":[]}"#).unwrap();
        assert!(needs_migration(root));
    }

    #[test]
    fn test_needs_migration_false_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("collections")).unwrap();
        std::fs::create_dir_all(root.join("projects")).unwrap();
        assert!(!needs_migration(root));
    }

    #[test]
    fn test_needs_migration_false_already_migrated() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("projects/default")).unwrap();
        std::fs::write(root.join("projects/default/project.json"), "{}").unwrap();
        assert!(!needs_migration(root));
    }

    #[test]
    fn test_migrate_flat_to_project() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Create flat structure
        std::fs::create_dir_all(root.join("collections")).unwrap();
        std::fs::create_dir_all(root.join("environments")).unwrap();
        std::fs::create_dir_all(root.join("projects")).unwrap();
        std::fs::write(
            root.join("collections/my-api.json"),
            r#"{"id":"00000000-0000-0000-0000-000000000001","name":"My API","requests":[]}"#,
        ).unwrap();
        std::fs::write(
            root.join("environments/dev.json"),
            r#"{"id":"00000000-0000-0000-0000-000000000002","name":"Dev","variables":{}}"#,
        ).unwrap();
        std::fs::write(root.join("history.jsonl"), "line1\nline2\n").unwrap();

        // Run migration
        migrate_flat_to_project(root).unwrap();

        // Verify project structure
        let default_dir = root.join("projects/default");
        assert!(default_dir.join("project.json").exists());
        assert!(default_dir.join("collections/my-api.json").exists());
        assert!(default_dir.join("environments/dev.json").exists());
        assert!(default_dir.join("history.jsonl").exists());

        // Global history still exists
        assert!(root.join("history.jsonl").exists());

        // Old dirs should be empty
        let col_count = std::fs::read_dir(root.join("collections")).unwrap().count();
        assert_eq!(col_count, 0);

        // Migration marker exists
        assert!(root.join("projects/.migration-complete").exists());
    }

    #[test]
    fn test_migration_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("collections")).unwrap();
        std::fs::create_dir_all(root.join("projects")).unwrap();
        std::fs::write(
            root.join("collections/test.json"),
            r#"{"id":"00000000-0000-0000-0000-000000000001","name":"Test","requests":[]}"#,
        ).unwrap();

        migrate_flat_to_project(root).unwrap();
        // Running again should not fail
        // (needs_migration returns false because marker exists)
        assert!(!needs_migration(root));
    }
}
```

- [ ] **Step 2: Add `pub mod migration;` to `lib.rs`**

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core -- migration::tests`
Expected: FAIL — functions not defined

- [ ] **Step 4: Implement migration functions**

```rust
use crate::types::Project;
use std::path::Path;

/// Check if migration from flat structure to project-based is needed.
pub fn needs_migration(root: &Path) -> bool {
    let marker = root.join("projects/.migration-complete");
    if marker.exists() {
        return false;
    }

    // Check if projects/ has any project directories
    let projects_dir = root.join("projects");
    if projects_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() && entry.path().join("project.json").exists() {
                    return false; // Already has projects
                }
            }
        }
    }

    // Check if flat collections/ or environments/ have json files
    has_json_files(&root.join("collections")) || has_json_files(&root.join("environments"))
}

fn has_json_files(dir: &Path) -> bool {
    if !dir.exists() {
        return false;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                return true;
            }
        }
    }
    false
}

/// Migrate flat collections/environments/history into a "Default" project.
/// Uses copy-then-delete for atomicity.
/// Also reads config.json to migrate `active_environment` into the project and
/// sets `open_projects`/`active_project` in the config.
pub fn migrate_flat_to_project(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let default_dir = root.join("projects/default");
    std::fs::create_dir_all(default_dir.join("collections"))?;
    std::fs::create_dir_all(default_dir.join("environments"))?;

    // Read existing config to migrate active_environment
    let config_path = root.join("config.json");
    let mut config = crate::config::AppConfig::load_from(&config_path)?;
    let active_env = config.active_environment.take(); // Move out of config

    // Write project.json with migrated active_environment
    let project = Project {
        id: uuid::Uuid::new_v4(),
        name: "Default".to_string(),
        active_environment: active_env,
    };
    let content = serde_json::to_string_pretty(&project)?;
    std::fs::write(default_dir.join("project.json"), content)?;

    // Copy collections
    let cols_dir = root.join("collections");
    if cols_dir.exists() {
        copy_json_files(&cols_dir, &default_dir.join("collections"))?;
    }

    // Copy environments
    let envs_dir = root.join("environments");
    if envs_dir.exists() {
        copy_json_files(&envs_dir, &default_dir.join("environments"))?;
    }

    // Copy history
    let history = root.join("history.jsonl");
    if history.exists() {
        std::fs::copy(&history, default_dir.join("history.jsonl"))?;
    }

    // All copies succeeded — now delete originals
    delete_json_files(&cols_dir)?;
    delete_json_files(&envs_dir)?;

    // Update config.json with session state
    config.open_projects = vec!["default".to_string()];
    config.active_project = Some("default".to_string());
    config.save_to(&config_path)?;

    // Write migration marker
    std::fs::write(root.join("projects/.migration-complete"), "")?;

    Ok(())
}

fn copy_json_files(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !src.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let filename = path.file_name().unwrap();
            std::fs::copy(&path, dst.join(filename))?;
        }
    }
    Ok(())
}

fn delete_json_files(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p curl-tui-core -- migration::tests`
Expected: All 5 tests PASS

- [ ] **Step 6: Update `init.rs` to create `projects/` dir and call migration**

In `init.rs`, add after `std::fs::create_dir_all(root.join("environments"))?;`:

```rust
    std::fs::create_dir_all(root.join("projects"))?;

    // Migrate flat structure if needed
    if crate::migration::needs_migration(root) {
        crate::migration::migrate_flat_to_project(root)?;
    }
```

- [ ] **Step 7: Run ALL tests**

Run: `cargo test --workspace`
Expected: All PASS

- [ ] **Step 8: Commit**

```bash
git add crates/curl-tui-core/src/migration.rs crates/curl-tui-core/src/lib.rs crates/curl-tui-core/src/init.rs
git commit -m "feat: add migration from flat structure to project-based layout"
```

---

## Task 7: Add new `Action` variants and keybindings

**Files:**
- Modify: `crates/curl-tui/src/app.rs` (Action enum only)
- Modify: `crates/curl-tui/src/input.rs`
- Modify: `crates/curl-tui-core/src/config.rs` (default keybindings)

- [ ] **Step 1: Add Action variants**

In `app.rs`, add to the `Action` enum (before `None`):

```rust
    // Project actions
    NextProject,
    PrevProject,
    OpenProjectPicker,
    CloseProject,
```

- [ ] **Step 2: Add default keybindings in `config.rs`**

In `default_keybindings()` function, add:

```rust
    map.insert("next_project".into(), "f6".into());
    map.insert("prev_project".into(), "f7".into());
    map.insert("open_project".into(), "ctrl+o".into());
```

- [ ] **Step 3: Wire keybindings in `input.rs`**

In the `action_map` vector in `build_keymap()`, add:

```rust
        ("next_project", Action::NextProject),
        ("prev_project", Action::PrevProject),
        ("open_project", Action::OpenProjectPicker),
```

Also add `Ctrl+Tab` / `Ctrl+Shift+Tab` as secondary bindings. After the `for` loop that processes `action_map`, add:

```rust
    // Secondary project switching bindings (Ctrl+Tab may not work on all terminals)
    map.entry((KeyModifiers::CONTROL, KeyCode::Tab))
        .or_insert(Action::NextProject);
    map.entry((KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyCode::BackTab))
        .or_insert(Action::PrevProject);
```

- [ ] **Step 4: Run tests**

Run: `cargo test --workspace`
Expected: All PASS (the config keybinding test should still pass — new keys are additive)

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui/src/app.rs crates/curl-tui/src/input.rs crates/curl-tui-core/src/config.rs
git commit -m "feat: add project switching actions and keybindings"
```

---

## Task 8: Refactor `App` to use `ProjectWorkspace`

This is the largest task. It refactors the `App` struct to hold per-project state in `ProjectWorkspace` structs.

**Files:**
- Modify: `crates/curl-tui/src/app.rs` (major refactor)
- Modify: `crates/curl-tui/src/main.rs` (update initialization)

- [ ] **Step 1a: Define `ProjectWorkspaceData` in `curl-tui-core/src/types.rs`**

Per the architecture convention (all business logic in core), the data portion of the workspace lives in the core crate. Add to `types.rs`:

```rust
/// Core per-project data — business logic fields only (no UI state).
#[derive(Debug, Clone)]
pub struct ProjectWorkspaceData {
    pub project: Project,
    pub slug: String,
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub active_environment: Option<usize>,
    pub selected_collection: Option<usize>,
    pub selected_request: Option<usize>,
    pub current_request: Option<Request>,
    pub last_response: Option<CurlResponse>,
    pub var_collection_idx: Option<usize>,
    pub var_environment_idx: Option<usize>,
}

impl ProjectWorkspaceData {
    pub fn new(project: Project, slug: String) -> Self {
        Self {
            project,
            slug,
            collections: Vec::new(),
            environments: Vec::new(),
            active_environment: None,
            selected_collection: None,
            selected_request: None,
            current_request: Some(Request {
                id: uuid::Uuid::new_v4(),
                name: "New Request".to_string(),
                method: Method::Get,
                url: String::new(),
                headers: Vec::new(),
                params: Vec::new(),
                body: None,
                auth: None,
            }),
            last_response: None,
            var_collection_idx: None,
            var_environment_idx: None,
        }
    }
}
```

- [ ] **Step 1b: Define `ProjectWorkspace` in `app.rs` wrapping the core data**

Add after the `Action` enum, before `pub struct App`:

```rust
use curl_tui_core::types::ProjectWorkspaceData;

/// Per-project state — wraps core data with TUI-specific fields.
pub struct ProjectWorkspace {
    pub data: ProjectWorkspaceData,
    pub request_tab: RequestTab,
    pub response_tab: ResponseTab,
    pub collection_scroll: usize,
    pub response_scroll: usize,
}

impl ProjectWorkspace {
    pub fn new(project: curl_tui_core::types::Project, slug: String) -> Self {
        Self {
            data: ProjectWorkspaceData::new(project, slug),
            request_tab: RequestTab::Headers,
            response_tab: ResponseTab::Body,
            collection_scroll: 0,
            response_scroll: 0,
        }
    }
}
```

Throughout the rest of the plan, access data fields via `ws.data.collections`, `ws.data.project`, `ws.data.slug`, etc. TUI fields via `ws.request_tab`, `ws.collection_scroll`, etc.
```

- [ ] **Step 2: Refactor `App` struct**

Replace the per-project fields with workspace-based fields:

```rust
pub struct App {
    pub config: AppConfig,
    // Project management
    pub open_projects: Vec<ProjectWorkspace>,
    pub active_project_idx: Option<usize>,
    pub project_tab_scroll: usize,  // for scrolling overflowing tabs
    // UI state (global, not per-project)
    pub active_pane: Pane,
    pub pane_visible: [bool; 3],
    pub should_quit: bool,
    pub show_help: bool,
    pub secrets_revealed: bool,
    pub status_message: Option<String>,
    pub input_mode: InputMode,
    pub edit_field: Option<EditField>,
    // Text inputs (shared, content swapped on project switch)
    pub url_input: crate::text_input::TextInput,
    pub body_input: crate::text_input::TextInput,
    pub header_key_inputs: Vec<crate::text_input::TextInput>,
    pub header_value_inputs: Vec<crate::text_input::TextInput>,
    pub param_key_inputs: Vec<crate::text_input::TextInput>,
    pub param_value_inputs: Vec<crate::text_input::TextInput>,
    pub name_input: crate::text_input::TextInput,
    // Collection picker
    pub show_collection_picker: bool,
    pub picker_cursor: usize,
    // Variables overlay
    pub show_variables: bool,
    pub var_tier: VarTier,
    pub var_cursor: usize,
    pub var_editing: Option<VarEditTarget>,
    pub var_key_input: crate::text_input::TextInput,
    pub var_value_input: crate::text_input::TextInput,
    // Project picker overlay
    pub show_project_picker: bool,
    pub project_picker_cursor: usize,
    pub all_projects: Vec<(curl_tui_core::types::Project, String)>,
    pub show_first_launch: bool,
}
```

- [ ] **Step 2b: Update `App::new()` constructor**

Initialize all new fields:
```rust
    open_projects: Vec::new(),
    active_project_idx: None,
    project_tab_scroll: 0,
    show_project_picker: false,
    project_picker_cursor: 0,
    all_projects: Vec::new(),
    show_first_launch: false,
```

Remove the old per-project fields that are now in `ProjectWorkspace`: `collections`, `environments`, `active_environment`, `selected_collection`, `selected_request`, `current_request`, `last_response`, `request_tab`, `response_tab`, `collection_scroll`, `response_scroll`, `var_collection_idx`, `var_environment_idx`.

- [ ] **Step 3: Add workspace accessor helpers**

```rust
impl App {
    pub fn active_workspace(&self) -> Option<&ProjectWorkspace> {
        self.active_project_idx
            .and_then(|i| self.open_projects.get(i))
    }

    pub fn active_workspace_mut(&mut self) -> Option<&mut ProjectWorkspace> {
        self.active_project_idx
            .and_then(|i| self.open_projects.get_mut(i))
    }
}
```

- [ ] **Step 4: Refactor all methods that access per-project state**

This is the bulk of the work. Every method on `App` that currently references:
- `self.collections` → `ws.collections` (via `active_workspace()`)
- `self.environments` → `ws.environments`
- `self.active_environment` → `ws.active_environment`
- `self.selected_collection` → `ws.selected_collection`
- `self.selected_request` → `ws.selected_request`
- `self.current_request` → `ws.current_request`
- `self.last_response` → `ws.last_response`
- `self.request_tab` → `ws.request_tab`
- `self.response_tab` → `ws.response_tab`
- `self.collection_scroll` → `ws.collection_scroll`
- `self.response_scroll` → `ws.response_scroll`
- `self.var_collection_idx` → `ws.var_collection_idx`
- `self.var_environment_idx` → `ws.var_environment_idx`

Key pattern — for methods that need mutable access:
```rust
pub fn some_method(&mut self) {
    let Some(ws) = self.active_workspace_mut() else { return; };
    // use ws.collections, ws.selected_collection, etc.
}
```

For `send_request`, the history path changes:
```rust
// Replace: let history_path = config_dir().join("history.jsonl");
// With:
let global_history = config_dir().join("history.jsonl");
let project_history = config_dir()
    .join("projects")
    .join(&self.active_workspace().map(|ws| ws.slug.clone()).unwrap_or_default())
    .join("history.jsonl");
```

And use `append_entry_dual` instead of `append_entry_redacted`.

For `save_request_to_collection`, `create_new_collection`, `delete_selected_in_collections`, and all methods that compute `config_dir().join("collections")`:
```rust
// Replace: let collections_dir = config_dir().join("collections");
// With:
let collections_dir = config_dir()
    .join("projects")
    .join(&ws.slug)
    .join("collections");
```

Same pattern for environments paths.

- [ ] **Step 5: Add project switching methods**

```rust
    /// Switch to a different open project by index.
    pub fn switch_project(&mut self, idx: usize) {
        if idx >= self.open_projects.len() { return; }
        // Flush current inputs to workspace
        self.flush_inputs_to_workspace();
        self.active_project_idx = Some(idx);
        self.load_request_into_inputs();
    }

    pub fn next_project(&mut self) {
        if self.open_projects.is_empty() { return; }
        let current = self.active_project_idx.unwrap_or(0);
        let next = (current + 1) % self.open_projects.len();
        self.switch_project(next);
    }

    pub fn prev_project(&mut self) {
        if self.open_projects.is_empty() { return; }
        let current = self.active_project_idx.unwrap_or(0);
        let prev = if current == 0 { self.open_projects.len() - 1 } else { current - 1 };
        self.switch_project(prev);
    }

    /// Flush text input contents back to the active workspace's current_request.
    fn flush_inputs_to_workspace(&mut self) {
        let Some(ws) = self.active_workspace_mut() else { return; };
        if let Some(request) = &mut ws.data.current_request {
            request.url = self.url_input.content().to_string();
            // Sync body
            let body_content = self.body_input.content().to_string();
            if body_content.is_empty() {
                request.body = None;
            } else {
                request.body = Some(curl_tui_core::types::Body::Json { content: body_content });
            }
            // Sync headers
            for (i, header) in request.headers.iter_mut().enumerate() {
                if let Some(ki) = self.header_key_inputs.get(i) {
                    header.key = ki.content().to_string();
                }
                if let Some(vi) = self.header_value_inputs.get(i) {
                    header.value = vi.content().to_string();
                }
            }
            // Sync params
            for (i, param) in request.params.iter_mut().enumerate() {
                if let Some(ki) = self.param_key_inputs.get(i) {
                    param.key = ki.content().to_string();
                }
                if let Some(vi) = self.param_value_inputs.get(i) {
                    param.value = vi.content().to_string();
                }
            }
        }
    }

    /// Close an open project (remove from tab bar, don't delete from disk).
    pub fn close_project(&mut self, idx: usize) {
        if idx >= self.open_projects.len() { return; }
        self.open_projects.remove(idx);
        if self.open_projects.is_empty() {
            self.active_project_idx = None;
            self.show_project_picker = true; // Force user to pick/create
        } else if let Some(active) = self.active_project_idx {
            if active >= self.open_projects.len() {
                self.active_project_idx = Some(self.open_projects.len() - 1);
            } else if active > idx {
                self.active_project_idx = Some(active - 1);
            }
            self.load_request_into_inputs();
        }
    }
```

- [ ] **Step 6: Update `main.rs` initialization**

Replace the current data loading (lines 41-59) with project-aware loading:

```rust
    let mut app = App::new(config);
    let projects_dir = config_root.join("projects");

    // Load session — open previously open projects
    let project_slugs = app.config.open_projects.clone();
    if project_slugs.is_empty() {
        // Check if any projects exist at all
        let all = curl_tui_core::project::list_projects(&projects_dir).unwrap_or_default();
        if all.is_empty() {
            app.show_first_launch = true;
        } else {
            // Open the first project by default
            let (project, path) = all.into_iter().next().unwrap();
            let slug = path.file_name().unwrap().to_string_lossy().to_string();
            let mut ws = app::ProjectWorkspace::new(project, slug.clone());
            ws.collections = curl_tui_core::collection::list_collections(&path.join("collections"))
                .unwrap_or_default();
            ws.environments = curl_tui_core::environment::list_environments(&path.join("environments"))
                .unwrap_or_default();
            app.open_projects.push(ws);
            app.active_project_idx = Some(0);
        }
    } else {
        for slug in &project_slugs {
            let path = projects_dir.join(slug);
            match curl_tui_core::project::load_project(&path) {
                Ok(project) => {
                    let mut ws = app::ProjectWorkspace::new(project, slug.clone());
                    ws.collections = curl_tui_core::collection::list_collections(&path.join("collections"))
                        .unwrap_or_default();
                    ws.environments = curl_tui_core::environment::list_environments(&path.join("environments"))
                        .unwrap_or_default();
                    // Restore active environment
                    if let Some(env_name) = &ws.project.active_environment {
                        ws.active_environment = ws.environments.iter().position(|e| &e.name == env_name);
                    }
                    app.open_projects.push(ws);
                }
                Err(_) => continue, // Skip missing projects gracefully
            }
        }
        // Set active project
        if let Some(active_slug) = &app.config.active_project {
            app.active_project_idx = app.open_projects.iter().position(|ws| &ws.slug == active_slug);
        }
        if app.active_project_idx.is_none() && !app.open_projects.is_empty() {
            app.active_project_idx = Some(0);
        }
    }

    let keymap = input::build_keymap(&app.config.keybindings);
    app.load_request_into_inputs();
```

- [ ] **Step 7: Add session persistence on quit**

In `main.rs`, before `run_loop`, save session state on exit. Add after the `run_loop` call:

```rust
    // Save session state
    app.config.open_projects = app.open_projects.iter().map(|ws| ws.slug.clone()).collect();
    app.config.active_project = app.active_project_idx.and_then(|i| app.open_projects.get(i)).map(|ws| ws.slug.clone());
    let _ = app.config.save_to(&config_path);
```

- [ ] **Step 8: Verify compilation**

Run: `cargo build --workspace`
Expected: Compiles (may have warnings for unused project picker fields)

- [ ] **Step 9: Run tests**

Run: `cargo test --workspace`
Expected: All PASS

- [ ] **Step 10: Commit**

```bash
git add crates/curl-tui/src/app.rs crates/curl-tui/src/main.rs
git commit -m "refactor: extract ProjectWorkspace, refactor App to hold Vec<ProjectWorkspace>"
```

---

## Task 9: Update UI rendering for workspace-based access

**Files:**
- Modify: `crates/curl-tui/src/ui/mod.rs`
- Modify: `crates/curl-tui/src/ui/collections.rs`
- Modify: `crates/curl-tui/src/ui/request.rs`
- Modify: `crates/curl-tui/src/ui/response.rs`
- Modify: `crates/curl-tui/src/ui/variables.rs`
- Modify: `crates/curl-tui/src/ui/picker.rs`
- Modify: `crates/curl-tui/src/ui/statusbar.rs`

- [ ] **Step 1: Update all UI modules to access data through `active_workspace()`**

Every UI module that reads `app.collections`, `app.environments`, `app.selected_collection`, `app.selected_request`, `app.current_request`, `app.last_response`, `app.active_environment`, `app.request_tab`, `app.response_tab`, `app.collection_scroll`, `app.response_scroll`, `app.var_collection_idx`, `app.var_environment_idx` must be updated to go through `app.active_workspace()`.

Pattern for read-only UI access:
```rust
// Before:
let collections = &app.collections;
// After:
let ws = match app.active_workspace() {
    Some(ws) => ws,
    None => return, // No active project — skip rendering this pane
};
let collections = &ws.data.collections;
```

When `active_workspace()` returns `None` and `show_first_launch` is false, render a centered message in the main area: "No project open — press Ctrl+O to open or create one". This avoids a blank screen.

- [ ] **Step 2: Update `ui/mod.rs` draw function**

Update the env_name extraction:
```rust
let env_name = app.active_workspace()
    .and_then(|ws| ws.active_environment.and_then(|i| ws.environments.get(i)))
    .map(|e| e.name.as_str())
    .unwrap_or("None");
```

- [ ] **Step 3: Update statusbar hints**

In `statusbar.rs`, add project-related hints to the Normal mode section:
```rust
Span::styled("F6", key_style), Span::raw(":project  "),
Span::styled("Ctrl+O", key_style), Span::raw(":projects  "),
```

- [ ] **Step 4: Verify compilation and test**

Run: `cargo build --workspace && cargo test --workspace`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui/src/ui/
git commit -m "refactor: update all UI modules to access data through active_workspace()"
```

---

## Task 10: Update `main.rs` dispatch for project actions

**Files:**
- Modify: `crates/curl-tui/src/main.rs`

- [ ] **Step 1: Add project action dispatch to `run_loop`**

In the normal dispatch match block (the `else` branch that handles `Action::*`), add before the existing arms:

```rust
Action::NextProject => app.next_project(),
Action::PrevProject => app.prev_project(),
Action::OpenProjectPicker => {
    // Refresh the project list from disk
    let projects_dir = config_dir().join("projects");
    app.all_projects = curl_tui_core::project::list_projects(&projects_dir)
        .unwrap_or_default()
        .into_iter()
        .map(|(p, path)| {
            let slug = path.file_name().unwrap().to_string_lossy().to_string();
            (p, slug)
        })
        .collect();
    app.project_picker_cursor = 0;
    app.show_project_picker = true;
},
Action::CloseProject => {
    if let Some(idx) = app.active_project_idx {
        app.close_project(idx);
    }
},
```

- [ ] **Step 2: Add project picker dispatch block**

In the `run_loop` input dispatch, add a new block for the project picker (after the `show_collection_picker` check, before the `show_variables` check):

```rust
} else if app.show_project_picker {
    match action {
        Action::Cancel => {
            app.show_project_picker = false;
        }
        Action::MoveUp => {
            if app.project_picker_cursor > 0 {
                app.project_picker_cursor -= 1;
            }
        }
        Action::MoveDown => {
            if app.project_picker_cursor + 1 < app.all_projects.len() {
                app.project_picker_cursor += 1;
            }
        }
        Action::Enter => {
            if let Some((project, slug)) = app.all_projects.get(app.project_picker_cursor).cloned() {
                // Check if already open
                if let Some(idx) = app.open_projects.iter().position(|ws| ws.slug == slug) {
                    app.switch_project(idx);
                } else {
                    // Open the project
                    let path = config_dir().join("projects").join(&slug);
                    let mut ws = app::ProjectWorkspace::new(project, slug);
                    ws.collections = curl_tui_core::collection::list_collections(&path.join("collections"))
                        .unwrap_or_default();
                    ws.environments = curl_tui_core::environment::list_environments(&path.join("environments"))
                        .unwrap_or_default();
                    app.open_projects.push(ws);
                    let idx = app.open_projects.len() - 1;
                    app.switch_project(idx);
                }
                app.show_project_picker = false;
            }
        }
        Action::NewRequest | Action::CharInput('n') => {
            // Create new project
            app.show_project_picker = false;
            app.name_input.set_content("New Project");
            app.start_editing(EditField::NewProjectName);
            app.status_message = Some("Name your project".to_string());
        }
        Action::DeleteItem | Action::CharInput('d') => {
            // Close project (remove from tab bar)
            if let Some((_, slug)) = app.all_projects.get(app.project_picker_cursor) {
                if let Some(idx) = app.open_projects.iter().position(|ws| &ws.slug == slug) {
                    app.close_project(idx);
                }
            }
        }
        Action::Quit => app.should_quit = true,
        _ => {}
    }
} else if app.show_variables {
```

(This replaces the existing `} else if app.show_variables {` block — the project picker check is inserted before it.)

Note: This requires adding `NewProjectName` to the `EditField` enum and handling it in `stop_editing`.

- [ ] **Step 3: Add `EditField::NewProjectName` variant**

In `app.rs`, add to the `EditField` enum:
```rust
    NewProjectName,
```

And add handling in `stop_editing` / `sync_field_to_request`:
```rust
EditField::NewProjectName => {
    // Handled by finalize_new_project — no sync needed
}
```

Add `finalize_new_project` method:
```rust
fn finalize_new_project(&mut self) {
    let name = self.name_input.content().to_string();
    let name = if name.is_empty() { "New Project".to_string() } else { name };
    let project = curl_tui_core::types::Project {
        id: uuid::Uuid::new_v4(),
        name: name.clone(),
        active_environment: None,
    };
    let projects_dir = config_dir().join("projects");
    match curl_tui_core::project::create_project(&projects_dir, &project) {
        Ok(dir) => {
            let slug = dir.file_name().unwrap().to_string_lossy().to_string();
            let ws = ProjectWorkspace::new(project, slug);
            self.open_projects.push(ws);
            let idx = self.open_projects.len() - 1;
            self.switch_project(idx);
            self.status_message = Some(format!("Created project '{}'", name));
        }
        Err(e) => self.status_message = Some(format!("Error: {}", e)),
    }
}
```

And update `stop_editing` to handle `NewProjectName`:
```rust
if field == EditField::NewCollectionName {
    self.finalize_new_collection();
} else if field == EditField::NewProjectName {
    self.finalize_new_project();
} else {
    self.sync_field_to_request(field);
}
```

- [ ] **Step 4: Add first-launch dispatch**

Add at the top of the input dispatch chain in `run_loop`:

```rust
if app.show_first_launch {
    match action {
        Action::Enter => {
            if app.input_mode == app::InputMode::Editing {
                app.stop_editing(); // This calls finalize_new_project
                app.show_first_launch = false;
            }
        }
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
        Action::Quit => app.should_quit = true,
        _ => {}
    }
} else if app.show_project_picker {
```

Also in `main.rs` initialization, when `show_first_launch` is true, start editing:
```rust
if app.show_first_launch {
    app.name_input.set_content("My Project");
    app.start_editing(EditField::NewProjectName);
}
```

- [ ] **Step 5: Verify build and tests**

Run: `cargo build --workspace && cargo test --workspace`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add crates/curl-tui/src/main.rs crates/curl-tui/src/app.rs
git commit -m "feat: add project picker dispatch, first-launch modal, and project CRUD in UI"
```

---

## Task 11: Render project tab bar in title bar

**Files:**
- Create: `crates/curl-tui/src/ui/project_tabs.rs`
- Modify: `crates/curl-tui/src/ui/mod.rs`

- [ ] **Step 1: Create `project_tabs.rs`**

```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();

    let active_idx = app.active_project_idx;
    let tab_scroll = app.project_tab_scroll;

    // Calculate how many tabs fit
    // Each tab: " [ Name ] " = name.len() + 6 chars
    let env_indicator_width = 15; // "  env: SomeName" approx
    let available = area.width.saturating_sub(env_indicator_width as u16) as usize;

    let tabs: Vec<(usize, &str)> = app.open_projects.iter().enumerate()
        .map(|(i, ws)| (i, ws.project.name.as_str()))
        .collect();

    let mut used_width = 0usize;
    let mut visible_start = tab_scroll;
    let mut visible_end = tabs.len();
    let show_left_arrow = visible_start > 0;

    if show_left_arrow {
        spans.push(Span::styled(" ◀ ", Style::default().fg(Color::DarkGray)));
        used_width += 3;
    }

    for (i, (idx, name)) in tabs.iter().enumerate().skip(visible_start) {
        let tab_width = name.len() + 6; // " [ name ] "
        if used_width + tab_width > available {
            visible_end = i;
            break;
        }
        let is_active = Some(*idx) == active_idx;
        if is_active {
            spans.push(Span::styled(
                format!(" [ {} ] ", name),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" [ {} ] ", name),
                Style::default().fg(Color::Gray),
            ));
        }
        used_width += tab_width;
    }

    let show_right_arrow = visible_end < tabs.len();
    if show_right_arrow {
        spans.push(Span::styled(" ▶ ", Style::default().fg(Color::DarkGray)));
    }

    // [+] button
    spans.push(Span::styled(" [+] ", Style::default().fg(Color::DarkGray)));

    // Right-align: environment indicator
    let env_name = app.active_workspace()
        .and_then(|ws| ws.active_environment.and_then(|i| ws.environments.get(i)))
        .map(|e| e.name.as_str())
        .unwrap_or("None");

    // Fill remaining space
    let total_used: usize = spans.iter().map(|s| s.content.len()).sum();
    let env_text = format!("env: {}", env_name);
    let padding = area.width as usize - total_used.min(area.width as usize) - env_text.len();
    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding)));
    }
    spans.push(Span::styled(env_text, Style::default().fg(Color::Yellow)));

    let line = Line::from(spans);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Black)),
        area,
    );
}
```

- [ ] **Step 2: Wire into `ui/mod.rs`**

Add `pub mod project_tabs;` at the top.

Replace the title bar rendering block in `draw()`:
```rust
// Title bar — now shows project tabs + env
project_tabs::draw(frame, app, pane_layout.title_bar);
```

Remove the old title bar Paragraph code.

- [ ] **Step 3: Verify visually**

Run: `cargo run -p curl-tui`
Expected: Title bar shows project tabs with the active project highlighted and env indicator on the right.

- [ ] **Step 4: Commit**

```bash
git add crates/curl-tui/src/ui/project_tabs.rs crates/curl-tui/src/ui/mod.rs
git commit -m "feat: render project tab bar in title bar row"
```

---

## Task 12: Render project picker overlay

**Files:**
- Create: `crates/curl-tui/src/ui/project_picker.rs`
- Modify: `crates/curl-tui/src/ui/mod.rs`

- [ ] **Step 1: Create `project_picker.rs`**

Follow the same pattern as `picker.rs` (the existing collection picker overlay):

```rust
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    // Center a box — 50% width, 60% height
    let popup_width = (area.width * 50 / 100).max(30).min(area.width);
    let popup_height = (area.height * 60 / 100).max(10).min(area.height);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Projects (Ctrl+O) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let open_slugs: Vec<&str> = app.open_projects.iter().map(|ws| ws.slug.as_str()).collect();

    let items: Vec<ListItem> = app.all_projects.iter().enumerate().map(|(i, (project, slug))| {
        let is_open = open_slugs.contains(&slug.as_str());
        let marker = if is_open { "● " } else { "  " };
        let style = if i == app.project_picker_cursor {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if is_open {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };
        ListItem::new(Line::from(vec![
            Span::raw(marker),
            Span::styled(&project.name, style),
        ]))
    }).collect();

    let list = List::new(items);
    frame.render_widget(list, inner);

    // Hints at the bottom
    // (Rendered outside inner if space allows)
}
```

- [ ] **Step 2: Wire into `ui/mod.rs`**

Add `pub mod project_picker;` and in `draw()`, add after the variables overlay check:

```rust
if app.show_project_picker {
    project_picker::draw(frame, app);
}
```

- [ ] **Step 3: Render first-launch modal**

In `ui/mod.rs` `draw()`, add:
```rust
if app.show_first_launch {
    // Simple centered dialog
    let area = frame.area();
    let w = 50u16.min(area.width);
    let h = 5u16.min(area.height);
    let x = (area.width - w) / 2;
    let y = (area.height - h) / 2;
    let popup = Rect::new(x, y, w, h);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Welcome to curl-tui! ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    // Render name input
    let label = Span::styled("Project name: ", Style::default().fg(Color::Yellow));
    let content = Span::raw(app.name_input.content());
    frame.render_widget(Paragraph::new(Line::from(vec![label, content])), inner);
}
```

- [ ] **Step 4: Verify visually**

Run: `cargo run -p curl-tui`
Test: Press Ctrl+O to see the project picker. On first launch with no projects, the first-launch modal should appear.

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui/src/ui/project_picker.rs crates/curl-tui/src/ui/mod.rs
git commit -m "feat: add project picker overlay and first-launch modal"
```

---

## Task 13: Update help overlay

**Files:**
- Modify: `crates/curl-tui/src/ui/help.rs`

- [ ] **Step 1: Add project keybindings to help**

Read the current help.rs, then add a "Projects" section with:
- `F6` — Next project
- `F7` — Previous project
- `Ctrl+O` — Open project picker
- `d` (in picker) — Close project

- [ ] **Step 2: Commit**

```bash
git add crates/curl-tui/src/ui/help.rs
git commit -m "feat: add project keybindings to help overlay"
```

---

## Task 14: Add integration tests for project workflows

**Files:**
- Modify: `crates/curl-tui-core/tests/integration_test.rs`

- [ ] **Step 1: Write integration tests**

```rust
#[test]
fn test_project_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let projects_dir = tmp.path().join("projects");

    // Create project
    let project = curl_tui_core::types::Project {
        id: uuid::Uuid::new_v4(),
        name: "Test Project".to_string(),
        active_environment: None,
    };
    let dir = curl_tui_core::project::create_project(&projects_dir, &project).unwrap();

    // Add a collection to the project
    let collection = curl_tui_core::types::Collection {
        id: uuid::Uuid::new_v4(),
        name: "API".to_string(),
        variables: std::collections::HashMap::new(),
        requests: vec![],
    };
    curl_tui_core::collection::save_collection(&dir.join("collections"), &collection).unwrap();

    // Add an environment
    let env = curl_tui_core::types::Environment {
        id: uuid::Uuid::new_v4(),
        name: "Dev".to_string(),
        variables: std::collections::HashMap::new(),
    };
    curl_tui_core::environment::save_environment(&dir.join("environments"), &env).unwrap();

    // List should show 1 collection, 1 environment
    let cols = curl_tui_core::collection::list_collections(&dir.join("collections")).unwrap();
    assert_eq!(cols.len(), 1);
    let envs = curl_tui_core::environment::list_environments(&dir.join("environments")).unwrap();
    assert_eq!(envs.len(), 1);

    // Delete project
    curl_tui_core::project::delete_project(&dir).unwrap();
    assert!(!dir.exists());
}

#[test]
fn test_migration_then_project_load() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Set up flat structure
    curl_tui_core::init::initialize(root).unwrap();
    let col = curl_tui_core::types::Collection {
        id: uuid::Uuid::new_v4(),
        name: "Legacy".to_string(),
        variables: std::collections::HashMap::new(),
        requests: vec![],
    };
    curl_tui_core::collection::save_collection(&root.join("collections"), &col).unwrap();

    // Migration should have run during init if needed, but let's trigger manually
    if curl_tui_core::migration::needs_migration(root) {
        curl_tui_core::migration::migrate_flat_to_project(root).unwrap();
    }

    // Now load the migrated project
    let projects = curl_tui_core::project::list_projects(&root.join("projects")).unwrap();
    assert_eq!(projects.len(), 1);
    let (project, path) = &projects[0];
    assert_eq!(project.name, "Default");

    let cols = curl_tui_core::collection::list_collections(&path.join("collections")).unwrap();
    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0].name, "Legacy");
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p curl-tui-core -- integration_test`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add crates/curl-tui-core/tests/integration_test.rs
git commit -m "test: add integration tests for project lifecycle and migration"
```

---

## Task 15: Final verification and polish

- [ ] **Step 1: Run full verification**

Run: `cargo fmt --all --check && cargo clippy --workspace -- -D warnings && cargo test --workspace`
Expected: All pass with no warnings.

- [ ] **Step 2: Fix any clippy warnings or formatting issues**

- [ ] **Step 3: Manual TUI testing checklist**

1. Delete `%APPDATA%/curl-tui/` (or backup) and launch — first-launch modal should appear
2. Create a project — tab appears in title bar with `env: None`
3. Create a collection, add a request, save it
4. Press Ctrl+O — project picker opens, shows the project with `●` marker
5. Create a second project from the picker (Ctrl+N)
6. F6 switches to the second project — its collections/environments are independent
7. F7 switches back
8. Close the app and relaunch — both projects should be open (session restored)
9. Close one project with `d` in the picker — tab removed, project still on disk
10. Ctrl+O shows both projects, the closed one without `●`

- [ ] **Step 4: Commit any polish fixes**

```bash
git add -A
git commit -m "fix: polish project layer implementation"
```

---

## Verification Summary

| What | Command | Expected |
|---|---|---|
| Format | `cargo fmt --all --check` | No changes needed |
| Lint | `cargo clippy --workspace -- -D warnings` | No warnings |
| Unit tests | `cargo test --workspace` | All pass |
| Core tests | `cargo test -p curl-tui-core` | ~85+ tests pass |
| Integration | `cargo test -p curl-tui-core -- integration_test` | All pass |
| Manual TUI | Launch and test checklist above | All scenarios work |
