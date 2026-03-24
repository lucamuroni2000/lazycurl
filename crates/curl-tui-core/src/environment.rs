use crate::collection::slugify;
use crate::types::Environment;
use std::path::Path;

/// Save an environment to the given directory.
pub fn save_environment(dir: &Path, env: &Environment) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let filename = format!("{}.json", slugify(&env.name));
    let path = dir.join(filename);
    let content = serde_json::to_string_pretty(env)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Load an environment from a specific file path.
pub fn load_environment(path: &Path) -> Result<Environment, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let env: Environment = serde_json::from_str(&content)?;
    Ok(env)
}

/// List all environments in a directory.
pub fn list_environments(dir: &Path) -> Result<Vec<Environment>, Box<dyn std::error::Error>> {
    let mut environments = Vec::new();
    if !dir.exists() {
        return Ok(environments);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            match load_environment(&path) {
                Ok(env) => environments.push(env),
                Err(_) => continue,
            }
        }
    }
    Ok(environments)
}

/// Delete an environment file.
pub fn delete_environment(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Environment, Variable};
    use std::collections::HashMap;

    fn make_env(name: &str) -> Environment {
        let mut vars = HashMap::new();
        vars.insert(
            "base_url".to_string(),
            Variable {
                value: "http://localhost".to_string(),
                secret: false,
            },
        );
        Environment {
            id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            variables: vars,
        }
    }

    #[test]
    fn test_save_and_load_environment() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("environments");

        let env = make_env("Development");
        save_environment(&dir, &env).unwrap();

        let loaded = load_environment(&dir.join("development.json")).unwrap();
        assert_eq!(loaded.name, "Development");
        assert!(loaded.variables.contains_key("base_url"));
    }

    #[test]
    fn test_list_environments() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("environments");

        save_environment(&dir, &make_env("Dev")).unwrap();
        save_environment(&dir, &make_env("Staging")).unwrap();
        save_environment(&dir, &make_env("Prod")).unwrap();

        let list = list_environments(&dir).unwrap();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_delete_environment() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("environments");

        save_environment(&dir, &make_env("ToDelete")).unwrap();
        let path = dir.join("todelete.json");
        assert!(path.exists());

        delete_environment(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_list_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("environments");
        let list = list_environments(&dir).unwrap();
        assert!(list.is_empty());
    }
}
