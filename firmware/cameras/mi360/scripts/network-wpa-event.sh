#!/bin/sh
# Renew only our DHCP client when association returns.
set -eu
[ "${1:-}" = wlan0 ] && [ "${2:-}" = CONNECTED ] || exit 0
record=/tmp/mi360-network/udhcpc.pid
[ -f "$record" ] || exit 0
read -r pid <"$record"
case "$pid" in ''|*[!0-9]*) exit 1 ;; esac
command=$(tr '\000' ' ' <"/proc/$pid/cmdline" 2>/dev/null) || exit 0
case "$command" in
    'udhcpc '*'-p /tmp/mi360-network/udhcpc.pid '*|'/sbin/udhcpc '*'-p /tmp/mi360-network/udhcpc.pid '*)
        kill -USR1 "$pid"
        ;;
esac
