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

- View and filter a list of Kubernetes resources.
- Create, read, update, and delete Kubernetes resources.
- View events for the highlighted resource.
- View logs for the highlighted pod or container.
- Open a shell session or attach to the highlighted container's main process.
- Enable port forwarding for the highlighted container.
- Inject an ephemeral container into the highlighted pod.
- Transfer files to and from containers (requires `tar` executable on the container).
- Run external binaries configured in a simple plugin system.
- Support mouse interactions in all views.

## Default Key Bindings

| Action                                     | Command         | Comments                                                    |
|:-------------------------------------------|:----------------|:------------------------------------------------------------|
| Attach to the container's main process     | `a`             | Works only in containers and pods view                      |
| Attach to the container's shell            | `s`             | Works only in containers and pods view                      |
| Copy YAML / logs / resources to clipboard  | `c`             | Works only in YAML, logs and resources views                |
| Create new resource                        | `n`             |                                                             |
| Decode highlighted secret                  | `x`             |                                                             |
| Delete selected resources                  | `CTRL` + `d`    | Displays a confirmation dialog                              |
| Enable / disable mouse support             | `CTRL` + `n`    | Not available inside a shell session                        |
| Forward container's port                   | `f`             | Works only in containers and pods view                      |
| Go back to namespace view; clear filter    | `ESC`           | Also clears input in the filter widget                      |
| Inject ephemeral container                 | `CTRL` + `i`    | Works only in pods view, displays a confirmation dialog     |
| Navigate to the involved object            | `i`             | Works only for `events` kind                                |
| Open / enter edit mode                     | `i`             | Press `ESC` to exit, then `ESC` for save dialog             |
| Open right mouse button menu               | `m`             | Navigate using `↑` or `↓`                                   |
| Pin active filter across resources         | `CTRL` + `p`    | Also works in the filter dialog                             |
| Quit the application                       | `CTRL` + `c`    | No confirmation dialog                                      |
| Reverse selection                          | `CTRL` + ` `    | (`CTRL` + `SPACE`)                                          |
| Save YAML / logs to a file                 | `s`             |                                                             |
| Select all resources                       | `CTRL` + `a`    | Then press `CTRL` + ` ` to deselect all                     |
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
| Transfer file from the pod's container     | `CTRL` + `t`    | Allows downloading a single file or a directory             |
| Transfer file to the pod's container       | `t`             | Allows uploading only a single file                         |

## Advanced Filtering

The resources and port forwards views support advanced filtering with prefixes:

- `ns:` - filter by namespace (e.g., `ns:kube-system`)
- `n:`  - filter by resource name (e.g., `n:nginx`)
- `a:`  - filter by annotations (e.g., `a:app.kubernetes.io/name=nginx`)
- `l:`  - filter by labels (e.g., `l:app=frontend`)

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

## File Transfer

Feature requires a `tar` executable on the container where files are uploaded to or downloaded from.

If `Overwrite files` is unchecked before the transfer, a check will be executed on the remote container that requires the presence of `sh` and `test` commands (if the container does not have these commands, as a workaround the checkbox can be checked, but be aware that files may be overwritten).

If the destination path (`To (dir):` textbox) contains `~`, there will be an attempt to resolve it to the home directory (this requires `sh` and `echo` commands to be present on the container). To bypass this, simply provide the full path without `~`.

> Note: Currently, the upload feature supports only uploading a single file.

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
output_type: plain   # highlight output as `plain`, `yaml` or `describe` (if keep_output: true and interactive: false)
auto_mouse: false    # automatically enable mouse support if app asks for it
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
terminal:
  system_cursor: false
  scrollback_lines: 1000
theme: light
debug_images:
- busybox
- alpine
- nicolaka/netshoot
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
- `terminal.system_cursor` - If true all terminal views will stop drawing its own cursor and start using the system one.
- `terminal.scrollback_lines` - A configurable maximum size limit of the terminal scrollback buffer.
- `theme` - The name of the currently selected theme. This should match a file in the `themes` directory (without the `.yaml` extension).
- `debug_images` - List of container images that are displayed during ephemeral container injection.
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
