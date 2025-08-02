# b4n

`b4n` is a terminal user interface for the Kubernetes API, created mainly for learning the Rust programming language. It is heavily based on the [`k9s` project](https://k9scli.io).

![b4n screenshot](assets/b4n.png?raw=true "b4n")

## Prerequisites

The [Cascadia Code font](https://github.com/microsoft/cascadia-code), or any other font with [Nerd Font](https://www.nerdfonts.com/font-downloads) symbols, is required for proper display of the user interface in the terminal.

## Features

### Currently Supported

As the project is in its early stages, the following features are currently supported:

- View a list of Kubernetes resources.
- Delete selected resources.
- Display the YAML configuration of the highlighted resource.
- View logs for the highlighted container.
- Open a shell session in the highlighted container.
- Enable port forwarding for the highlighted container.

### Planned

The following features are planned for future development:

- Show CPU/memory metrics for pods and clusters.
- View combined logs for all containers in a pod.
- Edit Kubernetes resources (YAML).
- Describe Kubernetes resources.

## Key Bindings

| Action                                  | Command         | Comments                                                     |
|:----------------------------------------|:----------------|:-------------------------------------------------------------|
| Attach to the container's shell         | `s`             | Works only in containers view                                |
| Copy YAML to the clipboard              | `c`             | Works only in YAML view                                      |
| Decode highlighted secret               | `x`             |                                                              |
| Delete selected resources               | `CTRL` + `d`    | Displays a confirmation dialog                               |
| Forward container's port                | `f`             | Works only in containers view                                |
| Go back to namespaces; clear filter     | `ESC`           | Also clears input in the filter widget                       |
| Quit the application                    | `CTRL` + `c`    |                                                              |
| Reverse selection                       | `CTRL` + ` `    | (`CTRL` + `SPACE`)                                           |
| Select resource                         | ` `             | (`SPACE`)                                                    |
| Show / hide log timestamps              | `t`             | Works only in logs view                                      |
| Show command palette                    | `:`             | For example, entering `:q`↲ quits the app                    |
| Show filter / search input              | `/`             | Filter operators: and `&`, or `\|`, negation `!`, `(`, `)`   |
| Show logs for the highlighted container | `l`             | Press `p` to display previous logs for the container         |
| Show namespaces selector                | `←`             | To select `all` quickly press `←` again                      |
| Show / hide port forwards               | `CTRL` + `f`    | Displays all active port forwarding rules                    |
| Show resources selector                 | `→`             | To select the first item quickly press `→` again             |
| Show YAML for the highlighted resource  | `y`             |                                                              |
| Sort column                             | `ALT` + `[0-9]` | Also works `ALT` + `[underlined letter]`                     |

## License

[MIT](./LICENSE)
