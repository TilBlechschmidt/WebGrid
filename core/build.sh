#!/usr/bin/env bash
SOURCE_DIR=$(pwd)
BUILD_DIR=$(pwd)/.cache
TARGET_DIR=$(pwd)/../.artifacts

set -e

if uname -a | grep -q "arm64"; then
	echo "Detected arm64 host, using ARM builder image!"
	BUILDER_TAG="arm64-root"
else
	BUILDER_TAG="amd64-root"
fi

if [[ $1 = "--release" ]];
then
	echo "Building in release configuration"
	RELEASE_FLAG="--release"
	BUILD_OUTPUT_DIR="release"
else
	echo "Building in debug configuration"
	BUILD_OUTPUT_DIR="debug"
fi

echo "Creating cache directories in $BUILD_DIR"
mkdir -p $BUILD_DIR
mkdir -p $BUILD_DIR/project
mkdir -p $BUILD_DIR/cargo-git
mkdir -p $BUILD_DIR/cargo-registry
mkdir -p $BUILD_DIR/target

echo "Copying project to build cache"
rsync -a $SOURCE_DIR/* $BUILD_DIR/project --exclude target --exclude .build --exclude .cache

echo "Running build in webgrid/rust-musl-builder:${BUILDER_TAG} container"
# We are mounting the repository into the container for the build-script to determine the version from git
docker run --rm --name core-build \
	-v "$BUILD_DIR/project":/home/rust/src \
	-v "$BUILD_DIR/cargo-git":/home/rust/.cargo/git \
	-v "$BUILD_DIR/cargo-registry":/home/rust/.cargo/registry \
	-v "$BUILD_DIR/target":/home/rust/src/target \
	-v "$(pwd)/../":/repository \
	-e WEBGRID_GIT_REPOSITORY=/repository \
	-e CARGO_TERM_COLOR=always \
	webgrid/rust-musl-builder:${BUILDER_TAG} \
	bash -c "cargo build ${RELEASE_FLAG} --bin webgrid --locked && cargo doc ${RELEASE_FLAG} --locked --no-deps && rm -f /home/rust/src/target/x86_64-unknown-linux-musl/doc/.lock"


echo "Creating output directories in $TARGET_DIR"
mkdir -p $TARGET_DIR/core-documentation
mkdir -p $TARGET_DIR/core-executable

echo "Copying documentation to output"
rsync -a $BUILD_DIR/target/x86_64-unknown-linux-musl/doc $TARGET_DIR/core-documentation
echo "Copying executable to output"
rsync -av $BUILD_DIR/target/x86_64-unknown-linux-musl/$BUILD_OUTPUT_DIR/webgrid $TARGET_DIR/core-executable

# Strip the binary
if [[ $1 = "--release" ]];
then
	if uname -a | grep -q "arm64"; then
		echo "Binary stripping not supported on arm64 architecture"
	else
		echo "Stripping binary"
		docker run --rm --name core-strip \
			-v $TARGET_DIR/core-executable:/output \
			alpine \
			sh -c "apk add --update binutils && ls /output && strip /output/webgrid"
	fi
fi
