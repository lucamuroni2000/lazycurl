use curl_tui_core::collection::{list_collections, load_collection, save_collection};
use curl_tui_core::command::CurlCommandBuilder;
use curl_tui_core::config::AppConfig;
use curl_tui_core::history::append_entry_redacted;
use curl_tui_core::init::initialize;
use curl_tui_core::types::*;
use curl_tui_core::variable::FileVariableResolver;
use std::collections::HashMap;

/// Full workflow: init -> create collection -> save -> reload -> verify
#[test]
fn test_full_collection_workflow() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("curl-tui");

    // Init
    initialize(&root).unwrap();
    assert!(root.join("config.json").exists());

    // Create collection with a request
    let mut vars = HashMap::new();
    vars.insert(
        "base_url".to_string(),
        Variable {
            value: "https://api.example.com".to_string(),
            secret: false,
        },
    );

    let collection = Collection {
        id: uuid::Uuid::new_v4(),
        name: "Integration Test API".to_string(),
        variables: vars,
        requests: vec![Request {
            id: uuid::Uuid::new_v4(),
            name: "List Users".to_string(),
            method: Method::Get,
            url: "{{base_url}}/users".to_string(),
            headers: vec![Header {
                key: "Accept".to_string(),
                value: "application/json".to_string(),
                enabled: true,
            }],
            params: vec![Param {
                key: "page".to_string(),
                value: "1".to_string(),
                enabled: true,
            }],
            body: None,
            auth: Some(Auth::None),
        }],
    };

    let col_dir = root.join("collections");
    save_collection(&col_dir, &collection).unwrap();

    // Reload
    let collections = list_collections(&col_dir).unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].name, "Integration Test API");
    assert_eq!(collections[0].requests.len(), 1);

    // Load by file
    let loaded = load_collection(&col_dir.join("integration-test-api.json")).unwrap();
    assert_eq!(loaded.id, collection.id);
}

/// Variable resolution with environment and collection layers
#[test]
fn test_variable_resolution_end_to_end() {
    let global = {
        let mut m = HashMap::new();
        m.insert(
            "timeout".to_string(),
            Variable {
                value: "30".to_string(),
                secret: false,
            },
        );
        m
    };

    let env = {
        let mut m = HashMap::new();
        m.insert(
            "base_url".to_string(),
            Variable {
                value: "https://staging.example.com".to_string(),
                secret: false,
            },
        );
        m.insert(
            "api_token".to_string(),
            Variable {
                value: "stg-secret-123".to_string(),
                secret: true,
            },
        );
        m
    };

    let col = {
        let mut m = HashMap::new();
        m.insert(
            "base_url".to_string(),
            Variable {
                value: "https://override.example.com".to_string(),
                secret: false,
            },
        );
        m
    };

    let resolver = FileVariableResolver::new(global, Some(env), Some(col));

    // Collection overrides environment
    let (url, _) = resolver.resolve("{{base_url}}/api").unwrap();
    assert_eq!(url, "https://override.example.com/api");

    // Secrets tracked
    let (auth, secrets) = resolver.resolve("Bearer {{api_token}}").unwrap();
    assert_eq!(auth, "Bearer stg-secret-123");
    assert_eq!(secrets, vec!["stg-secret-123".to_string()]);
}

/// Security: secrets never appear in history
#[test]
fn test_secrets_redacted_in_history() {
    let tmp = tempfile::tempdir().unwrap();
    let history_path = tmp.path().join("history.jsonl");

    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        collection_id: None,
        request_name: "Auth Request".to_string(),
        method: Method::Post,
        url: "https://api.example.com/login?key=super-secret-key".to_string(),
        status_code: Some(200),
        duration_ms: Some(50),
        environment: Some("Production".to_string()),
    };

    let secrets = vec!["super-secret-key".to_string()];
    append_entry_redacted(&history_path, &entry, &secrets).unwrap();

    let content = std::fs::read_to_string(&history_path).unwrap();
    assert!(!content.contains("super-secret-key"));
    assert!(content.contains("[REDACTED]"));
}

/// CurlCommandBuilder produces correct args and redacts secrets
#[test]
fn test_command_builder_end_to_end() {
    let cmd = CurlCommandBuilder::new("https://api.example.com/users")
        .method(Method::Post)
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer secret-token")
        .body_json(r#"{"name": "Alice"}"#)
        .query_param("page", "1")
        .timeout(30)
        .build();

    let args = cmd.to_args();
    assert!(args.contains(&"-X".to_string()));
    assert!(args.contains(&"POST".to_string()));
    assert!(args.contains(&"-d".to_string()));
    assert!(args.contains(&"--max-time".to_string()));

    // Display string redacts secrets
    let display = cmd.to_display_string(&["secret-token".to_string()]);
    assert!(!display.contains("secret-token"));
    assert!(display.contains("curl"));

    // URL contains query params
    let url = args.last().unwrap();
    assert!(url.contains("page=1"));
}

/// Config loads with defaults for missing fields
#[test]
fn test_config_defaults() {
    let config = AppConfig::load_from_str(r#"{"default_timeout": 60}"#).unwrap();
    assert_eq!(config.default_timeout, 60);
    assert_eq!(config.max_response_body_size_bytes, 10_485_760);
    // Keybindings should have all defaults
    assert!(config.keybindings.contains_key("send_request"));
    assert!(config.keybindings.contains_key("reveal_secrets"));
}
