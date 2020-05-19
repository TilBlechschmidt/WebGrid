FROM scratch

ARG type
COPY services/.build/orchestrator_${type} /usr/local/bin/orchestrator-service

EXPOSE 40004

CMD ["orchestrator-service"]
