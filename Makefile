.PHONY: build-core build-api clean

build: build-api build-core
bundle: bundle-api bundle-core

build-api:
	cd api && yarn install && yarn build
	mkdir -p .artifacts/api-executable
	mv api/dist/index.js .artifacts/api-executable

build-core:
	cd core && ./build.sh

bundle-api: build-api
	docker build -f distribution/docker/images/api/Dockerfile -t webgrid-api .

bundle-core: build-core
	docker build -f distribution/docker/images/core/Dockerfile -t webgrid-core .

bundle-node: build-core
	docker build -f distribution/docker/images/node/Dockerfile -t webgrid-node-firefox --build-arg browser=firefox .
	docker build -f distribution/docker/images/node/Dockerfile -t webgrid-node-chrome --build-arg browser=chrome .

clean:
	rm -rf .artifacts
	rm -rf core/.cache core/target
	rm -rf api/dist api/node_modules api/src/generated.ts