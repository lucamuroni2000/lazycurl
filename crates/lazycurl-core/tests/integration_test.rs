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
        request_id: None,
        request: RequestLogData {
            method: Method::Post,
            url: "https://api.example.com/login?key=super-secret-key".to_string(),
            url_template: None,
            headers: vec![],
            body: None,
            body_template: None,
            body_type: None,
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
        body: Some(Body::Raw {
            content: r#"{"name":"Alice"}"#.to_string(),
            content_type: RawBodyType::Json,
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
                body: Some(Body::Raw {
                    content: r#"{"name":"test"}"#.to_string(),
                    content_type: RawBodyType::Json,
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
                body: Some(Body::Raw {
                    content: r#"{"name":"test"}"#.to_string(),
                    content_type: RawBodyType::Json,
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

/// End-to-end: resolve variables, build curl command, verify secrets are redacted in display.
#[test]
fn test_variable_resolution_builds_correct_curl_with_secrets() {
    let mut global_vars = HashMap::new();
    global_vars.insert(
        "base_url".to_string(),
        Variable {
            value: "https://api.example.com".to_string(),
            secret: false,
        },
    );
    global_vars.insert(
        "api_token".to_string(),
        Variable {
            value: "super-secret-tok-42".to_string(),
            secret: true,
        },
    );

    let resolver = FileVariableResolver::new(global_vars, None, None);

    // Resolve URL
    let (resolved_url, _url_secrets) = resolver.resolve("{{base_url}}/users").unwrap();
    assert_eq!(resolved_url, "https://api.example.com/users");

    // Resolve header value
    let (resolved_auth, auth_secrets) = resolver.resolve("Bearer {{api_token}}").unwrap();
    assert_eq!(resolved_auth, "Bearer super-secret-tok-42");
    assert!(auth_secrets.contains(&"super-secret-tok-42".to_string()));

    // Build curl command with resolved values
    let cmd = CurlCommandBuilder::new(&resolved_url)
        .method(Method::Get)
        .header("Authorization", &resolved_auth)
        .build();

    // Combine all secrets
    let all_secrets: Vec<String> = [_url_secrets, auth_secrets].concat();

    let display = cmd.to_display_string(&all_secrets);

    // The resolved URL should be present
    assert!(display.contains("https://api.example.com/users"));

    // The secret token value must NOT appear in display
    assert!(
        !display.contains("super-secret-tok-42"),
        "Secret token must not appear in display string"
    );

    // The redacted placeholder must be present
    assert!(
        display.contains("••••••"),
        "Display string should contain redaction placeholder"
    );
}

/// Save a collection with all auth types, reload, verify each is preserved.
#[test]
fn test_collection_save_reload_preserves_all_auth_types() {
    let dir = tempfile::tempdir().unwrap();

    let bearer_auth = Auth::Bearer {
        token: "tok-123".to_string(),
    };
    let basic_auth = Auth::Basic {
        username: "admin".to_string(),
        password: "pass".to_string(),
    };
    let apikey_auth = Auth::ApiKey {
        key: "X-Api-Key".to_string(),
        value: "key-abc".to_string(),
        location: ApiKeyLocation::Header,
    };
    let oauth1_auth = Auth::OAuth1 {
        signature_method: OAuth1SignatureMethod::HmacSha1,
        consumer_key: "ck".to_string(),
        consumer_secret: "cs".to_string(),
        access_token: "at".to_string(),
        token_secret: "ts".to_string(),
        callback_url: "http://localhost/cb".to_string(),
        version: "1.0".to_string(),
        realm: String::new(),
        timestamp: String::new(),
        nonce: String::new(),
        include_body_hash: false,
        add_to: OAuth1AddTo::Header,
    };
    let oauth2_auth = Auth::OAuth2 {
        grant: OAuth2Grant::ClientCredentials {
            token_url: "https://auth.example.com/token".to_string(),
            client_id: "cid".to_string(),
            client_secret: "csecret".to_string(),
        },
        token_name: "My Token".to_string(),
        callback_url: String::new(),
        scope: "read".to_string(),
        state: String::new(),
        client_authentication: ClientAuthentication::BasicHeader,
        access_token: "at-456".to_string(),
        refresh_token: String::new(),
    };
    let awsv4_auth = Auth::AwsV4 {
        access_key: "AKIA123".to_string(),
        secret_key: "secret".to_string(),
        region: "eu-west-1".to_string(),
        service: "s3".to_string(),
        session_token: String::new(),
        add_to: AwsAddTo::Headers,
    };
    let digest_auth = Auth::Digest {
        username: "user".to_string(),
        password: "pw".to_string(),
        realm: String::new(),
        nonce: String::new(),
        algorithm: DigestAlgorithm::MD5,
        qop: String::new(),
        nonce_count: String::new(),
        client_nonce: String::new(),
        opaque: String::new(),
    };
    let none_auth = Auth::None;

    let auths = vec![
        bearer_auth,
        basic_auth,
        apikey_auth,
        oauth1_auth,
        oauth2_auth,
        awsv4_auth,
        digest_auth,
        none_auth,
    ];

    let requests: Vec<Request> = auths
        .iter()
        .enumerate()
        .map(|(i, auth)| Request {
            id: uuid::Uuid::new_v4(),
            name: format!("Request {}", i),
            method: Method::Get,
            url: "https://example.com".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(auth.clone()),
        })
        .collect();

    let collection = Collection {
        id: uuid::Uuid::new_v4(),
        name: "All Auth Types".to_string(),
        variables: HashMap::new(),
        requests,
    };

    save_collection(dir.path(), &collection).unwrap();
    let loaded_list = list_collections(dir.path()).unwrap();
    assert_eq!(loaded_list.len(), 1);
    let loaded = &loaded_list[0];
    assert_eq!(loaded.requests.len(), 8);

    for (i, auth) in auths.iter().enumerate() {
        assert_eq!(
            loaded.requests[i].auth.as_ref().unwrap(),
            auth,
            "Auth mismatch for request {}",
            i
        );
    }
}

/// Verify request_id on log entries enables finding auth from collections.
#[test]
fn test_log_entry_request_id_enables_auth_recovery() {
    let req_id = uuid::Uuid::new_v4();
    let auth = Auth::Bearer {
        token: "{{my_token}}".to_string(),
    };

    let collection = Collection {
        id: uuid::Uuid::new_v4(),
        name: "Test Collection".to_string(),
        variables: HashMap::new(),
        requests: vec![Request {
            id: req_id,
            name: "Authed Request".to_string(),
            method: Method::Post,
            url: "https://api.example.com".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(auth.clone()),
        }],
    };

    let mut ws = ProjectWorkspaceData::new(
        Project {
            id: uuid::Uuid::new_v4(),
            name: "Auth Recovery".to_string(),
            active_environment: None,
        },
        "auth-recovery".to_string(),
    );
    ws.collections = vec![collection];

    // Create a log entry with request_id set
    let _log_entry = RequestLogEntry {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        project: Some("Auth Recovery".to_string()),
        collection: Some("Test Collection".to_string()),
        request_id: Some(req_id),
        request: RequestLogData {
            method: Method::Post,
            url: "https://api.example.com".to_string(),
            url_template: None,
            headers: vec![],
            body: None,
            body_template: None,
            body_type: None,
            params: vec![],
        },
        response: None,
        curl_command: String::new(),
        error: None,
    };

    // Use the request_id to find auth
    let found_auth = ws.find_request_auth(req_id);
    assert_eq!(found_auth, Some(auth));

    // Unknown UUID returns None
    let unknown_id = uuid::Uuid::new_v4();
    assert_eq!(ws.find_request_auth(unknown_id), None);
}

/// Export as curl only includes enabled headers and params.
#[test]
fn test_export_curl_respects_disabled_headers_and_params() {
    use lazycurl_core::export::export_curl;

    let request = Request {
        id: uuid::Uuid::new_v4(),
        name: "Partial Request".to_string(),
        method: Method::Get,
        url: "https://api.example.com/data".to_string(),
        headers: vec![
            Header {
                key: "Accept".to_string(),
                value: "application/json".to_string(),
                enabled: true,
            },
            Header {
                key: "X-Debug".to_string(),
                value: "true".to_string(),
                enabled: false,
            },
        ],
        params: vec![
            Param {
                key: "page".to_string(),
                value: "1".to_string(),
                enabled: true,
            },
            Param {
                key: "debug_mode".to_string(),
                value: "on".to_string(),
                enabled: false,
            },
        ],
        body: None,
        auth: None,
    };

    let curl = export_curl(&request, &[]);

    // Enabled header and param should appear
    assert!(
        curl.contains("Accept: application/json"),
        "Enabled header missing"
    );
    assert!(curl.contains("page=1"), "Enabled param missing");

    // Disabled header and param should NOT appear
    assert!(
        !curl.contains("X-Debug"),
        "Disabled header should not appear in curl export"
    );
    assert!(
        !curl.contains("debug_mode"),
        "Disabled param should not appear in curl export"
    );
}

/// Three-tier variable resolution with secrets tracked across nested references.
#[test]
fn test_variable_resolution_with_secrets_across_all_tiers() {
    let global = {
        let mut m = HashMap::new();
        m.insert(
            "base".to_string(),
            Variable {
                value: "https://api.com".to_string(),
                secret: false,
            },
        );
        m
    };

    let env = {
        let mut m = HashMap::new();
        m.insert(
            "token".to_string(),
            Variable {
                value: "env-secret".to_string(),
                secret: true,
            },
        );
        // Environment overrides global base
        m.insert(
            "base".to_string(),
            Variable {
                value: "https://staging.com".to_string(),
                secret: false,
            },
        );
        m
    };

    let col = {
        let mut m = HashMap::new();
        // Collection var references an environment secret
        m.insert(
            "extra".to_string(),
            Variable {
                value: "{{token}}-extra".to_string(),
                secret: false,
            },
        );
        m
    };

    let resolver = FileVariableResolver::new(global, Some(env), Some(col));

    let (result, secrets) = resolver.resolve("{{base}}/{{extra}}").unwrap();

    // Environment overrides global for "base"
    // "extra" is collection-tier "{{token}}-extra", which resolves token from env
    assert_eq!(result, "https://staging.com/env-secret-extra");

    // The secret from nested resolution should be tracked
    assert!(
        secrets.contains(&"env-secret".to_string()),
        "Secret from nested reference should be tracked, got: {:?}",
        secrets
    );
}
