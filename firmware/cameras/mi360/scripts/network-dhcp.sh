#!/bin/sh
# IPv4 lease adapter for the independent Mi360 recovery session.
set -eu
PATH=/bin:/sbin:/usr/bin:/usr/sbin
export PATH
[ "${interface:-}" = wlan0 ] || exit 1
case "${1:-}" in
    deconfig)
        ifconfig wlan0 0.0.0.0 up
        ;;
    bound|renew)
        set -- wlan0 "$ip"
        [ -z "${broadcast:-}" ] || set -- "$@" broadcast "$broadcast"
        [ -z "${subnet:-}" ] || set -- "$@" netmask "$subnet"
        ifconfig "$@" up
        while route del default gw 0.0.0.0 dev wlan0 2>/dev/null; do :; done
        for gateway in ${router:-}; do
            if route add default gw "$gateway" dev wlan0; then break; fi
        done
        # Use the actual lease's DNS servers, without stock external fallbacks.
        umask 022
        resolver=$(mktemp /tmp/mi360-resolv.XXXXXX)
        for server in ${dns:-}; do
            case "$server" in ''|*[!0-9.]*) continue ;; esac
            printf 'nameserver %s\n' "$server" >>"$resolver"
        done
        printf 'options timeout:2 attempts:2\n' >>"$resolver"
        cat "$resolver" >/etc/resolv.conf
        rm -f "$resolver"
        ;;
    leasefail|nak)
        # udhcpc retries. Never erase the saved configuration or enter AP mode.
        ;;
    *) exit 1 ;;
esac
