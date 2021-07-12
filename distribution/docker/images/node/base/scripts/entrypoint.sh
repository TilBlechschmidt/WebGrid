#!/usr/bin/env bash
set -e
source /env.sh

export DISPLAY=:42
# TODO Use the RESOLUTION env variable instead
export ON_SESSION_CREATE="xwit -display $DISPLAY -all -resize 1920 1080"

./start-xvfb.sh

# Move the cursor out of the way
xwit -display $DISPLAY -root -warp 1920 1080

echo "Executing node service ..."
webgrid node