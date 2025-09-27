# Change Log

## 0.2.3 - 2025-09-27

### Features

- add option to navigate to the involved object for the selected resource

### Bug fixes

- fix bug that allowed opening the resource selector when `b4n` was disconnected from the k8s cluster

## 0.2.2 - 2025-09-25

### Features

- view events for the highlighted resource

### Bug fixes

- fix namespace handling in port forwards view

## 0.2.1 - 2025-09-20

### Features

- support for mouse interaction in the terminal interface

### Bug fixes

- fix cpu and memory metrics not refreshing correctly in the active view

## 0.2.0 - 2025-09-08

### Features

- show cpu and memory metrics for pods and nodes
- allow insecure connections
- add columns for CustomResourceDefinition kind
- add columns for Node kind

### Bug fixes

- fix clipboard behaviour on Linux
- fix observer to support resources that are not watchable
- preserve namespace for clustered resources

## 0.1.9 - 2025-08-23

### Features

- customizable key bindings
- added Linux build to the release workflow

### Bug fixes

- fixed issue with navigating to the next highlighted search match
- port forward can now be correctly removed from the list

## 0.1.8 - 2025-08-02

### Features

- search within YAML configurations and logs

## 0.1.7 - 2025-07-17

### Features

- display columns defined in custom resources

## 0.1.6 - 2025-07-01

### Features

- enable port forwarding for the highlighted container

## 0.1.5 - 2025-05-15

### Features

- attach to the highlighted container's shell

## 0.1.4 - 2025-04-08

### Features

- toggle timestamps in logs by pressing `t`
- add views for the following resource types: DaemonSets, Deployments, Events, Jobs, ReplicaSets and StatefulSets

### Bug fixes

- ensure the resource group is respected in the YAML view and during resource deletion

## 0.1.3 - 2025-04-04

### Features

- display logs by pressing `ENTER`, `l` or `p` on the selected container
- add init containers to the containers view
- add a disconnection indicator to the YAML view

## 0.1.2 - 2025-03-24

### Features

- display containers of a pod by pressing `ENTER` on the selected pod

## 0.1.1 - 2025-03-20

### Features

- decode the selected resource by pressing `x` in the resources list

## 0.1.0 - 2025-03-18

This is the first release of `b4n` app.  
As the project is still in its early stages of development, the list of features is limited but sufficient for conveniently displaying Kubernetes resources in YAML format.

### Features

- view a list of Kubernetes resources
- display YAML of selected resources
- delete selected resources
