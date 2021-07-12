#!/bin/sh
echo "Starting virtual X-Server ..."
Xvfb $DISPLAY -ac -wr +render -noreset +extension GLX -screen 0 ${RESOLUTION}x24 &

timeout=${XVFB_TIMEOUT:-5}
loopCount=0
until xdpyinfo -display ${DISPLAY} > /dev/null 2>&1
do
        loopCount=$((loopCount+1))
        sleep 1
        if [ ${loopCount} -gt ${timeout} ]
        then
            echo "[ERROR] xvfb failed to start."
            exit 1
	fi
done
