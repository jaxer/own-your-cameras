# How the door-station sidecar works

A sidecar is a small program running beside the vendor software. On DS-KD8003-IME1
V2.2.62, it reads the door and button hardware and sends state to MQTT. Opening
still goes through the vendor API, which owns the relock timer.

This page describes the implemented design. Source and an installer are not
included in this repository.

## Inputs and actions

| Job | Implementation |
|---|---|
| Read door position | `/dev/DDM/alarmV2`, ioctl `0x80044201`, bit 0; use a 64-byte buffer |
| Read electrical lock state | Read GPIO 39; high means unlocked |
| Observe call button | Read `/dev/event0`, EV_KEY code 5, value 1/0 for press/release |
| Request timed opening | Loopback ISAPI Digest-authenticated `PUT /ISAPI/AccessControl/RemoteControl/door/1`, body `<RemoteControl><cmd>open</cmd></RemoteControl>` |
| Home Assistant integration | Retained MQTT device discovery with a cover, open button, actuator sensor and doorbell event |

Keep door position and lock drive separate: an unlocked strike does not mean the
door is open, and an electrically locked strike does not prove the latch engaged.

The program uses one nonblocking loop with bounded buffers. It targets ARMv5TE
soft float and the device's uClibc. It leaves `hicore` running and observes the
button without grabbing it by default. See the [hardware map](../devices/ds-kd8003-ime1.md#hardware-map).

## MQTT contract

These are the implementation's fixed topics; multiple devices would need separate
namespaces and discovery IDs. Payloads are JSON; sample uptime values are illustrative. Commands and events
are not retained.

| Topic | Payload / meaning | Retained |
|---|---|---|
| `gate/sidecar/availability` | `"online"` / `"offline"` | Yes |
| `gate/sidecar/door/state` | `"open"` / `"closed"` | Yes |
| `gate/sidecar/lock/state` | `{"lock_actuator":"locked","lock_actuator_unlocked":false}`; unlocked/true or unknown/null | Yes |
| `gate/sidecar/call_button` | `{"event":"press","event_type":"press","uptime_ms":1234}`; release uses `release` in both fields | No |
| `gate/sidecar/door/command` | `{"command":"open"}` | No |
| `gate/sidecar/door/open_result` | `{"result":"ok","detail":"firmware timed open accepted","uptime_ms":1234}`; failures use `error` | No |
| `homeassistant/device/hikvision_gate_sidecar/config` | Device discovery document | Yes |

Discovery/state are republished after reconnect and Home Assistant birth.
The broker's Last Will marks the program unavailable. QoS 1 delivery can duplicate
messages, so consumers must handle repeats.

## Failure behavior

The program rejects retained, duplicate-marked, malformed and rapidly repeated
open commands. It does not retry an uncertain unlock result. Direct relay writes
were rejected in production because a killed process could leave the strike
unlocked; using ISAPI leaves timing with the firmware.

Failed lock-state reads publish unknown. A door-input failure stops the daemon;
its Last Will marks it unavailable. The last retained door state may remain, so
consumers must respect availability. The cover offers Open only—an
electric strike has no motorized Close or Stop action.

## What remains experimental

Quiet opening used a firmware-specific RAM patch. Early-relock experiments did
not relock early. Keypad framing and PIN-policy code were tested offline, but
live UART takeover stayed disabled: two UART readers split the traffic. Those
features should not be treated as requirements for the basic sidecar.

Reboot persistence needs a firmware startup hook on this door-station build.
It is separate from the sidecar itself; see the [firmware build guide](door-station-firmware.md)
for the offline builder and opt-in hook.
