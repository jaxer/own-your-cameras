#!/bin/sh
# Pure shell policy; sourced by launcher and isolated host tests. No side effects.
attempt_next() {
    case "$1" in 0|1|2) printf '%s\n' "$(( $1 + 1 ))" ;; *) return 1 ;; esac
}
unsigned() {
    case "$1" in ''|*[!0-9]*) return 1 ;; esac
    # Keep ash arithmetic below signed32; media counters compared by string only.
    [ "${#1}" -le 9 ]
}
decimal_greater() {
    case "$1:$2" in *[!0-9:]*) return 1 ;; esac
    [ -n "$1" ] && [ -n "$2" ] && [ "${#1}" -le 20 ] && [ "${#2}" -le 20 ] || return 1
    if [ "${#1}" -ne "${#2}" ]; then
        [ "${#1}" -gt "${#2}" ]
    else
        # Equal-width decimal lexical comparison avoids O32 arithmetic overflow.
        [ "$1" \> "$2" ]
    fi
}
health_qualified() {
    # pid, expectedpid, boot, expectedboot, healthy, seq, oldseq, main, oldmain,
    # sub, oldsub, audio, oldaudio, audioenabled. Values come from parsed JSON.
    [ "$1" = "$2" ] && [ "$3" = "$4" ] && [ "$5" = true ] || return 1
    shift 5
    unsigned "$1" && unsigned "$2" && [ "$1" -gt "$2" ] || return 1
    decimal_greater "$3" "$4" && decimal_greater "$5" "$6" || return 1
    [ "$9" = false ] || decimal_greater "$7" "$8"
}
health_expired() {
    # qualified, monotonic-now, launch-time, last-qualified-progress
    if [ "$1" = 0 ]; then
        [ "$(( $2 - $3 ))" -gt 90 ] || return 1
        printf '%s\n' 'startup health deadline exceeded'
    else
        [ "$(( $2 - $4 ))" -gt 15 ] || return 1
        printf '%s\n' 'qualified health stopped advancing'
    fi
}
