# Release workflow

The project generally follows the [SemVer 2.0](https://semver.org) convention. A new release usually follows these steps:

1. Contributors open PRs
2. Changes get merged into `main` branch
3. [GitHub Actions](https://github.com/TilBlechschmidt/WebGrid/actions?query=workflow%3A%22📝+Changelog+management%22) creates a draft release and updates it after every contribution
4. Once a large enough number of contributions have accumulated the draft is published
5. GitHub Actions release pipeline takes the following steps
    - Build all components
    - Create and push Docker images
    - Publish new documentation and Helm chart
    - Attach executables to GitHub Release
