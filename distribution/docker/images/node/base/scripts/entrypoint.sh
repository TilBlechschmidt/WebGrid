#!/usr/bin/env bash

onexit() {
    while caller $((n++)); do :; done;
    echo "EXIT trap called in ${FUNCNAME-main context}."
}

onerr() {
    while caller $((n++)); do :; done;
    echo "ERR trap called in ${FUNCNAME-main context}."
}

set -o errtrace
trap onexit EXIT
trap onerr ERR

source /env.sh

export DISPLAY=:42
# TODO Use the RESOLUTION env variable instead
export ON_SESSION_CREATE="xwit -display $DISPLAY -all -resize 1920 1080"

./start-xvfb.sh

# Move the cursor out of the way
xwit -display $DISPLAY -root -warp 1920 1080

echo "Executing node service ..."
webgrid node

echo "Node service exited."
