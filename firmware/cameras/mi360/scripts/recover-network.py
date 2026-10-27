#!/usr/bin/env python3
"""Recover this camera through its existing pinned SSH bootstrap.

inspect never associates with Wi-Fi or changes the camera. LAN recovery starts
the prepared camera rescue script. Hotspot recovery joins the known camera AP
and reboots into its installed independent Wi-Fi startup. No credentials enter
this tool.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import ipaddress
import json
import re
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass
from pathlib import Path

AP_IP = "192.168.14.1"
AP_SUBNET = ipaddress.IPv4Network("192.168.14.0/24")
CAMERA_RECOVER = "/mnt/data/mi360-network/recover.sh"
CAMERA_LOG = "/tmp/mi360-network-recover.log"
WPA_CONTROL = "/tmp/mi360-network/ctrl"
COMPLETION_MARKER = "/tmp/mi360-network/last-success"
NETWORKSETUP = "/usr/sbin/networksetup"
ROUTE = "/sbin/route"
IPCONFIG = "/usr/sbin/ipconfig"
IFCONFIG = "/sbin/ifconfig"
PING = "/sbin/ping"
SSH = "/usr/bin/ssh"
SSH_KEYGEN = "/usr/bin/ssh-keygen"


class RecoveryError(Exception):
    """A safe user-facing error; subprocess output must not be embedded."""


def run(argv: list[str], timeout: float = 8) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(argv, stdin=subprocess.DEVNULL, capture_output=True,
                              text=True, timeout=timeout, check=False)
    except subprocess.TimeoutExpired:
        raise RecoveryError(f"{Path(argv[0]).name} exceeded its deadline") from None
    except OSError:
        raise RecoveryError(f"cannot run {Path(argv[0]).name}") from None


def checked(argv: list[str], operation: str, timeout: float = 8) -> str:
    result = run(argv, timeout)
    if result.returncode:
        raise RecoveryError(f"{operation} failed (exit {result.returncode})")
    return result.stdout


def fields(output: str) -> dict[str, str]:
    return {key.strip(): value.strip() for line in output.splitlines()
            if ":" in line for key, value in [line.split(":", 1)]}


def hardware_ports(output: str) -> dict[str, str]:
    ports: dict[str, str] = {}
    port = None
    for line in output.splitlines():
        if line.startswith("Hardware Port: "):
            port = line.removeprefix("Hardware Port: ")
        elif line.startswith("Device: ") and port:
            ports[line.removeprefix("Device: ")] = port
    return ports


def valid_ssid(ssid: str) -> bool:
    return (0 < len(ssid.encode("utf-8")) <= 32 and "<redacted>" not in ssid.lower()
            and not any(ord(c) < 32 or ord(c) == 127 for c in ssid))


def current_ssid(interface: str) -> str | None:
    result = run([NETWORKSETUP, "-getairportnetwork", interface])
    if result.returncode:
        return None
    for prefix in ("Current Wi-Fi Network: ", "Current AirPort Network: "):
        if result.stdout.startswith(prefix):
            value = result.stdout[len(prefix):].rstrip("\r\n")
            return value if valid_ssid(value) else None
    # macOS can say "not associated" while Wi-Fi has an active address. Never
    # use that message as permission to discard the previous association.
    return None


def saved_ssids(interface: str) -> set[str]:
    output = checked([NETWORKSETUP, "-listpreferredwirelessnetworks", interface],
                     "read saved Wi-Fi network names")
    return {line.removeprefix("\t") for line in output.splitlines()[1:]}


def normal_mac(value: str) -> str | None:
    pieces = value.lower().split(":")
    try:
        if len(pieces) != 6 or any(not 1 <= len(p) <= 2 for p in pieces):
            return None
        return ":".join(f"{int(p, 16):02x}" for p in pieces)
    except ValueError:
        return None


def fingerprint_ok(output: str, expected: str) -> bool:
    fingerprints = []
    for line in output.splitlines():
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if parts[0].startswith("@"):
            # Revoked/cert-authority records are not this pinned raw key.
            return False
        if len(parts) < 3 or parts[1] != "ssh-ed25519":
            continue
        try:
            raw = base64.b64decode(parts[2], validate=True)
        except ValueError:
            return False
        digest = base64.b64encode(hashlib.sha256(raw).digest()).decode().rstrip("=")
        fingerprints.append("SHA256:" + digest)
    return bool(fingerprints) and all(f == expected for f in fingerprints)


@dataclass(frozen=True)
class Identity:
    host: str
    key_alias: str
    fingerprint: str
    mac: str
    ap_ssid: str

    @classmethod
    def from_args(cls, args):
        host_pattern = r"[A-Za-z0-9][A-Za-z0-9_.-]{0,252}"
        key_alias = args.host_key_alias or args.host
        if not re.fullmatch(host_pattern, args.host) or not re.fullmatch(host_pattern, key_alias):
            raise RecoveryError("use a DNS name, IPv4 address or SSH host alias")
        if not re.fullmatch(r"SHA256:[A-Za-z0-9+/]{43}", args.host_key_sha256):
            raise RecoveryError("supply the previously recorded Ed25519 SHA256 fingerprint")
        mac = normal_mac(args.mac)
        if mac is None:
            raise RecoveryError("supply the camera radio MAC recorded over trusted SSH")
        ssid = args.ap_ssid or "chuangmi-camera-046a01_miap" + mac.replace(":", "")[-4:].upper()
        if not valid_ssid(ssid):
            raise RecoveryError("invalid recovery SSID")
        return cls(args.host, key_alias, args.host_key_sha256, mac, ssid)


@dataclass(frozen=True)
class Route:
    interface: str
    gateway: str


class MacNetwork:
    def __init__(self) -> None:
        self.ports = hardware_ports(checked([NETWORKSETUP, "-listallhardwareports"],
                                           "inspect network interfaces"))
        wifi = [dev for dev, port in self.ports.items() if port == "Wi-Fi"]
        if len(wifi) != 1:
            raise RecoveryError("cannot identify exactly one Mac Wi-Fi interface")
        self.wifi = wifi[0]

    def route(self, destination: str = "default") -> Route:
        info = fields(checked([ROUTE, "-n", "get", destination], "read route"))
        interface, gateway = info.get("interface"), info.get("gateway", "")
        if not interface:
            raise RecoveryError("route has no interface")
        return Route(interface, gateway)

    def address(self, interface: str) -> str | None:
        result = run([IPCONFIG, "getifaddr", interface])
        value = result.stdout.strip()
        try:
            return str(ipaddress.IPv4Address(value)) if result.returncode == 0 else None
        except ipaddress.AddressValueError:
            return None

    def ethernet(self, expected: Route | None = None, *, probe: bool = True) -> Route:
        route = self.route()
        port = self.ports.get(route.interface, "")
        if (route.interface == self.wifi or not port
                or not re.search(r"Ethernet|LAN", port, re.I)):
            raise RecoveryError("default route must use a recognized wired Ethernet port")
        if expected is not None and route != expected:
            raise RecoveryError("Ethernet default route changed during recovery")
        address = self.address(route.interface)
        if not address:
            raise RecoveryError("Ethernet has no IPv4 address")
        status = checked([IFCONFIG, route.interface], "inspect Ethernet link")
        if "status: active" not in status:
            raise RecoveryError("Ethernet link is not active")
        try:
            gateway = ipaddress.IPv4Address(route.gateway)
        except ipaddress.AddressValueError:
            raise RecoveryError("Ethernet default gateway is not IPv4") from None
        if probe:
            # Source-bound one-packet check of the existing gateway, not a WAN
            # or Frank probe. No route, DNS, service-order or power changes.
            checked([PING, "-n", "-c", "1", "-W", "1000", "-b", route.interface, "-S", address,
                     str(gateway)], "verify Ethernet gateway reachability", timeout=3)
        return route

    def on_hotspot(self) -> bool:
        address = self.address(self.wifi)
        if not address or ipaddress.IPv4Address(address) not in AP_SUBNET:
            return False
        router = run([IPCONFIG, "getoption", self.wifi, "router"])
        return (router.returncode == 0 and router.stdout.strip() == AP_IP
                and self.route(AP_IP).interface == self.wifi)

    def associate(self, ssid: str) -> None:
        checked([NETWORKSETUP, "-setairportnetwork", self.wifi, ssid],
                "associate Mac Wi-Fi", timeout=25)

    def restore(self, ssid: str, original_address: str | None) -> None:
        self.associate(ssid)  # macOS retrieves its existing saved credential.
        deadline = time.monotonic() + 25
        while time.monotonic() < deadline:
            visible = current_ssid(self.wifi)
            address = self.address(self.wifi)
            # If privacy still hides SSID, report limited verification; an IP
            # alone is not proof that the exact original SSID was restored.
            if visible == ssid and address:
                print("Mac Wi-Fi restored; SSID and IPv4 confirmed.")
                return
            if (address and visible is None
                    and (address == original_address or
                         (original_address is None and ipaddress.IPv4Address(address) not in AP_SUBNET))):
                print("Mac Wi-Fi restore requested; non-camera IPv4 returned. "
                      "macOS hides SSID, so exact association is unverified.")
                return
            time.sleep(1)
        raise RecoveryError("Mac Wi-Fi restoration could not be verified; check Wi-Fi manually")


class Camera:
    def __init__(self, interface: str, hotspot: bool, identity: Identity) -> None:
        self.identity = identity
        known_hosts = Path.home() / ".ssh/known_hosts"
        output = checked([SSH_KEYGEN, "-F", identity.key_alias, "-f", str(known_hosts)],
                         "find pinned camera host key")
        if not fingerprint_ok(output, identity.fingerprint):
            raise RecoveryError("camera known_hosts entry does not match the recorded Ed25519 pin")
        self.args = [SSH, "-4", "-T", "-B", interface,
                     "-oBatchMode=yes", "-oConnectTimeout=4", "-oConnectionAttempts=1",
                     "-oServerAliveInterval=2", "-oServerAliveCountMax=1",
                     "-oStrictHostKeyChecking=yes", "-oUpdateHostKeys=no",
                     "-oIdentitiesOnly=yes", "-oHostKeyAlgorithms=ssh-ed25519",
                     "-oControlMaster=no", "-oControlPath=none", "-oClearAllForwardings=yes",
                     "-oLogLevel=ERROR", "-oUser=root", "-oPort=22",
                     "-oHostKeyAlias=" + identity.key_alias,
                     "-oUserKnownHostsFile=" + str(known_hosts)]
        if hotspot:
            self.args.append("-oHostName=" + AP_IP)
        self.args.append(identity.host)

    def call(self, command: str, timeout: float = 7) -> subprocess.CompletedProcess[str]:
        return run(self.args + [command], timeout)

    def inspect(self) -> dict[str, str]:
        command = (f"if [ -r {CAMERA_RECOVER} ]; then echo rescue=ready; "
                   "else echo rescue=missing; fi; "
                   "printf 'boot_id='; cat /proc/sys/kernel/random/boot_id; "
                   "printf 'boot_mode='; readlink /mnt/data/bin; "
                   "if [ -x /mnt/usr/etc/init.d/S99mi360_network ] && "
                   "[ -r /mnt/usr/etc/mi360/network-start.sh ]; then echo native_network=ready; fi; "
                   f"/usr/bin/wpa_cli -p {WPA_CONTROL} -i wlan0 status 2>/dev/null | "
                   "sed -n 's/^wpa_state=/wpa_state=/p'; "
                   f"sed -n '1s/^/completed_id=/p' {COMPLETION_MARKER} 2>/dev/null; :")
        result = self.call(command)
        if result.returncode:
            raise RecoveryError("pinned camera SSH inspection failed")
        state = dict(line.split("=", 1) for line in result.stdout.splitlines()
                     if re.fullmatch(r"(?:rescue|wpa_state|native_network)=[A-Z_a-z]+|completed_id=[0-9a-f]{32}"
                                     r"|boot_id=[0-9a-f-]{36}|boot_mode=/[A-Za-z0-9_/-]+", line))
        return state

    def hotspot_identity(self) -> bool:
        # The authenticated radio address remains available even when macOS
        # does not expose the Wi-Fi neighbor in its ARP command's lookup.
        result = self.call("cat /sys/class/net/wlan0/address")
        return result.returncode == 0 and normal_mac(result.stdout.strip()) == self.identity.mac

    def start_recovery(self, request_id: str) -> bool:
        if not re.fullmatch(r"[0-9a-f]{32}", request_id):
            raise RecoveryError("invalid recovery correlation ID")
        command = (f"test -r {CAMERA_RECOVER} && test -x /usr/bin/nohup || exit 1; "
                   f"/usr/bin/nohup /bin/sh {CAMERA_RECOVER} {request_id} "
                   f"</dev/null >{CAMERA_LOG} 2>&1 & "
                   "echo rescue_started")
        result = self.call(command)
        # An immediate AP shutdown can lose the ACK. Never retry this mutation;
        # the caller observes LAN recovery before declaring its outcome.
        return result.returncode == 0 and result.stdout.strip() == "rescue_started"

    def reboot_into_network(self) -> bool:
        # A clean driver boot is qualified. In-place AP-to-station teardown has
        # left both interfaces unreachable, so hotspot recovery uses reboot.
        command = ("{ test \"$(readlink /mnt/data/bin)\" = /mnt/data/mi360-network/bin || "
                   "{ test -x /mnt/usr/etc/init.d/S99mi360_network && "
                   "test -r /mnt/usr/etc/mi360/network-start.sh; }; } "
                   "&& test -s /mnt/data/mi360-network/wpa.conf || exit 1; "
                   "/usr/bin/nohup /bin/sh -c 'sync; sleep 1; /sbin/reboot' "
                   f"</dev/null >{CAMERA_LOG} 2>&1 & echo reboot_started")
        result = self.call(command)
        return result.returncode == 0 and result.stdout.strip() == "reboot_started"


def prepared_boot(state: dict[str, str]) -> bool:
    return (state.get("boot_mode") == "/mnt/data/mi360-network/bin"
            or state.get("native_network") == "ready")


def recover(args: argparse.Namespace, network: MacNetwork) -> None:
    identity = Identity.from_args(args)
    ap_ssid = identity.ap_ssid
    wired = network.ethernet()
    camera = Camera(network.wifi if args.hotspot else wired.interface, args.hotspot, identity)
    restore_ssid = args.restore_ssid
    original_address = network.address(network.wifi)
    if original_address and ipaddress.IPv4Address(original_address) in AP_SUBNET:
        original_address = None
    association_attempted = False
    temporary_hotspot_preference = False
    primary_error: BaseException | None = None
    try:
        if args.hotspot:
            old_ssid = current_ssid(network.wifi)
            if old_ssid == ap_ssid:
                old_ssid = None
                original_address = None
            if old_ssid and restore_ssid and old_ssid != restore_ssid:
                raise RecoveryError("--restore-ssid differs from the readable current Wi-Fi SSID")
            restore_ssid = old_ssid or restore_ssid
            if not restore_ssid or not valid_ssid(restore_ssid) or restore_ssid == ap_ssid:
                raise RecoveryError("macOS hides the current SSID; provide --restore-ssid "
                                    "with the current saved non-camera network name")
            original_saved = saved_ssids(network.wifi)
            if restore_ssid not in original_saved:
                raise RecoveryError("return SSID is not a saved Mac network; automatic restore "
                                    "without reading a password is not assured")
            temporary_hotspot_preference = ap_ssid not in original_saved
            info = checked([NETWORKSETUP, "-getinfo", "Wi-Fi"], "inspect Wi-Fi IPv4 setup")
            if not info.startswith("DHCP Configuration"):
                raise RecoveryError("Wi-Fi must already use DHCP; static settings will not be changed")
            association_attempted = True
            network.associate(ap_ssid)
            deadline = time.monotonic() + 45
            retry_after = time.monotonic() + 8
            attempts = 1
            while not network.on_hotspot():
                if time.monotonic() >= deadline:
                    raise RecoveryError("camera hotspot DHCP/interface verification timed out")
                if attempts < 3 and time.monotonic() >= retry_after:
                    network.associate(ap_ssid)
                    attempts += 1
                    retry_after = time.monotonic() + 8
                time.sleep(1)
            network.ethernet(wired)
            # Verify the actual NIC over pinned, interface-bound SSH. A missing
            # macOS ARP lookup must not prevent an authenticated recovery.
            if not camera.hotspot_identity():
                raise RecoveryError("camera hotspot pinned SSH/radio identity is unavailable")
        state = camera.inspect()
        if state.get("rescue") != "ready":
            raise RecoveryError("prepared camera rescue script is missing")
        previous_boot = state.get("boot_id")
        if args.hotspot and (not prepared_boot(state) or not previous_boot):
            raise RecoveryError("hotspot recovery requires the installed independent boot path and boot ID")
        network.ethernet(wired)
        request_id = uuid.uuid4().hex
        try:
            acknowledged = (camera.reboot_into_network() if args.hotspot
                            else camera.start_recovery(request_id))
        except RecoveryError:
            acknowledged = False
        operation = "reboot" if args.hotspot else "rescue"
        print(f"Camera {operation} launch acknowledged." if acknowledged else
              f"Camera {operation} launch outcome uncertain; checking LAN without retrying launch.")
        lan = Camera(wired.interface, False, identity)
        deadline = time.monotonic() + args.timeout
        while time.monotonic() < deadline:
            network.ethernet(wired, probe=False)
            try:
                state = lan.inspect()
                fresh = (bool(state.get("boot_id")) and state.get("boot_id") != previous_boot
                         if args.hotspot else state.get("completed_id") == request_id)
                if state.get("wpa_state") == "COMPLETED" and fresh:
                    network.ethernet(wired)
                    print("Camera LAN recovery confirmed by pinned SSH and WPA COMPLETED.")
                    return
            except RecoveryError:
                pass
            time.sleep(2)
        raise RecoveryError("camera LAN recovery was not confirmed before the deadline; "
                            "the rescue script may still be running")
    except BaseException as error:
        primary_error = error
        raise
    finally:
        if association_attempted:
            try:
                assert restore_ssid is not None
                try:
                    network.restore(restore_ssid, original_address)
                finally:
                    if temporary_hotspot_preference:
                        checked([NETWORKSETUP, "-removepreferredwirelessnetwork", network.wifi,
                                 ap_ssid], "remove temporary hotspot preference")
                        if ap_ssid in saved_ssids(network.wifi):
                            raise RecoveryError("temporary camera hotspot preference remains saved")
                network.ethernet(wired, probe=False)
            except (RecoveryError, KeyboardInterrupt) as error:
                if primary_error is None:
                    raise
                print(f"Additional restoration failure: {error}", file=sys.stderr)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("inspect", "recover"))
    parser.add_argument("--host", required=True, help="existing root SSH alias or camera host")
    parser.add_argument("--host-key-alias", help="known_hosts name, defaults to --host")
    parser.add_argument("--host-key-sha256", required=True, help="previously recorded Ed25519 fingerprint")
    parser.add_argument("--mac", required=True, help="radio MAC previously read over trusted SSH")
    parser.add_argument("--ap-ssid", help="recovery SSID; defaults to the model's MAC-derived name")
    parser.add_argument("--hotspot", action="store_true", help="use the known open camera AP")
    parser.add_argument("--restore-ssid", help="current saved Mac SSID when macOS hides its name")
    parser.add_argument("--timeout", type=int, default=60, help="LAN recovery wait in seconds (10..180)")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("this host helper requires macOS")
    if not 10 <= args.timeout <= 180:
        parser.error("--timeout must be in 10..180")
    if args.restore_ssid and not valid_ssid(args.restore_ssid):
        parser.error("--restore-ssid must be a nonempty SSID of at most32 UTF-8 bytes")
    try:
        identity = Identity.from_args(args)
        network = MacNetwork()
        if args.action == "inspect":
            wired = network.ethernet(probe=False)
            if args.hotspot and not network.on_hotspot():
                raise RecoveryError("inspect does not join Wi-Fi; the Mac is not on the camera hotspot")
            camera = Camera(network.wifi if args.hotspot else wired.interface, args.hotspot, identity)
            if args.hotspot:
                if not camera.hotspot_identity():
                    raise RecoveryError("camera hotspot pinned SSH/radio identity is unavailable")
            print(json.dumps({"wifi_interface": network.wifi,
                              "default_interface": wired.interface,
                              "current_ssid_readable": current_ssid(network.wifi) is not None,
                              "camera": camera.inspect()}, sort_keys=True))
        else:
            recover(args, network)
        return 0
    except RecoveryError as error:
        print(f"Recovery stopped: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("Recovery interrupted; any started camera rescue may continue.", file=sys.stderr)
        return 130


if __name__ == "__main__":
    sys.exit(main())
