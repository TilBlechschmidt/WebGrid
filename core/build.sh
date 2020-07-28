#!/usr/bin/env bash
SOURCE_DIR=$(pwd)
BUILD_DIR=$(pwd)/.cache
TARGET_DIR=$(pwd)/../.artifacts

set -e

echo "Creating cache directories in $BUILD_DIR"
mkdir -p $BUILD_DIR
mkdir -p $BUILD_DIR/project
mkdir -p $BUILD_DIR/cargo-git
mkdir -p $BUILD_DIR/cargo-registry
mkdir -p $BUILD_DIR/target

echo "Copying project to build cache"
rsync -a --progress $SOURCE_DIR/* $BUILD_DIR/project --exclude target --exclude .build --exclude .cache

echo "Running build in webgrid/rust-musl-builder container"
docker run --rm \
	-v "$BUILD_DIR/project":/home/rust/src \
	-v "$BUILD_DIR/cargo-git":/home/rust/.cargo/git \
	-v "$BUILD_DIR/cargo-registry":/home/rust/.cargo/registry \
	-v "$BUILD_DIR/target":/home/rust/src/target \
	-e CARGO_TERM_COLOR=always \
	webgrid/rust-musl-builder \
	bash -c "cargo build --release --workspace --locked && cargo doc --release --workspace --locked --no-deps --document-private-items && rm /home/rust/src/target/x86_64-unknown-linux-musl/doc/.lock"

# TODO: Strip debug symbols from binary

echo "Creating output directories in $TARGET_DIR"
mkdir -p $TARGET_DIR/core-documentation
mkdir -p $TARGET_DIR/core-executable

echo "Copying documentation to output"
rsync -a --progress $BUILD_DIR/target/x86_64-unknown-linux-musl/doc $TARGET_DIR/core-documentation
echo "Copying executable to output"
rsync -av --progress $BUILD_DIR/target/x86_64-unknown-linux-musl/release/webgrid $TARGET_DIR/core-executable