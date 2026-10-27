#!/bin/sh
# Root-only source copies; never rewrite the existing WPA credential file.
set -eu
PATH=/mnt/usr/bin:/bin:/sbin:/usr/bin:/usr/sbin
LD_LIBRARY_PATH=/lib:/mnt/usr/lib
export PATH LD_LIBRARY_PATH
umask 077
base=/mnt/data/mi360-network
run=/tmp/mi360-network
[ ! -L "$run" ]
mkdir -p "$run"
chmod 700 "$run"
# init may invoke an entry twice; no second driver/session transition this boot.
mkdir "$run/firmware-started" 2>/dev/null || exit 0
# DATA/config can be absent/full. A copy failure still gets an AP from immutable
# application files and RAM; root SSH startup must not depend on a successful copy.
trap 'status=$?; if [ "$status" != 0 ]; then
    (/bin/sh /mnt/usr/etc/mi360/network/ap.sh >"$run/bootstrap-ap.log" 2>&1) </dev/null &
fi' 0
[ ! -L "$base" ]
mkdir -p "$base"
chmod 700 "$base"
changed=0
for name in recover dhcp wpa-event ap; do
    source=/mnt/usr/etc/mi360/network/$name.sh
    target=$base/$name.sh
    [ -f "$source" ] && [ ! -L "$source" ]
    [ ! -L "$target" ] && { [ ! -e "$target" ] || [ -f "$target" ]; }
    expected=$(sha256sum "$source"); expected=${expected%% *}
    actual=
    if [ -f "$target" ]; then
        actual=$(sha256sum "$target"); actual=${actual%% *}
    fi
    if [ "$expected" != "$actual" ]; then
        [ ! -L "$target.new" ]
        rm -f "$target.new"
        cp "$source" "$target.new"
        chmod 700 "$target.new"
        chown 0:0 "$target.new"
        mv -f "$target.new" "$target"
        changed=1
    fi
done
[ "$changed" = 0 ] || sync
boot=$(tr -d '-' </proc/sys/kernel/random/boot_id)
case "$boot" in ''|*[!0-9a-f]*) exit 1 ;; esac
printf '%s\n' "boot$boot" >"$run/boot-request"
# Permanent closure/exec, not temporary fd redirection (old ash descriptor leak).
(
    if /bin/sh "$base/recover.sh" "boot$boot" >"$run/boot-recover.log" 2>&1; then
        exit 0
    fi
    # recover's trap may have already started AP. Avoid repeating AP->AP driver
    # transitions; cover failures before recover could install its fallback trap.
    if ! /bin/busybox pidof hostapd >/dev/null; then
        /bin/sh "$base/ap.sh" >"$run/boot-ap.log" 2>&1 || :
    fi
) </dev/null &
exit 0
