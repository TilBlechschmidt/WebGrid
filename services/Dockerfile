FROM ekidd/rust-musl-builder:1.42.0 AS builder

ADD --chown=rust:rust . ./
RUN cargo build --release --workspace --locked
