use crate::config::AppConfig;
use crate::types::Project;
use std::path::Path;

/// Returns true if the root directory has a flat structure that needs migration.
/// Specifically: root `collections/` or `environments/` have `.json` files AND
/// `projects/` has no project dirs AND no `.migration-complete` marker.
pub fn needs_migration(root: &Path) -> bool {
    // If the marker exists, migration is already done
    if root.join("projects/.migration-complete").exists() {
        return false;
    }

    // Check if projects/ already has project directories
    let projects_dir = root.join("projects");
    if projects_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    return false; // Already has project directories
                }
            }
        }
    }

    // Check if there are any .json files in root collections/ or environments/
    for dir_name in &["collections", "environments"] {
        let dir = root.join(dir_name);
        if dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("json") {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Migrate the flat structure to project-based layout.
/// Copies (not moves) json files from root collections/environments into `projects/default/`,
/// copies history.jsonl, updates config.json with session state, writes marker file,
/// THEN deletes originals.
pub fn migrate_flat_to_project(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let projects_dir = root.join("projects");
    let default_dir = projects_dir.join("default");

    // 1. Create projects/default/ with collections/ and environments/ subdirs
    std::fs::create_dir_all(default_dir.join("collections"))?;
    std::fs::create_dir_all(default_dir.join("environments"))?;

    // 2. Read config.json to get active_environment
    let config_path = root.join("config.json");
    let mut config = AppConfig::load_from(&config_path)?;
    let active_env = config.active_environment.clone();

    // 3. Write project.json with name "Default" and migrated active_environment
    let project = Project {
        id: uuid::Uuid::new_v4(),
        name: "Default".to_string(),
        active_environment: active_env,
    };
    let project_json = serde_json::to_string_pretty(&project)?;
    std::fs::write(default_dir.join("project.json"), project_json)?;

    // 4. Copy all .json files from root collections/ to projects/default/collections/
    let src_collections = root.join("collections");
    if src_collections.exists() {
        for entry in std::fs::read_dir(&src_collections)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let file_name = path.file_name().unwrap();
                std::fs::copy(&path, default_dir.join("collections").join(file_name))?;
            }
        }
    }

    // 5. Copy all .json files from root environments/ to projects/default/environments/
    let src_environments = root.join("environments");
    if src_environments.exists() {
        for entry in std::fs::read_dir(&src_environments)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let file_name = path.file_name().unwrap();
                std::fs::copy(&path, default_dir.join("environments").join(file_name))?;
            }
        }
    }

    // 6. Copy root history.jsonl to projects/default/history.jsonl (keep original as global)
    let src_history = root.join("history.jsonl");
    if src_history.exists() {
        std::fs::copy(&src_history, default_dir.join("history.jsonl"))?;
    }

    // 7. Update config.json: set open_projects, active_project, clear active_environment
    config.open_projects = vec!["default".to_string()];
    config.active_project = Some("default".to_string());
    config.active_environment = None;

    // 8. Save config
    config.save_to(&config_path)?;

    // 9. Write projects/.migration-complete marker
    std::fs::write(projects_dir.join(".migration-complete"), "")?;

    // 10. Delete original .json files from root collections/ and environments/
    if src_collections.exists() {
        for entry in std::fs::read_dir(&src_collections)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                std::fs::remove_file(&path)?;
            }
        }
    }
    if src_environments.exists() {
        for entry in std::fs::read_dir(&src_environments)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                std::fs::remove_file(&path)?;
            }
        }
    }

    Ok(())
}

/// Returns true if any collection file still uses the old `"json"` or `"text"` body types
/// and the body-type migration marker has not been written yet.
pub fn needs_body_type_migration(root: &Path) -> bool {
    if root.join("projects/.body-type-migration-complete").exists() {
        return false;
    }

    let projects_dir = root.join("projects");
    if !projects_dir.exists() {
        return false;
    }

    if let Ok(project_entries) = std::fs::read_dir(&projects_dir) {
        for project_entry in project_entries.flatten() {
            let collections_dir = project_entry.path().join("collections");
            if !collections_dir.is_dir() {
                continue;
            }
            if let Ok(col_entries) = std::fs::read_dir(&collections_dir) {
                for col_entry in col_entries.flatten() {
                    let path = col_entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("json") {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(requests) = val.get("requests").and_then(|r| r.as_array()) {
                                for req in requests {
                                    if let Some(body_type) = req
                                        .get("body")
                                        .and_then(|b| b.get("type"))
                                        .and_then(|t| t.as_str())
                                    {
                                        if body_type == "json" || body_type == "text" {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    false
}

/// Migrate old `"json"` and `"text"` body types to `"raw"` with a `content_type` field.
pub fn migrate_body_types(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let projects_dir = root.join("projects");

    if let Ok(project_entries) = std::fs::read_dir(&projects_dir) {
        for project_entry in project_entries.flatten() {
            let collections_dir = project_entry.path().join("collections");
            if !collections_dir.is_dir() {
                continue;
            }
            if let Ok(col_entries) = std::fs::read_dir(&collections_dir) {
                for col_entry in col_entries.flatten() {
                    let path = col_entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("json") {
                        continue;
                    }
                    let content = std::fs::read_to_string(&path)?;
                    let mut val: serde_json::Value = serde_json::from_str(&content)?;
                    let mut changed = false;

                    if let Some(requests) = val.get_mut("requests").and_then(|r| r.as_array_mut()) {
                        for req in requests.iter_mut() {
                            if let Some(body) = req.get_mut("body") {
                                if let Some(body_type) =
                                    body.get("type").and_then(|t| t.as_str()).map(String::from)
                                {
                                    if body_type == "json" || body_type == "text" {
                                        let content_val = body
                                            .get("content")
                                            .cloned()
                                            .unwrap_or(serde_json::Value::String(String::new()));
                                        *body = serde_json::json!({
                                            "type": "raw",
                                            "content": content_val,
                                            "content_type": body_type,
                                        });
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }

                    if changed {
                        let updated = serde_json::to_string_pretty(&val)?;
                        std::fs::write(&path, updated)?;
                    }
                }
            }
        }
    }

    // Write marker
    std::fs::write(projects_dir.join(".body-type-migration-complete"), "")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_migration_true() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("collections")).unwrap();
        std::fs::create_dir_all(root.join("projects")).unwrap();
        std::fs::write(
            root.join("collections/test.json"),
            r#"{"id":"00000000-0000-0000-0000-000000000001","name":"Test","requests":[]}"#,
        )
        .unwrap();
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
        std::fs::create_dir_all(root.join("projects")).unwrap();
        std::fs::write(root.join("projects/.migration-complete"), "").unwrap();
        assert!(!needs_migration(root));
    }

    #[test]
    fn test_migrate_flat_to_project() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Create flat structure with config
        std::fs::create_dir_all(root.join("collections")).unwrap();
        std::fs::create_dir_all(root.join("environments")).unwrap();
        std::fs::create_dir_all(root.join("projects")).unwrap();

        // Write a config with active_environment
        let config = crate::config::AppConfig::default();
        config.save_to(&root.join("config.json")).unwrap();

        std::fs::write(
            root.join("collections/my-api.json"),
            r#"{"id":"00000000-0000-0000-0000-000000000001","name":"My API","requests":[]}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("environments/dev.json"),
            r#"{"id":"00000000-0000-0000-0000-000000000002","name":"Dev","variables":{}}"#,
        )
        .unwrap();
        std::fs::write(root.join("history.jsonl"), "line1\nline2\n").unwrap();

        migrate_flat_to_project(root).unwrap();

        let default_dir = root.join("projects/default");
        assert!(default_dir.join("project.json").exists());
        assert!(default_dir.join("collections/my-api.json").exists());
        assert!(default_dir.join("environments/dev.json").exists());
        assert!(default_dir.join("history.jsonl").exists());
        assert!(root.join("history.jsonl").exists()); // global history kept
        assert!(root.join("projects/.migration-complete").exists());

        // Old dirs should be empty
        let col_count = std::fs::read_dir(root.join("collections")).unwrap().count();
        assert_eq!(col_count, 0);

        // Config should have session state
        let config = crate::config::AppConfig::load_from(&root.join("config.json")).unwrap();
        assert_eq!(config.open_projects, vec!["default"]);
        assert_eq!(config.active_project, Some("default".to_string()));
    }

    #[test]
    fn test_migration_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("collections")).unwrap();
        std::fs::create_dir_all(root.join("projects")).unwrap();
        let config = crate::config::AppConfig::default();
        config.save_to(&root.join("config.json")).unwrap();
        std::fs::write(
            root.join("collections/test.json"),
            r#"{"id":"00000000-0000-0000-0000-000000000001","name":"Test","requests":[]}"#,
        )
        .unwrap();

        migrate_flat_to_project(root).unwrap();
        assert!(!needs_migration(root));
    }

    #[test]
    fn needs_body_type_migration_with_old_json_body() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let project_dir = root.join("projects/default/collections");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(root.join("projects/.migration-complete"), "").unwrap();

        let collection = serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "name": "Test",
            "requests": [{
                "id": "00000000-0000-0000-0000-000000000002",
                "name": "Old Request",
                "method": "GET",
                "url": "https://example.com",
                "headers": [],
                "params": [],
                "body": {"type": "json", "content": "{\"key\": \"value\"}"}
            }]
        });
        std::fs::write(
            project_dir.join("test.json"),
            serde_json::to_string_pretty(&collection).unwrap(),
        )
        .unwrap();

        assert!(needs_body_type_migration(root));
    }

    #[test]
    fn no_body_type_migration_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("projects")).unwrap();
        std::fs::write(root.join("projects/.body-type-migration-complete"), "").unwrap();
        assert!(!needs_body_type_migration(root));
    }

    #[test]
    fn migrate_body_types_converts_json_and_text() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let project_dir = root.join("projects/default/collections");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(root.join("projects/.migration-complete"), "").unwrap();

        let collection = serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "name": "Test",
            "requests": [
                {
                    "id": "00000000-0000-0000-0000-000000000002",
                    "name": "JSON Request",
                    "method": "POST",
                    "url": "https://api.example.com",
                    "headers": [],
                    "params": [],
                    "body": {"type": "json", "content": "{\"key\": \"value\"}"}
                },
                {
                    "id": "00000000-0000-0000-0000-000000000003",
                    "name": "Text Request",
                    "method": "POST",
                    "url": "https://api.example.com",
                    "headers": [],
                    "params": [],
                    "body": {"type": "text", "content": "hello world"}
                }
            ]
        });
        std::fs::write(
            project_dir.join("test.json"),
            serde_json::to_string_pretty(&collection).unwrap(),
        )
        .unwrap();

        migrate_body_types(root).unwrap();

        let data: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(project_dir.join("test.json")).unwrap())
                .unwrap();

        let req0_body = &data["requests"][0]["body"];
        assert_eq!(req0_body["type"], "raw");
        assert_eq!(req0_body["content"], "{\"key\": \"value\"}");
        assert_eq!(req0_body["content_type"], "json");

        let req1_body = &data["requests"][1]["body"];
        assert_eq!(req1_body["type"], "raw");
        assert_eq!(req1_body["content"], "hello world");
        assert_eq!(req1_body["content_type"], "text");

        assert!(root.join("projects/.body-type-migration-complete").exists());
        assert!(!needs_body_type_migration(root));
    }

    #[test]
    fn migrate_body_types_leaves_form_and_none_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let project_dir = root.join("projects/default/collections");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(root.join("projects/.migration-complete"), "").unwrap();

        let collection = serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "name": "Test",
            "requests": [
                {
                    "id": "00000000-0000-0000-0000-000000000002",
                    "name": "Form Request",
                    "method": "POST",
                    "url": "https://api.example.com",
                    "headers": [],
                    "params": [],
                    "body": {"type": "form", "fields": [{"key": "name", "value": "test", "enabled": true}]}
                },
                {
                    "id": "00000000-0000-0000-0000-000000000003",
                    "name": "No Body",
                    "method": "GET",
                    "url": "https://api.example.com",
                    "headers": [],
                    "params": []
                }
            ]
        });
        std::fs::write(
            project_dir.join("test.json"),
            serde_json::to_string_pretty(&collection).unwrap(),
        )
        .unwrap();

        migrate_body_types(root).unwrap();

        let data: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(project_dir.join("test.json")).unwrap())
                .unwrap();

        assert_eq!(data["requests"][0]["body"]["type"], "form");
        assert!(data["requests"][1]["body"].is_null());
    }
}
