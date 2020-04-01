# TODO: Extract builder for one build process only

service-compile:
	cd services && docker build -f Dockerfile -t webgrid-services .

proxy: service-compile
	docker build -f images/proxy/Dockerfile -t webgrid-proxy .

manager: service-compile
	docker build -f images/manager/Dockerfile -t webgrid-manager .

orchestrator: service-compile
	docker build -f images/orchestrator/Dockerfile -t webgrid-orchestrator .

node: service-compile
	docker build -f images/node/Dockerfile -t webgrid-node:firefox --build-arg browser=firefox .
	docker build -f images/node/Dockerfile -t webgrid-node:chrome --build-arg browser=chrome .

images: proxy manager node orchestrator

run:
	docker network create webgrid
	docker run -it -d --rm --network webgrid --name webgrid-redis -p 6379:6379 redis --notify-keyspace-events Kgx
	sleep 1
	docker run -it -d --rm --network webgrid --name webgrid-proxy-1 -p 80:8080 webgrid-proxy
	docker run -it -d --rm --network webgrid --name webgrid-manager-1 -e WEBGRID_MANAGER_ID=manager-1 -e WEBGRID_MANAGER_HOST=webgrid-manager-1 webgrid-manager
	docker run -it -d --rm --network webgrid --name webgrid-orchestrator-1 -v /var/run/docker.sock:/var/run/docker.sock -e WEBGRID_ORCHESTRATOR_id=orchestrator-1 -e WEBGRID_SLOTS=5 webgrid-orchestrator

clean:
	-docker rm --force webgrid-redis
	-docker rm --force webgrid-proxy
	-docker rm --force webgrid-manager-1
	-docker rm --force webgrid-orchestrator-1
	-docker network remove webgrid
