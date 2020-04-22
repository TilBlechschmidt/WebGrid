FROM scratch

ARG type
COPY services/.build/orchestrator_${type} /usr/local/bin/orchestrator-service

COPY services/orchestrators/${type}/configs /configs
ENV WEBGRID_CONFIG_DIR=/configs

EXPOSE 3034

CMD ["orchestrator-service"]
