FROM scratch

COPY --from=webgrid-services /home/rust/src/target/x86_64-unknown-linux-musl/release/orchestrator_docker /usr/local/bin/orchestrator-service

EXPOSE 3034

CMD ["orchestrator-service"]
