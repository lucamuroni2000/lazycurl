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

fn default_log_retention_days() -> u32 {
    7
}

fn default_max_log_body_size() -> u64 {
    65536 // 64 KB
}

fn default_preset_keybindings() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("quit".into(), "q".into());
    map.insert("send_request".into(), "ctrl+enter".into());
    map.insert("save_request".into(), "ctrl+s".into());
    map.insert("cancel".into(), "escape".into());
    map.insert("help".into(), "f1".into());
    map.insert("search".into(), "/".into());
    map.insert("new_request".into(), "ctrl+n".into());
    map.insert("switch_env".into(), "ctrl+e".into());
    map.insert("manage_envs".into(), "ctrl+shift+e".into());
    map.insert("open_variables".into(), "v".into());
    map.insert("open_export".into(), "x".into());
    map.insert("open_log_viewer".into(), "ctrl+l".into());
    map.insert("open_project_picker".into(), "ctrl+o".into());
    map.insert("reveal_secrets".into(), "f8".into());
    map.insert("move_up".into(), "up".into());
    map.insert("move_down".into(), "down".into());
    map.insert("enter".into(), "enter".into());
    map.insert("next_tab".into(), "right".into());
    map.insert("prev_tab".into(), "left".into());
    map.insert("cycle_pane_forward".into(), "tab".into());
    map.insert("cycle_pane_backward".into(), "shift+backtab".into());
    map.insert("next_project".into(), "ctrl+right".into());
    map.insert("prev_project".into(), "ctrl+left".into());
    map.insert("focus_collections".into(), "1".into());
    map.insert("focus_request".into(), "2".into());
    map.insert("focus_response".into(), "3".into());
    map.insert("add_item".into(), "a".into());
    map.insert("delete_item".into(), "d".into());
    map.insert("rename".into(), "r".into());
    map.insert("cycle_method".into(), "m".into());
    map.insert("toggle_enabled".into(), "s".into());
    map.insert("copy".into(), "y".into());
    map
}

fn vim_preset_keybindings() -> HashMap<String, String> {
    let mut map = default_preset_keybindings();
    map.insert("quit".into(), "q".into());
    map.insert("help".into(), "?".into());
    map.insert("new_request".into(), "o".into());
    map.insert("switch_env".into(), "e".into());
    map.insert("manage_envs".into(), "E".into());
    map.insert("open_project_picker".into(), "p".into());
    map.insert("move_up".into(), "k".into());
    map.insert("move_down".into(), "j".into());
    map.insert("next_tab".into(), "]".into());
    map.insert("prev_tab".into(), "[".into());
    map.insert("cycle_pane_forward".into(), "l".into());
    map.insert("cycle_pane_backward".into(), "h".into());
    map.insert("next_project".into(), "}".into());
    map.insert("prev_project".into(), "{".into());
    map
}

/// Returns the keybinding map for the named preset.
/// Falls back to the default preset for unknown names.
pub fn preset_keybindings(name: &str) -> HashMap<String, String> {
    match name {
        "vim" => vim_preset_keybindings(),
        _ => default_preset_keybindings(),
    }
}

fn default_keybindings() -> HashMap<String, String> {
    default_preset_keybindings()
}

fn default_keymap_preset() -> String {
    "default".to_string()
}

/// Intermediate struct used only for deserialization so we can distinguish
/// between "user provided no keybindings" and "user provided an empty map".
#[derive(Deserialize)]
struct RawAppConfig {
    #[serde(default)]
    variables: HashMap<String, Variable>,
    keybindings: Option<HashMap<String, String>>,
    #[serde(default = "default_keymap_preset")]
    keymap_preset: String,
    #[serde(default)]
    active_environment: Option<String>,
    #[serde(default = "default_timeout")]
    default_timeout: u32,
    #[serde(default = "default_max_body_size")]
    max_response_body_size_bytes: u64,
    #[serde(default)]
    debug_logging: bool,
    #[serde(default = "default_log_retention_days")]
    log_retention_days: u32,
    #[serde(default = "default_max_log_body_size")]
    max_log_body_size_bytes: u64,
    #[serde(default)]
    open_projects: Vec<String>,
    #[serde(default)]
    active_project: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub variables: HashMap<String, Variable>,
    #[serde(default = "default_keybindings")]
    pub keybindings: HashMap<String, String>,
    #[serde(default = "default_keymap_preset")]
    pub keymap_preset: String,
    #[serde(default)]
    pub active_environment: Option<String>,
    #[serde(default = "default_timeout")]
    pub default_timeout: u32,
    #[serde(default = "default_max_body_size")]
    pub max_response_body_size_bytes: u64,
    #[serde(default)]
    pub debug_logging: bool,
    #[serde(default = "default_log_retention_days")]
    pub log_retention_days: u32,
    #[serde(default = "default_max_log_body_size")]
    pub max_log_body_size_bytes: u64,
    #[serde(default)]
    pub open_projects: Vec<String>,
    #[serde(default)]
    pub active_project: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            keybindings: default_keybindings(),
            keymap_preset: default_keymap_preset(),
            active_environment: None,
            default_timeout: default_timeout(),
            max_response_body_size_bytes: default_max_body_size(),
            debug_logging: false,
            log_retention_days: default_log_retention_days(),
            max_log_body_size_bytes: default_max_log_body_size(),
            open_projects: Vec::new(),
            active_project: None,
        }
    }
}

fn apply_key_aliases(keybindings: &mut HashMap<String, String>) {
    let aliases = [
        ("copy_curl", "open_export"),
        ("toggle_collections", "focus_collections"),
        ("toggle_request", "focus_request"),
        ("toggle_response", "focus_response"),
        ("open_project", "open_project_picker"),
        ("cycle_panes", "cycle_pane_forward"),
    ];
    for (old, new) in aliases {
        if let Some(value) = keybindings.remove(old) {
            keybindings.entry(new.into()).or_insert(value);
        }
    }
}

/// One-time migration: strip keybindings whose values match the pre-refactor
/// defaults. These were auto-saved by the old code and are NOT intentional
/// user choices. Without this, switching to a new preset has no effect because
/// the old defaults override it.
fn strip_pre_refactor_defaults(keybindings: &mut HashMap<String, String>) {
    let old_defaults: &[(&str, &str)] = &[
        ("send_request", "ctrl+enter"),
        ("save_request", "ctrl+s"),
        ("switch_env", "ctrl+e"),
        ("manage_envs", "ctrl+shift+e"),
        ("open_export", "ctrl+y"),
        ("new_request", "ctrl+n"),
        ("cycle_pane_forward", "tab"),
        ("search", "/"),
        ("help", "?"),
        ("cancel", "escape"),
        ("focus_collections", "ctrl+1"),
        ("focus_request", "ctrl+2"),
        ("focus_response", "ctrl+3"),
        ("reveal_secrets", "f8"),
        ("next_project", "f6"),
        ("prev_project", "f7"),
        ("open_project_picker", "ctrl+o"),
        ("open_log_viewer", "ctrl+l"),
    ];
    for &(key, old_value) in old_defaults {
        if keybindings.get(key).is_some_and(|v| v == old_value) {
            keybindings.remove(key);
        }
    }
}

impl AppConfig {
    /// Load config from a JSON string, merging keybindings over defaults.
    pub fn load_from_str(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let raw: RawAppConfig = serde_json::from_str(json)?;
        // Start from the preset, then apply any user overrides (after alias resolution)
        let mut user_keybindings = raw.keybindings.unwrap_or_default();
        apply_key_aliases(&mut user_keybindings);
        strip_pre_refactor_defaults(&mut user_keybindings);
        let mut merged = preset_keybindings(&raw.keymap_preset);
        merged.extend(user_keybindings);
        Ok(AppConfig {
            variables: raw.variables,
            keybindings: merged,
            keymap_preset: raw.keymap_preset,
            active_environment: raw.active_environment,
            default_timeout: raw.default_timeout,
            max_response_body_size_bytes: raw.max_response_body_size_bytes,
            debug_logging: raw.debug_logging,
            log_retention_days: raw.log_retention_days,
            max_log_body_size_bytes: raw.max_log_body_size_bytes,
            open_projects: raw.open_projects,
            active_project: raw.active_project,
        })
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
    /// Only persists keybindings that differ from the current preset defaults,
    /// so switching presets later works correctly.
    pub fn save_to(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut config_to_save = self.clone();
        let preset_defaults = preset_keybindings(&self.keymap_preset);
        config_to_save
            .keybindings
            .retain(|key, value| preset_defaults.get(key).is_none_or(|d| d != value));
        let content = serde_json::to_string_pretty(&config_to_save)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Returns the platform-specific config directory for lazycurl.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lazycurl")
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
        assert_eq!(config.keybindings.get("copy").unwrap(), "y");
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
        assert!(path.to_str().unwrap().contains("lazycurl"));
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
    }

    #[test]
    fn test_config_session_fields_roundtrip() {
        let json = r#"{"open_projects":["my-api","other"],"active_project":"my-api"}"#;
        let config = AppConfig::load_from_str(json).unwrap();
        assert_eq!(config.open_projects, vec!["my-api", "other"]);
        assert_eq!(config.active_project, Some("my-api".to_string()));
    }

    #[test]
    fn test_config_backward_compat_no_session_fields() {
        let json = r#"{"default_timeout": 30}"#;
        let config = AppConfig::load_from_str(json).unwrap();
        assert!(config.open_projects.is_empty());
        assert!(config.active_project.is_none());
    }

    #[test]
    fn test_config_log_retention_days_default() {
        let config = AppConfig::default();
        assert_eq!(config.log_retention_days, 7);
    }

    #[test]
    fn test_config_max_log_body_size_default() {
        let config = AppConfig::default();
        assert_eq!(config.max_log_body_size_bytes, 65536);
    }

    #[test]
    fn test_config_backward_compat_no_log_fields() {
        let json = r#"{"default_timeout": 30}"#;
        let config = AppConfig::load_from_str(json).unwrap();
        assert_eq!(config.log_retention_days, 7);
        assert_eq!(config.max_log_body_size_bytes, 65536);
    }

    #[test]
    fn test_config_custom_log_retention() {
        let json = r#"{"log_retention_days": 14, "max_log_body_size_bytes": 131072}"#;
        let config = AppConfig::load_from_str(json).unwrap();
        assert_eq!(config.log_retention_days, 14);
        assert_eq!(config.max_log_body_size_bytes, 131072);
    }

    #[test]
    fn test_config_old_key_aliases_are_resolved() {
        // Use a non-old-default value to verify aliasing works
        let json = r#"{"keybindings": {"copy_curl": "ctrl+shift+y"}}"#;
        let config = AppConfig::load_from_str(json).unwrap();
        assert_eq!(
            config.keybindings.get("open_export").unwrap(),
            "ctrl+shift+y"
        );
    }

    #[test]
    fn test_config_old_key_aliases_full_set() {
        // Use non-old-default values to verify aliasing works independently of migration
        let json = r#"{"keybindings": {
            "copy_curl": "ctrl+shift+y",
            "toggle_collections": "alt+1",
            "toggle_request": "alt+2",
            "toggle_response": "alt+3",
            "open_project": "ctrl+p",
            "cycle_panes": "shift+tab"
        }}"#;
        let config = AppConfig::load_from_str(json).unwrap();
        assert_eq!(
            config.keybindings.get("open_export").unwrap(),
            "ctrl+shift+y"
        );
        assert_eq!(
            config.keybindings.get("focus_collections").unwrap(),
            "alt+1"
        );
        assert_eq!(config.keybindings.get("focus_request").unwrap(), "alt+2");
        assert_eq!(config.keybindings.get("focus_response").unwrap(), "alt+3");
        assert_eq!(
            config.keybindings.get("open_project_picker").unwrap(),
            "ctrl+p"
        );
        assert_eq!(
            config.keybindings.get("cycle_pane_forward").unwrap(),
            "shift+tab"
        );
    }

    #[test]
    fn test_pre_refactor_defaults_are_stripped() {
        // Simulate an old config that was auto-saved with the pre-refactor defaults
        let json = r#"{"keybindings": {
            "focus_collections": "ctrl+1",
            "focus_request": "ctrl+2",
            "focus_response": "ctrl+3",
            "open_export": "ctrl+y",
            "next_project": "f6",
            "prev_project": "f7",
            "help": "?"
        }}"#;
        let config = AppConfig::load_from_str(json).unwrap();
        // Old defaults should be stripped, new preset defaults take over
        assert_eq!(config.keybindings.get("focus_collections").unwrap(), "1");
        assert_eq!(config.keybindings.get("focus_request").unwrap(), "2");
        assert_eq!(config.keybindings.get("focus_response").unwrap(), "3");
        assert_eq!(config.keybindings.get("open_export").unwrap(), "x");
        assert_eq!(
            config.keybindings.get("next_project").unwrap(),
            "ctrl+right"
        );
        assert_eq!(config.keybindings.get("prev_project").unwrap(), "ctrl+left");
        assert_eq!(config.keybindings.get("help").unwrap(), "f1");
    }

    #[test]
    fn test_pre_refactor_strip_then_vim_preset() {
        // Old config with pre-refactor values + switch to vim
        let json = r#"{
            "keymap_preset": "vim",
            "keybindings": {
                "focus_collections": "ctrl+1",
                "open_export": "ctrl+y",
                "next_project": "f6",
                "prev_project": "f7"
            }
        }"#;
        let config = AppConfig::load_from_str(json).unwrap();
        // Old defaults stripped, vim preset takes over
        assert_eq!(config.keybindings.get("focus_collections").unwrap(), "1");
        assert_eq!(config.keybindings.get("open_export").unwrap(), "x");
        assert_eq!(config.keybindings.get("next_project").unwrap(), "}");
        assert_eq!(config.keybindings.get("prev_project").unwrap(), "{");
        assert_eq!(config.keybindings.get("move_up").unwrap(), "k");
        assert_eq!(config.keybindings.get("cycle_pane_forward").unwrap(), "l");
    }

    #[test]
    fn test_save_strips_preset_defaults_so_preset_switch_works() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.json");
        // Save with default preset
        let config = AppConfig::default();
        config.save_to(&path).unwrap();
        // Verify saved file has no keybindings (all match preset defaults)
        let raw: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let saved_kb = raw["keybindings"].as_object().unwrap();
        assert!(
            saved_kb.is_empty(),
            "saved keybindings should be empty when all match preset defaults, but got: {:?}",
            saved_kb
        );
        // Now manually edit the file to switch to vim preset
        let content = std::fs::read_to_string(&path).unwrap();
        let content = content.replace(r#""keymap_preset": "default""#, r#""keymap_preset": "vim""#);
        std::fs::write(&path, content).unwrap();
        // Load — should get vim keybindings, not default
        let loaded = AppConfig::load_from(&path).unwrap();
        assert_eq!(loaded.keymap_preset, "vim");
        assert_eq!(loaded.keybindings.get("help").unwrap(), "?");
        assert_eq!(loaded.keybindings.get("move_up").unwrap(), "k");
        assert_eq!(loaded.keybindings.get("cycle_pane_forward").unwrap(), "l");
    }

    #[test]
    fn test_save_preserves_user_overrides() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.json");
        let mut config = AppConfig::default();
        // User overrides help to a non-default value
        config.keybindings.insert("help".into(), "f2".into());
        config.save_to(&path).unwrap();
        // Verify saved file has ONLY the override
        let raw: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let saved_kb = raw["keybindings"].as_object().unwrap();
        assert_eq!(saved_kb.len(), 1);
        assert_eq!(saved_kb["help"], "f2");
        // Reload — override preserved, rest from preset
        let loaded = AppConfig::load_from(&path).unwrap();
        assert_eq!(loaded.keybindings.get("help").unwrap(), "f2");
        assert_eq!(
            loaded.keybindings.get("send_request").unwrap(),
            "ctrl+enter"
        );
    }

    #[test]
    fn test_config_keymap_preset_defaults_to_default() {
        let config = AppConfig::default();
        assert_eq!(config.keymap_preset, "default");
    }

    #[test]
    fn test_config_vim_preset_loads_vim_keybindings() {
        let json = r#"{"keymap_preset": "vim"}"#;
        let config = AppConfig::load_from_str(json).unwrap();
        assert_eq!(config.keymap_preset, "vim");
        assert_eq!(config.keybindings.get("help").unwrap(), "?");
        assert_eq!(config.keybindings.get("move_up").unwrap(), "k");
        assert_eq!(config.keybindings.get("cycle_pane_forward").unwrap(), "l");
    }

    #[test]
    fn test_config_vim_preset_with_user_override() {
        let json = r#"{"keymap_preset": "vim", "keybindings": {"help": "f1"}}"#;
        let config = AppConfig::load_from_str(json).unwrap();
        assert_eq!(config.keybindings.get("help").unwrap(), "f1");
        assert_eq!(config.keybindings.get("move_up").unwrap(), "k");
    }

    #[test]
    fn test_preset_keybindings_dispatches_by_name() {
        let default_kb = preset_keybindings("default");
        assert_eq!(default_kb["help"], "f1");

        let vim_kb = preset_keybindings("vim");
        assert_eq!(vim_kb["help"], "?");

        let fallback_kb = preset_keybindings("nonexistent");
        assert_eq!(fallback_kb["help"], "f1");
    }

    #[test]
    fn test_vim_preset_has_all_31_keys() {
        let kb = vim_preset_keybindings();
        assert_eq!(kb.len(), 32);
        // vim-specific overrides
        assert_eq!(kb["quit"], "q");
        assert_eq!(kb["help"], "?");
        assert_eq!(kb["new_request"], "o");
        assert_eq!(kb["switch_env"], "e");
        assert_eq!(kb["manage_envs"], "E");
        assert_eq!(kb["open_project_picker"], "p");
        assert_eq!(kb["move_up"], "k");
        assert_eq!(kb["move_down"], "j");
        assert_eq!(kb["next_tab"], "]");
        assert_eq!(kb["prev_tab"], "[");
        assert_eq!(kb["cycle_pane_forward"], "l");
        assert_eq!(kb["cycle_pane_backward"], "h");
        assert_eq!(kb["next_project"], "}");
        assert_eq!(kb["prev_project"], "{");
        // same as default
        assert_eq!(kb["send_request"], "ctrl+enter");
        assert_eq!(kb["save_request"], "ctrl+s");
        assert_eq!(kb["cancel"], "escape");
        assert_eq!(kb["search"], "/");
        assert_eq!(kb["open_variables"], "v");
        assert_eq!(kb["open_export"], "x");
        assert_eq!(kb["open_log_viewer"], "ctrl+l");
        assert_eq!(kb["reveal_secrets"], "f8");
        assert_eq!(kb["enter"], "enter");
        assert_eq!(kb["cycle_pane_backward"], "h");
        assert_eq!(kb["focus_collections"], "1");
        assert_eq!(kb["focus_request"], "2");
        assert_eq!(kb["focus_response"], "3");
        assert_eq!(kb["add_item"], "a");
        assert_eq!(kb["delete_item"], "d");
        assert_eq!(kb["rename"], "r");
        assert_eq!(kb["cycle_method"], "m");
        assert_eq!(kb["toggle_enabled"], "s");
        assert_eq!(kb["copy"], "y");
    }

    #[test]
    fn test_default_preset_has_all_31_keys() {
        let kb = default_preset_keybindings();
        assert_eq!(kb.len(), 32);
        assert_eq!(kb["quit"], "q");
        assert_eq!(kb["send_request"], "ctrl+enter");
        assert_eq!(kb["save_request"], "ctrl+s");
        assert_eq!(kb["cancel"], "escape");
        assert_eq!(kb["help"], "f1");
        assert_eq!(kb["search"], "/");
        assert_eq!(kb["new_request"], "ctrl+n");
        assert_eq!(kb["switch_env"], "ctrl+e");
        assert_eq!(kb["manage_envs"], "ctrl+shift+e");
        assert_eq!(kb["open_variables"], "v");
        assert_eq!(kb["open_export"], "x");
        assert_eq!(kb["open_log_viewer"], "ctrl+l");
        assert_eq!(kb["open_project_picker"], "ctrl+o");
        assert_eq!(kb["reveal_secrets"], "f8");
        assert_eq!(kb["move_up"], "up");
        assert_eq!(kb["move_down"], "down");
        assert_eq!(kb["enter"], "enter");
        assert_eq!(kb["next_tab"], "right");
        assert_eq!(kb["prev_tab"], "left");
        assert_eq!(kb["cycle_pane_forward"], "tab");
        assert_eq!(kb["cycle_pane_backward"], "shift+backtab");
        assert_eq!(kb["next_project"], "ctrl+right");
        assert_eq!(kb["prev_project"], "ctrl+left");
        assert_eq!(kb["focus_collections"], "1");
        assert_eq!(kb["focus_request"], "2");
        assert_eq!(kb["focus_response"], "3");
        assert_eq!(kb["add_item"], "a");
        assert_eq!(kb["delete_item"], "d");
        assert_eq!(kb["rename"], "r");
        assert_eq!(kb["cycle_method"], "m");
        assert_eq!(kb["toggle_enabled"], "s");
        assert_eq!(kb["copy"], "y");
    }
}
