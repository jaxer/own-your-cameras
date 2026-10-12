# DS-2CD2386G2-ISU/SL — G5

Tested: G5 / B-R-G5-0, C variant. V5.5.113 was upgraded to official V5.7.23
build 260320. That same image was then installed again from V5.7.23.
Key-only root SSH survived both operations and their reboots.

ARMv7, two CPUs and Linux 4.19.91 were observed on the inspected build. The exact
SoC is not identified here; do not assume it matches the G3 camera.

## Upgrade without losing access

This documents the tested sequence, not a complete installation recipe. Supply
your own SSH worker and loader; G3 also requires firmware-extraction tooling.

**Requires a working root shell.** See [root and persistent SSH](../security/owner-access.md).
V5.7.23's startup script calls `/dav/custom_initrun.sh`, so the official firmware
can start an owner-supplied access worker without repacking the image.

1. Save settings and recovery material privately. Prepare key-only SSH, persistent
   keys and a fallback server binary as described in the access guide.
2. Put the worker on persistent storage. Create `/dav/custom_initrun.sh` to call it.
   The tested setup kept the main access files under `/davinci` and a second copy
   under `/dav`. Use paths chosen for your implementation.
3. On the old build, add a temporary call to the loader in `/dav/initrun.sh`.
   Reboot once and verify key authentication and `uid=0` before upgrading.
4. Obtain the matching [official firmware archive](https://assets.hikvision.com/prd/normal/all/files/202603/Firmware__V5.7.23_260320_S3000708441.zip)
   and verify the extracted DAV hash below.
5. Upload the **untouched** image through `PUT /ISAPI/System/updateFirmware`.
   Read back the loader/access files before reboot. The recorded upload returned
   HTTP 200 with reboot required.
6. Reboot with `PUT /ISAPI/System/reboot`. Verify the reported build, the same pinned
   SSH host key, `uid=0`, and fresh images from channels 101 and 102.

```text
Official DAV SHA-256:
f3a8ee2a5b7b1c8ec57f885edaab1c6f4262e6fd894b348b71a974e18e4f5fa7
```

This tests one old-to-new transition and one same-version reinstall, not an unknown
future release or factory reset. Recording, automation and talkback were not
independently checked in the upgrade session. The scripts and installer are not
included; the steps document the mechanism and order used.

The official custom hook made repacking unnecessary. Modified-image experiments
did not bypass the vendor signature; the successful upgrade used untouched firmware.

## Inspect vendor firmware

[Decode the G5 file table and payloads offline](../software/g5-firmware-decoding.md)
to inspect startup scripts before an upgrade. The included G5 seeds let you decode
the tested builds without camera access or root.
