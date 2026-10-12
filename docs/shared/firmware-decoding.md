# Decode G3/G5 firmware offline

One offline decoder for the tested G3 and G5 camera packages. No camera connection
or root access is needed; both shared seed pairs are included.

## 1. Prepare inputs

- Download a tested firmware archive and extract `digicap.dav` from its ZIP.
  [G3 V5.5.820 archive](https://assets.hikvision.com/prd/public/all/files/202312/Firmware__V5.5.820_231108_S3000539625.zip) or
  [G5 V5.7.23 archive](https://assets.hikvision.com/prd/normal/all/files/202603/Firmware__V5.7.23_260320_S3000708441.zip).

Expected DAV SHA-256:

```text
G3 V5.5.820 build 231108:
0887eac3807f9afcf1912c0faba5b580874000de27b0f184ed5ef17bd67b42dc
G5 V5.7.23 build 260320:
f3a8ee2a5b7b1c8ec57f885edaab1c6f4262e6fd894b348b71a974e18e4f5fa7
```

## 2. Decode

Run from the repository root with Python 3.14. Replace `STOCK_DAV` with
its local file path. The output directory must be new.

```sh
python3.14 -m venv .venv
.venv/bin/python -m pip install -r tools/camera-firmware/requirements.txt
.venv/bin/python tools/camera-firmware/decode.py STOCK_DAV \
  --output build/camera-decoded
```

The default tries G3 and G5 seeds and selects a profile only after all integrity
checks pass. Add `--platform g3` or `--platform g5` to restrict the choice.
The selected seed profile is printed and recorded in the output manifest.

The decoder validates the table, every payload's CRC32 and SHA-512, package
lengths and outer checksums before writing files. It does not contact a camera,
execute extracted files, or verify the vendor RSA signature.

## 3. Inspect

```sh
cat build/camera-decoded/manifest.json
sed -n '1,160p' build/camera-decoded/initrun.sh
```

Expected: 19 payloads for G3 V5.5.820 or 32 for G5 V5.7.23, plus a manifest with
`integrity_checks: passed`. Matching a seed profile does not establish hardware
compatibility or permission to flash a package.

G5 V5.7.23 calls `/dav/custom_initrun.sh`; G3 V5.5.820 does not. See the
[G3](../devices/ds-2cd2t87g2-l/README.md) and [G5](../devices/ds-2cd2386g2-isu-sl/README.md)
upgrade guides.
Extracted payloads may themselves be archives or filesystem images.

## Format reference

All integers below are little-endian. Offsets in the table are relative to its
**decrypted body**, excluding the 16-byte HK30 header.

| Part | Decoding |
|---|---|
| HK20 outer header | 108 bytes; XOR mask indexed by `(i + (i >> 4)) & 15` |
| HK30 header | 16 bytes; restart the same XOR indexing; size includes this header |
| HK30 table body | AES-256-ECB with K1; count at `0x1f0`, records at `0x3f0`, stride 256 |
| File record | Name at +0 (32 bytes), offset +32, length +36, CRC +40, SHA-512 +44 (64 bytes) |
| Payload | Starts at `108 + offset`; full 16-byte blocks use AES-256-ECB with K2; partial tail stays plaintext |
| CRC | `zlib.crc32(plaintext) ^ 0xffffffff`; HK30 CRC covers decrypted section bytes from +12 |
| Outer checksums | Header sum over decoded header[12:]; body sum over stored bytes after offset 108 |

The decoder derives K1 and K2 from the selected platform's included seeds.
The G3 pair came from V5.5.154 build 200928; the G5 pair from V5.5.113 build 210317.
For the inspected G5 seed table only, `--davinci VENDOR_DAVINCI` uses your own
vendor executable instead of the included seeds; it cannot be combined with
`--platform g3`. This optional mode locates `tFirmAesKey` through ELF symbols
rather than a fixed file address; another build may use a different table.
Its two records contain a 32-byte seed after a 4-byte ID:
`0x00f00000` supplies K1, `0x00f00001` supplies K2. For seed byte `ch` at index `i`:

```text
key[i] = ((i*i + ch*ch + ch%32 + ch*32*i) & 255) ^ ch
```

The included seeds are shared firmware-decryption values, not per-camera
credentials or signing keys.
Checksums are integrity checks, not authorization: edited payloads also need
updated SHA-512 metadata and a trusted signature. Preserving only length and CRC
was insufficient in the tested modified-image experiments.

## Seed reuse checks

Offline checks on 2026-10-12. Each profile uses one fixed seed pair across its
tested builds.

| G3 firmware sample | G3 profile result |
|---|---|
| V5.5.131 build 200220 | All 18 payload CRCs and SHA-512 digests pass |
| V5.5.160 build 210108 | All 23 payload CRCs and SHA-512 digests pass |
| V5.5.820 build 231108 | All 19 payload CRCs and SHA-512 digests pass |

| Firmware sample | G5 profile result |
|---|---|
| Standard G5 V5.5.800 build 210628 | All 22 payload CRCs and SHA-512 digests pass |
| Standard G5 V5.7.23 build 260320 | All 32 payload CRCs and SHA-512 digests pass |
| G5 XS6A solar / 6A25 V5.5.801 build 210701 | All 22 payload CRCs and SHA-512 digests pass |
| G1 V5.6.821 build 230831 | G5 table key fails HK30 CRC |
| G3 V5.5.131 build 200220, V5.5.160 build 210108, V5.5.820 build 231108 | G5 table key fails HK30 CRC |
| H8 explosion-proof PTZ V5.7.1 build 210714 | G5 table key fails HK30 CRC |

The results suggest seed reuse within each tested platform. We assume reuse only
for the tested builds; other models and future releases need separate checks.
The failed table checks do not establish whether the payload key is shared.

All samples passed outer package length and checksum checks. Vendor RSA signatures
were not verified. The additional cross-family samples were downloaded from the
vendor UK portal with TLS verification disabled because of a local certificate
error; source authenticity for those downloads remains unverified.
