# Development environment

This guide describes how to setup a local development environment. It covers the necessary build-tools and IDE setup.

## Prerequisites

### Docker & Kubernetes

All components are distributed as Docker images and Helm charts. For this to work you have to install [Docker](https://www.docker.com/get-started), [kubectl](https://kubernetes.io/docs/tasks/tools/install-kubectl/) and [Helm](https://helm.sh/docs/intro/install/) from their respective websites.

### Rust

The core component uses Rust and some additional tools like linters and formatters.

1. Visit [rustup.rs](https://rustup.rs/#) and follow their install instructions
2. Open a terminal in the `core/` directory
3. Run `cargo check` and `cargo clippy` to see if it works

### NodeJS

The API component is written in TypeScript. To develop it you have to install NodeJS and yarn.

1. Follow the install instructions on [nodejs.org](https://nodejs.org/en/)
2. Open a terminal in the `api/` directory
3. Run `yarn install` and `yarn build` to see if it works

### Visual Studio Code

While you can use any editor of your choice the recommended one is VSCode. This repository contains a workspace file with recommended extensions and settings.

1. Download VSCode from [their website](https://code.visualstudio.com/Download)
2. Open the file `webgrid.code-workspace`
3. Install all recommended installations (popup in the bottom right)
4. Install `rust-analyzer` when prompted to do so

### Documentation

If you want to build the documentation locally you have to install some tools. Make sure you have a recent version of [Python 3](https://www.python.org/downloads/) installed before running the commands below.

```bash
pip3 install 'mkdocs-git-revision-date-localized-plugin>=0.4' \
             'mkdocs-material' \
             'mkdocs-mermaid2-plugin' \
             'mkdocs-codeinclude-plugin' \
             'mkdocs-material-extensions' \
             'mkdocs-simple-hooks' \
             'git+http://github.com/TilBlechschmidt/mkdocs-helm'
```

!!! bug
    The above command might not work on Windows. Write everything in one line and remove the `\` instead.

## Running locally

Below are explanations on how to execute each component locally as well as some tips on how to reduce compile times during development.

### API

To run the API, open a terminal in the `api/` directory and execute the following commands:

```bash
yarn codegen
yarn start
```

!!! note "Code hot-reload"
    It currently does not auto-reload if you change the sources. If you have some time to spare, why not contribute this feature? 🙂

### Core

The core project contains a number of services. Each service can be started locally. For this to work you have to start a Redis database locally. To keep things simple we will just use Docker for this.

```bash
docker run -it --rm --name webgrid-local-redis -p 6379:6379 redis:alpine
```

Once you have the database up and running you can start a specific component by opening a terminal in the `core/` directory. Here is an example that shows all available components:

```bash
cargo run -- -r redis://localhost/ --help
```

To run a specific one, replace the `--help` flag with the name of a component listed in the help message. Note that every component has different requirements in terms of arguments, use the help flag on subcommands to find out more!

## Running in docker

If you want to test the whole grid in Docker you can use docker-compose together with the Makefile.

### Building the images locally

To build the images using the local code run use one of the following commands:

```bash
# Build all images
make

# Build the core image (excluding node images)
make bundle-core

# Build the api image
make bundle-api

# Build the node images
make bundle-node

# Clear all build caches
make clean
```

### Running locally built images

When not deployed by GitHub Actions the docker-compose file uses images tagged with `webgrid/{node|core|api}:latest`. These get created by the Makefile explained above. You can now start and stop the grid with the following commands:

```bash
# Start grid
make install

# Stop grid (keeps the stored videos)
make uninstall

# Purge the associated volume and network
docker network rm webgrid
docker volume rm webgrid 
```