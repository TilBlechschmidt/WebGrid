#!/bin/sh
STDIN_FILE=/var/lock/ffmpeg.stdin
PID_FILE=/var/lock/ffmpeg.pid
LOG_FILE=${FFMPEG_LOG:-/host/ffmpeg.log}
OUT_FILE=${FFMPEG_OUT:-/host/test.mp4}
touch $STDIN_FILE

export FFREPORT=file=$LOG_FILE:level=32

FF_CMDLINE="ffmpeg -y -rtbufsize 1500M -probesize 100M -framerate 15 -video_size 1920x1080 -f x11grab -i ${DISPLAY} -c:v libx264rgb -crf 0 -preset ultrafast -g 30 -f mp4 -movflags +frag_keyframe+empty_moov+default_base_moof $OUT_FILE"

case "$1" in
	start)
		echo "Starting recording ..."
		<$STDIN_FILE $FF_CMDLINE 2>>$LOG_FILE >>$LOG_FILE &
		echo $! > $PID_FILE
		;;
	stop)
		echo "Stopping recording ..."
		echo 'q' > $STDIN_FILE
		# TODO: This waiting logic sucks. Use lsof on the output file followed by a sync call instead.
		wait $(cat $PID_FILE)
		sleep 1
		;;
esac
