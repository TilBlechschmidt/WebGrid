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

images: node webgrid

run:
	docker network create webgrid
	docker volume create webgrid
	docker run -it -d --rm --network webgrid --name webgrid-redis -p 6379:6379 redis --notify-keyspace-events Kgx
	sleep 1
	docker run -it -d --rm --network webgrid --name webgrid-proxy -p 80:40005 webgrid proxy --log debug,hyper=warn
	docker run -it -d --rm --network webgrid --name webgrid-manager webgrid manager example-manager webgrid-manager --log debug,hyper=warn
	docker run -it -d --rm --network webgrid --name webgrid-orchestrator -v /var/run/docker.sock:/var/run/docker.sock webgrid orchestrator --slot-count 5 example-orchestrator docker --images "webgrid-node-firefox=firefox::68.7.0esr,webgrid-node-chrome=chrome::81.0.4044.122" --log debug,hyper=warn
	docker run -it -d --rm --network webgrid --name webgrid-metrics -p 40002:40002 webgrid metrics --log debug,hyper=warn
	docker run -it -d --rm --network webgrid --name webgrid-storage --volume webgrid:/storage webgrid storage --log debug,hyper=warn --host webgrid-storage --storage-directory /storage

clean:
	-docker rm --force webgrid-redis
	-docker rm --force webgrid-metrics
	-docker rm --force webgrid-proxy
	-docker rm --force webgrid-manager
	-docker rm --force webgrid-orchestrator
	-docker rm --force webgrid-storage
	-docker network remove webgrid
	-docker volume rm webgrid
