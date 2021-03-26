.PHONY: core api clean

bundle: bundle-api bundle-core bundle-node
build: api core
build-debug: api core-debug
all: build bundle

builder:
	# The source image is manually built from the TilBlechschmidt/rust-musl-builder repository
	docker build --platform linux/arm64 --build-arg TAG=arm64 -f distribution/docker/images/builder/Dockerfile -t webgrid/rust-musl-builder:arm64-root .
	docker build --platform linux/amd64 --build-arg TAG=amd64 -f distribution/docker/images/builder/Dockerfile -t webgrid/rust-musl-builder:amd64-root .

core:
	cd core && ./build.sh --release

core-debug:
	cd core && ./build.sh

bundle-core: core
	docker build --platform linux/amd64 -f distribution/docker/images/core/Dockerfile -t webgrid/core:latest .

bundle-node: core
	docker build --platform linux/amd64 -f distribution/docker/images/node/Dockerfile -t webgrid/node-firefox:latest --build-arg browser=firefox .
	docker build --platform linux/amd64 -f distribution/docker/images/node/Dockerfile -t webgrid/node-chrome:latest --build-arg browser=chrome .

clean:
	rm -rf .artifacts
	rm -rf core/.cache core/target
	rm -rf api/dist api/node_modules api/src/generated.ts

install:
	-docker network create webgrid
	-docker volume create webgrid
	docker-compose -f distribution/docker/docker-compose.yml up -d

uninstall:
	docker-compose -f distribution/docker/docker-compose.yml down