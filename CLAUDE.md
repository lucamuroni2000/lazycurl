# curl-tui

Terminal-native Postman replacement built in Rust.

## Quick Start

```bash
cargo build --workspace          # Build everything
cargo run -p curl-tui            # Launch the TUI
cargo test --workspace           # Run all 88 tests
cargo install --path crates/curl-tui  # Install to ~/.cargo/bin/
```

**Windows gotcha:** If `cargo` is not in PATH, use the full path: `/c/Users/<user>/.cargo/bin/cargo`.

## Architecture

Cargo workspace with two crates:
- `curl-tui-core` (library) — all business logic, fully testable without a terminal
- `curl-tui` (binary) — thin Ratatui + crossterm TUI layer

### Core modules (`crates/curl-tui-core/src/`)

| Module | Purpose |
|---|---|
| `types.rs` | All shared data types: Request, Collection, Environment, Variable, Body, Auth, Method |
| `command.rs` | `CurlCommandBuilder` — builds curl CLI args, executes subprocess, parses response |
| `variable.rs` | Hierarchical variable resolver (Collection > Environment > Global) with cycle detection |
| `secret.rs` | Redaction (`••••••` for display, `[REDACTED]` for logs), gitignore generation |
| `collection.rs` | Collection CRUD with slug-based file naming |
| `environment.rs` | Environment CRUD |
| `config.rs` | `AppConfig` with keybinding merge, file persistence |
| `history.rs` | Append-only JSONL history with secret scrubbing |
| `init.rs` | First-run directory setup |

### TUI modules (`crates/curl-tui/src/`)

| Module | Purpose |
|---|---|
| `app.rs` | App state, `InputMode` (Normal/Editing), `Action` enum, all state mutation methods |
| `input.rs` | Config-driven keybinding dispatch, `resolve_action`/`resolve_navigation`/`resolve_editing` |
| `text_input.rs` | Reusable single-line text input with cursor |
| `ui/` | All rendering: layout, collections, request, response, statusbar, help, variables overlays |

### Input mode pattern

All input goes through a two-mode system:
- **Normal** — keypresses map to Actions via the configurable keymap, then fall back to `resolve_navigation` for arrow keys/vim keys
- **Editing** — keypresses go to the focused `TextInput` field; only Esc, Enter, Ctrl+Q, and Tab escape editing

This is the central dispatch pattern in `main.rs:run_loop`.

## Data Storage

First run creates the config directory:
- **Linux/macOS:** `~/.config/curl-tui/`
- **Windows:** `%APPDATA%/curl-tui/`

Contains: `config.json`, `collections/`, `environments/`, `history.jsonl`, `.gitignore`

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

- Run all tests: `cargo test --workspace` (88 tests: 72 unit + 11 text_input + 5 integration)
- Run specific crate: `cargo test -p curl-tui-core`
- Formatting: `cargo fmt --all --check`
- Linting: `cargo clippy --workspace -- -D warnings`
- Full verify: `/verify-rust` (runs fmt + clippy + test)

## Dependencies

Core: serde, serde_json, tokio, uuid, dirs, chrono, thiserror, tempfile
TUI: ratatui, crossterm, serde_json
