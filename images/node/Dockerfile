FROM bitnami/minideb:buster AS base

ENV DBUS_SESSION_BUS_ADDRESS=/dev/null

RUN echo "#!/usr/bin/env bash\necho 'Loading environment ...'" > /env.sh && chmod +x /env.sh

WORKDIR /install
COPY images/node/base/install .
RUN ./packages.sh

WORKDIR /scripts
COPY images/node/base/scripts .

COPY --from=webgrid-services /home/rust/src/target/x86_64-unknown-linux-musl/release/node /usr/local/bin/node-service

CMD /scripts/entrypoint.sh
EXPOSE 3030

# ----

FROM base AS driver

ARG browser
COPY images/node/driver /driver
RUN cd /driver && ./web-driver.sh $browser
