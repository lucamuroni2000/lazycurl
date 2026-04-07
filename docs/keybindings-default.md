# LazyCurl Keybindings — Default Preset

## Normal Mode

### Navigation

| Key | Action |
|-----|--------|
| `Up` | Move up in current pane |
| `Down` | Move down in current pane |
| `Left` | Previous tab (Headers / Body / Auth / Params) |
| `Right` | Next tab |
| `Tab` | Cycle pane forward (Collections > Request > Response) |
| `Shift+Tab` | Cycle pane backward |
| `Ctrl+Left` | Previous project |
| `Ctrl+Right` | Next project |
| `1` | Focus Collections pane |
| `2` | Focus Request pane |
| `3` | Focus Response pane |
| `Enter` | Select item / start editing |
| `q` | Quit |

### Request Commands

| Key | Action |
|-----|--------|
| `Ctrl+Enter` | Send request |
| `F5` | Send request (fallback) |
| `Ctrl+S` | Save request to collection |
| `n` | New request or collection (depends on the focused pane) |
| `Escape` | Cancel / close overlay / stop editing |
| `F1` | Toggle help overlay |
| `/` | Search |

### Request Editing

| Key | Action |
|-----|--------|
| `Ctrl+U` | Focus URL bar |
| `Ctrl+M` | Open HTTP method picker |
| `Ctrl+A` | Open auth type picker |
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
| `V` | Open variables editor |
| `Ctrl+E` | Cycle active environment |
| `Ctrl+Shift+E` | Open environment manager (available only from variables editor) |
| `Ctrl+X` | Open export picker |
| `Ctrl+L` | Open log viewer |
| `Ctrl+O` | Open project picker |
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
| `]` | Cycle container forward (Global > Environment > Collection) |
| `[` | Cycle container backward |
| `a` | Add variable |
| `d` | Delete variable |
| `s` | Toggle variable secret / plain |
| `Enter` | Edit variable |

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
| `Enter` | Confirm |
| `Tab` | Move to next part of the item (key -> value) |
