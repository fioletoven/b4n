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

Then compile the project in release mode for optimal performance:

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

> Note: `CTRL` + `p` will pin the active filter across resource changes

## Logs View

When viewing logs for a single container, you can fetch earlier log entries by pressing the `↑` (up arrow) key. This feature is only available when you are scrolled to the top of the currently displayed logs and there are additional log entries available before the first visible line.

> Note: This functionality works only in single container logs view, not when viewing combined logs for all containers in a pod.

## Text Selection and Editing

When mouse support is enabled, you can:

- **Select text** by clicking and dragging in YAML, logs, shell, and attach view
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

> Note: These shortcuts currently cannot be changed in the `key_bindings` configuration section

## Configuration Files

Configuration files are stored in the `$HOME/.b4n` directory. The directory structure is as follows:

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

This directory contains application logs, with one log file generated per day.

### plugins/

This folder contains custom command configurations that will be added to the command palette options in the resources view (main `b4n` view). Each command must be stored in a separate file with the `.yaml` extension.

```yaml
name: plugin-name
aliases: []          # additional name aliases that the command palette will recognise
description: "plugin description"
shortcut: Ctrl+Y
command: dive
args: []             # command arguments, see possible variables below
scopes:
  - pods             # scopes in which the plugin will be visible, empty - all (format: 'plural[.group/version]')
excluded_scopes: []  # scopes from which the plugin will be excluded, empty - none
confirm: false       # show run confirmation dialog
interactive: true    # run command in terminal as an interactive application, if false command will be run in the background
keep_output: false   # do not close terminal on command exit
keep_error: true     # do not close terminal if command exited with error (if keep_output: false)
pin_to_top: false    # stay at the beginning of the command output
highlighted: true    # allow running the plugin only if any resource on the list is highlighted
selected: false      # allow running the plugin only if any resource on the list is selected (if interactive: false)
for_each: false      # run each selected resource separately (if interactive: false)
```

| Variable name       | Description                                               |
|:--------------------|:----------------------------------------------------------|
| `$CONTEXT`          | currently selected kubeconfig context                     |
| `$PLURAL`           | displayed resource kind plural name                       |
| `$GROUP`            | displayed resource group                                  |
| `$VERSION`          | displayed resource version                                |
| `$NAMESPACE`        | currently selected namespace                              |
| `$RES[NAME]`        | highlighted / selected resource name                      |
| `$RES[NAMESPACE]`   | highlighted / selected resource namespace                 |
| `$RES[UID]`         | highlighted / selected resource uid                       |
| `$RES[CONTAINER]`   | highlighted / selected resource container (if pods)       |
| `$COL[COLUMN_NAME]` | highlighted / selected resource any visible column value  |

You can find example plugins in the `plugins` folder.

### themes/

This folder stores all TUI (Text User Interface) themes.  
The `default.yaml` theme will be automatically generated by the application if it doesn't already exist.

You can place additional theme files here by copying them from the `themes` folder or creating your own.

### config.yaml

This file contains configuration settings that control the behaviour of the `b4n` application.  
Here is an example structure:

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

- `logs.lines` - The number of log lines to retrieve from the Kubernetes API for the selected container.
- `logs.timestamps` - Indicates whether timestamps are enabled by default for logs; this setting can still be toggled while viewing the logs.
- `mouse` - Indicates if mouse support should be enabled when the application starts. Mouse support can also be toggled while the app is running.
- `theme` - The name of the currently selected theme. This should match a file in the `themes` directory (without the `.yaml` extension).
- `contexts` - _(Optional)_ A map of context names to their corresponding colors. Useful for highlighting important Kubernetes clusters with distinct header colors.
- `aliases` - Command palette aliases.
- `key_bindings` - Defines custom key bindings for various application actions.  
  Example key bindings: `Ctrl+C`, `Ctrl+Alt+A`, `F7`, `Z`, `Left`, `Enter`.

> Note: If `config.yaml` does not exist, the application will create it automatically with default values.

### history.yaml

This file stores history for filters, search patterns, and the last selected resource for each Kubernetes context.
To remove history entries (either for a specific context or entirely), you can manually edit this file or even delete it.  
History entries can also be deleted from the UI, just highlight one and press `Ctrl+D` to delete it.

## Screenshots

![b4n pods](assets/screenshots/b4n_048-0.png?raw=true "b4n showing all pods")
![b4n pods light](assets/screenshots/b4n_048-1.png?raw=true "b4n showing all pods (light theme)")
![b4n describe](assets/screenshots/b4n_048-2.png?raw=true "describe resource")

## License

[MIT](./LICENSE)
