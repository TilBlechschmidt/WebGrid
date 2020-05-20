# Tooling
This list contains the set of tools used for development.

## IDE
Currently, VSCode is one of the best IDEs for developing Rust applications. This project contains a set of configuration files and a list of recommended extensions in the `.vscode` directory  — you may however use whichever editor you fancy. 😉

Additionally [clippy](https://github.com/rust-lang/rust-clippy) is used as a linter and [rustfmt](https://github.com/rust-lang/rustfmt) for standardized code formatting. Installation instructions can be found on the linked pages.

## Build tools
This project is divided into multiple build phases which are controlled by either the `Makefile` or the CI configuration file.

### Phase 1 — Services
In this phase, the contents of the `services` directory will be built. Currently this executes the `services/build.sh` script which puts the resulting binaries into the `services/.build` directory.

### Phase 2 — Images
After the services have been built, they are bundled in Docker images with additional dependencies (e.g. Browsers, X11, Recording Software). This is done by multiple calls to the `docker` command with the project root as the working directory. The instructions for this stage are located in the `images` directory.

### Phase 3 — Kubernetes
Finally, the Helm Chart is packaged for distribution and optionally signed. This stage may also include uploading the images to public registries. This stage is currently not implemented!

## Version control system
This project uses git and does its best to follow the [GitHub Flow](https://guides.github.com/introduction/flow/). Rebasing is preferred over merging where possible to retain a clean history. It is recommended to do an interactive rebase on a feature branch before creating a merge request to ensure atomic commits.

### Commit messages
Commit messages are expected to use [gitmoji](https://gitmoji.carloscuesta.me) and the content should follow the [seven great rules of Git commit messages](https://chris.beams.io/posts/git-commit/#separate)!

### Git Hooks
Through the help of the [rusty-hooks](https://github.com/swellaby/rusty-hook) a number of git-hooks have been enabled. These format the code using rustfmt and run the linter.

#### Note
A bug within the commit hook leaves the format changes made by rustfmt uncommitted. This entails a manual add and commit amend if format changes have been applied:

```bash
git add ...
git commit --amend
```