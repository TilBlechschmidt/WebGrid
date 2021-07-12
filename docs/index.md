<h1 align="center">WebGrid</h1>

<!-- Logo & Summary -->
<p align="center">
  <img width="75%" src="https://placekitten.com/882/250" alt="Banner">
</p>

<!-- Navigation -->
<p align="center">
  <b>
  <a href="#install">
    Install
  </a>
  <span> | </span>
  <a href="#usage">
    Usage
  </a>
  <span> | </span>
  <a href="https://webgrid.dev">
    Docs
  </a>
  </b>
</p>

<!-- Badges -->
<p align="center">
  <a href="https://github.com/TilBlechschmidt/WebGrid/blob/main/CODE_OF_CONDUCT.md">
    <img src="https://img.shields.io/badge/Contributor%20Covenant-v2.0%20adopted-ff69b4.svg" alt="Contributor Covenant">
  </a>
  <a href="https://github.com/TilBlechschmidt/WebGrid/blob/main/LICENSE.md">
    <img alt="GitHub" src="https://img.shields.io/github/license/TilBlechschmidt/WebGrid">
  </a>

  <br>

  <a href="">
    <img alt="Maintenance" src="https://img.shields.io/maintenance/yes/2021">
  </a>
  <a href="https://github.com/TilBlechschmidt/WebGrid/commits/main">
    <img alt="GitHub last commit" src="https://img.shields.io/github/last-commit/TilBlechschmidt/WebGrid">
  </a>

  <br/>
  <sub>You have an idea for a logo? <a href="https://github.com/TilBlechschmidt/WebGrid/issues/1">Submit it here!</a></sub>
</p>

---

<!-- Bullet points -->
* **Cluster ready.** Designed with concurrency and on-demand scalability<sup>1</sup> in mind
* **Debuggable.** Provides browser screen recordings, extensive logs, and tracing
* **Fast.** Built for speed and performance on a single grid instance
* **[W3C Specification](https://www.w3.org/TR/webdriver1/) compilant.** Fully compatible with existing Selenium 4 clients

<p align="center">
  <sub><sup>1</sup>All the way down to zero, obviously</sub>
</p>

---

## Install

Below are quick-start tutorials to get you started. For a more detailed introduction visit the dedicated [Getting Started guide](https://webgrid.dev/getting-started/)!

### 🐳 Docker

To run a basic grid in Docker you can use Docker Compose. Below is a bare-bones example of getting all required components up and running!

```bash
# Create prerequisites
docker volume create webgrid
docker network create webgrid

# Download compose file
curl -fsSLO webgrid.dev/docker-compose.yml

# Launch the grid
docker-compose up
```

You can now point your Selenium client to [`localhost:8080`](http://localhost/) and browse the API at [`/api`](http://localhost/api).

### ☸️ Kube

For deployment to Kubernetes a Helm repository is available. The default values provide a good starting point for basic cluster setups like [K3s](https://k3s.io) or [microk8s](https://microk8s.io).

```bash
# Add the repository
helm repo add webgrid https://webgrid.dev/

# Install the chart
helm install example webgrid/webgrid

# Make it accessible locally for evaluation
kubectl port-forward service/example-webgrid 8080:80
```

Your grid is now available at [`localhost:8080`](http://localhost:8080/).

If you are deploying to a RBAC enabled cluster you might have to tweak some settings. Take a look at the [documentation](https://webgrid.dev/kubernetes/configuration/) on how to use your own ServiceAccount and PersistentVolumeClaims.

## Usage

Once you have your grid up and running there is a couple of things you can do!

### 🚀 Launch browser instances

Point your selenium client to [`http://localhost:8080`](http://localhost:8080) to create a new browser container/pod and interact with it! You can use all features supported by Selenium.

### 🔍 Browse the API

The grid provides a GraphQL API at [`/api`](http://localhost:8080/api) with a Playground for you to explore. It exposes all available metadata about sessions, grid health and advanced features like video recordings.

### 📺 Watch your browsers

You can take a **live** look at what your browsers are doing by taking the [Session ID](https://webgrid.dev/features/screen-recording/#session-id) of a instance and visiting [`localhost:8080`](http://localhost:8080). You can also embed the videos in your existing tools! Head over to the <a href="https://webgrid.dev/features/screen-recording/#embedding">embedding documentation</a> to learn how.

!!! warning "Screen recordings in clusters"
    Video recordings are disabled by default in K8s as every cluster has specific requirements for file storage. The <a href="https://webgrid.dev/kubernetes/storage/">storage documentation</a> explains how to enable it.

## Developing

If you want to build the project locally you can use the [Makefile](https://github.com/TilBlechschmidt/WebGrid/blob/main/Makefile). To create Docker images for every component and run them locally run these commands:

```bash
# Build docker images
make

# Start components in docker
make install
```

To start individual components outside of Docker or setup the development environment, see the [development environment documentation](https://webgrid.dev/contribute/dev-environment/).

## License

This project is licensed under the MIT License. While this does grant you a lot of freedom in how to use the software and keeps the legal headache to a minimum, it also no longer requires you to publish modifications made to the project. The original intention behind the AGPL license was to encourage contributions by users who added features for their own use.

Since this project is so small at this stage, it heavily relies on feedback and contributions from the community (thats you!). So *please* strongly consider contributing any changes you make for the benefit of all users 🙂