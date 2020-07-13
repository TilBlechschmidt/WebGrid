FROM bitnami/minideb:buster AS base

ENV DBUS_SESSION_BUS_ADDRESS=/dev/null

RUN echo "#!/usr/bin/env bash\necho 'Loading environment ...'" > /env.sh && chmod +x /env.sh

WORKDIR /install
COPY images/node/base/install .
RUN ./packages.sh

WORKDIR /scripts
COPY images/node/base/scripts .

COPY services/.build/webgrid /usr/local/bin/webgrid

CMD /scripts/entrypoint.sh
EXPOSE 40003

# ----

FROM base AS driver

ARG browser
COPY images/node/driver /driver
RUN cd /driver && ./web-driver.sh $browser
