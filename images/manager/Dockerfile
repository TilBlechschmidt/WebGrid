FROM scratch

COPY --from=webgrid-services /home/rust/src/target/x86_64-unknown-linux-musl/release/manager /usr/local/bin/manager-service

EXPOSE 3033

CMD ["manager-service"]
