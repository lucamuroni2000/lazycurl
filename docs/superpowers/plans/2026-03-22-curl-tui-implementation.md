# curl-tui Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a terminal-native Postman replacement in Rust with security-first design, collections, environments, hierarchical variables, and a multi-pane TUI.

**Architecture:** Cargo workspace with two crates: `curl-tui-core` (library, all business logic) and `curl-tui` (binary, thin Ratatui + crossterm TUI layer). All testable logic lives in core; the binary crate only handles rendering and input.

**Tech Stack:** Rust, Ratatui 0.29+, crossterm, serde + serde_json, tokio, uuid, dirs

**Spec:** `docs/superpowers/specs/2026-03-22-curl-tui-design.md`

---

## File Structure

### `curl-tui-core` (library crate)

| File | Responsibility |
|---|---|
| `crates/curl-tui-core/src/lib.rs` | Re-exports all public modules |
| `crates/curl-tui-core/src/types.rs` | All shared data types: `Collection`, `Request`, `Environment`, `Variable`, `Body`, `Auth`, `Header`, `Param`, `HistoryEntry`, `Method` |
| `crates/curl-tui-core/src/variable.rs` | `VariableResolver` trait + `FileVariableResolver` — hierarchical resolution (collection > env > global), cycle detection, undefined tracking |
| `crates/curl-tui-core/src/secret.rs` | `redact()`, `is_secret()`, `generate_gitignore()`, `RedactedString` wrapper |
| `crates/curl-tui-core/src/config.rs` | `AppConfig` struct, loading/saving `config.json`, keybinding parsing, defaults |
| `crates/curl-tui-core/src/collection.rs` | Collection CRUD: create, load, save, delete, slugify name to filename |
| `crates/curl-tui-core/src/environment.rs` | Environment CRUD: create, load, save, delete, list, switch active |
| `crates/curl-tui-core/src/command.rs` | `CurlCommandBuilder` — builds curl CLI args, renders display string (with redaction), executes subprocess, parses response |
| `crates/curl-tui-core/src/history.rs` | Append-only JSONL history writer, secret scrubbing, entry cap with pruning |

### `curl-tui` (binary crate)

| File | Responsibility |
|---|---|
| `crates/curl-tui/src/main.rs` | Entry point: init terminal, run app, restore terminal |
| `crates/curl-tui/src/app.rs` | `App` struct (all state), `Action` enum, state transitions |
| `crates/curl-tui/src/events.rs` | Terminal event polling loop, maps crossterm events to `Action` |
| `crates/curl-tui/src/input.rs` | Keybinding dispatch: reads config keybindings, maps key events to actions |
| `crates/curl-tui/src/ui/mod.rs` | Top-level `draw()` function, delegates to sub-renderers |
| `crates/curl-tui/src/ui/layout.rs` | Multi-pane layout calculation with toggle support |
| `crates/curl-tui/src/ui/collections.rs` | Left pane: collection tree widget |
| `crates/curl-tui/src/ui/request.rs` | Top-right pane: request editor with tabs (Headers, Body, Auth, Params) |
| `crates/curl-tui/src/ui/response.rs` | Bottom-right pane: response viewer with tabs (Body, Headers, Timing) |
| `crates/curl-tui/src/ui/statusbar.rs` | Bottom bar: keybinding hints, active environment |

### Tests

| File | Responsibility |
|---|---|
| `crates/curl-tui-core/src/types.rs` | Inline `#[cfg(test)]` for serde roundtrip tests |
| `crates/curl-tui-core/src/variable.rs` | Inline tests for resolution, cycles, undefined vars |
| `crates/curl-tui-core/src/secret.rs` | Inline tests for redaction, gitignore generation |
| `crates/curl-tui-core/src/config.rs` | Inline tests for loading, defaults, keybinding parsing |
| `crates/curl-tui-core/src/collection.rs` | Inline tests for CRUD, slugification, roundtrips |
| `crates/curl-tui-core/src/environment.rs` | Inline tests for CRUD, listing, switching |
| `crates/curl-tui-core/src/command.rs` | Inline tests for arg building, redaction, response parsing |
| `crates/curl-tui-core/src/history.rs` | Inline tests for entry formatting, secret scrubbing, cap |
| `tests/integration_test.rs` | End-to-end: command build + execute against mock server |

### Project Configuration

| File | Responsibility |
|---|---|
| `.claude/settings.json` | Hooks: post-write cargo check, pre-commit verify |
| `.claude/commands/verify-rust.md` | Skill: fmt + clippy + test |
| `CLAUDE.md` | Project conventions for Claude |
| `.gitignore` | Ignore target/, IDE files |

---

## Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/curl-tui-core/Cargo.toml`
- Create: `crates/curl-tui-core/src/lib.rs`
- Create: `crates/curl-tui/Cargo.toml`
- Create: `crates/curl-tui/src/main.rs`
- Create: `.gitignore`

- [ ] **Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
members = ["crates/curl-tui-core", "crates/curl-tui"]
resolver = "2"
```

- [ ] **Step 2: Create curl-tui-core crate**

`crates/curl-tui-core/Cargo.toml`:
```toml
[package]
name = "curl-tui-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
dirs = "6"
tokio = { version = "1", features = ["process", "fs", "rt-multi-thread", "macros"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"

[dev-dependencies]
tempfile = "3"
```

`crates/curl-tui-core/src/lib.rs`:
```rust
pub mod types;
```

Create placeholder `crates/curl-tui-core/src/types.rs`:
```rust
// Data types — implemented in Task 3
```

- [ ] **Step 3: Create curl-tui binary crate**

`crates/curl-tui/Cargo.toml`:
```toml
[package]
name = "curl-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
curl-tui-core = { path = "../curl-tui-core" }
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

`crates/curl-tui/src/main.rs`:
```rust
fn main() {
    println!("curl-tui v0.1.0");
}
```

- [ ] **Step 4: Create .gitignore**

```
/target
**/*.rs.bk
*.pdb
.env
```

- [ ] **Step 5: Verify workspace builds**

Run: `cargo build --workspace`
Expected: Compiles successfully with no errors.

- [ ] **Step 6: Initialize git and commit**

```bash
git init
git add Cargo.toml crates/ .gitignore
git commit -m "feat: scaffold Cargo workspace with core and TUI crates"
```

---

## Task 2: Claude Project Configuration

**Files:**
- Create: `CLAUDE.md`
- Create: `.claude/settings.json`
- Create: `.claude/commands/verify-rust.md`

- [ ] **Step 1: Create CLAUDE.md**

```markdown
# curl-tui

Terminal-native Postman replacement built in Rust.

## Architecture

Cargo workspace with two crates:
- `curl-tui-core` (library) — all business logic, fully testable without a terminal
- `curl-tui` (binary) — thin Ratatui + crossterm TUI layer

## Conventions

- All business logic goes in `curl-tui-core`. The binary crate contains NO logic.
- Use `thiserror` for error types in core, propagate with `?`.
- All public types derive `Serialize, Deserialize, Debug, Clone, PartialEq`.
- Use `#[cfg(test)]` inline modules for unit tests in each source file.
- Follow TDD: write failing test first, then implement.

## Security Rules

- Secret variables (marked `secret: true`) must NEVER appear in:
  - Log output
  - History files
  - "Copy as curl" output (unless user explicitly opts in)
  - Debug output or error messages
- Use `[REDACTED]` for secret values in any persisted output.
- Collections store `{{variable_references}}` only, never resolved secrets.

## Testing

- Run all tests: `cargo test --workspace`
- Run specific crate: `cargo test -p curl-tui-core`
- Formatting: `cargo fmt --all --check`
- Linting: `cargo clippy --workspace -- -D warnings`

## Dependencies

Key crates: ratatui, crossterm, serde, serde_json, tokio, uuid, dirs, chrono, thiserror
```

- [ ] **Step 2: Create verify-rust skill**

`.claude/commands/verify-rust.md`:
```markdown
---
description: Run full Rust verification suite (format, lint, test)
---

Run the following commands in sequence. Stop on first failure and report the error:

1. `cargo fmt --all --check` — verify formatting
2. `cargo clippy --workspace -- -D warnings` — lint with warnings as errors
3. `cargo test --workspace` — run all tests

If all pass, report: "All checks passed: formatting, linting, and tests."
If any fail, report which step failed and the error output.
```

- [ ] **Step 3: Create .claude/settings.json with hooks**

`.claude/settings.json`:
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "if echo \"$TOOL_INPUT\" | grep -q '\\.rs'; then cd /c/Users/lucam/Projects/curl-tui && cargo check --workspace 2>&1 | tail -20; fi",
            "timeout": 30000
          }
        ]
      }
    ],
    "PreCommit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cd /c/Users/lucam/Projects/curl-tui && cargo fmt --all --check && cargo clippy --workspace -- -D warnings && cargo test --workspace",
            "timeout": 120000
          }
        ]
      }
    ]
  }
}
```

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md .claude/
git commit -m "feat: add Claude project config with verify-rust skill and hooks"
```

---

## Task 3: Core Data Types

**Files:**
- Create: `crates/curl-tui-core/src/types.rs`
- Test: inline `#[cfg(test)]` in same file

- [ ] **Step 1: Write failing tests for data type serialization roundtrips**

Add to `crates/curl-tui-core/src/types.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_serialization() {
        let method = Method::Post;
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, "\"POST\"");
        let deserialized: Method = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Method::Post);
    }

    #[test]
    fn test_variable_with_secret() {
        let var = Variable {
            value: "my-secret".to_string(),
            secret: true,
        };
        let json = serde_json::to_string(&var).unwrap();
        let deserialized: Variable = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.secret, true);
        assert_eq!(deserialized.value, "my-secret");
    }

    #[test]
    fn test_variable_default_not_secret() {
        let json = r#"{"value": "hello"}"#;
        let var: Variable = serde_json::from_str(json).unwrap();
        assert_eq!(var.secret, false);
    }

    #[test]
    fn test_body_json_roundtrip() {
        let body = Body::Json {
            content: r#"{"key": "value"}"#.to_string(),
        };
        let json = serde_json::to_string(&body).unwrap();
        let deserialized: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, body);
    }

    #[test]
    fn test_body_form_roundtrip() {
        let body = Body::Form {
            fields: vec![FormField {
                key: "user".to_string(),
                value: "alice".to_string(),
                enabled: true,
            }],
        };
        let json = serde_json::to_string(&body).unwrap();
        let deserialized: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, body);
    }

    #[test]
    fn test_body_none_roundtrip() {
        let body = Body::None;
        let json = serde_json::to_string(&body).unwrap();
        let deserialized: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Body::None);
    }

    #[test]
    fn test_auth_bearer_roundtrip() {
        let auth = Auth::Bearer {
            token: "{{api_token}}".to_string(),
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains("bearer"));
        let deserialized: Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, auth);
    }

    #[test]
    fn test_auth_api_key_roundtrip() {
        let auth = Auth::ApiKey {
            key: "X-API-Key".to_string(),
            value: "{{key}}".to_string(),
            location: ApiKeyLocation::Header,
        };
        let json = serde_json::to_string(&auth).unwrap();
        let deserialized: Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, auth);
    }

    #[test]
    fn test_request_roundtrip() {
        let request = Request {
            id: uuid::Uuid::new_v4(),
            name: "Get Users".to_string(),
            method: Method::Get,
            url: "https://api.example.com/users".to_string(),
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
        };
        let json = serde_json::to_string_pretty(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Get Users");
        assert_eq!(deserialized.method, Method::Get);
        assert_eq!(deserialized.headers.len(), 1);
        assert_eq!(deserialized.params.len(), 1);
    }

    #[test]
    fn test_collection_roundtrip() {
        let collection = Collection {
            id: uuid::Uuid::new_v4(),
            name: "My API".to_string(),
            variables: std::collections::HashMap::new(),
            requests: vec![],
        };
        let json = serde_json::to_string(&collection).unwrap();
        let deserialized: Collection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "My API");
    }

    #[test]
    fn test_environment_roundtrip() {
        let mut vars = std::collections::HashMap::new();
        vars.insert(
            "base_url".to_string(),
            Variable {
                value: "http://localhost:3000".to_string(),
                secret: false,
            },
        );
        let env = Environment {
            id: uuid::Uuid::new_v4(),
            name: "Development".to_string(),
            variables: vars,
        };
        let json = serde_json::to_string(&env).unwrap();
        let deserialized: Environment = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Development");
        assert!(deserialized.variables.contains_key("base_url"));
    }

    #[test]
    fn test_history_entry_roundtrip() {
        let entry = HistoryEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            collection_id: None,
            request_name: "Get Users".to_string(),
            method: Method::Get,
            url: "https://api.example.com/users".to_string(),
            status_code: Some(200),
            duration_ms: Some(142),
            environment: Some("Development".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: HistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.request_name, "Get Users");
        assert_eq!(deserialized.status_code, Some(200));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core`
Expected: FAIL — types not defined yet.

- [ ] **Step 3: Implement all data types**

Replace the top of `crates/curl-tui-core/src/types.rs` (above `#[cfg(test)]`):
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::Get => write!(f, "GET"),
            Method::Post => write!(f, "POST"),
            Method::Put => write!(f, "PUT"),
            Method::Delete => write!(f, "DELETE"),
            Method::Patch => write!(f, "PATCH"),
            Method::Head => write!(f, "HEAD"),
            Method::Options => write!(f, "OPTIONS"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub value: String,
    #[serde(default)]
    pub secret: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormField {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultipartPart {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Body {
    Json { content: String },
    Text { content: String },
    Form { fields: Vec<FormField> },
    Multipart { parts: Vec<MultipartPart> },
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    Header,
    Query,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Auth {
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
        #[serde(rename = "in")]
        location: ApiKeyLocation,
    },
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub id: uuid::Uuid,
    pub name: String,
    pub method: Method,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<Header>,
    #[serde(default)]
    pub params: Vec<Param>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Body>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub id: uuid::Uuid,
    pub name: String,
    #[serde(default)]
    pub variables: HashMap<String, Variable>,
    #[serde(default)]
    pub requests: Vec<Request>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    pub id: uuid::Uuid,
    pub name: String,
    #[serde(default)]
    pub variables: HashMap<String, Variable>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: uuid::Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<uuid::Uuid>,
    pub request_name: String,
    pub method: Method,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
}

/// Timing breakdown for curl responses
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseTiming {
    pub dns_lookup_ms: f64,
    pub tcp_connect_ms: f64,
    pub tls_handshake_ms: f64,
    pub transfer_start_ms: f64,
    pub total_ms: f64,
}

/// Parsed response from a curl execution
#[derive(Debug, Clone)]
pub struct CurlResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub timing: ResponseTiming,
    pub raw_command: String,
}

fn default_true() -> bool {
    true
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p curl-tui-core`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui-core/src/types.rs crates/curl-tui-core/src/lib.rs
git commit -m "feat: add core data types with serde serialization"
```

---

## Task 4: Secret Handling Module

**Files:**
- Create: `crates/curl-tui-core/src/secret.rs`
- Modify: `crates/curl-tui-core/src/lib.rs` — add `pub mod secret;`

- [ ] **Step 1: Write failing tests**

Add to `crates/curl-tui-core/src/secret.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_replaces_value() {
        assert_eq!(redact("my-secret-token"), "••••••");
    }

    #[test]
    fn test_redact_empty_string() {
        assert_eq!(redact(""), "••••••");
    }

    #[test]
    fn test_redact_in_string_replaces_occurrences() {
        let input = "Authorization: Bearer my-secret-token";
        let result = redact_in_string(input, "my-secret-token");
        assert_eq!(result, "Authorization: Bearer ••••••");
    }

    #[test]
    fn test_redact_in_string_multiple_occurrences() {
        let input = "token=abc&verify=abc";
        let result = redact_in_string(input, "abc");
        assert_eq!(result, "token=••••••&verify=••••••");
    }

    #[test]
    fn test_redact_in_string_no_match() {
        let input = "no secrets here";
        let result = redact_in_string(input, "missing");
        assert_eq!(result, "no secrets here");
    }

    #[test]
    fn test_generate_gitignore() {
        let content = generate_gitignore();
        assert!(content.contains("environments/"));
        assert!(content.contains("history.jsonl"));
    }

    #[test]
    fn test_redact_secrets_in_url() {
        let url = "https://api.example.com/users?token=secret123";
        let secrets = vec!["secret123".to_string()];
        let result = redact_secrets(url, &secrets);
        assert_eq!(
            result,
            "https://api.example.com/users?token=••••••"
        );
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_redact_secrets_multiple() {
        let text = "user=admin pass=s3cret key=s3cret";
        let secrets = vec!["admin".to_string(), "s3cret".to_string()];
        let result = redact_secrets(text, &secrets);
        assert!(!result.contains("admin"));
        assert!(!result.contains("s3cret"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core -- secret`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement secret module**

Add to top of `crates/curl-tui-core/src/secret.rs` (above `#[cfg(test)]`):
```rust
/// The redaction placeholder shown in place of secret values.
pub const REDACTED_DISPLAY: &str = "••••••";

/// The redaction placeholder used in persisted output (history, logs).
pub const REDACTED_LOG: &str = "[REDACTED]";

/// Replace any value with the display redaction placeholder.
pub fn redact(_value: &str) -> &'static str {
    REDACTED_DISPLAY
}

/// Replace all occurrences of `secret` within `input` with the redaction placeholder.
pub fn redact_in_string(input: &str, secret: &str) -> String {
    input.replace(secret, REDACTED_DISPLAY)
}

/// Replace all known secret values in `text`.
pub fn redact_secrets(text: &str, secrets: &[String]) -> String {
    let mut result = text.to_string();
    for secret in secrets {
        if !secret.is_empty() {
            result = result.replace(secret.as_str(), REDACTED_DISPLAY);
        }
    }
    result
}

/// Generate the default `.gitignore` content for the config directory.
pub fn generate_gitignore() -> String {
    [
        "# curl-tui: auto-generated gitignore",
        "# Environment files may contain secrets",
        "environments/",
        "",
        "# History contains request metadata",
        "history.jsonl",
        "",
    ]
    .join("\n")
}
```

- [ ] **Step 4: Add module to lib.rs**

Add `pub mod secret;` to `crates/curl-tui-core/src/lib.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p curl-tui-core -- secret`
Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/curl-tui-core/src/secret.rs crates/curl-tui-core/src/lib.rs
git commit -m "feat: add secret redaction and gitignore generation"
```

---

## Task 5: Variable Resolution

**Files:**
- Create: `crates/curl-tui-core/src/variable.rs`
- Modify: `crates/curl-tui-core/src/lib.rs` — add `pub mod variable;`

- [ ] **Step 1: Write failing tests**

Add to `crates/curl-tui-core/src/variable.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Variable;
    use std::collections::HashMap;

    fn make_vars(pairs: &[(&str, &str, bool)]) -> HashMap<String, Variable> {
        pairs
            .iter()
            .map(|(k, v, s)| {
                (
                    k.to_string(),
                    Variable {
                        value: v.to_string(),
                        secret: *s,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn test_resolve_simple_variable() {
        let global = make_vars(&[("base_url", "https://api.example.com", false)]);
        let resolver = FileVariableResolver::new(global, None, None);
        let (result, _) = resolver.resolve("{{base_url}}/users").unwrap();
        assert_eq!(result, "https://api.example.com/users");
    }

    #[test]
    fn test_resolve_no_variables() {
        let resolver = FileVariableResolver::new(HashMap::new(), None, None);
        let (result, _) = resolver.resolve("plain text").unwrap();
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_resolve_collection_overrides_environment() {
        let global = HashMap::new();
        let env_vars = make_vars(&[("url", "http://env.com", false)]);
        let col_vars = make_vars(&[("url", "http://collection.com", false)]);
        let resolver = FileVariableResolver::new(global, Some(env_vars), Some(col_vars));
        let (result, _) = resolver.resolve("{{url}}").unwrap();
        assert_eq!(result, "http://collection.com");
    }

    #[test]
    fn test_resolve_environment_overrides_global() {
        let global = make_vars(&[("url", "http://global.com", false)]);
        let env_vars = make_vars(&[("url", "http://env.com", false)]);
        let resolver = FileVariableResolver::new(global, Some(env_vars), None);
        let (result, _) = resolver.resolve("{{url}}").unwrap();
        assert_eq!(result, "http://env.com");
    }

    #[test]
    fn test_resolve_tracks_secrets() {
        let global = make_vars(&[("token", "secret123", true)]);
        let resolver = FileVariableResolver::new(global, None, None);
        let (result, secrets) = resolver.resolve("Bearer {{token}}").unwrap();
        assert_eq!(result, "Bearer secret123");
        assert_eq!(secrets, vec!["secret123".to_string()]);
    }

    #[test]
    fn test_resolve_undefined_variable() {
        let resolver = FileVariableResolver::new(HashMap::new(), None, None);
        let result = resolver.resolve("{{undefined}}");
        assert!(matches!(result, Err(ResolveError::UndefinedVariables(_))));
        if let Err(ResolveError::UndefinedVariables(vars)) = result {
            assert_eq!(vars, vec!["undefined".to_string()]);
        }
    }

    #[test]
    fn test_resolve_multiple_variables() {
        let global = make_vars(&[
            ("host", "example.com", false),
            ("port", "8080", false),
        ]);
        let resolver = FileVariableResolver::new(global, None, None);
        let (result, _) = resolver.resolve("http://{{host}}:{{port}}/api").unwrap();
        assert_eq!(result, "http://example.com:8080/api");
    }

    #[test]
    fn test_resolve_circular_reference() {
        let global = make_vars(&[
            ("a", "{{b}}", false),
            ("b", "{{a}}", false),
        ]);
        let resolver = FileVariableResolver::new(global, None, None);
        let result = resolver.resolve("{{a}}");
        assert!(matches!(result, Err(ResolveError::CircularReference(_))));
    }

    #[test]
    fn test_resolve_nested_variable() {
        let global = make_vars(&[
            ("greeting", "hello {{name}}", false),
            ("name", "world", false),
        ]);
        let resolver = FileVariableResolver::new(global, None, None);
        let (result, _) = resolver.resolve("{{greeting}}").unwrap();
        assert_eq!(result, "hello world");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core -- variable`
Expected: FAIL — module not defined.

- [ ] **Step 3: Implement variable resolver**

Add to top of `crates/curl-tui-core/src/variable.rs` (above `#[cfg(test)]`):
```rust
use crate::types::Variable;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

const MAX_RESOLVE_DEPTH: usize = 10;

#[derive(Debug, Error, PartialEq)]
pub enum ResolveError {
    #[error("Undefined variables: {0:?}")]
    UndefinedVariables(Vec<String>),
    #[error("Circular variable reference detected: {0}")]
    CircularReference(String),
}

/// Resolves `{{variable}}` placeholders using a three-tier hierarchy.
pub struct FileVariableResolver {
    /// Merged variables: collection > environment > global
    variables: HashMap<String, Variable>,
}

impl FileVariableResolver {
    /// Create a new resolver. Pass `None` for layers that don't apply.
    pub fn new(
        global: HashMap<String, Variable>,
        environment: Option<HashMap<String, Variable>>,
        collection: Option<HashMap<String, Variable>>,
    ) -> Self {
        let mut merged = global;
        if let Some(env_vars) = environment {
            merged.extend(env_vars);
        }
        if let Some(col_vars) = collection {
            merged.extend(col_vars);
        }
        Self { variables: merged }
    }

    /// Resolve all `{{var}}` placeholders in `input`.
    ///
    /// Returns `(resolved_string, secret_values)` where `secret_values` contains
    /// the raw values of any secret variables that were substituted.
    pub fn resolve(&self, input: &str) -> Result<(String, Vec<String>), ResolveError> {
        let mut secrets = Vec::new();
        let mut visiting = HashSet::new();
        let result = self.resolve_inner(input, &mut secrets, &mut visiting, 0)?;
        Ok((result, secrets))
    }

    fn resolve_inner(
        &self,
        input: &str,
        secrets: &mut Vec<String>,
        visiting: &mut HashSet<String>,
        depth: usize,
    ) -> Result<String, ResolveError> {
        if depth > MAX_RESOLVE_DEPTH {
            let chain: Vec<String> = visiting.iter().cloned().collect();
            return Err(ResolveError::CircularReference(chain.join(" -> ")));
        }

        let mut result = String::new();
        let mut remaining = input;
        let mut undefined = Vec::new();

        while let Some(start) = remaining.find("{{") {
            result.push_str(&remaining[..start]);
            let after_open = &remaining[start + 2..];

            if let Some(end) = after_open.find("}}") {
                let var_name = &after_open[..end];

                if visiting.contains(var_name) {
                    let mut chain: Vec<String> = visiting.iter().cloned().collect();
                    chain.push(var_name.to_string());
                    return Err(ResolveError::CircularReference(chain.join(" -> ")));
                }

                if let Some(variable) = self.variables.get(var_name) {
                    visiting.insert(var_name.to_string());
                    let resolved =
                        self.resolve_inner(&variable.value, secrets, visiting, depth + 1)?;
                    visiting.remove(var_name);

                    if variable.secret {
                        secrets.push(resolved.clone());
                    }
                    result.push_str(&resolved);
                } else {
                    undefined.push(var_name.to_string());
                }

                remaining = &after_open[end + 2..];
            } else {
                // No closing }}, treat as literal
                result.push_str("{{");
                remaining = after_open;
            }
        }

        result.push_str(remaining);

        if !undefined.is_empty() {
            return Err(ResolveError::UndefinedVariables(undefined));
        }

        Ok(result)
    }
}
```

- [ ] **Step 4: Add module to lib.rs**

Add `pub mod variable;` to `crates/curl-tui-core/src/lib.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p curl-tui-core -- variable`
Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/curl-tui-core/src/variable.rs crates/curl-tui-core/src/lib.rs
git commit -m "feat: add hierarchical variable resolver with cycle detection"
```

---

## Task 6: Config Module

**Files:**
- Create: `crates/curl-tui-core/src/config.rs`
- Modify: `crates/curl-tui-core/src/lib.rs` — add `pub mod config;`

- [ ] **Step 1: Write failing tests**

Add to `crates/curl-tui-core/src/config.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.default_timeout, 30);
        assert_eq!(config.max_response_body_size_bytes, 10_485_760);
        assert_eq!(config.debug_logging, false);
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
        // Other fields should get defaults
        assert_eq!(config.max_response_body_size_bytes, 10_485_760);
        assert_eq!(config.debug_logging, false);
    }

    #[test]
    fn test_config_custom_keybinding_override_preserves_defaults() {
        let json = r#"{"keybindings": {"send_request": "f5"}}"#;
        let config = AppConfig::load_from_str(json).unwrap();
        // Custom override applied
        assert_eq!(config.keybindings.get("send_request").unwrap(), "f5");
        // Other defaults preserved
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
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core -- config`
Expected: FAIL — module not defined.

- [ ] **Step 3: Implement config module**

Add to top of `crates/curl-tui-core/src/config.rs` (above `#[cfg(test)]`):
```rust
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
    map
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
```

- [ ] **Step 4: Add module to lib.rs**

Add `pub mod config;` to `crates/curl-tui-core/src/lib.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p curl-tui-core -- config`
Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/curl-tui-core/src/config.rs crates/curl-tui-core/src/lib.rs
git commit -m "feat: add config module with defaults and file persistence"
```

---

## Task 7: Collection Management

**Files:**
- Create: `crates/curl-tui-core/src/collection.rs`
- Modify: `crates/curl-tui-core/src/lib.rs` — add `pub mod collection;`

- [ ] **Step 1: Write failing tests**

Add to `crates/curl-tui-core/src/collection.rs`:
```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core -- collection`
Expected: FAIL — module not defined.

- [ ] **Step 3: Implement collection module**

Add to top of `crates/curl-tui-core/src/collection.rs` (above `#[cfg(test)]`):
```rust
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
pub fn save_collection(dir: &Path, collection: &Collection) -> Result<(), Box<dyn std::error::Error>> {
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
```

- [ ] **Step 4: Add module to lib.rs**

Add `pub mod collection;` to `crates/curl-tui-core/src/lib.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p curl-tui-core -- collection`
Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/curl-tui-core/src/collection.rs crates/curl-tui-core/src/lib.rs
git commit -m "feat: add collection CRUD with slug-based file naming"
```

---

## Task 8: Environment Management

**Files:**
- Create: `crates/curl-tui-core/src/environment.rs`
- Modify: `crates/curl-tui-core/src/lib.rs` — add `pub mod environment;`

- [ ] **Step 1: Write failing tests**

Add to `crates/curl-tui-core/src/environment.rs`:
```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core -- environment`
Expected: FAIL.

- [ ] **Step 3: Implement environment module**

Add to top of `crates/curl-tui-core/src/environment.rs` (above `#[cfg(test)]`):
```rust
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
```

- [ ] **Step 4: Add module to lib.rs**

Add `pub mod environment;` to `crates/curl-tui-core/src/lib.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p curl-tui-core -- environment`
Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/curl-tui-core/src/environment.rs crates/curl-tui-core/src/lib.rs
git commit -m "feat: add environment management with file persistence"
```

---

## Task 9: CurlCommandBuilder

**Files:**
- Create: `crates/curl-tui-core/src/command.rs`
- Modify: `crates/curl-tui-core/src/lib.rs` — add `pub mod command;`

- [ ] **Step 1: Write failing tests**

Add to `crates/curl-tui-core/src/command.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Method;

    #[test]
    fn test_basic_get_args() {
        let cmd = CurlCommandBuilder::new("https://api.example.com/users")
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-s".to_string()));
        assert!(args.contains(&"https://api.example.com/users".to_string()));
        // GET is default, no -X needed
        assert!(!args.contains(&"-X".to_string()));
    }

    #[test]
    fn test_post_method() {
        let cmd = CurlCommandBuilder::new("https://api.example.com/users")
            .method(Method::Post)
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-X".to_string()));
        assert!(args.contains(&"POST".to_string()));
    }

    #[test]
    fn test_headers() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .build();
        let args = cmd.to_args();
        let h_positions: Vec<usize> = args
            .iter()
            .enumerate()
            .filter(|(_, a)| *a == "-H")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(h_positions.len(), 2);
        assert_eq!(args[h_positions[0] + 1], "Content-Type: application/json");
        assert_eq!(args[h_positions[1] + 1], "Accept: application/json");
    }

    #[test]
    fn test_json_body() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .method(Method::Post)
            .body_json(r#"{"name": "Alice"}"#)
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-d".to_string()));
        assert!(args.contains(&r#"{"name": "Alice"}"#.to_string()));
    }

    #[test]
    fn test_timeout() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .timeout(30)
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"--max-time".to_string()));
        assert!(args.contains(&"30".to_string()));
    }

    #[test]
    fn test_basic_auth() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .basic_auth("user", "pass")
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-u".to_string()));
        assert!(args.contains(&"user:pass".to_string()));
    }

    #[test]
    fn test_cookie() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .cookie("session=abc123")
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-b".to_string()));
        assert!(args.contains(&"session=abc123".to_string()));
    }

    #[test]
    fn test_follow_redirects() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .follow_redirects(true)
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-L".to_string()));
    }

    #[test]
    fn test_display_string_basic() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .method(Method::Get)
            .build();
        let display = cmd.to_display_string(&[]);
        assert!(display.starts_with("curl"));
        assert!(display.contains("https://example.com"));
    }

    #[test]
    fn test_display_string_redacts_secrets() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .header("Authorization", "Bearer secret-token-123")
            .build();
        let display = cmd.to_display_string(&["secret-token-123".to_string()]);
        assert!(!display.contains("secret-token-123"));
        assert!(display.contains("••••••"));
    }

    #[test]
    fn test_query_params() {
        let cmd = CurlCommandBuilder::new("https://example.com/api")
            .query_param("page", "1")
            .query_param("limit", "20")
            .build();
        let args = cmd.to_args();
        // URL should have query params appended
        let url = args.last().unwrap();
        assert!(url.contains("page=1"));
        assert!(url.contains("limit=20"));
        assert!(url.contains('?'));
        assert!(url.contains('&'));
    }

    #[test]
    fn test_form_data() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .method(Method::Post)
            .form_field("username", "alice")
            .form_field("password", "s3cret")
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"--data-urlencode".to_string()));
        assert!(args.contains(&"username=alice".to_string()));
    }

    #[test]
    fn test_multipart_upload() {
        let cmd = CurlCommandBuilder::new("https://example.com/upload")
            .method(Method::Post)
            .multipart_field("description", "My file")
            .multipart_file("file", "/path/to/file.png")
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-F".to_string()));
        assert!(args.contains(&"description=My file".to_string()));
        assert!(args.contains(&"file=@/path/to/file.png".to_string()));
    }

    #[test]
    fn test_parse_response_headers() {
        let raw_headers = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Custom: value\r\n\r\n";
        let headers = parse_headers(raw_headers);
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0], ("Content-Type".to_string(), "application/json".to_string()));
        assert_eq!(headers[1], ("X-Custom".to_string(), "value".to_string()));
    }

    #[test]
    fn test_curl_binary_name() {
        let name = curl_binary();
        if cfg!(windows) {
            assert_eq!(name, "curl.exe");
        } else {
            assert_eq!(name, "curl");
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core -- command`
Expected: FAIL.

- [ ] **Step 3: Implement CurlCommandBuilder**

Add to top of `crates/curl-tui-core/src/command.rs` (above `#[cfg(test)]`):
```rust
use crate::secret;
use crate::types::{CurlResponse, Method, ResponseTiming};

/// Returns the platform-appropriate curl binary name.
pub fn curl_binary() -> &'static str {
    if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    }
}

/// A built curl command, ready to execute or display.
pub struct CurlCommand {
    url: String,
    method: Option<Method>,
    headers: Vec<(String, String)>,
    body: Option<String>,
    form_fields: Vec<(String, String)>,
    multipart_fields: Vec<(String, String)>,
    multipart_files: Vec<(String, String)>,
    timeout: Option<u32>,
    basic_auth: Option<(String, String)>,
    cookies: Vec<String>,
    follow_redirects: bool,
    query_params: Vec<(String, String)>,
}

impl CurlCommand {
    /// Build the curl argument vector (without the `curl` binary itself).
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Always silent
        args.push("-s".to_string());

        // Method (skip for GET as it's the default)
        if let Some(method) = &self.method {
            if !matches!(method, Method::Get) {
                args.push("-X".to_string());
                args.push(method.to_string());
            }
        }

        // Headers
        for (key, value) in &self.headers {
            args.push("-H".to_string());
            args.push(format!("{}: {}", key, value));
        }

        // Body
        if let Some(body) = &self.body {
            args.push("-d".to_string());
            args.push(body.clone());
        }

        // Form fields (url-encoded)
        for (key, value) in &self.form_fields {
            args.push("--data-urlencode".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Multipart fields
        for (key, value) in &self.multipart_fields {
            args.push("-F".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Multipart files
        for (key, path) in &self.multipart_files {
            args.push("-F".to_string());
            args.push(format!("{}=@{}", key, path));
        }

        // Timeout
        if let Some(t) = self.timeout {
            args.push("--max-time".to_string());
            args.push(t.to_string());
        }

        // Basic auth
        if let Some((user, pass)) = &self.basic_auth {
            args.push("-u".to_string());
            args.push(format!("{}:{}", user, pass));
        }

        // Cookies
        for cookie in &self.cookies {
            args.push("-b".to_string());
            args.push(cookie.clone());
        }

        // Follow redirects
        if self.follow_redirects {
            args.push("-L".to_string());
        }

        // URL with query params
        let url = self.build_url();
        args.push(url);

        args
    }

    fn build_url(&self) -> String {
        if self.query_params.is_empty() {
            return self.url.clone();
        }
        let params: Vec<String> = self
            .query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let separator = if self.url.contains('?') { "&" } else { "?" };
        format!("{}{}{}", self.url, separator, params.join("&"))
    }

    /// Render the command as a display string, redacting secret values.
    pub fn to_display_string(&self, secrets: &[String]) -> String {
        let args = self.to_args();
        let mut parts = vec![curl_binary().to_string()];
        parts.extend(args);
        let full = parts.join(" ");
        secret::redact_secrets(&full, secrets)
    }

    /// Execute the curl command as a subprocess.
    pub async fn execute(&self) -> Result<CurlResponse, Box<dyn std::error::Error>> {
        let mut args = self.to_args();

        // Create temp files for body and headers
        let body_file = tempfile::NamedTempFile::new()?;
        let header_file = tempfile::NamedTempFile::new()?;

        // Remove the URL (last arg) and re-add with output options
        let url = args.pop().unwrap();
        args.push("-o".to_string());
        args.push(body_file.path().to_string_lossy().to_string());
        args.push("-D".to_string());
        args.push(header_file.path().to_string_lossy().to_string());
        args.push("-w".to_string());
        args.push("%{json}".to_string());
        args.push(url);

        let output = tokio::process::Command::new(curl_binary())
            .args(&args)
            .output()
            .await?;

        let write_out = String::from_utf8_lossy(&output.stdout).to_string();
        let body = std::fs::read_to_string(body_file.path()).unwrap_or_default();
        let raw_headers = std::fs::read_to_string(header_file.path()).unwrap_or_default();
        let headers = parse_headers(&raw_headers);

        // Parse write-out JSON for status code and timing
        let (status_code, timing) = parse_write_out(&write_out);

        Ok(CurlResponse {
            status_code,
            headers,
            body,
            timing,
            raw_command: self.to_display_string(&[]),
        })
    }
}

/// Parse raw HTTP headers from curl's -D output.
pub fn parse_headers(raw: &str) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.starts_with("HTTP/") || line.is_empty() {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    headers
}

/// Parse curl's -w %{json} write-out.
fn parse_write_out(json_str: &str) -> (u16, ResponseTiming) {
    let default_timing = ResponseTiming {
        dns_lookup_ms: 0.0,
        tcp_connect_ms: 0.0,
        tls_handshake_ms: 0.0,
        transfer_start_ms: 0.0,
        total_ms: 0.0,
    };

    let parsed: serde_json::Value = match serde_json::from_str(json_str.trim()) {
        Ok(v) => v,
        Err(_) => return (0, default_timing),
    };

    let status_code = parsed
        .get("http_code")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u16;

    let get_ms = |key: &str| -> f64 {
        parsed
            .get(key)
            .and_then(|v| v.as_f64())
            .map(|s| s * 1000.0) // curl reports seconds, we want ms
            .unwrap_or(0.0)
    };

    let timing = ResponseTiming {
        dns_lookup_ms: get_ms("time_namelookup"),
        tcp_connect_ms: get_ms("time_connect"),
        tls_handshake_ms: get_ms("time_appconnect"),
        transfer_start_ms: get_ms("time_starttransfer"),
        total_ms: get_ms("time_total"),
    };

    (status_code, timing)
}

pub struct CurlCommandBuilder {
    url: String,
    method: Option<Method>,
    headers: Vec<(String, String)>,
    body: Option<String>,
    form_fields: Vec<(String, String)>,
    multipart_fields: Vec<(String, String)>,
    multipart_files: Vec<(String, String)>,
    timeout: Option<u32>,
    basic_auth: Option<(String, String)>,
    cookies: Vec<String>,
    follow_redirects: bool,
    query_params: Vec<(String, String)>,
}

impl CurlCommandBuilder {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: None,
            headers: Vec::new(),
            body: None,
            form_fields: Vec::new(),
            multipart_fields: Vec::new(),
            multipart_files: Vec::new(),
            timeout: None,
            basic_auth: None,
            cookies: Vec::new(),
            follow_redirects: false,
            query_params: Vec::new(),
        }
    }

    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }

    pub fn body_json(mut self, json: &str) -> Self {
        self.body = Some(json.to_string());
        self
    }

    pub fn body_text(mut self, text: &str) -> Self {
        self.body = Some(text.to_string());
        self
    }

    pub fn form_field(mut self, key: &str, value: &str) -> Self {
        self.form_fields.push((key.to_string(), value.to_string()));
        self
    }

    pub fn multipart_field(mut self, key: &str, value: &str) -> Self {
        self.multipart_fields
            .push((key.to_string(), value.to_string()));
        self
    }

    pub fn multipart_file(mut self, key: &str, path: &str) -> Self {
        self.multipart_files
            .push((key.to_string(), path.to_string()));
        self
    }

    pub fn timeout(mut self, seconds: u32) -> Self {
        self.timeout = Some(seconds);
        self
    }

    pub fn basic_auth(mut self, user: &str, pass: &str) -> Self {
        self.basic_auth = Some((user.to_string(), pass.to_string()));
        self
    }

    pub fn cookie(mut self, cookie: &str) -> Self {
        self.cookies.push(cookie.to_string());
        self
    }

    pub fn follow_redirects(mut self, follow: bool) -> Self {
        self.follow_redirects = follow;
        self
    }

    pub fn query_param(mut self, key: &str, value: &str) -> Self {
        self.query_params
            .push((key.to_string(), value.to_string()));
        self
    }

    pub fn build(self) -> CurlCommand {
        CurlCommand {
            url: self.url,
            method: self.method,
            headers: self.headers,
            body: self.body,
            form_fields: self.form_fields,
            multipart_fields: self.multipart_fields,
            multipart_files: self.multipart_files,
            timeout: self.timeout,
            basic_auth: self.basic_auth,
            cookies: self.cookies,
            follow_redirects: self.follow_redirects,
            query_params: self.query_params,
        }
    }
}
```

- [ ] **Step 4: Add `tempfile` to runtime dependencies in Cargo.toml**

Add to `crates/curl-tui-core/Cargo.toml` under `[dependencies]`:
```toml
tempfile = "3"
```

And remove it from `[dev-dependencies]` since it's now a runtime dependency.

- [ ] **Step 5: Add module to lib.rs**

Add `pub mod command;` to `crates/curl-tui-core/src/lib.rs`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p curl-tui-core -- command`
Expected: All tests PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/curl-tui-core/src/command.rs crates/curl-tui-core/src/lib.rs crates/curl-tui-core/Cargo.toml
git commit -m "feat: add CurlCommandBuilder with arg generation, redaction, and response parsing"
```

---

## Task 10: History Module

**Files:**
- Create: `crates/curl-tui-core/src/history.rs`
- Modify: `crates/curl-tui-core/src/lib.rs` — add `pub mod history;`

- [ ] **Step 1: Write failing tests**

Add to `crates/curl-tui-core/src/history.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HistoryEntry, Method};

    fn make_entry(name: &str) -> HistoryEntry {
        HistoryEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            collection_id: None,
            request_name: name.to_string(),
            method: Method::Get,
            url: "https://api.example.com/users".to_string(),
            status_code: Some(200),
            duration_ms: Some(142),
            environment: Some("Dev".to_string()),
        }
    }

    #[test]
    fn test_append_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("history.jsonl");

        append_entry(&path, &make_entry("Request 1")).unwrap();
        append_entry(&path, &make_entry("Request 2")).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_entry_contains_no_secrets() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("history.jsonl");

        let mut entry = make_entry("Secret Request");
        entry.url = "https://api.example.com/users?token=secret123".to_string();

        let secrets = vec!["secret123".to_string()];
        append_entry_redacted(&path, &entry, &secrets).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("secret123"));
        assert!(content.contains("[REDACTED]"));
    }

    #[test]
    fn test_prune_respects_cap() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("history.jsonl");

        for i in 0..15 {
            append_entry(&path, &make_entry(&format!("Request {}", i))).unwrap();
        }

        prune_history(&path, 10).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 10);
        // Should keep the most recent entries
        assert!(content.contains("Request 14"));
        assert!(!content.contains("Request 0"));
    }

    #[test]
    fn test_append_to_nonexistent_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("subdir").join("history.jsonl");

        append_entry(&path, &make_entry("First")).unwrap();
        assert!(path.exists());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core -- history`
Expected: FAIL.

- [ ] **Step 3: Implement history module**

Add to top of `crates/curl-tui-core/src/history.rs` (above `#[cfg(test)]`):
```rust
use crate::secret;
use crate::types::HistoryEntry;
use std::path::Path;

const DEFAULT_MAX_ENTRIES: usize = 10_000;

/// Append a history entry to the JSONL file.
pub fn append_entry(
    path: &Path,
    entry: &HistoryEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(entry)?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

/// Append a history entry with secret values redacted.
pub fn append_entry_redacted(
    path: &Path,
    entry: &HistoryEntry,
    secrets: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut redacted = entry.clone();
    redacted.url = secret::redact_secrets(&redacted.url, secrets)
        .replace(secret::REDACTED_DISPLAY, secret::REDACTED_LOG);
    append_entry(path, &redacted)
}

/// Prune history file to keep only the most recent `max_entries` entries.
pub fn prune_history(
    path: &Path,
    max_entries: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() <= max_entries {
        return Ok(());
    }

    let keep = &lines[lines.len() - max_entries..];
    let pruned = keep.join("\n") + "\n";
    std::fs::write(path, pruned)?;
    Ok(())
}
```

- [ ] **Step 4: Add module to lib.rs**

Add `pub mod history;` to `crates/curl-tui-core/src/lib.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p curl-tui-core -- history`
Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/curl-tui-core/src/history.rs crates/curl-tui-core/src/lib.rs
git commit -m "feat: add JSONL history with secret redaction and pruning"
```

---

## Task 11: TUI Shell — Basic App with Event Loop

**Files:**
- Create: `crates/curl-tui/src/app.rs`
- Create: `crates/curl-tui/src/events.rs`
- Modify: `crates/curl-tui/src/main.rs`

- [ ] **Step 1: Implement App struct with state**

Create `crates/curl-tui/src/app.rs`:
```rust
use curl_tui_core::config::AppConfig;
use curl_tui_core::types::{Collection, Environment, CurlResponse, Request, Method};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pane {
    Collections,
    Request,
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequestTab {
    Headers,
    Body,
    Auth,
    Params,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResponseTab {
    Body,
    Headers,
    Timing,
}

pub enum Action {
    Quit,
    Cancel,
    CyclePaneForward,
    CyclePaneBackward,
    SendRequest,
    SaveRequest,
    SwitchEnvironment,
    NewRequest,
    CopyCurl,
    ToggleCollections,
    ToggleRequest,
    ToggleResponse,
    RevealSecrets,
    Help,
    None,
}

pub struct App {
    pub config: AppConfig,
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub active_environment: Option<usize>,
    pub selected_collection: Option<usize>,
    pub selected_request: Option<usize>,
    pub current_request: Option<Request>,
    pub last_response: Option<CurlResponse>,
    pub active_pane: Pane,
    pub request_tab: RequestTab,
    pub response_tab: ResponseTab,
    pub pane_visible: [bool; 3], // [collections, request, response]
    pub should_quit: bool,
    pub show_help: bool,
    pub secrets_revealed: bool,
    pub status_message: Option<String>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
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
            active_pane: Pane::Request,
            request_tab: RequestTab::Headers,
            response_tab: ResponseTab::Body,
            pane_visible: [true, true, true],
            should_quit: false,
            show_help: false,
            secrets_revealed: false,
            status_message: None,
        }
    }

    /// Toggle a pane's visibility. Ensures at least one pane stays visible.
    pub fn toggle_pane(&mut self, index: usize) {
        let visible_count = self.pane_visible.iter().filter(|&&v| v).count();
        if self.pane_visible[index] && visible_count <= 1 {
            return; // Can't hide the last visible pane
        }
        self.pane_visible[index] = !self.pane_visible[index];

        // If active pane was hidden, switch to first visible
        let active_index = match self.active_pane {
            Pane::Collections => 0,
            Pane::Request => 1,
            Pane::Response => 2,
        };
        if !self.pane_visible[active_index] {
            self.cycle_pane_forward();
        }
    }

    pub fn cycle_pane_forward(&mut self) {
        let panes = [Pane::Collections, Pane::Request, Pane::Response];
        let current = panes.iter().position(|p| *p == self.active_pane).unwrap();
        for i in 1..=3 {
            let next = (current + i) % 3;
            if self.pane_visible[next] {
                self.active_pane = panes[next];
                return;
            }
        }
    }

    pub fn cycle_pane_backward(&mut self) {
        let panes = [Pane::Collections, Pane::Request, Pane::Response];
        let current = panes.iter().position(|p| *p == self.active_pane).unwrap();
        for i in 1..=3 {
            let prev = (current + 3 - i) % 3;
            if self.pane_visible[prev] {
                self.active_pane = panes[prev];
                return;
            }
        }
    }
}
```

- [ ] **Step 2: Implement event handling**

Create `crates/curl-tui/src/events.rs`:
```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;

use crate::app::Action;

/// Poll for terminal events and convert to app actions.
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Map a key event to an app action using default keybindings.
pub fn key_to_action(key: KeyEvent) -> Action {
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }

    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('q')) => Action::Quit,
        (KeyModifiers::NONE, KeyCode::Tab) => Action::CyclePaneForward,
        (KeyModifiers::SHIFT, KeyCode::BackTab) => Action::CyclePaneBackward,
        (KeyModifiers::CONTROL, KeyCode::Enter) => Action::SendRequest,
        (KeyModifiers::NONE, KeyCode::F(5)) => Action::SendRequest,
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => Action::SaveRequest,
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => Action::SwitchEnvironment,
        (KeyModifiers::CONTROL, KeyCode::Char('n')) => Action::NewRequest,
        (KeyModifiers::CONTROL, KeyCode::Char('y')) => Action::CopyCurl,
        (KeyModifiers::CONTROL, KeyCode::Char('1')) => Action::ToggleCollections,
        (KeyModifiers::CONTROL, KeyCode::Char('2')) => Action::ToggleRequest,
        (KeyModifiers::CONTROL, KeyCode::Char('3')) => Action::ToggleResponse,
        (KeyModifiers::NONE, KeyCode::F(8)) => Action::RevealSecrets,
        (KeyModifiers::NONE, KeyCode::Char('?')) => Action::Help,
        (KeyModifiers::NONE, KeyCode::Esc) => Action::Cancel,
        _ => Action::None,
    }
}
```

- [ ] **Step 3: Update main.rs with terminal setup and event loop**

Replace `crates/curl-tui/src/main.rs`:
```rust
mod app;
mod events;

use std::io;
use std::time::Duration;

use crossterm::{
    event::Event,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{Action, App};
use curl_tui_core::config::{config_dir, AppConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load config
    let config_path = config_dir().join("config.json");
    let config = AppConfig::load_from(&config_path)?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(config);

    // Main loop
    let result = run_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| {
            // Placeholder: render a simple message
            let area = frame.area();
            let text = ratatui::text::Text::from("curl-tui v0.1.0 — Press Ctrl+Q or Esc to quit");
            frame.render_widget(
                ratatui::widgets::Paragraph::new(text)
                    .alignment(ratatui::layout::Alignment::Center),
                area,
            );
        })?;

        if let Some(event) = events::poll_event(Duration::from_millis(100))? {
            if let Event::Key(key) = event {
                let action = events::key_to_action(key);
                match action {
                    Action::Quit => {
                        app.should_quit = true;
                    }
                    Action::Cancel => {
                        // Close popups/overlays; if nothing open, do nothing
                        if app.show_help {
                            app.show_help = false;
                        }
                    }
                    Action::CyclePaneForward => app.cycle_pane_forward(),
                    Action::CyclePaneBackward => app.cycle_pane_backward(),
                    Action::ToggleCollections => app.toggle_pane(0),
                    Action::ToggleRequest => app.toggle_pane(1),
                    Action::ToggleResponse => app.toggle_pane(2),
                    Action::RevealSecrets => {
                        app.secrets_revealed = !app.secrets_revealed;
                    }
                    Action::Help => {
                        app.show_help = !app.show_help;
                    }
                    // SendRequest, SaveRequest, SwitchEnvironment, NewRequest,
                    // CopyCurl are wired in Tasks 15-17
                    _ => {}
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
```

- [ ] **Step 4: Verify it compiles and runs**

Run: `cargo build -p curl-tui`
Expected: Compiles successfully.

Run: `cargo run -p curl-tui` (then press Esc or Ctrl+Q to quit)
Expected: Shows placeholder text, quits on keypress.

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui/src/
git commit -m "feat: add TUI shell with event loop, app state, and keybinding dispatch"
```

---

## Task 12: Multi-Pane Layout

**Files:**
- Create: `crates/curl-tui/src/ui/mod.rs`
- Create: `crates/curl-tui/src/ui/layout.rs`
- Create: `crates/curl-tui/src/ui/statusbar.rs`
- Create: `crates/curl-tui/src/ui/collections.rs`
- Create: `crates/curl-tui/src/ui/request.rs`
- Create: `crates/curl-tui/src/ui/response.rs`
- Modify: `crates/curl-tui/src/main.rs` — use `ui::draw`

- [ ] **Step 1: Create layout module**

Create `crates/curl-tui/src/ui/layout.rs`:
```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Computed pane areas based on visibility flags.
pub struct PaneLayout {
    pub title_bar: Rect,
    pub collections: Option<Rect>,
    pub request: Option<Rect>,
    pub response: Option<Rect>,
    pub status_bar: Rect,
}

/// Compute the layout based on which panes are visible.
pub fn compute_layout(area: Rect, visible: [bool; 3]) -> PaneLayout {
    // Top: title bar (1 line), Bottom: status bar (1 line), Middle: panes
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

    let title_bar = vertical[0];
    let main_area = vertical[1];
    let status_bar = vertical[2];

    let [col_vis, req_vis, res_vis] = visible;

    // If only one pane visible, it gets the full area
    let visible_count = visible.iter().filter(|&&v| v).count();
    if visible_count == 0 {
        return PaneLayout {
            title_bar,
            collections: None,
            request: None,
            response: None,
            status_bar,
        };
    }

    // Split: left (collections) | right (request + response)
    let (left_area, right_area) = if col_vis && (req_vis || res_vis) {
        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(main_area);
        (Some(h[0]), Some(h[1]))
    } else if col_vis {
        (Some(main_area), None)
    } else {
        (None, Some(main_area))
    };

    // Split right area into request (top) and response (bottom)
    let (req_area, res_area) = match right_area {
        Some(right) if req_vis && res_vis => {
            let v = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(right);
            (Some(v[0]), Some(v[1]))
        }
        Some(right) if req_vis => (Some(right), None),
        Some(right) if res_vis => (None, Some(right)),
        _ => (None, None),
    };

    PaneLayout {
        title_bar,
        collections: if col_vis { left_area } else { None },
        request: req_area,
        response: res_area,
        status_bar,
    }
}
```

- [ ] **Step 2: Create placeholder pane widgets**

Create `crates/curl-tui/src/ui/collections.rs`:
```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Collections ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    if app.collections.is_empty() {
        let text = Paragraph::new("No collections.\nPress Ctrl+N to create one.")
            .block(block);
        frame.render_widget(text, area);
    } else {
        let items: Vec<String> = app
            .collections
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let marker = if Some(i) == app.selected_collection {
                    ">"
                } else {
                    " "
                };
                format!("{} {}", marker, c.name)
            })
            .collect();
        let text = Paragraph::new(items.join("\n")).block(block);
        frame.render_widget(text, area);
    }
}
```

Create `crates/curl-tui/src/ui/request.rs`:
```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, RequestTab};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = format!(
        " {} | {} | {} | {} ",
        tab_label("Headers", app.request_tab == RequestTab::Headers),
        tab_label("Body", app.request_tab == RequestTab::Body),
        tab_label("Auth", app.request_tab == RequestTab::Auth),
        tab_label("Params", app.request_tab == RequestTab::Params),
    );

    let block = Block::default()
        .title(" Request ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let content = if let Some(req) = &app.current_request {
        format!(
            "[{}] {}\n{}\n\nEdit request details here...",
            req.method, req.url, tabs
        )
    } else {
        format!("{}\n\nNo request selected.", tabs)
    };

    let text = Paragraph::new(content).block(block);
    frame.render_widget(text, area);
}

fn tab_label(name: &str, active: bool) -> String {
    if active {
        format!("[{}]", name)
    } else {
        name.to_string()
    }
}
```

Create `crates/curl-tui/src/ui/response.rs`:
```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, ResponseTab};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = format!(
        " {} | {} | {} ",
        tab_label("Body", app.response_tab == ResponseTab::Body),
        tab_label("Headers", app.response_tab == ResponseTab::Headers),
        tab_label("Timing", app.response_tab == ResponseTab::Timing),
    );

    let block = Block::default()
        .title(" Response ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let content = if let Some(resp) = &app.last_response {
        format!(
            "[{} {}] {:.0}ms\n{}\n\n{}",
            resp.status_code,
            status_text(resp.status_code),
            resp.timing.total_ms,
            tabs,
            &resp.body[..resp.body.len().min(500)]
        )
    } else {
        format!("{}\n\nSend a request to see the response.", tabs)
    };

    let text = Paragraph::new(content).block(block);
    frame.render_widget(text, area);
}

fn tab_label(name: &str, active: bool) -> String {
    if active {
        format!("[{}]", name)
    } else {
        name.to_string()
    }
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "",
    }
}
```

Create `crates/curl-tui/src/ui/statusbar.rs`:
```rust
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let hints = vec![
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(":pane  "),
        Span::styled("Ctrl+Enter/F5", Style::default().fg(Color::Yellow)),
        Span::raw(":send  "),
        Span::styled("Ctrl+S", Style::default().fg(Color::Yellow)),
        Span::raw(":save  "),
        Span::styled("Ctrl+E", Style::default().fg(Color::Yellow)),
        Span::raw(":env  "),
        Span::styled("?", Style::default().fg(Color::Yellow)),
        Span::raw(":help  "),
        Span::styled("Ctrl+Q", Style::default().fg(Color::Yellow)),
        Span::raw(":quit"),
    ];

    let status = Paragraph::new(Line::from(hints))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status, area);
}
```

- [ ] **Step 3: Create ui/mod.rs to wire everything together**

Create `crates/curl-tui/src/ui/mod.rs`:
```rust
pub mod collections;
pub mod layout;
pub mod request;
pub mod response;
pub mod statusbar;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let pane_layout = layout::compute_layout(frame.area(), app.pane_visible);

    // Title bar
    let env_name = app
        .active_environment
        .and_then(|i| app.environments.get(i))
        .map(|e| e.name.as_str())
        .unwrap_or("None");

    let title = Line::from(vec![
        Span::styled(" curl-tui", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(
            format!("[env: {}]", env_name),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  "),
        Span::styled("[v0.1.0]", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().bg(Color::Black)),
        pane_layout.title_bar,
    );

    // Panes
    if let Some(area) = pane_layout.collections {
        collections::draw(frame, app, area);
    }
    if let Some(area) = pane_layout.request {
        request::draw(frame, app, area);
    }
    if let Some(area) = pane_layout.response {
        response::draw(frame, app, area);
    }

    // Status bar
    statusbar::draw(frame, app, pane_layout.status_bar);
}
```

- [ ] **Step 4: Update main.rs to use ui::draw**

In `crates/curl-tui/src/main.rs`, add `mod ui;` and replace the `terminal.draw` closure:
```rust
terminal.draw(|frame| {
    ui::draw(frame, app);
})?;
```

- [ ] **Step 5: Verify it compiles and runs**

Run: `cargo build -p curl-tui`
Expected: Compiles successfully.

Run: `cargo run -p curl-tui` (then press Esc or Ctrl+Q to quit)
Expected: Shows multi-pane layout with title bar, collections, request, response, and status bar.

- [ ] **Step 6: Commit**

```bash
git add crates/curl-tui/src/
git commit -m "feat: add multi-pane TUI layout with collections, request, response, and status bar"
```

---

## Task 13: First-Run Initialization

**Files:**
- Create: `crates/curl-tui-core/src/init.rs`
- Modify: `crates/curl-tui-core/src/lib.rs` — add `pub mod init;`
- Modify: `crates/curl-tui/src/main.rs` — call init on startup

- [ ] **Step 1: Write failing tests**

Add to `crates/curl-tui-core/src/init.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_creates_directory_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("curl-tui");

        initialize(&root).unwrap();

        assert!(root.join("config.json").exists());
        assert!(root.join("collections").is_dir());
        assert!(root.join("environments").is_dir());
        assert!(root.join(".gitignore").exists());
    }

    #[test]
    fn test_init_does_not_overwrite_existing_config() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("curl-tui");

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
        let root = tmp.path().join("curl-tui");

        initialize(&root).unwrap();

        let gitignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(gitignore.contains("environments/"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p curl-tui-core -- init`
Expected: FAIL.

- [ ] **Step 3: Implement init module**

Add to top of `crates/curl-tui-core/src/init.rs` (above `#[cfg(test)]`):
```rust
use crate::config::AppConfig;
use crate::secret;
use std::path::Path;

/// Initialize the curl-tui config directory if it doesn't exist.
/// Creates config.json (with defaults), collections/, environments/, and .gitignore.
/// Does NOT overwrite existing files.
pub fn initialize(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(root)?;
    std::fs::create_dir_all(root.join("collections"))?;
    std::fs::create_dir_all(root.join("environments"))?;

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

    Ok(())
}
```

- [ ] **Step 4: Add module to lib.rs**

Add `pub mod init;` to `crates/curl-tui-core/src/lib.rs`.

- [ ] **Step 5: Update main.rs to call init on startup**

In `crates/curl-tui/src/main.rs`, before loading config, add:
```rust
// Initialize config directory
let config_root = config_dir();
curl_tui_core::init::initialize(&config_root)?;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p curl-tui-core -- init`
Expected: All tests PASS.

- [ ] **Step 7: Verify full app still runs**

Run: `cargo run -p curl-tui`
Expected: App starts, config directory is created at the platform-appropriate path.

- [ ] **Step 8: Commit**

```bash
git add crates/curl-tui-core/src/init.rs crates/curl-tui-core/src/lib.rs crates/curl-tui/src/main.rs
git commit -m "feat: add first-run initialization with config, directories, and gitignore"
```

---

## Task 14: Run Full Verification

- [ ] **Step 1: Run the verify-rust skill equivalent**

```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

Expected: All three commands pass. If clippy or fmt fails, fix the issues and recommit.

- [ ] **Step 2: Verify the app runs end-to-end**

```bash
cargo run -p curl-tui
```

Expected: Multi-pane TUI appears. Press Tab to cycle panes. Press Ctrl+1/2/3 to toggle panes. Press Esc to quit. Terminal is restored cleanly.

- [ ] **Step 3: Tag as v0.1.0-alpha**

```bash
git tag v0.1.0-alpha
```

---

## Task 15: Configurable Keybinding Dispatch

**Files:**
- Create: `crates/curl-tui/src/input.rs`
- Modify: `crates/curl-tui/src/events.rs` — use input module instead of hardcoded mappings
- Modify: `crates/curl-tui/src/main.rs` — pass config to event handling

- [ ] **Step 1: Create input.rs with config-driven keybinding dispatch**

Create `crates/curl-tui/src/input.rs`:
```rust
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::collections::HashMap;

use crate::app::Action;

/// Maps a string keybinding (from config) to a (modifiers, keycode) pair.
fn parse_binding(binding: &str) -> Option<(KeyModifiers, KeyCode)> {
    let parts: Vec<&str> = binding.split('+').collect();
    let mut modifiers = KeyModifiers::NONE;
    let key_part;

    if parts.len() == 1 {
        key_part = parts[0];
    } else {
        for &part in &parts[..parts.len() - 1] {
            match part.to_lowercase().as_str() {
                "ctrl" => modifiers |= KeyModifiers::CONTROL,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                "alt" => modifiers |= KeyModifiers::ALT,
                _ => {}
            }
        }
        key_part = parts[parts.len() - 1];
    }

    let code = match key_part.to_lowercase().as_str() {
        "enter" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "escape" | "esc" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        "/" => KeyCode::Char('/'),
        "?" => KeyCode::Char('?'),
        s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        _ => return None,
    };

    Some((modifiers, code))
}

/// Build a lookup table from (modifiers, keycode) -> Action using config keybindings.
pub fn build_keymap(keybindings: &HashMap<String, String>) -> HashMap<(KeyModifiers, KeyCode), Action> {
    let mut map = HashMap::new();

    let action_map: Vec<(&str, Action)> = vec![
        ("send_request", Action::SendRequest),
        ("save_request", Action::SaveRequest),
        ("switch_env", Action::SwitchEnvironment),
        ("copy_curl", Action::CopyCurl),
        ("new_request", Action::NewRequest),
        ("cycle_panes", Action::CyclePaneForward),
        ("search", Action::Search),
        ("help", Action::Help),
        ("cancel", Action::Cancel),
        ("toggle_collections", Action::ToggleCollections),
        ("toggle_request", Action::ToggleRequest),
        ("toggle_response", Action::ToggleResponse),
        ("reveal_secrets", Action::RevealSecrets),
    ];

    for (key, action) in action_map {
        if let Some(binding) = keybindings.get(key) {
            if let Some(parsed) = parse_binding(binding) {
                map.insert(parsed, action);
            }
        }
    }

    // Always register Ctrl+Q as quit (not remappable)
    map.insert(
        (KeyModifiers::CONTROL, KeyCode::Char('q')),
        Action::Quit,
    );
    // F5 as fallback for send (Ctrl+Enter compatibility)
    map.entry((KeyModifiers::NONE, KeyCode::F(5)))
        .or_insert(Action::SendRequest);
    // Shift+Tab for backward cycling
    map.insert(
        (KeyModifiers::SHIFT, KeyCode::BackTab),
        Action::CyclePaneBackward,
    );

    map
}

/// Look up a key event in the keymap.
pub fn resolve_action(
    key: KeyEvent,
    keymap: &HashMap<(KeyModifiers, KeyCode), Action>,
) -> Action {
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }
    keymap
        .get(&(key.modifiers, key.code))
        .cloned()
        .unwrap_or(Action::None)
}
```

- [ ] **Step 2: Add `Search` variant to Action enum and derive Clone**

In `crates/curl-tui/src/app.rs`, add `Search` to the `Action` enum and add `#[derive(Clone)]`:
```rust
#[derive(Clone)]
pub enum Action {
    Quit,
    Cancel,
    CyclePaneForward,
    CyclePaneBackward,
    SendRequest,
    SaveRequest,
    SwitchEnvironment,
    NewRequest,
    CopyCurl,
    ToggleCollections,
    ToggleRequest,
    ToggleResponse,
    RevealSecrets,
    Help,
    Search,
    None,
}
```

- [ ] **Step 3: Update events.rs to use input module**

Simplify `crates/curl-tui/src/events.rs` — remove the `key_to_action` function (it's replaced by `input::resolve_action`):
```rust
use crossterm::event::{self, Event};
use std::time::Duration;

/// Poll for terminal events.
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
```

- [ ] **Step 4: Update main.rs to build keymap and use it**

In `crates/curl-tui/src/main.rs`, add `mod input;` and update the event loop:
```rust
// After creating app:
let keymap = input::build_keymap(&app.config.keybindings);

// In the event loop, replace events::key_to_action with:
let action = input::resolve_action(key, &keymap);
```

- [ ] **Step 5: Verify it compiles and runs**

Run: `cargo build -p curl-tui`
Expected: Compiles. Keybindings now driven by config.

- [ ] **Step 6: Commit**

```bash
git add crates/curl-tui/src/input.rs crates/curl-tui/src/events.rs crates/curl-tui/src/app.rs crates/curl-tui/src/main.rs
git commit -m "feat: add configurable keybinding dispatch from config.json"
```

---

## Task 16: Startup Data Loading & Request Send/Save Wiring

**Files:**
- Modify: `crates/curl-tui/src/main.rs` — load collections and environments on startup
- Modify: `crates/curl-tui/src/app.rs` — add methods for send, save, new request, switch env

- [ ] **Step 1: Add data loading to main.rs after init**

In `crates/curl-tui/src/main.rs`, after `App::new(config)`:
```rust
use curl_tui_core::collection::list_collections;
use curl_tui_core::environment::list_environments;

// Load existing data
app.collections = list_collections(&config_root.join("collections")).unwrap_or_default();
app.environments = list_environments(&config_root.join("environments")).unwrap_or_default();

// Set active environment from config
if let Some(env_name) = &app.config.active_environment {
    app.active_environment = app
        .environments
        .iter()
        .position(|e| &e.name == env_name);
}
```

- [ ] **Step 2: Add send_request method to App**

In `crates/curl-tui/src/app.rs`, add:
```rust
use curl_tui_core::command::CurlCommandBuilder;
use curl_tui_core::variable::FileVariableResolver;
use curl_tui_core::history::append_entry_redacted;
use curl_tui_core::config::config_dir;
use curl_tui_core::types::{HistoryEntry, Auth, Body};

impl App {
    pub async fn send_request(&mut self) {
        let Some(request) = &self.current_request else {
            self.status_message = Some("No request to send".to_string());
            return;
        };

        if request.url.is_empty() {
            self.status_message = Some("URL is empty".to_string());
            return;
        }

        // Build variable resolver
        let global_vars = self.config.variables.clone();
        let env_vars = self
            .active_environment
            .and_then(|i| self.environments.get(i))
            .map(|e| e.variables.clone());
        let col_vars = self
            .selected_collection
            .and_then(|i| self.collections.get(i))
            .map(|c| c.variables.clone());

        let resolver = FileVariableResolver::new(global_vars, env_vars, col_vars);

        // Resolve URL
        let (resolved_url, mut secrets) = match resolver.resolve(&request.url) {
            Ok(r) => r,
            Err(e) => {
                self.status_message = Some(format!("Variable error: {}", e));
                return;
            }
        };

        // Build command
        let mut builder = CurlCommandBuilder::new(&resolved_url)
            .method(request.method)
            .timeout(self.config.default_timeout);

        // Add headers
        for header in &request.headers {
            if header.enabled {
                let (resolved_val, s) = match resolver.resolve(&header.value) {
                    Ok(r) => r,
                    Err(e) => {
                        self.status_message = Some(format!("Header variable error: {}", e));
                        return;
                    }
                };
                secrets.extend(s);
                builder = builder.header(&header.key, &resolved_val);
            }
        }

        // Add query params
        for param in &request.params {
            if param.enabled {
                let (resolved_val, s) = match resolver.resolve(&param.value) {
                    Ok(r) => r,
                    Err(e) => {
                        self.status_message = Some(format!("Param variable error: {}", e));
                        return;
                    }
                };
                secrets.extend(s);
                builder = builder.query_param(&param.key, &resolved_val);
            }
        }

        // Add body
        if let Some(body) = &request.body {
            match body {
                Body::Json { content } => {
                    let (resolved, s) = match resolver.resolve(content) {
                        Ok(r) => r,
                        Err(e) => {
                            self.status_message = Some(format!("Body variable error: {}", e));
                            return;
                        }
                    };
                    secrets.extend(s);
                    builder = builder.body_json(&resolved);
                }
                Body::Text { content } => {
                    builder = builder.body_text(content);
                }
                Body::Form { fields } => {
                    for field in fields {
                        if field.enabled {
                            let (val, s) = match resolver.resolve(&field.value) {
                                Ok(r) => r,
                                Err(e) => {
                                    self.status_message =
                                        Some(format!("Form variable error: {}", e));
                                    return;
                                }
                            };
                            secrets.extend(s);
                            builder = builder.form_field(&field.key, &val);
                        }
                    }
                }
                Body::Multipart { parts } => {
                    for part in parts {
                        if let Some(file_path) = &part.file_path {
                            builder = builder.multipart_file(&part.name, file_path);
                        } else if let Some(value) = &part.value {
                            builder = builder.multipart_field(&part.name, value);
                        }
                    }
                }
                Body::None => {}
            }
        }

        // Add auth
        if let Some(auth) = &request.auth {
            match auth {
                Auth::Bearer { token } => {
                    let (val, s) = match resolver.resolve(token) {
                        Ok(r) => r,
                        Err(e) => {
                            self.status_message = Some(format!("Auth variable error: {}", e));
                            return;
                        }
                    };
                    secrets.extend(s);
                    builder = builder.header("Authorization", &format!("Bearer {}", val));
                }
                Auth::Basic { username, password } => {
                    let (user, _) = match resolver.resolve(username) {
                        Ok(r) => r,
                        Err(e) => {
                            self.status_message = Some(format!("Auth variable error: {}", e));
                            return;
                        }
                    };
                    let (pass, s) = match resolver.resolve(password) {
                        Ok(r) => r,
                        Err(e) => {
                            self.status_message = Some(format!("Auth variable error: {}", e));
                            return;
                        }
                    };
                    secrets.extend(s);
                    builder = builder.basic_auth(&user, &pass);
                }
                Auth::ApiKey { key, value, location } => {
                    let (val, s) = match resolver.resolve(value) {
                        Ok(r) => r,
                        Err(e) => {
                            self.status_message = Some(format!("Auth variable error: {}", e));
                            return;
                        }
                    };
                    secrets.extend(s);
                    match location {
                        curl_tui_core::types::ApiKeyLocation::Header => {
                            builder = builder.header(key, &val);
                        }
                        curl_tui_core::types::ApiKeyLocation::Query => {
                            builder = builder.query_param(key, &val);
                        }
                    }
                }
                Auth::None => {}
            }
        }

        self.status_message = Some("Sending...".to_string());
        let cmd = builder.build();

        match cmd.execute().await {
            Ok(response) => {
                // Truncate body if needed
                let max_size = self.config.max_response_body_size_bytes as usize;
                let mut resp = response;
                if resp.body.len() > max_size {
                    resp.body.truncate(max_size);
                    resp.body.push_str("\n\n[TRUNCATED — response exceeded size limit]");
                }
                // Update display command with redaction
                resp.raw_command = cmd.to_display_string(&secrets);

                self.status_message = Some(format!(
                    "{} {} — {:.0}ms",
                    resp.status_code,
                    request.method,
                    resp.timing.total_ms
                ));

                // Log to history
                let history_path = config_dir().join("history.jsonl");
                let entry = HistoryEntry {
                    id: uuid::Uuid::new_v4(),
                    timestamp: chrono::Utc::now(),
                    collection_id: self.selected_collection.and_then(|i| self.collections.get(i)).map(|c| c.id),
                    request_name: request.name.clone(),
                    method: request.method,
                    url: resolved_url,
                    status_code: Some(resp.status_code),
                    duration_ms: Some(resp.timing.total_ms as u64),
                    environment: self.active_environment.and_then(|i| self.environments.get(i)).map(|e| e.name.clone()),
                };
                let _ = append_entry_redacted(&history_path, &entry, &secrets);

                self.last_response = Some(resp);
            }
            Err(e) => {
                self.status_message = Some(format!("Error: {}", e));
            }
        }
    }

    pub fn save_current_request(&mut self) {
        let Some(request) = &self.current_request else {
            self.status_message = Some("No request to save".to_string());
            return;
        };

        if let Some(col_idx) = self.selected_collection {
            if let Some(collection) = self.collections.get_mut(col_idx) {
                // Update existing or add new
                if let Some(req_idx) = self.selected_request {
                    if let Some(existing) = collection.requests.get_mut(req_idx) {
                        *existing = request.clone();
                    }
                } else {
                    collection.requests.push(request.clone());
                    self.selected_request = Some(collection.requests.len() - 1);
                }
                let collections_dir = curl_tui_core::config::config_dir().join("collections");
                match curl_tui_core::collection::save_collection(&collections_dir, collection) {
                    Ok(_) => self.status_message = Some("Saved!".to_string()),
                    Err(e) => self.status_message = Some(format!("Save error: {}", e)),
                }
            }
        } else {
            self.status_message = Some("No collection selected. Create one first.".to_string());
        }
    }

    pub fn new_request(&mut self) {
        self.current_request = Some(Request {
            id: uuid::Uuid::new_v4(),
            name: "New Request".to_string(),
            method: Method::Get,
            url: String::new(),
            headers: Vec::new(),
            params: Vec::new(),
            body: None,
            auth: None,
        });
        self.selected_request = None;
        self.last_response = None;
        self.status_message = Some("New request created".to_string());
    }

    pub fn cycle_environment(&mut self) {
        if self.environments.is_empty() {
            self.status_message = Some("No environments configured".to_string());
            return;
        }
        self.active_environment = Some(match self.active_environment {
            Some(i) => (i + 1) % self.environments.len(),
            None => 0,
        });
        if let Some(env) = self.active_environment.and_then(|i| self.environments.get(i)) {
            self.status_message = Some(format!("Environment: {}", env.name));
        }
    }
}
```

- [ ] **Step 3: Wire actions in the event loop**

In `crates/curl-tui/src/main.rs`, update the action match to handle the new actions. Since `send_request` is async, the main loop needs to be async (using tokio runtime):

Update `main.rs` to use `#[tokio::main]`:
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... existing setup code ...
    let result = run_loop(&mut terminal, &mut app, &keymap).await;
    // ... existing cleanup ...
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    keymap: &HashMap<(KeyModifiers, KeyCode), Action>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Some(event) = events::poll_event(Duration::from_millis(100))? {
            if let Event::Key(key) = event {
                let action = input::resolve_action(key, keymap);
                match action {
                    Action::Quit => app.should_quit = true,
                    Action::Cancel => {
                        if app.show_help { app.show_help = false; }
                    }
                    Action::CyclePaneForward => app.cycle_pane_forward(),
                    Action::CyclePaneBackward => app.cycle_pane_backward(),
                    Action::ToggleCollections => app.toggle_pane(0),
                    Action::ToggleRequest => app.toggle_pane(1),
                    Action::ToggleResponse => app.toggle_pane(2),
                    Action::RevealSecrets => app.secrets_revealed = !app.secrets_revealed,
                    Action::Help => app.show_help = !app.show_help,
                    Action::SendRequest => app.send_request().await,
                    Action::SaveRequest => app.save_current_request(),
                    Action::NewRequest => app.new_request(),
                    Action::SwitchEnvironment => app.cycle_environment(),
                    Action::CopyCurl => {
                        // Build display string from current request and copy
                        if let Some(req) = &app.current_request {
                            let cmd = CurlCommandBuilder::new(&req.url).method(req.method).build();
                            let display = cmd.to_display_string(&[]);
                            app.status_message = Some(format!("Copied: {}", display));
                            // Full clipboard integration deferred — display in status for now
                        }
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit { return Ok(()); }
    }
}
```

- [ ] **Step 4: Verify it compiles and runs**

Run: `cargo build -p curl-tui`
Expected: Compiles. App loads collections/environments from disk and wires all actions.

- [ ] **Step 5: Commit**

```bash
git add crates/curl-tui/src/
git commit -m "feat: wire send, save, new request, switch env, and startup data loading"
```

---

## Task 17: Integration Tests

**Files:**
- Create: `tests/integration_test.rs`
- Modify: `Cargo.toml` (workspace root) — ensure test dependencies

- [ ] **Step 1: Add test dependencies to workspace root**

In root `Cargo.toml`, add:
```toml
[workspace.dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

- [ ] **Step 2: Write integration tests**

Create `tests/integration_test.rs`:
```rust
use curl_tui_core::collection::{save_collection, load_collection, list_collections, slugify};
use curl_tui_core::command::CurlCommandBuilder;
use curl_tui_core::config::AppConfig;
use curl_tui_core::environment::{save_environment, load_environment};
use curl_tui_core::history::{append_entry_redacted, prune_history};
use curl_tui_core::init::initialize;
use curl_tui_core::secret::{redact_secrets, generate_gitignore};
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
            Variable { value: "30".to_string(), secret: false },
        );
        m
    };

    let env = {
        let mut m = HashMap::new();
        m.insert(
            "base_url".to_string(),
            Variable { value: "https://staging.example.com".to_string(), secret: false },
        );
        m.insert(
            "api_token".to_string(),
            Variable { value: "stg-secret-123".to_string(), secret: true },
        );
        m
    };

    let col = {
        let mut m = HashMap::new();
        m.insert(
            "base_url".to_string(),
            Variable { value: "https://override.example.com".to_string(), secret: false },
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
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test --workspace`
Expected: All unit and integration tests pass.

- [ ] **Step 4: Commit**

```bash
git add tests/ Cargo.toml
git commit -m "feat: add integration tests for collection workflow, variables, security, and command builder"
```

---

## Task 18: Final Verification

- [ ] **Step 1: Run the full verify-rust suite**

```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

Expected: All pass. Fix any issues and recommit.

- [ ] **Step 2: Verify the app runs end-to-end**

```bash
cargo run -p curl-tui
```

Expected: Multi-pane TUI appears. Tab cycles panes. Ctrl+1/2/3 toggles panes. Ctrl+N creates a new request. Ctrl+Q quits. Terminal is restored cleanly.

- [ ] **Step 3: Tag as v0.1.0-alpha**

```bash
git tag v0.1.0-alpha
```

---

## Summary

| Task | Module | Tests |
|---|---|---|
| 1 | Project scaffolding | Build check |
| 2 | `.claude/` config | N/A (config files) |
| 3 | Core data types | 12 serde roundtrip tests |
| 4 | Secret handling | 8 redaction tests |
| 5 | Variable resolution | 9 resolver tests |
| 6 | Config module | 8 config tests |
| 7 | Collection management | 7 CRUD tests |
| 8 | Environment management | 4 CRUD tests |
| 9 | CurlCommandBuilder | 15 builder/parser tests |
| 10 | History module | 4 history tests |
| 11 | TUI shell | Build + manual run |
| 12 | Multi-pane layout | Build + manual run |
| 13 | First-run init | 3 init tests |
| 14 | Verification checkpoint | All tests + manual run |
| 15 | Configurable keybinding dispatch | Build + manual run |
| 16 | Send/Save/New/Env wiring | Build + manual run |
| 17 | Integration tests | 5 end-to-end tests |
| 18 | Final verification | All tests + tag |

**Total: ~75 automated tests across 18 tasks.**
