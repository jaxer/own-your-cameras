#!/bin/sh
# One daemon launch per boot; bounded consecutive-boot recovery budget.
set -eu
PATH=/mnt/usr/bin:/bin:/sbin:/usr/bin:/usr/sbin
LD_LIBRARY_PATH=/lib:/mnt/usr/lib
export PATH LD_LIBRARY_PATH
umask 077
. /mnt/usr/etc/mi360/boot-policy.sh
base=/mnt/data/mi360
run=/tmp/mi360-launcher
cam=/mnt/usr/bin/cam
health=/tmp/mi360-runtime.health
[ ! -L "$run" ]
mkdir "$run" 2>/dev/null || exit 0
chmod 700 "$run"
log() { printf 'mi360 launcher: %s\n' "$*"; }
recover_only() { log "$*; recovery-only network/SSH, bootstrap watchdog retained"; exit 0; }
private_file() {
    [ -f "$1" ] && [ ! -L "$1" ] || return 1
    set -- $(ls -ldn "$1")
    [ "$1" = -rw------- ] && [ "$2" = 1 ] && [ "$3" = 0 ]
}
write_attempts() {
    # Callers use `||`: ash then suppresses errexit inside this function too.
    # Propagate every failure explicitly before watchdog ownership can change.
    [ ! -L "$base/attempts.new" ] || return 1
    { [ ! -e "$base/attempts.new" ] || [ -f "$base/attempts.new" ]; } || return 1
    rm -f "$base/attempts.new" || return 1
    printf '%s\n' "$1" >"$base/attempts.new" || return 1
    chmod 600 "$base/attempts.new" || return 1
    chown 0:0 "$base/attempts.new" || return 1
    sync || return 1
    mv -f "$base/attempts.new" "$base/attempts" || return 1
    sync
}
force_recovery_boot() {
    log "$*; forcing a clean boot, no daemon respawn"
    sync
    /sbin/reboot -f || :
    # If reboot returned, arm stock at3s and wait120s between pings. If native
    # still owns it, its own bounded feeding must expire; never kill/reopen it.
    /mnt/usr/bin/imi_watchdog 120 3 >"$run/emergency-watchdog.log" 2>&1 &
    while :; do sleep 60; done
}
uptime_seconds() { read -r up rest </proc/uptime; printf '%s\n' "${up%%.*}"; }
wait_child() {
    # Run bounded preflight while bootstrap watchdog continues to feed.
    "$@" >"$run/check-config.log" 2>&1 &
    check_pid=$!
    tries=0
    while kill -0 "$check_pid" 2>/dev/null; do
        tries=$((tries + 1))
        if [ "$tries" -gt 10 ]; then
            kill -KILL "$check_pid" 2>/dev/null || :
            wait "$check_pid" 2>/dev/null || :
            return 1
        fi
        sleep 1
    done
    wait "$check_pid"
}
[ -d "$base" ] && [ ! -L "$base" ] || recover_only 'no private config directory'
set -- $(ls -ldn "$base")
[ "$1" = drwx------ ] && [ "$3" = 0 ] || recover_only 'config directory needs root0700'
private_file "$base/enabled" || recover_only 'no private enable flag'
[ "$(wc -c <"$base/enabled")" -le 2 ] || recover_only 'invalid enable flag'
[ "$(cat "$base/enabled")" = 1 ] || recover_only 'daemon disabled'
private_file "$base/cam.toml" || recover_only 'config needs root0600, one link'
[ "$(wc -c <"$base/cam.toml")" -le 16384 ] || recover_only 'config exceeds16KiB'
attempts=0
if [ -e "$base/attempts" ] || [ -L "$base/attempts" ]; then
    private_file "$base/attempts" || recover_only 'invalid attempts file'
    [ "$(wc -c <"$base/attempts")" -le 2 ] || recover_only 'invalid attempts length'
    attempts=$(cat "$base/attempts")
fi
next=$(attempt_next "$attempts") || recover_only 'boot-attempt limit or corrupt counter'
[ -x "$cam" ] && [ ! -L "$cam" ] || recover_only 'missing packaged daemon'
sha256sum -c /mnt/usr/etc/mi360/cam.sha256 >"$run/cam-hash.log" 2>&1 || recover_only 'daemon hash mismatch'
# Build verifies ELF32/MIPS32r2/O32/interpreter. Runtime hashes identify that exact
# packaged ELF; check-config only parses config and hashes SDK files (no dlopen).
wait_child "$cam" check-config --config "$base/cam.toml" || recover_only 'config/SDK preflight failed'
audio=$(sed -n 's/^audio=\(true\|false\)$/\1/p' "$run/check-config.log")
case "$audio" in true|false) ;; *) recover_only 'preflight audio contract missing' ;; esac
for file in /dev/watchdog /dev/motor /dev/speaker /dev/tx-isp /proc/jz/reset/wdt_control \
            /mnt/usr/lib/libimp.so /mnt/usr/lib/libalog.so /lib/libm.so.0; do
    [ -e "$file" ] || recover_only "missing required path $file"
done
for pin in 8 9 49 51 54; do
    [ "$(cat /sys/class/gpio/gpio$pin/direction)" = out ] || recover_only "GPIO$pin is not output"
done
[ "$(cat /sys/class/gpio/gpio57/direction)" = in ] || recover_only 'reset GPIO57 is not input'
[ "$(cat /proc/jz/reset/wdt_control)" = '>off<on' ] || recover_only 'kernel watchdog feeder active'
boot=$(cat /proc/sys/kernel/random/boot_id)
expected_request=boot$(printf '%s' "$boot" | tr -d '-')
tries=0
while [ ! -f /tmp/mi360-network/last-success ] || \
      [ "$(cat /tmp/mi360-network/last-success)" != "$expected_request" ]; do
    tries=$((tries + 1))
    [ "$tries" -le 90 ] || recover_only 'independent LAN not ready'
    sleep 1
done
# Commit the attempt before stopping the currently working bootstrap owner.
write_attempts "$next" || recover_only 'cannot durably record attempt'
rm -f "$health"
bootstrap=$(/bin/busybox pidof imi_watchdog) || recover_only 'bootstrap watchdog absent'
case "$bootstrap" in ''|*[!0-9]*) recover_only 'ambiguous bootstrap watchdog PID' ;; esac
# Check identity again before signaling; never broad killall or SIGKILL watchdog.
command=$(tr '\000' ' ' <"/proc/$bootstrap/cmdline") || recover_only 'bootstrap disappeared'
case "$command" in 'imi_watchdog 10 40 '|'/mnt/usr/bin/imi_watchdog 10 40 ') ;; *) recover_only 'unexpected bootstrap command' ;; esac
kill -TERM "$bootstrap" || recover_only 'bootstrap TERM failed'
tries=0
while kill -0 "$bootstrap" 2>/dev/null; do
    tries=$((tries + 1))
    [ "$tries" -le 5 ] || force_recovery_boot 'bootstrap did not relinquish ownership'
    sleep 1
done
"$cam" serve --config "$base/cam.toml" >"$run/cam.log" 2>&1 &
pid=$!
printf '%s\n' "$pid" >"$run/cam.pid"
started=$(uptime_seconds)
last_progress=$started
seq=0 main=0 sub=0 pcm=0
qualified=0 have_sample=0
while :; do
    kill -0 "$pid" 2>/dev/null || force_recovery_boot 'daemon exited'
    now=$(uptime_seconds)
    # Read one immutable snapshot; runtime replaces the source atomically.
    if [ -f "$health" ] && [ ! -L "$health" ] && \
       [ "$(wc -c <"$health")" -le 4096 ]; then
        cp "$health" "$run/health.json"
        hp=$(jshon -e pid <"$run/health.json" 2>/dev/null) || hp=
        hb=$(jshon -e boot_id -u <"$run/health.json" 2>/dev/null) || hb=
        hh=$(jshon -e healthy <"$run/health.json" 2>/dev/null) || hh=
        hs=$(jshon -e sequence <"$run/health.json" 2>/dev/null) || hs=
        hm=$(jshon -e main_frames <"$run/health.json" 2>/dev/null) || hm=
        hv=$(jshon -e sub_frames <"$run/health.json" 2>/dev/null) || hv=
        ha=$(jshon -e audio_frames <"$run/health.json" 2>/dev/null) || ha=
        if [ "$have_sample" = 0 ] && [ "$hp" = "$pid" ] && [ "$hb" = "$boot" ] && \
           unsigned "$hs" && decimal_greater "$hm" 0 && decimal_greater "$hv" 0; then
            # Establish a real first snapshot; require a second advancing record
            # before resetting the durable boot budget.
            seq=$hs main=$hm sub=$hv pcm=$ha
            have_sample=1
        elif [ "$have_sample" = 1 ] && health_qualified "$hp" "$pid" "$hb" "$boot" "$hh" "$hs" "$seq" \
                            "$hm" "$main" "$hv" "$sub" "$ha" "$pcm" "$audio"; then
            last_progress=$now
            if [ "$qualified" = 0 ]; then
                # Once per boot, never on each health tick. Avoid a redundant write.
                [ "$next" = 1 ] || write_attempts 1 || force_recovery_boot 'healthy budget reset failed'
                qualified=1
                log 'actual advancing runtime qualified; failure budget reset'
            fi
            seq=$hs main=$hm sub=$hv pcm=$ha
        fi
    fi
    if reason=$(health_expired "$qualified" "$now" "$started" "$last_progress"); then
        force_recovery_boot "$reason"
    fi
    sleep 1
done
