.PHONY: build-core build-api clean

bundle: bundle-api bundle-core bundle-node
build: build-api build-core

build-api:
	cd api && yarn install && yarn build
	mkdir -p .artifacts/api-executable
	mv api/dist/index.js .artifacts/api-executable

build-core:
	cd core && ./build.sh

bundle-api: build-api
	docker build -f distribution/docker/images/api/Dockerfile -t webgrid/api:latest .

bundle-core: build-core
	docker build -f distribution/docker/images/core/Dockerfile -t webgrid/core:latest .

bundle-node: build-core
	docker build -f distribution/docker/images/node/Dockerfile -t webgrid/node-firefox:latest --build-arg browser=firefox .
	docker build -f distribution/docker/images/node/Dockerfile -t webgrid/node-chrome:latest --build-arg browser=chrome .

clean:
	rm -rf .artifacts
	rm -rf core/.cache core/target
	rm -rf api/dist api/node_modules api/src/generated.ts

install: bundle
	-docker network create webgrid
	-docker volume create webgrid
	docker-compose -f distribution/docker/docker-compose.yml up -d

uninstall:
	docker-compose -f distribution/docker/docker-compose.yml down