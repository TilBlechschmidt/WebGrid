#!/usr/bin/env bash
set -e
source /env.sh

export DISPLAY=:42
export WEBGRID_ON_SESSION_CREATE="xwit -display $DISPLAY -all -resize 1920 1080"
export WEBGRID_DRIVER=$DRIVER

./start-xvfb.sh
./recording.sh start

# Move the cursor out of the way
xwit -display $DISPLAY -root -warp 1920 1080

echo "Executing node service ..."
node-service

./recording.sh stop
