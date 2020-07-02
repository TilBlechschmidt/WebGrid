service-compile:
	cd services && ./build.sh

proxy: service-compile
	docker build -f images/proxy/Dockerfile -t webgrid-proxy .

manager: service-compile
	docker build -f images/manager/Dockerfile -t webgrid-manager .

metrics: service-compile
	docker build -f images/metrics/Dockerfile -t webgrid-metrics .

orchestrator: service-compile
	docker build -f images/orchestrator/Dockerfile -t webgrid-orchestrator-docker --build-arg type=docker .
	docker build -f images/orchestrator/Dockerfile -t webgrid-orchestrator-k8s --build-arg type=k8s .

node: service-compile
	docker build -f images/node/Dockerfile -t webgrid-node-firefox --build-arg browser=firefox .
	docker build -f images/node/Dockerfile -t webgrid-node-chrome --build-arg browser=chrome .

images: node proxy manager orchestrator metrics

run:
	docker network create webgrid
	docker run -it -d --rm --network webgrid --name webgrid-redis -p 6379:6379 redis --notify-keyspace-events Kgx
	sleep 1
	docker run -it -d --rm --network webgrid --name webgrid-metrics -e RUST_LOG=debug,warp=warn -p 40002:40002 webgrid-metrics
	docker run -it -d --rm --network webgrid --name webgrid-proxy-1 -e RUST_LOG=debug,hyper=warn -p 80:40005 webgrid-proxy
	docker run -it -d --rm --network webgrid --name webgrid-manager-1 -e RUST_LOG=debug,hyper=warn -e WEBGRID_MANAGER_ID=manager-1 -e WEBGRID_MANAGER_HOST=webgrid-manager-1 webgrid-manager
	docker run -it -d --rm --network webgrid --name webgrid-orchestrator-1 -e RUST_LOG=debug,hyper=warn -v /var/run/docker.sock:/var/run/docker.sock -e WEBGRID_ORCHESTRATOR_ID=orchestrator-1 -e WEBGRID_SLOTS=5 -e WEBGRID_IMAGES="webgrid-node-firefox=firefox::68.7.0esr,webgrid-node-chrome=chrome::81.0.4044.122" webgrid-orchestrator-docker

clean:
	-docker rm --force webgrid-redis
	-docker rm --force webgrid-metrics
	-docker rm --force webgrid-proxy-1
	-docker rm --force webgrid-manager-1
	-docker rm --force webgrid-orchestrator-1
	-docker network remove webgrid
