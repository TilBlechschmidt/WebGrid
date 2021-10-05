FROM bitnami/minideb:bullseye AS base

ENV DBUS_SESSION_BUS_ADDRESS=/dev/null
ENV RESOLUTION=1920x1080

RUN echo "#!/usr/bin/env bash\necho 'Loading environment ...'" > /env.sh && chmod +x /env.sh

WORKDIR /install
COPY distribution/docker/images/node/base/install .
RUN ./packages.sh

WORKDIR /scripts
COPY distribution/docker/images/node/base/scripts .

COPY .artifacts/core-executable/webgrid /usr/local/bin/webgrid

CMD /scripts/entrypoint.sh
EXPOSE 40003

# ----

FROM base AS driver

ARG browser
COPY distribution/docker/images/node/driver /driver
RUN cd /driver && ./web-driver.sh $browser
