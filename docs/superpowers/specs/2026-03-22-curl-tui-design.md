# curl-tui Design Specification

**Date:** 2026-03-22
**Status:** Draft
**Goal:** A terminal-native Postman replacement built in Rust, security-first, easily customisable, inspired by LazyCurl.

---

## 1. Overview

curl-tui is a TUI application for composing, managing, and executing HTTP requests via curl. It provides a multi-pane interface with collections, environments, and hierarchical variables — similar to Postman but without the security pitfalls (leaked secrets in logs, cloud sync exposure, telemetry).

The project targets developers who prefer terminal workflows and want full control over their HTTP tooling.

## 2. Architecture

**Cargo workspace with two crates:**

- **`curl-tui-core`** (library) — All business logic: command building, variable resolution, collection management, secret handling, config parsing. Fully testable without a terminal.
- **`curl-tui`** (binary) — Thin TUI layer using Ratatui + crossterm. Reads state from core, dispatches actions to core. Contains no business logic.

```
curl-tui/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── curl-tui-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── command.rs       # CurlCommandBuilder
│   │       ├── collection.rs    # Collection CRUD, serialization
│   │       ├── variable.rs      # Hierarchical variable resolution
│   │       ├── secret.rs        # Secret marking, redaction, gitignore
│   │       ├── environment.rs   # Environment management
│   │       ├── config.rs        # App config, keybindings
│   │       └── history.rs       # Request history (v1: write-only logging, browsing deferred)
│   └── curl-tui/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── app.rs           # App state, event loop
│           ├── ui/
│           │   ├── mod.rs
│           │   ├── layout.rs    # Multi-pane layout manager
│           │   ├── collections.rs
│           │   ├── request.rs
│           │   ├── response.rs
│           │   └── statusbar.rs
│           ├── input.rs         # Keybinding dispatch
│           └── events.rs        # Terminal event handling
├── .claude/                    # Project-local Claude hooks & skills
├── docs/
└── tests/                      # Integration tests
```

**Key dependencies:**
- `ratatui` + `crossterm` — TUI rendering and terminal backend
- `serde` + `serde_json` — Serialization
- `tokio` — Async subprocess execution
- `uuid` — Request/collection IDs
- `dirs` — Default config directory resolution

## 3. Data Model

### Collections

Stored as individual JSON files in `collections/`. Files are named by slugifying the collection name (lowercase, spaces to hyphens, special characters stripped) — e.g., "My API" becomes `my-api.json`. If a slug collision occurs, a numeric suffix is appended (`my-api-2.json`). The `id` field (UUID) is the canonical identifier; the filename is for human readability only.

```json
{
  "id": "uuid",
  "name": "My API",
  "variables": {
    "base_path": { "value": "/api/v1", "secret": false }
  },
  "requests": [
    {
      "id": "uuid",
      "name": "Get Users",
      "method": "GET",
      "url": "{{base_url}}{{base_path}}/users",
      "headers": [
        { "key": "Authorization", "value": "Bearer {{api_token}}", "enabled": true }
      ],
      "body": {
        "type": "json",
        "content": "{\"page\": 1}"
      },
      "params": [
        { "key": "page", "value": "1", "enabled": true },
        { "key": "limit", "value": "20", "enabled": true }
      ],
      "auth": {
        "type": "bearer",
        "token": "{{api_token}}"
      }
    }
  ]
}
```

### Environments

Separate JSON files in `environments/`:

```json
{
  "id": "uuid",
  "name": "Development",
  "variables": {
    "base_url": { "value": "http://localhost:3000", "secret": false },
    "api_token": { "value": "dev-token-123", "secret": true }
  }
}
```

### Global Variables

In app `config.json`:

```json
{
  "variables": {
    "default_timeout": { "value": "30", "secret": false }
  }
}
```

### Unified Config Schema

All app configuration lives in a single `config.json`:

```json
{
  "variables": {
    "default_timeout": { "value": "30", "secret": false }
  },
  "keybindings": {
    "send_request": "ctrl+enter",
    "save_request": "ctrl+s",
    "switch_env": "ctrl+e",
    "copy_curl": "ctrl+y",
    "new_request": "ctrl+n",
    "cycle_panes": "tab",
    "search": "/",
    "help": "?",
    "cancel": "escape",
    "toggle_collections": "ctrl+1",
    "toggle_request": "ctrl+2",
    "toggle_response": "ctrl+3",
    "reveal_secrets": "f8"
  },
  "active_environment": null,
  "default_timeout": 30,
  "max_response_body_size_bytes": 10485760,
  "debug_logging": false
}
```

### Storage Paths

All data lives under a single config root:
- **Linux/macOS:** `~/.config/curl-tui/`
- **Windows:** `%APPDATA%/curl-tui/`

```
~/.config/curl-tui/
├── config.json
├── collections/
│   ├── my-api.json
│   └── auth-api.json
├── environments/
│   ├── development.json
│   └── production.json
├── history.jsonl
└── .gitignore
```

### History Format

History is a JSONL (JSON Lines) file — one JSON object per line, append-only. In v1, history is write-only (no browsing UI). Each entry:

```json
{
  "id": "uuid",
  "timestamp": "2026-03-22T14:30:00Z",
  "collection_id": "uuid-or-null",
  "request_name": "Get Users",
  "method": "GET",
  "url": "https://api.example.com/api/v1/users",
  "status_code": 200,
  "duration_ms": 142,
  "environment": "Development"
}
```

Secret values in the URL or headers are replaced with `[REDACTED]` before writing. The response body is **not** stored in history (to keep the file small). History is capped at 10,000 entries; when exceeded, the oldest entries are pruned on next write.

### Variable Resolution

Priority (highest wins): Collection > Active Environment > Global.

Syntax: `{{variable_name}}` (Postman-compatible).

**Undefined variables:** If a `{{var}}` has no value in any scope, the request is **not blocked**. The literal `{{var}}` is highlighted in red in the UI as a warning, and the user is shown a confirmation prompt before sending: `"Unresolved variables: {{var}}. Send anyway? (y/n)"`. If confirmed, the literal string `{{var}}` is sent as-is.

**Circular variable references:** If resolution detects a cycle (e.g., `a` references `{{b}}` and `b` references `{{a}}`), resolution is aborted with a maximum depth of 10. The offending variables are highlighted in red and an error is shown: `"Circular variable reference detected: a -> b -> a"`. The request cannot be sent until the cycle is fixed.

### Auth Precedence

If a request has both an `auth` object and a manually added `Authorization` header, the `auth` object takes precedence and the manual header is ignored (with a visual indicator in the Headers tab showing it's overridden). This avoids silent conflicts.

### Body Types

The `body.type` field accepts: `json`, `form`, `multipart`, `text`, `none`.

For `json` and `text`, the body is a simple content string:
```json
{ "type": "json", "content": "{\"page\": 1}" }
{ "type": "text", "content": "raw body content" }
```

For `form` (`application/x-www-form-urlencoded`):
```json
{
  "type": "form",
  "fields": [
    { "key": "username", "value": "{{user}}", "enabled": true },
    { "key": "password", "value": "{{pass}}", "enabled": true }
  ]
}
```

For `multipart` (`multipart/form-data`, file uploads and form fields):
```json
{
  "type": "multipart",
  "parts": [
    { "name": "file", "file_path": "/path/to/file.png" },
    { "name": "description", "value": "Profile photo" }
  ]
}
```

For `none` — the `body` field is either absent or: `{ "type": "none" }`. Both representations are equivalent. When `body` is absent, it is treated as `none`.

### Query Parameters

The `params` field is a first-class array on the request object. Parameters can be enabled/disabled individually. When building the curl command, enabled params are appended to the URL as `?key=value&key2=value2` (URL-encoded). The Params tab in the UI provides a key-value editor; editing params there updates the URL bar and vice versa (they are kept in sync).

### Auth Types

The `auth` object supports:
- `{"type": "bearer", "token": "{{api_token}}"}` — Adds `Authorization: Bearer <token>` header
- `{"type": "basic", "username": "user", "password": "{{pass}}"}` — Adds `Authorization: Basic <base64>` header
- `{"type": "api_key", "key": "X-API-Key", "value": "{{key}}", "in": "header"}` — `in` can be `"header"` or `"query"`
- `{"type": "none"}` — No auth

## 4. CurlCommandBuilder & Execution

The builder constructs curl CLI arguments and can:
1. Render the command as a display string (with secret redaction)
2. Execute it as a subprocess

**Builder API:**
```rust
CurlCommandBuilder::new("https://api.example.com/users")
    .method(Method::POST)
    .header("Content-Type", "application/json")
    .header("Authorization", "Bearer tok_123")
    .body_json(r#"{"name": "Alice"}"#)
    .timeout(30)
    .follow_redirects(true)
    .cookie("session=abc123")
    .upload_file("/path/to/file.png")
    .basic_auth("user", "pass")
    .build()
```

**Execution flow:**
1. Builder produces `Vec<String>` of args
2. Variable resolver interpolates `{{vars}}`, tracking which are secret
3. Spawn `curl` subprocess with args plus:
   - `-s` (silent)
   - `-o <tempfile>` (write response body to a temp file to avoid stdout interleaving)
   - `-D <tempfile>` (dump response headers to a separate temp file)
   - `-w '\n%{json}'` (structured write-out to stdout — now the only stdout content)
4. Read body from temp file, headers from header temp file, parse write-out JSON from stdout
5. Parse into `CurlResponse`:
   ```rust
   struct CurlResponse {
       status_code: u16,
       headers: Vec<(String, String)>,
       body: String,
       timing: ResponseTiming,
       raw_command: String,  // redacted
   }
   ```
6. Clean up temp files
7. Log to history with secrets redacted

**Response limits:**
- Response body is capped at `max_response_body_size_bytes` (default 10 MB) from config
- If the body exceeds the limit, it is truncated with a `[TRUNCATED — response exceeded 10 MB]` marker
- Streaming/chunked responses are accumulated up to the limit, then cut off

**Error handling:**
- curl not in PATH: clear error with install instructions. On Windows, explicitly invoke `curl.exe` to avoid the PowerShell `curl` alias.
- curl exit codes mapped to meaningful errors
- Malformed responses: show raw output with error context
- Default timeout of 30s if none specified (configurable in config.json)

## 5. TUI Layout

Multi-pane layout with toggling:

```
┌─────────────────────────────────────────────────────────────────┐
│ curl-tui                              [env: Development] [v0.1]│
├──────────────┬──────────────────────────────────────────────────┤
│ Collections  │  Request                                        │
│              │  [GET ▼] https://{{base_url}}/api/v1/users      │
│ ▼ My API     │  Headers │ Body │ Auth │ Params                 │
│   GET Users  │  Content-Type: application/json                 │
│   POST User  │  Authorization: Bearer ••••••                   │
│ ▶ Auth API   │                                                 │
│              ├──────────────────────────────────────────────────┤
│              │  Response                    [200 OK] [142ms]   │
│              │  Body │ Headers │ Timing                        │
│              │  {                                              │
│              │    "users": [{"id": 1, "name": "Alice"}]        │
│              │  }                                              │
├─────────────────────────────────────────────────────────────────┤
│ Tab:pane  Ctrl+Enter:send  Ctrl+S:save  Ctrl+E:env  ?:help    │
└─────────────────────────────────────────────────────────────────┘
```

**Panes:**
- **Collections (left):** Tree view, expand/collapse, CRUD operations
- **Request (top-right):** Tabbed (Headers, Body, Auth, Params), inline editing, variable autocomplete on `{{`
- **Response (bottom-right):** Tabbed (Body, Headers, Timing), syntax highlighting for JSON/XML/HTML
- **Status bar:** Context-sensitive keybinding hints, active environment

**Pane toggling:** `Ctrl+1/2/3` to hide/show panes; remaining panes resize to fill. At least one pane must remain visible — toggling off the last visible pane is a no-op.

## 6. Keybindings

Default keybindings (all remappable via `config.json`):

| Action | Default | Config key |
|---|---|---|
| Cycle panes | `Tab` / `Shift+Tab` | `cycle_panes` |
| Send request | `Ctrl+Enter` | `send_request` |
| Save request | `Ctrl+S` | `save_request` |
| Switch environment | `Ctrl+E` | `switch_env` |
| New request | `Ctrl+N` | `new_request` |
| Copy as curl | `Ctrl+Y` | `copy_curl` |
| Search in pane | `/` | `search` |
| Help overlay | `?` | `help` |
| Close popup / cancel | `Esc` | `cancel` |
| Toggle collections pane | `Ctrl+1` | `toggle_collections` |
| Toggle request pane | `Ctrl+2` | `toggle_request` |
| Toggle response pane | `Ctrl+3` | `toggle_response` |
| Reveal secrets (temporary) | `F8` | `reveal_secrets` |

**Note on `Ctrl+Enter`:** Some terminal emulators (older xterm, basic TTYs) do not distinguish `Ctrl+Enter` from `Enter`. Crossterm detects this via the Kitty keyboard protocol where available. For terminals that lack support, the app falls back to `F5` as an alternative send binding. Both bindings are active simultaneously; the user can remap either via config.

Config format:
```json
{
  "keybindings": {
    "send_request": "ctrl+enter",
    "save_request": "ctrl+s",
    "switch_env": "ctrl+e"
  }
}
```

## 7. Security Model

**Principle: secrets never leak by default.**

### Secret Variables
- Marked `"secret": true` in definition
- Displayed as `••••••` everywhere in UI
- Temporary reveal via `F8` — reverts on pane switch or configurable timeout
- "Copy as curl" redacts secrets unless user confirms exposure

### History & Logging
- History replaces secret values with `[REDACTED]`
- No debug logs written to disk by default
- Debug logging (if enabled) scrubs secrets before writing

### File Security
- First run generates `.gitignore` excluding environment files with secrets
- Warning if saving secret-containing environment to a non-gitignored path
- Collections store `{{variable_references}}` only, never resolved secret values

### What We Avoid (Postman's Mistakes)
- No cloud sync
- No telemetry/analytics
- No storing resolved secrets in any file
- No logging secrets in crash reports
- Environments with secrets gitignored by default

### Future: External Secret Stores
- `VariableResolver` trait in `variable.rs` with default file-based implementation
- Future resolvers: `KeychainResolver`, `VaultResolver` via `secret://` URI scheme
- Designed into the trait boundary, not implemented in v1

## 8. Testing Strategy

### Unit Tests (curl-tui-core)
- **command.rs** — Correct arg vectors for all method/header/body/auth combinations; secret redaction in string output
- **variable.rs** — Hierarchical resolution, undefined variables, circular references, edge cases
- **secret.rs** — Redaction in all contexts (display, history, copy); gitignore generation; secrets never appear in serialized output
- **collection.rs** — CRUD operations, serialization roundtrips, malformed file handling
- **environment.rs** — Loading, switching, variable merging
- **config.rs** — Keybinding parsing, default fallbacks, invalid config

### Integration Tests
- End-to-end: build command with variables → resolve → execute against local test server (`wiremock`/`mockito`) → parse response → verify
- Collection workflow: create → add request → save → reload → verify identical
- Security: construct request with secrets → serialize to history → assert no secrets in output

### What We Don't Test
- TUI rendering (visual, fragile). The UI layer is thin — logic lives in core.

## 9. Local Installation & Development

**Prerequisites:** Rust toolchain (via `rustup`), `curl` in PATH.

```bash
cargo build              # Development build
cargo run                # Run in dev
cargo test --workspace   # Run all tests
cargo build --release    # Release binary
cargo install --path crates/curl-tui  # Install to ~/.cargo/bin/
```

**First run** creates `~/.config/curl-tui/` (Linux/macOS) or `%APPDATA%/curl-tui/` (Windows) with default config, empty collections/environments, and `.gitignore`.

## 10. `.claude/` Project Configuration

- **`verify-rust` skill** — Runs `cargo fmt --all --check` + `cargo clippy --workspace` + `cargo test --workspace`. Used after every feature or bugfix.
- **Pre-commit hook** — Runs verify-rust before commits. Blocks on failure.
- **Post-write hook** — Runs `cargo check --workspace` after writes to `*.rs` files for fast compilation feedback.
- **CLAUDE.md** — Project conventions, security rules, testing requirements.

## 11. Initial Feature Scope (v1)

**Included:**
- GET/POST/PUT/DELETE/PATCH/HEAD/OPTIONS methods
- Custom headers with enable/disable toggle
- Request body (JSON, form-data, raw text)
- Authentication helpers (Bearer, Basic, API key)
- File uploads (multipart)
- Cookie support
- Response display with syntax highlighting
- Collections with save/load
- Environments with switching
- Hierarchical variables with `{{}}` syntax (Collection overrides Environment overrides Global)
- Secret variable protection
- Copy as curl command
- Customisable keybindings
- `.gitignore` auto-generation for secrets

**Deferred to later versions:**
- Request history browsing UI (v1 writes history, but no TUI for browsing it)
- Proxy support
- Client certificates
- Request chaining
- External secret store integration
- Plugin system
- Import from Postman/Insomnia
