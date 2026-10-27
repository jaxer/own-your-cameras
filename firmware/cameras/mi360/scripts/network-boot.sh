#!/bin/sh
# Enable/disable the DATA-only network startup override. Run explicitly over SSH.
# The vendor image, recovery slot, factory data and U-Boot environment stay intact.
set -eu
PATH=/bin:/sbin:/usr/bin:/usr/sbin
export PATH
umask 077
base=/mnt/data/mi360-network
case "${1:-status}" in
    status)
        readlink /mnt/data/bin
        ;;
    enable)
        [ "$(readlink /mnt/data/bin)" = /mnt/usr/bin ]
        [ -d "$base" ] && [ ! -L "$base" ]
        [ -s "$base/wpa.conf" ] && [ ! -L "$base/wpa.conf" ]
        [ -x "$base/recover.sh" ] && [ -x "$base/dhcp.sh" ] && [ -x "$base/wpa-event.sh" ] && [ -x "$base/ap.sh" ]
        [ ! -e "$base/bin" ] && [ ! -L "$base/bin" ]
        mkdir "$base/bin"
        # Preserve every original command through its original absolute path.
        for program in /mnt/usr/bin/*; do
            [ -f "$program" ] || [ -L "$program" ] || exit 1
            ln -s "$program" "$base/bin/${program##*/}"
        done
        for name in wifi_start.sh network_governor.sh miio_client miio_client_helper_nomqtt.sh; do
            [ -L "$base/bin/$name" ] || exit 1
            rm "$base/bin/$name"
        done
        cat >"$base/bin/wifi_start.sh" <<'EOF'
#!/bin/sh
# Independent network starts at S99; ignore vendor AP/credential commands.
exit 0
EOF
        cat >"$base/bin/network_governor.sh" <<'EOF'
#!/bin/sh
exec /bin/sh /mnt/data/mi360-network/recover.sh
EOF
        for name in miio_client miio_client_helper_nomqtt.sh; do
            cat >"$base/bin/$name" <<'EOF'
#!/bin/sh
# Hold until our S99 network owner stops this supervised pairing service.
exec /bin/busybox sleep 2147483647
EOF
        done
        chmod 700 "$base/bin"/wifi_start.sh "$base/bin"/network_governor.sh \
            "$base/bin"/miio_client "$base/bin"/miio_client_helper_nomqtt.sh
        sync
        # -n replaces the symlink itself. If power fails in this tiny unlink/link
        # interval, S00init recreates the original vendor link on the next boot.
        ln -snf "$base/bin" /mnt/data/bin
        sync
        [ "$(readlink /mnt/data/bin)" = "$base/bin" ]
        echo 'Independent network selected for subsequent boots; no reboot issued.'
        ;;
    disable)
        [ "$(readlink /mnt/data/bin)" = "$base/bin" ]
        ln -snf /mnt/usr/bin /mnt/data/bin
        sync
        [ "$(readlink /mnt/data/bin)" = /mnt/usr/bin ]
        echo 'Vendor network selected for subsequent boots; no reboot issued.'
        ;;
    *) echo 'usage: network-boot.sh status|enable|disable' >&2; exit 2 ;;
esac
