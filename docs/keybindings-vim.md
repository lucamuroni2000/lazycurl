# LazyCurl Keybindings — Vim Preset

Enable by setting `"keymap_preset": "vim"` in your config file.

## Normal Mode

### Navigation

| Key | Action |
|-----|--------|
| `k` | Move up in current pane |
| `j` | Move down in current pane |
| `[` | Previous tab (Headers / Body / Auth / Params) |
| `]` | Next tab |
| `l` | Cycle pane forward (Collections > Request > Response) |
| `h` | Cycle pane backward |
| `{` | Previous project |
| `}` | Next project |
| `1` | Focus Collections pane |
| `2` | Focus Request pane |
| `3` | Focus Response pane |
| `Enter` | Select item / start editing |
| `q` | Quit |

### Commands

| Key | Action |
|-----|--------|
| `Ctrl+Enter` | Send request |
| `F5` | Send request (fallback) |
| `Ctrl+S` | Save request to collection |
| `n` | New request or collection (depends on the focused pane) |
| `Escape` | Cancel / close overlay / stop editing |
| `?` | Toggle help overlay |
| `/` | Search |

### Request Editing

| Key | Action |
|-----|--------|
| `u` | Focus URL bar |
| `M` | Open HTTP method picker |
| `t` | Open auth type picker |
| `a` | Add header / param / variable |
| `d` | Delete selected item |
| `r` | Rename selected item |
| `s` | Toggle item enabled / disabled |
| `c` | Duplicate item |
| `m` | Move request to another collection |

### Collections

| Key | Action |
|-----|--------|
| `Space` | Toggle collapse / expand collection |

### Overlays & Pickers

| Key | Action |
|-----|--------|
| `v` | Open variables editor |
| `e` | Cycle active environment |
| `E` | Open environment manager (available only from variables editor) |
| `x` | Open export picker |
| `L` | Open log viewer |
| `p` | Open project picker |
| `F8` | Reveal / hide secret values |

### Clipboard

| Key | Action |
|-----|--------|
| `y` | Copy response body to clipboard |

### Projects

| Key | Action |
|-----|--------|
| `Enter` | Open selected project |
| `n` | New project |
| `d` | Delete project |
| `r` | Rename project |
| `Space` | Close project |
| `Escape` | Cancel / close overlay |

---

## Log Viewer

These keys are active when the log viewer overlay is open.

| Key | Action |
|-----|--------|
| `f` | Filter by method, status, or URL |
| `c` | Clear current filter |
| `/` | Search log entries |
| `C` | Clear current search |
| `n` | Jump to next search match |
| `N` | Jump to previous search match |
| `e` | Export filtered view to JSONL |
| `y` | Copy response body to clipboard |
| `Y` | Copy log file path to clipboard |
| `r` | Re-send selected log entry |

---

## Variables Editor

These keys are active when the variables overlay is open.

| Key | Action |
|-----|--------|
| `}` | Cycle container forward (Global > Environment > Collection) |
| `{` | Cycle container backward |
| `a` | Add variable |
| `d` | Delete variable |
| `s` | Toggle variable secret / plain |
| `Enter` | Edit variable |

> **Note:** `[` and `]` are used for tab switching in the vim preset, so the variables container cycling uses `{` and `}` instead.

---

## Text Editing Mode

When a text field is focused, these hardcoded keys apply (not configurable).

| Key | Action |
|-----|--------|
| Any printable character | Insert at cursor |
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character after cursor |
| `Home` | Jump to start of field |
| `End` | Jump to end of field |
| `Left` | Move cursor left |
| `Right` | Move cursor right |
| `Escape` | Exit editing mode |
| `Enter` | Confirm / move to next field |
| `Tab` | Cycle to next pane |

---

## Key Differences from Default

The vim preset replaces modifier-heavy shortcuts with single-key equivalents:

| Action | Default | Vim |
|--------|---------|-----|
| Move up / down | `Up` / `Down` | `k` / `j` |
| Cycle panes | `Tab` / `Shift+Tab` | `l` / `h` |
| Switch tabs | `Left` / `Right` | `[` / `]` |
| Switch projects | `Ctrl+Left` / `Ctrl+Right` | `{` / `}` |
| Help | `F1` | `?` |
| Focus URL | `Ctrl+U` | `u` |
| Method picker | `Ctrl+M` | `M` |
| Auth type | `Ctrl+A` | `t` |
| Variables | `V` | `v` |
| Environment | `Ctrl+E` | `e` |
| Manage envs | `Ctrl+Shift+E` | `E` |
| Export | `Ctrl+X` | `x` |
| Log viewer | `Ctrl+L` | `L` |
| Project picker | `Ctrl+O` | `p` |
