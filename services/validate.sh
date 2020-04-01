#!/usr/bin/env sh
# Executes the linter, code formatter and test suites
# Install linter and formatter like this:
# rustup component add clippy rustfmt
echo "Formatting ..."
cargo fmt --all
echo "Linting ..."
cargo clippy
echo "Testing ..."
cargo test
