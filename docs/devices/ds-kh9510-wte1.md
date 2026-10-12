# DS-KH9510-WTE1 indoor station

| Item | Observed value |
|---|---|
| Firmware | V2.2.28 build 230613 |
| Platform | R1 / rk312x |
| OS | Android 5.1.1, API 22 |
| Relevant media path | Android hardware AVC decoding |

Native video/touch applications and a remote screen viewer were implemented in
separate work. Persistent key-authenticated root SSH was also established. This
repository does not yet contain the panel's root procedure or application source.

System-partition changes survived reboots, but firmware updates can replace them.
Inspected reference firmware did not exactly match the installed build; matching
individual binaries is not enough to choose a flash image.
