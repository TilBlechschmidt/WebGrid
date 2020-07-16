.PHONY: core api clean

core:
	cd core && ./build.sh

webgrid: core
	docker build -f distribution/docker/images/core/Dockerfile -t webgrid .

node: core
	docker build -f distribution/docker/images/node/Dockerfile -t webgrid-node-firefox --build-arg browser=firefox .
	docker build -f distribution/docker/images/node/Dockerfile -t webgrid-node-chrome --build-arg browser=chrome .

api:
	docker build -f distribution/docker/images/api/Dockerfile -t webgrid-api .

images: node webgrid api

clean:
	rm -rf core/.build core/.cache core/target
	rm -rf api/dist api/node_modules api/src/generated.ts