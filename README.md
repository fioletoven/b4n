# b4n

`b4n` is a terminal user interface for the Kubernetes API, created mainly for learning the Rust programming language. It is heavily based on the [`k9s` project](https://k9scli.io).

![b4n screenshot](assets/b4n.png?raw=true "b4n")

## Prerequisites

The [Cascadia Code font](https://github.com/microsoft/cascadia-code), or any other font with Nerd Font symbols, is required for proper display of the user interface in the terminal.

## Features

### Currently supported

As the project is in its early stages, for this moment the only supported functions are:

- viewing a list of kubernetes resources
- displaying YAML of the selected resource
- deleting selected resources

### Planned

General planned features:

- describing resources
- showing pod containers and their logs
- editing resources (YAML)
- port forwarding
- shell

## License

[MIT](./LICENSE)
