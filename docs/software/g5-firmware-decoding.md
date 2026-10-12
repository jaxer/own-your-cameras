# Decode G5 firmware offline

Tested on DS-2CD2386G2-ISU/SL, G5 / B-R-G5-0, C variant:
V5.5.800 build 210628 and V5.7.23 build 260320. Other platforms and builds
need separate inspection.

## 1. Prepare inputs

- Download a tested firmware archive and extract `digicap.dav` from its ZIP.
  [V5.7.23 vendor archive](https://assets.hikvision.com/prd/normal/all/files/202603/Firmware__V5.7.23_260320_S3000708441.zip).
- No camera connection or root access is needed. The decoder includes the shared
  G5 firmware seeds recovered from V5.5.113 build 210317.

The V5.7.23 DAV must have SHA-256:

```text
f3a8ee2a5b7b1c8ec57f885edaab1c6f4262e6fd894b348b71a974e18e4f5fa7
```

## 2. Decode

Run from the repository root with Python 3.14. Replace `STOCK_DAV` with
its local file path. The output directory must be new.

```sh
python3.14 -m venv .venv
.venv/bin/python -m pip install -r tools/g5-firmware/requirements.txt
.venv/bin/python tools/g5-firmware/decode.py STOCK_DAV \
  --output build/g5-decoded
```

The decoder validates the table, every payload's CRC32 and SHA-512, package
lengths and outer checksums before writing files. It does not contact a camera,
execute extracted files, or verify the vendor RSA signature.

## 3. Inspect

```sh
cat build/g5-decoded/manifest.json
sed -n '1,160p' build/g5-decoded/initrun.sh
```

Expected: 22 payloads for V5.5.800 or 32 for V5.7.23, plus a manifest with
`integrity_checks: passed`.
In V5.7.23, `initrun.sh` calls `/dav/custom_initrun.sh`; see the
[upgrade guide](../devices/ds-2cd2386g2-isu-sl.md).
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

By default, the decoder derives K1 and K2 from the included G5 seeds. To use
seeds from your own vendor executable, add `--davinci VENDOR_DAVINCI` to the
command above. This optional mode locates `tFirmAesKey` through ELF symbols
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

Offline checks on 2026-10-12, using the same V5.5.113 G5 seed pair throughout.
[File hashes and results; URLs for new downloads](g5-seed-checks.json).

| Firmware sample | Result |
|---|---|
| Standard G5 V5.5.800 build 210628 | All 22 payload CRCs and SHA-512 digests pass |
| Standard G5 V5.7.23 build 260320 | All 32 payload CRCs and SHA-512 digests pass |
| G5 XS6A solar / 6A25 V5.5.801 build 210701 | All 22 payload CRCs and SHA-512 digests pass |
| G1 V5.6.821 build 230831 | G5 table key fails HK30 CRC |
| G3 V5.5.131 build 200220, V5.5.160 build 210108, V5.5.820 build 231108 | G5 table key fails HK30 CRC |
| H8 explosion-proof PTZ V5.7.1 build 210714 | G5 table key fails HK30 CRC |

All samples passed outer package length and checksum checks. The failed table
checks establish that this G5 pair cannot decode those samples with the inspected
HK30 scheme; payload-key reuse was not independently tested there.

The new standard G5 download matched the previously retained DAV SHA-256 above.
The G1, solar G5 and H8 samples came from the vendor's
[UK download portal](https://www.hikvisioneurope.com/uk/portal/).
Those downloads required disabling TLS certificate verification locally;
checksums establish internal consistency, not source authenticity. Vendor RSA
signatures were not verified. Results apply to these files, not every model in a family.
