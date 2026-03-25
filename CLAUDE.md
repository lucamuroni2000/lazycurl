# lazycurl

Terminal-native Postman replacement built in Rust.

## Quick Start

```bash
cargo build --workspace          # Build everything
cargo run -p lazycurl            # Launch the TUI
cargo test --workspace           # Run all tests (~115)
cargo install --path crates/lazycurl  # Install to ~/.cargo/bin/
```

**Windows gotcha:** If `cargo` is not in PATH, use the full path: `/c/Users/<user>/.cargo/bin/cargo`.

**Prerequisite:** `curl` must be installed and in PATH. On Windows, `curl.exe` is used explicitly to avoid the PowerShell alias.

## Architecture

Cargo workspace with two crates:
- `lazycurl-core` (library) — all business logic, fully testable without a terminal
- `lazycurl` (binary) — thin Ratatui + crossterm TUI layer

### Core modules (`crates/lazycurl-core/src/`)

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
| `project.rs` | Project CRUD, slug-based directory management |
| `migration.rs` | One-time migration from flat layout to project-based structure |

### TUI modules (`crates/lazycurl/src/`)

| Module | Purpose |
|---|---|
| `app.rs` | App state, `InputMode` (Normal/Editing), `Action` enum, all state mutation methods |
| `input.rs` | Config-driven keybinding dispatch, `resolve_action`/`resolve_navigation`/`resolve_editing` |
| `text_input.rs` | Reusable single-line text input with cursor |
| `ui/` | All rendering (see below) |

### UI sub-modules (`crates/lazycurl/src/ui/`)

| Module | Purpose |
|---|---|
| `layout.rs` | Pane layout computation |
| `collections.rs` | Collections sidebar rendering |
| `request.rs` | Request pane (method bar, headers, body, params) + method picker dropdown |
| `response.rs` | Response pane (body, headers, timing) |
| `statusbar.rs` | Context-sensitive status bar with keybinding hints |
| `help.rs` | Help overlay with keybinding reference |
| `variables.rs` | Variables editor overlay (Global/Environment/Collection tiers) |
| `environment_manager.rs` | Environment CRUD modal (accessed via variables overlay) |
| `picker.rs` | Collection picker for save-to-collection flow |
| `project_picker.rs` | Project switcher overlay |
| `project_tabs.rs` | Project tab bar in title row |

### Input mode pattern

All input goes through a two-mode system:
- **Normal** — keypresses map to Actions via the configurable keymap, then fall back to `resolve_navigation` for arrow keys/vim keys
- **Editing** — keypresses go to the focused `TextInput` field; only Esc, Enter, Ctrl+Q, and Tab escape editing

This is the central dispatch pattern in `main.rs:run_loop`.

Several modal overlays intercept input when open, checked in priority order in `run_loop`: method picker > collection picker > project picker > env manager > variables overlay > delete confirmation > normal dispatch. Each modal fully captures keypresses while active.

## Data Storage

First run creates the config directory:
- **Linux/macOS:** `~/.config/lazycurl/`
- **Windows:** `%APPDATA%/lazycurl/`

Structure:
```
lazycurl/
  config.json
  history.jsonl
  .gitignore
  projects/
    <project-slug>/
      project.json
      collections/
      environments/
```

Legacy flat layouts (collections/environments at root) are auto-migrated to project-based structure on first run via `migration.rs`.

## Conventions

- All business logic goes in `lazycurl-core`. The binary crate contains NO logic.
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

## TDD Workflow

Every new feature or bugfix follows test-first development:

1. **Write the failing test** in the relevant module's `#[cfg(test)]` block
2. **Run it** to confirm it fails: `cargo test -p lazycurl-core -- <module>::tests::<test_name>`
3. **Write the minimal implementation** to make it pass
4. **Run again** to confirm it passes
5. **Commit** the test and implementation together

### Where tests live

- **Unit tests:** Inline `#[cfg(test)] mod tests` at the bottom of each source file in `crates/lazycurl-core/src/`
- **Integration tests:** `crates/lazycurl-core/tests/integration_test.rs` — end-to-end workflows (collection CRUD, variable resolution, secret redaction, command building)
- **Widget tests:** `crates/lazycurl/src/text_input.rs` — inline tests for the TextInput widget
- **TUI rendering:** Not tested in CI — the UI layer is kept thin, all logic lives in testable core modules

### Running tests

```bash
cargo test --workspace                          # All tests (~115)
cargo test -p lazycurl-core                     # Core library only (~96)
cargo test -p lazycurl-core -- secret           # Only secret module tests
cargo test -p lazycurl-core -- variable::tests  # Only variable module tests
cargo test -p lazycurl                          # TUI crate tests (11 text_input tests)
```

### Verification

- Formatting: `cargo fmt --all --check`
- Linting: `cargo clippy --workspace -- -D warnings`
- Full verify: `/verify-rust` (runs fmt + clippy + test in sequence)

## Gotchas

- Crossterm reports modifiers inconsistently across terminals. `resolve_action` in `input.rs` normalizes by trying without SHIFT for punctuation and lowercase for Ctrl+letter combos.
- Characters like `{`, `[`, `@` on non-US keyboards require AltGr (reported as ALT|CONTROL by crossterm). The editing resolver accepts any modifier combo for printable chars, excluding only pure Ctrl+letter shortcuts.
- `Ctrl+Enter` doesn't work on all terminals. `F5` is always registered as a fallback for Send Request.
- JSON response detection checks body content first (not Content-Type header) since many APIs return wrong headers.

## CI/CD

- **CI** (`.github/workflows/ci.yml`): Runs on push to main and PRs — fmt check, clippy, cross-platform tests (Linux, macOS, Windows)
- **Release** (`.github/workflows/release.yml`): Triggered by `v*.*.*` tags — builds release binaries for Linux x86_64, macOS x86_64 + ARM, Windows x86_64, then creates a GitHub Release with assets

## Dependencies

Core: serde, serde_json, tokio, uuid, dirs, chrono, thiserror, tempfile
TUI: ratatui, crossterm, serde_json
