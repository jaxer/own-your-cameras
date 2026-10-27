#!/bin/sh
# AP-only fallback. The vendor SET_AP_MODE branch can fall through into station
# setup when bound; this entry never reads or changes Xiaomi binding/NVRAM.
set -eu
PATH=/mnt/usr/bin:/bin:/sbin:/usr/bin:/usr/sbin
LD_LIBRARY_PATH=/lib:/mnt/usr/lib
export PATH LD_LIBRARY_PATH
umask 077
run=/tmp/mi360-network
mac=$(cat /sys/class/net/wlan0/address)
printf '%s\n' "$mac" | grep -Eq '^[0-9a-fA-F]{2}(:[0-9a-fA-F]{2}){5}$'
suffix=$(printf '%s' "$mac" | tr -d ':' | cut -c9-12 | tr '[:lower:]' '[:upper:]')
case "$suffix" in ???? ) ;; *) exit 1 ;; esac
[ -d "$run" ] && [ ! -L "$run" ]
/bin/busybox killall -q -TERM wpa_cli udhcpc wpa_supplicant hostapd udhcpd || :
tries=0
while /bin/busybox pidof wpa_cli udhcpc wpa_supplicant hostapd udhcpd >/dev/null; do
    tries=$((tries + 1))
    [ "$tries" -lt 5 ] || exit 1
    sleep 1
done
ifconfig wlan0 down
ifconfig wlan0 up
sleep 1
ifconfig wlan0 192.168.14.1 netmask 255.255.255.0
mkdir -p /var/run/hostapd
{ cat /etc/hostapd.conf
  printf '\nssid=chuangmi-camera-046a01_miap%s\n' "$suffix"
} >"$run/hostapd.conf"
hostapd "$run/hostapd.conf" -B >"$run/hostapd.log" 2>&1
cat >"$run/ap-udhcpd.conf" <<'EOF'
start 192.168.14.10
end 192.168.14.20
interface wlan0
max_leases 10
lease_file /tmp/mi360-network/ap.leases
pidfile /tmp/mi360-network/ap-dhcp.pid
option subnet 255.255.255.0
opt router 192.168.14.1
option lease 600
EOF
udhcpd -S "$run/ap-udhcpd.conf" >"$run/ap-dhcp.log" 2>&1
echo 'Camera recovery hotspot started at 192.168.14.1'
