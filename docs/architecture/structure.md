# Project structure

This page covers various topics that are required to understand the project structure both on a file-system as well as a conceptual level.

## Repository layout

The repository contains multiple subfolders:

* `/docs` This documentation
* `/core` Core component
* `/api`  API component
* `/distribution` Platform specific packaging scripts
    * `docker` Docker images and stuff
    * `kubernetes` Helm chart

## High level concepts

The project has been divided into multiple hierarchical levels of abstraction. They are outlined in top-to-bottom order below.

### Components

At the very top there are components which rest at the root directory of the project. These each provide a comprehensive feature-set of the grid and would be totally isolated in an ideal world. However, as some data sharing is required they access each other's data sources through clearly defined interfaces. Below are all components that are currently in existence and planned:

#### Core

The *Core* component houses all services that are critical to the operation of the grid and need to be redundant, resilient and highly performant. It is written in [Rust](https://www.rust-lang.org) to achieve these goals.

#### API

The *API* component provides an external interface to the internal metadata of the grid. As of now it is read-only and available through the *Core* components *Proxy* service. It is written in [TypeScript](https://www.typescriptlang.org) and uses GraphQL as the query language.

#### Dashboard

A future component that is on the roadmap will be the *Dashboard*. It is expected to provide a comprehensive and user-friendly overview of the grid's status. Current plans are for it to be written using [Svelte](https://svelte.dev).

### Services

Services are lower level puzzle pieces that each serve a distinct role and can be scaled individually. Currently, only the *Core* component makes use of this concept as every other component would only have one service. The *Core* services are [described seperately](./services.md).

### Jobs & Tasks

In order to improve error handling and recovery a concept of Jobs & Tasks has been introduced. Each service process internally launches a set of jobs which each serve a single purpose e.g. serving HTTP POST requests from selenium clients in case of the manager. Each Job may spawn tasks which are ephemeral units that run once to e.g. serve an incoming request. To read more about jobs and how they improve error processing, head to the [error handling page](./error-handling.md).