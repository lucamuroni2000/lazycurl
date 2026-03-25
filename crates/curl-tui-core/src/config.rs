use crate::types::Variable;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn default_timeout() -> u32 {
    30
}

fn default_max_body_size() -> u64 {
    10_485_760 // 10 MB
}

fn default_keybindings() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("send_request".into(), "ctrl+enter".into());
    map.insert("save_request".into(), "ctrl+s".into());
    map.insert("switch_env".into(), "ctrl+e".into());
    map.insert("manage_envs".into(), "ctrl+shift+e".into());
    map.insert("copy_curl".into(), "ctrl+y".into());
    map.insert("new_request".into(), "ctrl+n".into());
    map.insert("cycle_panes".into(), "tab".into());
    map.insert("search".into(), "/".into());
    map.insert("help".into(), "?".into());
    map.insert("cancel".into(), "escape".into());
    map.insert("toggle_collections".into(), "ctrl+1".into());
    map.insert("toggle_request".into(), "ctrl+2".into());
    map.insert("toggle_response".into(), "ctrl+3".into());
    map.insert("reveal_secrets".into(), "f8".into());
    map.insert("next_project".into(), "f6".into());
    map.insert("prev_project".into(), "f7".into());
    map.insert("open_project".into(), "ctrl+o".into());
    map
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionRestore {
    #[default]
    Auto,
    Prompt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub variables: HashMap<String, Variable>,
    #[serde(default = "default_keybindings")]
    pub keybindings: HashMap<String, String>,
    #[serde(default)]
    pub active_environment: Option<String>,
    #[serde(default = "default_timeout")]
    pub default_timeout: u32,
    #[serde(default = "default_max_body_size")]
    pub max_response_body_size_bytes: u64,
    #[serde(default)]
    pub debug_logging: bool,
    #[serde(default)]
    pub open_projects: Vec<String>,
    #[serde(default)]
    pub active_project: Option<String>,
    #[serde(default)]
    pub restore_session: SessionRestore,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            keybindings: default_keybindings(),
            active_environment: None,
            default_timeout: default_timeout(),
            max_response_body_size_bytes: default_max_body_size(),
            debug_logging: false,
            open_projects: Vec::new(),
            active_project: None,
            restore_session: SessionRestore::default(),
        }
    }
}

impl AppConfig {
    /// Load config from a JSON string, merging keybindings over defaults.
    pub fn load_from_str(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config: AppConfig = serde_json::from_str(json)?;
        // Merge user keybindings over defaults so unspecified keys keep defaults
        let mut merged = default_keybindings();
        merged.extend(config.keybindings);
        config.keybindings = merged;
        Ok(config)
    }

    /// Load config from a file path. Returns default if file doesn't exist.
    pub fn load_from(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        Self::load_from_str(&content)
    }

    /// Save config to a file path, creating parent directories if needed.
    pub fn save_to(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Returns the platform-specific config directory for curl-tui.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("curl-tui")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.default_timeout, 30);
        assert_eq!(config.max_response_body_size_bytes, 10_485_760);
        assert!(!config.debug_logging);
        assert!(config.active_environment.is_none());
    }

    #[test]
    fn test_default_keybindings() {
        let config = AppConfig::default();
        assert_eq!(
            config.keybindings.get("send_request").unwrap(),
            "ctrl+enter"
        );
        assert_eq!(config.keybindings.get("save_request").unwrap(), "ctrl+s");
        assert_eq!(config.keybindings.get("copy_curl").unwrap(), "ctrl+y");
        assert_eq!(config.keybindings.get("reveal_secrets").unwrap(), "f8");
    }

    #[test]
    fn test_config_roundtrip() {
        let config = AppConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.default_timeout, config.default_timeout);
        assert_eq!(deserialized.keybindings, config.keybindings);
    }

    #[test]
    fn test_config_partial_json_uses_defaults() {
        let json = r#"{"default_timeout": 60}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.default_timeout, 60);
        assert_eq!(config.max_response_body_size_bytes, 10_485_760);
        assert!(!config.debug_logging);
    }

    #[test]
    fn test_config_custom_keybinding_override_preserves_defaults() {
        let json = r#"{"keybindings": {"send_request": "f5"}}"#;
        let config = AppConfig::load_from_str(json).unwrap();
        assert_eq!(config.keybindings.get("send_request").unwrap(), "f5");
        assert_eq!(config.keybindings.get("save_request").unwrap(), "ctrl+s");
    }

    #[test]
    fn test_config_dir_returns_path() {
        let path = config_dir();
        assert!(path.to_str().unwrap().contains("curl-tui"));
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.json");
        let config = AppConfig::load_from(&path).unwrap();
        assert_eq!(config.default_timeout, 30);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.json");
        let config = AppConfig::default();
        config.save_to(&path).unwrap();
        let loaded = AppConfig::load_from(&path).unwrap();
        assert_eq!(loaded.default_timeout, config.default_timeout);
        assert_eq!(loaded.keybindings, config.keybindings);
    }

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
}
