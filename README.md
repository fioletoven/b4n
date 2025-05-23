# b4n

`b4n` is a terminal user interface for the Kubernetes API, created mainly for learning the Rust programming language. It is heavily based on the [`k9s` project](https://k9scli.io).

![b4n screenshot](assets/b4n.png?raw=true "b4n")

## Prerequisites

The [Cascadia Code font](https://github.com/microsoft/cascadia-code), or any other font with Nerd Font symbols, is required for proper display of the user interface in the terminal.

## Features

### Currently supported

As the project is in its early stages, for this moment the only supported functions are:

- viewing a list of kubernetes resources
- displaying YAML of the highlighted resource
- viewing logs for the highlighted container
- opening a shell for the highlighted container
- deleting selected resources

### Planned

General planned features:

- describing resources
- viewing combined logs for a pod
- editing resources (YAML)
- port forwarding

## Key Bindings

| Action                                  | Command         | Comments                                                     |
|:----------------------------------------|:----------------|:-------------------------------------------------------------|
| Attach to the container's shell         | `s`             | Works only in containers view                                |
| Copy YAML to the clipboard              | `c`             | Works only in YAML view                                      |
| Decode highlighted secret               | `x`             |                                                              |
| Delete selected resources               | `CTRL` + `d`    | Displays a confirmation dialog                               |
| Go back to namespaces; clear filter     | `ESC`           | Also clears input in the filter widget                       |
| Quit the application                    | `CTRL` + `c`    |                                                              |
| Reverse selection                       | `CTRL` + ` `    |  (`CTRL` + `SPACE`)                                          |
| Select resource                         | ` `             | (`SPACE`)                                                    |
| Show / hide log timestamps              | `t`             | Works only in logs view                                      |
| Show command palette                    | `:`             | For example, entering `:q`↲ quits the app                    |
| Show filter input                       | `/`             | Possible operators: and `&`, or `\|`, negation `!`, `(`, `)` |
| Show logs for the highlighted container | `l`             | Press `p` to display previous logs for the container         |
| Show namespaces selector                | `←`             | To select `all` quickly press `←` again                      |
| Show resources selector                 | `→`             | To select the first item quickly press `→` again             |
| Show YAML for the highlighted resource  | `y`             |                                                              |
| Sort column                             | `ALT` + `[0-9]` | Also works `ALT` + `[underlined letter]`                     |

## License

[MIT](./LICENSE)
