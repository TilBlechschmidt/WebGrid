# Development
The repository has been split into multiple sub-folders. Namely `services` for the implementation of the service applications and `images` for the Docker images. The services have been implemented in Rust using the standard toolchain and development environment.

## Rust IDE
VSCode has been used with the following extensions:

- crates
- Rust (rls)

Additionally [clippy](https://github.com/rust-lang/rust-clippy) is used as a linter and [rustfmt](https://github.com/rust-lang/rustfmt) for standardised code formatting which can both be executed through the `services/validate.sh` script (which contains comments on how to install them).

## Commit messages
Commit messages are expected to use [gitmoji](https://gitmoji.carloscuesta.me) and the content should follow the [seven great rules of Git commit messages](https://chris.beams.io/posts/git-commit/#separate)!