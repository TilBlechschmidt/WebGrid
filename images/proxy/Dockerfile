FROM scratch

COPY --from=webgrid-services /home/rust/src/target/x86_64-unknown-linux-musl/release/proxy /usr/local/bin/proxy-service

EXPOSE 8080

CMD ["proxy-service"]
