# b4n

<div align="center">
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MIT-008939?logo=mit&logoColor=fff&style=for-the-badge" alt="MIT license"></a>
  <a href="https://rust-lang.org"><img src="https://img.shields.io/badge/Rust-c02c30?logo=rust&logoColor=fff&style=for-the-badge" alt="Rust badge"></a>
  <a href="https://kube.rs"><img src="https://img.shields.io/badge/kube--rs-326ce5?logo=kubernetes&logoColor=fff&style=for-the-badge" alt="Built with kube-rs"></a>
  <a href="https://ratatui.rs"><img src="https://img.shields.io/badge/Ratatui-000?logo=ratatui&logoColor=fff&style=for-the-badge" alt="Built with Ratatui"></a>
  <a href="https://brainmade.org/">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://brainmade.org/white-logo.svg">
      <img alt="Brainmade mark" src="https://brainmade.org/black-logo.svg" height="28">
    </picture>
  </a>
</div>

`b4n` is a terminal user interface (TUI) for the Kubernetes API, created mainly for learning the Rust programming language. It is heavily based on the [`k9s` project](https://k9scli.io) and built using the [`kube-rs`](https://kube.rs) and [`ratatui`](https://ratatui.rs) crates.

![b4n demo](assets/b4n_048.gif?raw=true "b4n")

## Prerequisites

The [Cascadia Code font](https://github.com/microsoft/cascadia-code), or any other font with [Nerd Font](https://www.nerdfonts.com/font-downloads) symbols, is required for proper display of the user interface in the terminal.

## Building

To build `b4n` ensure you have a recent Rust toolchain installed (1.95+). Clone the repository and navigate into the project directory:

```bash
git clone https://github.com/fioletoven/b4n.git
cd b4n
```

Then compile the project in release mode for the best performance:

```bash
cargo build --release
```

The resulting binary will be available at `./target/release/b4n`.

## Features

### Currently Supported

The following features are currently supported:

- View and filter a list of Kubernetes resources.
- Create, read, update, and delete Kubernetes resources.
- View events for the highlighted resource.
- View logs for the highlighted pod or container.
- Open a shell session or attach to the highlighted container's main process.
- Enable port forwarding for the highlighted container.
- Simple plugin system to run external binaries.
- Mouse support in all views.

### Planned

The following features are planned for future development:

- File transfer from/to a pod.
- Ephemeral Containers.

## Default Key Bindings

| Action                                     | Command         | Comments                                                    |
|:-------------------------------------------|:----------------|:------------------------------------------------------------|
| Attach to the container's main process     | `a`             | Works only in containers view                               |
| Attach to the container's shell            | `s`             | Works only in containers view                               |
| Copy YAML / logs / resources to clipboard  | `c`             | Works only in YAML, logs and resources views                |
| Create new resource                        | `n`             |                                                             |
| Decode highlighted secret                  | `x`             |                                                             |
| Delete selected resources                  | `CTRL` + `d`    | Displays a confirmation dialog                              |
| Enable / disable mouse support             | `CTRL` + `n`    | Not available inside a shell session                        |
| Forward container's port                   | `f`             | Works only in containers view                               |
| Go back to namespaces; clear filter        | `ESC`           | Also clears input in the filter widget                      |
| Navigate to the involved object            | `i`             | Works only for `events` kind                                |
| Open / switch to edit mode                 | `i`             | Press `ESC` to exit, then `ESC` for save dialog             |
| Open right mouse button menu               | `m`             | Navigate using `↑` or `↓`                                   |
| Pin active filter across resources         | `CTRL` + `p`    | Also works in the filter dialog                             |
| Quit the application                       | `CTRL` + `c`    | No confirmation dialog                                      |
| Reverse selection                          | `CTRL` + ` `    | (`CTRL` + `SPACE`)                                          |
| Save YAML / logs to a file                 | `s`             |                                                             |
| Select resource                            | ` `             | (`SPACE`)                                                   |
| Show / hide log timestamps                 | `t`             | Works only in logs view                                     |
| Show / hide port forwards                  | `CTRL` + `f`    | Displays all active port forwarding rules                   |
| Show command palette                       | `:`, `>`        | For example, entering `:q`↲ quits the application           |
| Show describe for the highlighted resource | `d`             |                                                             |
| Show events for the highlighted resource   | `e`             |                                                             |
| Show filter / search input                 | `/`             | Filter operators: and `&`, or `\|`, negation `!`, `(`, `)`  |
| Show footer messages history pane          | `h`             | Also works with left mouse button click on the footer       |
| Show logs for the pod or container         | `l`             | Press `p` to display previous logs                          |
| Show namespaces selector                   | `←`             | To select `all` rapidly press `←` again                     |
| Show resources selector                    | `→`             | To select `pods` rapidly press `→` again                    |
| Show YAML for the highlighted resource     | `y`             |                                                             |
| Sort column                                | `ALT` + `[0-9]` | Also works with `ALT` + `[underlined letter]`               |

## Advanced Filtering

The resources and port forwards views support advanced filtering with prefixes:

- `ns:` - filter by namespace (e.g., `ns:kube-system`)
- `n:` - filter by resource name (e.g., `n:nginx`)
- `a:` - filter by annotations (e.g., `a:app.kubernetes.io/name=nginx`)
- `l:` - filter by labels (e.g., `l:app=frontend`)

Filters can be combined using logical operators: `&` (and), `|` (or), `!` (negation), and parentheses `()`.

Example: `ns:default & (l:app=web | l:app=api)`

> Note: `CTRL` + `p` keeps the active filter pinned when you switch resources.

## Logs View

When viewing logs for a single container, you can fetch earlier entries by pressing `↑` (up arrow). This works only when you are already scrolled to the top of the current log output and earlier entries are still available.

> Note: This functionality works only in single container logs view, not when viewing combined logs for all containers in a pod.

## Text Selection and Editing

When mouse support is enabled, you can:

- **Select text** by clicking and dragging in the YAML, logs, shell, and attach views
- **Select whole words** by double-clicking
- **Select whole lines** by triple-clicking
- **Copy selected text** to clipboard using standard key bindings

In edit mode, the following shortcuts are available:

- `CTRL` + `c` - copy selected text
- `CTRL` + `x` - cut selected text
- `CTRL` + `v` - paste text from clipboard
- `CTRL` + `a` - select all text
- `CTRL` + `d` - delete current line
- `CTRL` + `z` - undo
- `CTRL` + `y` - redo
- `ALT`  + `↑` - move current line up
- `ALT`  + `↓` - move current line down

> Note: These shortcuts currently cannot be changed in the `key_bindings` configuration section.

## Configuration Files

Configuration files are stored in the `$HOME/.b4n` directory. The layout looks like this:

```
.b4n/
├─ logs/
├─ plugins/
├─ themes/
│  └─ default.yaml
├─ config.yaml
└─ history.yaml
```

### logs/

This directory contains application logs, with one file created per day.

### plugins/

This folder contains custom command definitions that appear in the command palette in the resources view (the main `b4n` screen).
Store each command in a separate `.yaml` file.

```yaml
name: plugin-name
aliases: []          # additional aliases recognised by the command palette
description: "plugin description"
shortcut: Ctrl+Y
command: dive
args: []             # command arguments; see the available variables below
scopes:
  - pods             # scopes where the plugin will be visible; empty means all (format: 'plural[.group/version]')
excluded_scopes: []  # scopes where the plugin will be hidden; empty means none
confirm: false       # show run confirmation dialog
interactive: true    # run the command as an interactive terminal application; otherwise run it in the background
keep_output: false   # do not close terminal on command exit
keep_error: true     # do not close terminal if command exited with error (if keep_output: false)
pin_to_top: false    # stay at the beginning of the command output
highlighted: true    # allow running the plugin only when a resource in the list is highlighted
selected: false      # allow running the plugin only when at least one resource is selected
for_each: false      # run each selected resource separately (if interactive: false)
```

| Variable name       | Description                                                        |
|:--------------------|:-------------------------------------------------------------------|
| `$CONTEXT`          | currently selected kubeconfig context                              |
| `$PLURAL`           | plural name of the displayed resource kind                         |
| `$GROUP`            | displayed resource group                                           |
| `$VERSION`          | displayed resource version                                         |
| `$NAMESPACE`        | currently selected namespace                                       |
| `$RES[NAME]`        | name of the highlighted or selected resource                       |
| `$RES[NAMESPACE]`   | namespace of the highlighted or selected resource                  |
| `$RES[UID]`         | UID of the highlighted or selected resource                        |
| `$RES[CONTAINER]`   | container name of the highlighted or selected resource (pods only) |
| `$COL[COLUMN_NAME]` | any visible column value from the highlighted or selected resource |

Example plugins are available in the `plugins` folder.

### themes/

This folder stores all TUI themes.  
If `default.yaml` does not exist, the application will create it automatically.

You can add more theme files here by copying the ones from the repository `themes` folder or by creating your own.

### config.yaml

This file contains settings that control how `b4n` behaves.  
Example structure:

```yaml
logs:
  lines: 800
  timestamps: true
mouse: true
theme: light
contexts:
  test-cluster: '#43464f:#8aad81'
  production: '#d8d8d8:#e1140a'
aliases:
  daemonsets: ds,dms
  namespace: nn
  namespaces: ns,na,nam
  services: svc
key_bindings:
  action.name: list of key bindings for that action
  command-palette.open: :, >, Shift+:, Shift+>
  [...]
```

#### Configuration Options

- `logs.lines` - Number of log lines to retrieve from the Kubernetes API for the selected container.
- `logs.timestamps` - Whether timestamps are enabled by default for logs. You can still toggle this while viewing logs.
- `mouse` - Whether mouse support is enabled when the application starts. You can also toggle it while the app is running.
- `theme` - The name of the currently selected theme. This should match a file in the `themes` directory (without the `.yaml` extension).
- `contexts` - _(Optional)_ A map of context names to their corresponding colors. Useful for highlighting important Kubernetes clusters with distinct header colors.
- `aliases` - Command palette aliases.
- `key_bindings` - Defines custom key bindings for various application actions.  
  Example key bindings: `Ctrl+C`, `Ctrl+Alt+A`, `F7`, `Z`, `Left`, `Enter`.

> Note: If `config.yaml` does not exist, the application will create it automatically with default values.

### history.yaml

This file stores the history for filters, search patterns, and the last selected resource for each Kubernetes context.
To remove entries for a specific context, or to clear the file entirely, you can edit or delete it manually.  
You can also delete history entries from the UI by highlighting one and pressing `Ctrl+D`.

## Screenshots

![b4n pods](assets/screenshots/b4n_048-0.png?raw=true "b4n showing all pods")
![b4n pods light](assets/screenshots/b4n_048-1.png?raw=true "b4n showing all pods (light theme)")
![b4n describe](assets/screenshots/b4n_048-2.png?raw=true "describe resource")

## License

[MIT](./LICENSE)
