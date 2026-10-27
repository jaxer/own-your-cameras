#!/bin/bash
# shoot.sh <pan> <tilt> <zoom> <out.jpg>: absolute move over MQTT, wait for the confirmed state, let the
# stream catch up, grab one frame. Name frames p<pan>_t<tilt>_z<zoom>.jpg for fit_lens.py.
D=$(dirname "$0")
bash "$D/cam.sh" pub zoom "$3"
bash "$D/cam.sh" pub pan "$1"
bash "$D/cam.sh" pub tilt "$2"
bash "$D/cam.sh" wait "$1" "$2" "$3" >/dev/null || exit 1
sleep 4
bash "$D/cam.sh" frame "$4"
file "$4" | grep -q JPEG || { echo "no frame" >&2; exit 1; }
echo "shot $1 $2 $3 -> $4"
