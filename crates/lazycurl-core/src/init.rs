use crate::config::AppConfig;
use crate::secret;
use std::path::Path;

/// Initialize the lazycurl config directory if it doesn't exist.
/// Creates config.json (with defaults), collections/, environments/, and .gitignore.
/// Does NOT overwrite existing files.
pub fn initialize(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(root)?;
    std::fs::create_dir_all(root.join("collections"))?;
    std::fs::create_dir_all(root.join("environments"))?;
    std::fs::create_dir_all(root.join("projects"))?;

    // config.json — only create if missing
    let config_path = root.join("config.json");
    if !config_path.exists() {
        let config = AppConfig::default();
        config.save_to(&config_path)?;
    }

    // .gitignore — only create if missing
    let gitignore_path = root.join(".gitignore");
    if !gitignore_path.exists() {
        std::fs::write(&gitignore_path, secret::generate_gitignore())?;
    }

    // Migrate flat structure to project-based layout if needed
    if crate::migration::needs_migration(root) {
        crate::migration::migrate_flat_to_project(root)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_creates_directory_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("lazycurl");

        initialize(&root).unwrap();

        assert!(root.join("config.json").exists());
        assert!(root.join("collections").is_dir());
        assert!(root.join("environments").is_dir());
        assert!(root.join(".gitignore").exists());
    }

    #[test]
    fn test_init_does_not_overwrite_existing_config() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("lazycurl");

        initialize(&root).unwrap();

        // Modify config
        let config_path = root.join("config.json");
        std::fs::write(&config_path, r#"{"default_timeout": 99}"#).unwrap();

        // Re-init should not overwrite
        initialize(&root).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("99"));
    }

    #[test]
    fn test_gitignore_contains_environments() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("lazycurl");

        initialize(&root).unwrap();

        let gitignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(gitignore.contains("environments/"));
    }
}
