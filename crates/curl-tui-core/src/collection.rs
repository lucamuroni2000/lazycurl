use crate::types::Collection;
use std::path::{Path, PathBuf};

/// Convert a collection name to a filesystem-safe slug.
pub fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("-")
}

/// Find the file path for a collection, handling slug collisions.
fn find_path_for(dir: &Path, collection: &Collection) -> PathBuf {
    let base_slug = slugify(&collection.name);

    // If a file already exists with this collection's id, reuse its path
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if let Ok(existing) = serde_json::from_str::<Collection>(&content) {
                    if existing.id == collection.id {
                        return entry.path();
                    }
                }
            }
        }
    }

    // Try base slug
    let candidate = dir.join(format!("{}.json", base_slug));
    if !candidate.exists() {
        return candidate;
    }

    // Add numeric suffix
    let mut counter = 2;
    loop {
        let candidate = dir.join(format!("{}-{}.json", base_slug, counter));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

/// Save a collection to the given directory.
pub fn save_collection(
    dir: &Path,
    collection: &Collection,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let path = find_path_for(dir, collection);
    let content = serde_json::to_string_pretty(collection)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Load a collection from a specific file path.
pub fn load_collection(path: &Path) -> Result<Collection, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let collection: Collection = serde_json::from_str(&content)?;
    Ok(collection)
}

/// List all collections in a directory.
pub fn list_collections(dir: &Path) -> Result<Vec<Collection>, Box<dyn std::error::Error>> {
    let mut collections = Vec::new();
    if !dir.exists() {
        return Ok(collections);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            match load_collection(&path) {
                Ok(col) => collections.push(col),
                Err(_) => continue, // skip malformed files
            }
        }
    }
    Ok(collections)
}

/// Delete a collection file.
pub fn delete_collection(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Collection, Method, Request};

    #[test]
    fn test_slugify_simple() {
        assert_eq!(slugify("My API"), "my-api");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("My API (v2)!"), "my-api-v2");
    }

    #[test]
    fn test_slugify_multiple_spaces() {
        assert_eq!(slugify("  My   Cool   API  "), "my-cool-api");
    }

    #[test]
    fn test_slugify_unicode() {
        assert_eq!(slugify("API Test"), "api-test");
    }

    #[test]
    fn test_create_and_load_collection() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("collections");

        let collection = Collection {
            id: uuid::Uuid::new_v4(),
            name: "Test API".to_string(),
            variables: std::collections::HashMap::new(),
            requests: vec![],
        };

        save_collection(&dir, &collection).unwrap();
        let loaded = load_collection(&dir.join("test-api.json")).unwrap();
        assert_eq!(loaded.name, "Test API");
        assert_eq!(loaded.id, collection.id);
    }

    #[test]
    fn test_list_collections() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("collections");

        let c1 = Collection {
            id: uuid::Uuid::new_v4(),
            name: "First".to_string(),
            variables: std::collections::HashMap::new(),
            requests: vec![],
        };
        let c2 = Collection {
            id: uuid::Uuid::new_v4(),
            name: "Second".to_string(),
            variables: std::collections::HashMap::new(),
            requests: vec![],
        };

        save_collection(&dir, &c1).unwrap();
        save_collection(&dir, &c2).unwrap();

        let list = list_collections(&dir).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_delete_collection() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("collections");

        let collection = Collection {
            id: uuid::Uuid::new_v4(),
            name: "Delete Me".to_string(),
            variables: std::collections::HashMap::new(),
            requests: vec![],
        };

        save_collection(&dir, &collection).unwrap();
        let path = dir.join("delete-me.json");
        assert!(path.exists());

        delete_collection(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_slug_collision_adds_suffix() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("collections");

        let c1 = Collection {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            variables: std::collections::HashMap::new(),
            requests: vec![],
        };
        save_collection(&dir, &c1).unwrap();

        // Save another with the same name but different id
        let c2 = Collection {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            variables: std::collections::HashMap::new(),
            requests: vec![],
        };
        save_collection(&dir, &c2).unwrap();

        // Both should exist
        assert!(dir.join("test.json").exists());
        assert!(dir.join("test-2.json").exists());
    }

    #[test]
    fn test_save_existing_collection_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("collections");

        let mut collection = Collection {
            id: uuid::Uuid::new_v4(),
            name: "My API".to_string(),
            variables: std::collections::HashMap::new(),
            requests: vec![],
        };
        save_collection(&dir, &collection).unwrap();

        // Add a request and save again (same id)
        collection.requests.push(Request {
            id: uuid::Uuid::new_v4(),
            name: "New".to_string(),
            method: Method::Get,
            url: "http://test.com".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: None,
        });
        save_collection(&dir, &collection).unwrap();

        let loaded = load_collection(&dir.join("my-api.json")).unwrap();
        assert_eq!(loaded.requests.len(), 1);
    }
}
