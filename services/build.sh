#!/usr/bin/env bash
SOURCE_DIR=$(pwd)
BUILD_DIR=/tmp/webgrid-build-cache

echo "Building @ $BUILD_DIR"
mkdir -p $BUILD_DIR
mkdir -p $BUILD_DIR/project
mkdir -p $BUILD_DIR/cargo-git
mkdir -p $BUILD_DIR/cargo-registry
mkdir -p $BUILD_DIR/target

rsync -av --progress $SOURCE_DIR/* $BUILD_DIR/project --exclude target --exclude .build

docker run --rm -it \
	-v "$BUILD_DIR/project":/home/rust/src \
	-v "$BUILD_DIR/cargo-git":/home/rust/.cargo/git \
	-v "$BUILD_DIR/cargo-registry":/home/rust/.cargo/registry \
	-v "$BUILD_DIR/target":/home/rust/src/target \
	ekidd/rust-musl-builder \
	cargo build --release --workspace --locked

mkdir -p $SOURCE_DIR/.build
find $BUILD_DIR/target/x86_64-unknown-linux-musl/release -type f -perm 0755 -maxdepth 1 -exec cp {} $SOURCE_DIR/.build/ \;
