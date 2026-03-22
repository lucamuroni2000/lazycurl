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
