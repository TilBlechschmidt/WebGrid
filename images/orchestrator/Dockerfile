FROM scratch

ARG type
COPY services/.build/orchestrator_${type} /usr/local/bin/orchestrator-service

EXPOSE 3034

CMD ["orchestrator-service"]
