#!/bin/bash
# Drive the installed daemon over MQTT and grab frames through go2rtc (calibration and acceptance only).
#   cam.sh pub <id> <payload>          publish camera/$CAM/<id>/set
#   cam.sh state                       print pan/tilt/zoom states
#   cam.sh wait <pan> <tilt> [<zoom>]  wait until the states read these values (40 s)
#   cam.sh frame <out.jpg>             one main-stream frame
# Environment: MQTT_HOST, MQTT_USER, MQTT_PASSWORD (default: read from the camera's cam.toml over
# `ssh $SSH_HOST`), GO2RTC (base URL), STREAM (go2rtc stream), CAM (the camera id in its topics).
set -u
: "${CAM:?set CAM to the camera id}"
SSH_HOST=${SSH_HOST:-$CAM}
: "${MQTT_HOST:?set MQTT_HOST}" "${MQTT_USER:?set MQTT_USER}" "${GO2RTC:?set GO2RTC}"
STREAM=${STREAM:-${CAM}_high}
PW=${MQTT_PASSWORD:-$(ssh "$SSH_HOST" "sed -n 's/^password = \"\(.*\)\"/\1/p' /mnt/para/cam.toml")}
sub() { mosquitto_sub -h "$MQTT_HOST" -u "$MQTT_USER" -P "$PW" -t "camera/$CAM/+/state" -v -W 2 2>/dev/null |
  grep -E '/(pan|tilt|zoom)/state' | sort | tr '\n' ' '; }
case $1 in
pub) mosquitto_pub -h "$MQTT_HOST" -u "$MQTT_USER" -P "$PW" -t "camera/$CAM/$2/set" -m "$3" ;;
state) sub; echo ;;
wait)
  for _ in $(seq 40); do
    s=$(sub)
    [[ "$s" == *"pan/state $2 "* && "$s" == *"tilt/state $3 "* && ( -z "${4:-}" || "$s" == *"zoom/state $4 "* ) ]] &&
      { echo "ok $s"; exit 0; }
    sleep 1
  done
  echo "timeout $s"; exit 1 ;;
frame) curl -s -m 15 -o "$2" "$GO2RTC/api/frame.jpeg?src=$STREAM" ;;
*) sed -n '2,8p' "$0"; exit 2 ;;
esac
