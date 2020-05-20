# Project structure
The repository has been split into multiple sub-folders. Below is a description of the items:

## `services`
The project has been divided into multiple service applications. Each sub-folder in this directory corresponds to a service application or closely related group of services (e.g. different orchestrator implementations and the shared core library).

One exception is the `services/shared` folder which contains a shared library that is used by (almost all) services. It houses functionalities that are required by two or more services, which includes logging, lifecycle management, capability matchers, metric collectors, and port assignments amongst other helper functions.

As of now all services are implemented in the [Rust language](http://rust-lang.org/). For this reason (and for dependency resolving) the directory is a [Cargo Workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) and can be opened with any IDE that supports the language. See [Tooling](./Tooling.md) for more details.

### Build
Due to the way the project is built, the service directory facilitates the necessary scripts to compile the release binary into a subfolder called `services/.build`. The build operation itself creates a temporary copy of the working directory in the `services/.cache` directory. This also houses build caches and cloned dependencies for the target platform (which might differ from the current host).

## `images`
Since the main distribution target is either Docker or Kubernetes, a set of container images are required to run the application. This directory contains the build instructions for each image split by services. The Dockerfiles are written relative to the project root. As with the `services` directory the different orchestrator types share one image folder, since the images are practically identical. Build arguments are then used to determine which type to bundle.

## `helm-chart`
In order to deploy the application to Kubernetes, a Helm Chart has been created which lives in this directory.

## `docs`
This folder houses the documentation for the project. It is split into multiple different domains, namely `architecture`, `development`, and `end-user`.