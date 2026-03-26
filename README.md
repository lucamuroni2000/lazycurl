# lazycurl

A terminal-native API client, built as a lightweight alternative to Postman. Runs entirely in your terminal — no browser, no Electron, no account required.

Built with Rust using [Ratatui](https://ratatui.rs/) and [crossterm](https://github.com/crossterm-rs/crossterm).

## Why lazycurl

- **Fast** — starts instantly, no loading screens
- **Portable** — single binary, works over SSH
- **Offline** — no cloud sync, no telemetry, your data stays local
- **Keyboard-driven** — vim-style navigation, everything accessible without a mouse

## Features

- Send HTTP requests (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
- **Authentication** — Bearer token, Basic auth, API key (header or query)
- Organize requests into **collections** and **projects**
- **Environment variables** with three-tier resolution (Collection > Environment > Global)
- **Secret variables** — marked values are redacted in history, logs, and exports
- **Export** — curl command, Postman Collection v2.1, OpenAPI 3.0
- Configurable keybindings
- Request history with automatic secret scrubbing
- JSON response highlighting
- Built-in log viewer with filtering

## Installation

### From source

Requires [Rust](https://rustup.rs/) and `curl` in PATH.

```bash
git clone https://github.com/lucamuroni2000/lazycurl.git
cd lazycurl
cargo install --path crates/lazycurl
```

### From releases

Download a prebuilt binary from the [Releases](https://github.com/lucamuroni2000/lazycurl/releases) page. Available for Linux (x86_64), macOS (x86_64, ARM), and Windows (x86_64).

## Quick start

Launch lazycurl:

```bash
lazycurl
```

On first run it creates a config directory (`~/.config/lazycurl/` on Linux/macOS, `%APPDATA%/lazycurl/` on Windows) with a default project.

### Basic workflow

1. Press `Enter` to edit the URL field
2. Type your URL and press `Esc` to confirm
3. Press `m` to change the HTTP method
4. Press `F5` to send the request
5. Press `Ctrl+S` to save to a collection

### Key bindings

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle between panes |
| `Enter` | Edit field / load request |
| `Esc` | Stop editing / close overlay |
| `m` | Change HTTP method |
| `F5` / `Ctrl+Enter` | Send request |
| `Ctrl+S` | Save request |
| `Ctrl+N` | New request / collection |
| `Ctrl+E` | Cycle environment |
| `Ctrl+Shift+E` | Manage environments |
| `Ctrl+Y` | Export request (curl / Postman / OpenAPI) |
| `v` | Open variables editor |
| `F8` | Reveal secret values |
| `y` | Copy response body to clipboard |
| `/` | Search |
| `Ctrl+L` | Open log viewer |
| `?` | Help |
| `Ctrl+Q` | Quit |

All keybindings are configurable via `config.json`.

## Project structure

```
lazycurl/
  crates/
    lazycurl-core/   # Library — all business logic, no UI dependencies
    lazycurl/        # Binary — thin TUI layer
```

All logic lives in `lazycurl-core` so it can be tested without a terminal. The binary crate handles only rendering and input.

## Development

```bash
cargo build --workspace           # Build
cargo test --workspace            # Run tests (~149)
cargo fmt --all --check           # Check formatting
cargo clippy --workspace -- -D warnings  # Lint
```

## Contributing

Contributions are welcome. The project follows a test-first approach — new features and bug fixes should include tests in the relevant module's `#[cfg(test)]` block.

See [CLAUDE.md](CLAUDE.md) for architecture details, conventions, and gotchas.
