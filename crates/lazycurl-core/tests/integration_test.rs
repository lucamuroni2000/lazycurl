use lazycurl_core::collection::{list_collections, load_collection, save_collection};
use lazycurl_core::command::CurlCommandBuilder;
use lazycurl_core::config::AppConfig;
use lazycurl_core::init::initialize;
use lazycurl_core::logging;
use lazycurl_core::types::*;
use lazycurl_core::variable::FileVariableResolver;
use std::collections::HashMap;

/// Full workflow: init -> create collection -> save -> reload -> verify
#[test]
fn test_full_collection_workflow() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("lazycurl");

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

/// Security: secrets never appear in request logs
#[test]
fn test_secrets_redacted_in_logs() {
    let tmp = tempfile::tempdir().unwrap();
    let logs_path = tmp.path().join("logs");

    let entry = RequestLogEntry {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        project: None,
        collection: None,
        request: RequestLogData {
            method: Method::Post,
            url: "https://api.example.com/login?key=super-secret-key".to_string(),
            url_template: None,
            headers: vec![],
            body: None,
            body_template: None,
            params: vec![],
        },
        response: Some(ResponseLogData {
            status_code: 200,
            status_text: "OK".to_string(),
            headers: vec![],
            body: None,
            body_size_bytes: 0,
            body_truncated: false,
            body_type: "text".to_string(),
            time_ms: 50,
        }),
        curl_command: String::new(),
        error: None,
    };

    let secrets = vec!["super-secret-key".to_string()];
    logging::log_request(&logs_path, &entry, &secrets, 65536).unwrap();

    // Find the log file and verify secrets are redacted
    let files: Vec<_> = std::fs::read_dir(&logs_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 1);
    let content = std::fs::read_to_string(files[0].path()).unwrap();
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

#[test]
fn test_project_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let projects_dir = tmp.path().join("projects");

    // Create project
    let project = lazycurl_core::types::Project {
        id: uuid::Uuid::new_v4(),
        name: "Test Project".to_string(),
        active_environment: None,
    };
    let dir = lazycurl_core::project::create_project(&projects_dir, &project).unwrap();

    // Add a collection to the project
    let collection = lazycurl_core::types::Collection {
        id: uuid::Uuid::new_v4(),
        name: "API".to_string(),
        variables: std::collections::HashMap::new(),
        requests: vec![],
    };
    lazycurl_core::collection::save_collection(&dir.join("collections"), &collection).unwrap();

    // Add an environment
    let env = lazycurl_core::types::Environment {
        id: uuid::Uuid::new_v4(),
        name: "Dev".to_string(),
        variables: std::collections::HashMap::new(),
    };
    lazycurl_core::environment::save_environment(&dir.join("environments"), &env).unwrap();

    // List should show 1 collection, 1 environment
    let cols = lazycurl_core::collection::list_collections(&dir.join("collections")).unwrap();
    assert_eq!(cols.len(), 1);
    let envs = lazycurl_core::environment::list_environments(&dir.join("environments")).unwrap();
    assert_eq!(envs.len(), 1);

    // Delete project
    lazycurl_core::project::delete_project(&dir).unwrap();
    assert!(!dir.exists());
}

#[test]
fn test_migration_then_project_load() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Set up flat structure
    lazycurl_core::init::initialize(root).unwrap();
    let col = lazycurl_core::types::Collection {
        id: uuid::Uuid::new_v4(),
        name: "Legacy".to_string(),
        variables: std::collections::HashMap::new(),
        requests: vec![],
    };
    lazycurl_core::collection::save_collection(&root.join("collections"), &col).unwrap();

    // Migration should have run during init if needed, but let's trigger manually
    if lazycurl_core::migration::needs_migration(root) {
        lazycurl_core::migration::migrate_flat_to_project(root).unwrap();
    }

    // Now load the migrated project
    let projects = lazycurl_core::project::list_projects(&root.join("projects")).unwrap();
    assert_eq!(projects.len(), 1);
    let (project, path) = &projects[0];
    assert_eq!(project.name, "Default");

    let cols = lazycurl_core::collection::list_collections(&path.join("collections")).unwrap();
    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0].name, "Legacy");
}

#[test]
fn test_environment_sync_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let projects_dir = tmp.path().join("projects");

    // Create project with two environments
    let project = lazycurl_core::types::Project {
        id: uuid::Uuid::new_v4(),
        name: "Env Sync Test".to_string(),
        active_environment: None,
    };
    let dir = lazycurl_core::project::create_project(&projects_dir, &project).unwrap();

    let env1 = lazycurl_core::types::Environment {
        id: uuid::Uuid::new_v4(),
        name: "Development".to_string(),
        variables: std::collections::HashMap::new(),
    };
    let env2 = lazycurl_core::types::Environment {
        id: uuid::Uuid::new_v4(),
        name: "Production".to_string(),
        variables: std::collections::HashMap::new(),
    };
    lazycurl_core::environment::save_environment(&dir.join("environments"), &env1).unwrap();
    lazycurl_core::environment::save_environment(&dir.join("environments"), &env2).unwrap();

    // Build workspace, select Production (index 1), sync
    let mut ws =
        lazycurl_core::types::ProjectWorkspaceData::new(project, "env-sync-test".to_string());
    ws.environments = vec![env1, env2];
    ws.active_environment = Some(1);
    ws.sync_active_environment_name();

    assert_eq!(
        ws.project.active_environment,
        Some("Production".to_string())
    );

    // Persist and reload
    lazycurl_core::project::save_project(&dir, &ws.project).unwrap();
    let reloaded = lazycurl_core::project::load_project(&dir).unwrap();
    assert_eq!(reloaded.active_environment, Some("Production".to_string()));

    // Simulate restoring index from name (what switch_project does at load time)
    let envs = lazycurl_core::environment::list_environments(&dir.join("environments")).unwrap();
    let restored_idx = reloaded
        .active_environment
        .as_ref()
        .and_then(|name| envs.iter().position(|e| &e.name == name));
    assert!(restored_idx.is_some());
    assert_eq!(envs[restored_idx.unwrap()].name, "Production");
}

#[test]
fn test_export_full_request_as_curl() {
    use lazycurl_core::export::export_curl;
    use lazycurl_core::types::*;

    let request = Request {
        id: uuid::Uuid::new_v4(),
        name: "Full Request".to_string(),
        method: Method::Post,
        url: "https://api.example.com/users".to_string(),
        headers: vec![
            Header {
                key: "Content-Type".to_string(),
                value: "application/json".to_string(),
                enabled: true,
            },
            Header {
                key: "Authorization".to_string(),
                value: "Bearer {{token}}".to_string(),
                enabled: true,
            },
        ],
        params: vec![Param {
            key: "page".to_string(),
            value: "1".to_string(),
            enabled: true,
        }],
        body: Some(Body::Json {
            content: r#"{"name":"Alice"}"#.to_string(),
        }),
        auth: None,
    };

    let curl = export_curl(&request, &[]);
    assert!(curl.contains("curl"));
    assert!(curl.contains("-X POST"));
    assert!(curl.contains("-H"));
    assert!(curl.contains("Content-Type: application/json"));
    assert!(curl.contains("Authorization: Bearer {{token}}"));
    assert!(curl.contains("-d"));
    assert!(curl.contains("page=1"));
}

#[test]
fn test_export_collection_as_postman() {
    use lazycurl_core::export::export_postman_collection;
    use lazycurl_core::types::*;
    use std::collections::HashMap;

    let collection = Collection {
        id: uuid::Uuid::new_v4(),
        name: "Test API".to_string(),
        variables: {
            let mut m = HashMap::new();
            m.insert(
                "base_url".to_string(),
                Variable {
                    value: "https://api.test.com".to_string(),
                    secret: false,
                },
            );
            m
        },
        requests: vec![
            Request {
                id: uuid::Uuid::new_v4(),
                name: "List".to_string(),
                method: Method::Get,
                url: "https://api.test.com/items".to_string(),
                headers: vec![],
                params: vec![],
                body: None,
                auth: None,
            },
            Request {
                id: uuid::Uuid::new_v4(),
                name: "Create".to_string(),
                method: Method::Post,
                url: "https://api.test.com/items".to_string(),
                headers: vec![],
                params: vec![],
                body: Some(Body::Json {
                    content: r#"{"name":"test"}"#.to_string(),
                }),
                auth: Some(Auth::Bearer {
                    token: "{{token}}".to_string(),
                }),
            },
        ],
    };

    let json = export_postman_collection(&collection);
    let json_str = serde_json::to_string_pretty(&json).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["info"]["name"].as_str().unwrap(), "Test API");
    assert_eq!(parsed["item"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["variable"].as_array().unwrap().len(), 1);
}

#[test]
fn test_export_collection_as_openapi() {
    use lazycurl_core::export::export_openapi_collection;
    use lazycurl_core::types::*;
    use std::collections::HashMap;

    let collection = Collection {
        id: uuid::Uuid::new_v4(),
        name: "Test API".to_string(),
        variables: HashMap::new(),
        requests: vec![
            Request {
                id: uuid::Uuid::new_v4(),
                name: "List Items".to_string(),
                method: Method::Get,
                url: "https://api.test.com/items".to_string(),
                headers: vec![],
                params: vec![],
                body: None,
                auth: None,
            },
            Request {
                id: uuid::Uuid::new_v4(),
                name: "Create Item".to_string(),
                method: Method::Post,
                url: "https://api.test.com/items".to_string(),
                headers: vec![],
                params: vec![],
                body: Some(Body::Json {
                    content: r#"{"name":"test"}"#.to_string(),
                }),
                auth: None,
            },
        ],
    };

    let json = export_openapi_collection(&collection);
    assert_eq!(json["openapi"].as_str().unwrap(), "3.0.3");
    let paths = json["paths"].as_object().unwrap();
    assert_eq!(paths.len(), 1);
    let items_path = &paths["/items"];
    assert!(items_path["get"].is_object());
    assert!(items_path["post"].is_object());
}

#[test]
fn test_collection_with_oauth2_auth_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let collection = Collection {
        id: uuid::Uuid::new_v4(),
        name: "Auth Test".to_string(),
        variables: HashMap::new(),
        requests: vec![Request {
            id: uuid::Uuid::new_v4(),
            name: "OAuth Request".to_string(),
            method: Method::Post,
            url: "https://api.example.com/data".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(Auth::OAuth2 {
                grant: OAuth2Grant::AuthorizationCode {
                    auth_url: "https://auth.example.com/authorize".to_string(),
                    token_url: "https://auth.example.com/token".to_string(),
                    client_id: "{{client_id}}".to_string(),
                    client_secret: "{{client_secret}}".to_string(),
                },
                token_name: "My Token".to_string(),
                callback_url: "http://localhost:9876/callback".to_string(),
                scope: "read write".to_string(),
                state: "".to_string(),
                client_authentication: ClientAuthentication::BasicHeader,
                access_token: "stored-token-123".to_string(),
                refresh_token: "refresh-456".to_string(),
            }),
        }],
    };

    lazycurl_core::collection::save_collection(dir.path(), &collection).unwrap();
    let loaded = lazycurl_core::collection::list_collections(dir.path()).unwrap();
    assert_eq!(loaded.len(), 1);
    let loaded_req = &loaded[0].requests[0];
    match &loaded_req.auth {
        Some(Auth::OAuth2 {
            grant,
            access_token,
            scope,
            ..
        }) => {
            assert_eq!(access_token, "stored-token-123");
            assert_eq!(scope, "read write");
            match grant {
                OAuth2Grant::AuthorizationCode { client_id, .. } => {
                    assert_eq!(client_id, "{{client_id}}");
                }
                _ => panic!("Wrong grant type"),
            }
        }
        _ => panic!("Wrong auth type"),
    }
}

#[test]
fn test_variable_resolution_in_auth_fields() {
    let mut global_vars = HashMap::new();
    global_vars.insert(
        "aws_key".to_string(),
        Variable {
            value: "AKIATEST".to_string(),
            secret: false,
        },
    );
    global_vars.insert(
        "aws_secret".to_string(),
        Variable {
            value: "secretkey".to_string(),
            secret: true,
        },
    );

    let resolver = FileVariableResolver::new(global_vars, None, None);

    let (resolved, _) = resolver.resolve("{{aws_key}}").unwrap();
    assert_eq!(resolved, "AKIATEST");

    let (resolved, secrets) = resolver.resolve("{{aws_secret}}").unwrap();
    assert_eq!(resolved, "secretkey");
    assert!(secrets.contains(&"secretkey".to_string()));
}

#[test]
fn test_collection_with_digest_auth_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let collection = Collection {
        id: uuid::Uuid::new_v4(),
        name: "Digest Auth Test".to_string(),
        variables: HashMap::new(),
        requests: vec![Request {
            id: uuid::Uuid::new_v4(),
            name: "Digest Request".to_string(),
            method: Method::Get,
            url: "https://example.com/protected".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(Auth::Digest {
                username: "admin".to_string(),
                password: "{{password}}".to_string(),
                realm: String::new(),
                nonce: String::new(),
                algorithm: DigestAlgorithm::SHA256,
                qop: String::new(),
                nonce_count: String::new(),
                client_nonce: String::new(),
                opaque: String::new(),
            }),
        }],
    };

    lazycurl_core::collection::save_collection(dir.path(), &collection).unwrap();
    let loaded = lazycurl_core::collection::list_collections(dir.path()).unwrap();
    assert_eq!(loaded.len(), 1);
    match &loaded[0].requests[0].auth {
        Some(Auth::Digest {
            username,
            password,
            algorithm,
            ..
        }) => {
            assert_eq!(username, "admin");
            assert_eq!(password, "{{password}}");
            assert_eq!(*algorithm, DigestAlgorithm::SHA256);
        }
        _ => panic!("Wrong auth type"),
    }
}

#[test]
fn test_collection_with_awsv4_auth_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let collection = Collection {
        id: uuid::Uuid::new_v4(),
        name: "AWS Auth Test".to_string(),
        variables: HashMap::new(),
        requests: vec![Request {
            id: uuid::Uuid::new_v4(),
            name: "AWS Request".to_string(),
            method: Method::Get,
            url: "https://s3.amazonaws.com/bucket".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(Auth::AwsV4 {
                access_key: "{{aws_key}}".to_string(),
                secret_key: "{{aws_secret}}".to_string(),
                region: "us-west-2".to_string(),
                service: "s3".to_string(),
                session_token: String::new(),
                add_to: AwsAddTo::Headers,
            }),
        }],
    };

    lazycurl_core::collection::save_collection(dir.path(), &collection).unwrap();
    let loaded = lazycurl_core::collection::list_collections(dir.path()).unwrap();
    assert_eq!(loaded.len(), 1);
    match &loaded[0].requests[0].auth {
        Some(Auth::AwsV4 {
            access_key,
            region,
            service,
            ..
        }) => {
            assert_eq!(access_key, "{{aws_key}}");
            assert_eq!(region, "us-west-2");
            assert_eq!(service, "s3");
        }
        _ => panic!("Wrong auth type"),
    }
}
