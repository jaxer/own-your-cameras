#!/usr/bin/env python3
"""Offline-only, deliberately narrow 4417 rootfs UBI template repacker.

No flash or upload functionality. Structural checks cannot establish bootability or updater acceptance.
"""
import hashlib
import struct
from pathlib import Path
import zlib

STOCK_SHA256 = "ae1227a89eae4d801381490c95b3fbc38e5e0040d7c839d90f1993ec0b771d5c"
PEB_SIZE = 131072
VID_OFFSET = 2048
DATA_OFFSET = 4096
LEB_SIZE = PEB_SIZE - DATA_OFFSET
LAYOUT_VOLUME_ID = 0x7fffefff
VTBL_RECORD_SIZE = 172
VTBL_RECORDS = 128
PARTITION_BYTES = 0xf00000
# Leave room for layout-independent UBI reserves and some bad blocks. The real
# device's bad-block count is unknown, so this is a guard, not an attach proof.
MIN_SPARE_PEBS = 6


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def ubi_crc(data):
    """Linux UBI CRC32 starts at 0xffffffff and has no final complement."""
    return zlib.crc32(data) ^ 0xffffffff


def check_crc(header, label):
    require(struct.unpack_from(">I", header, len(header) - 4)[0] == ubi_crc(header[:-4]),
            f"{label}: CRC mismatch")


def refresh_crc(header):
    struct.pack_into(">I", header, len(header) - 4, ubi_crc(header[:-4]))


def squashfs_payload(data):
    require(len(data) >= 96 and data[:4] == b"hsqs", "Expected little-endian SquashFS")
    require(struct.unpack_from("<HH", data, 28) == (4, 0), "Expected SquashFS version 4.0")
    used = struct.unpack_from("<Q", data, 40)[0]
    require(96 <= used <= len(data), "Invalid SquashFS bytes_used")
    # mksquashfs may append zero padding, but never accept arbitrary trailing data.
    require(len(data) <= ((used + 4095) // 4096) * 4096,
            "SquashFS has excessive trailing data")
    require(not any(data[used:]), "SquashFS trailing bytes are not zero padding")
    return data[:used]


def read_stock_pak(path):
    data = Path(path).read_bytes()
    require(sha256(data) == STOCK_SHA256, "Only the exact stock 4417 PAK is supported")
    require(struct.unpack_from("<I", data)[0] == 0x32725913, "Wrong PAK magic")
    stored = struct.unpack_from("<I", data, 4)[0]
    crc = zlib.crc32(data[0x728:], 0xffffffff)
    crc = zlib.crc32(struct.pack("<I", 2), crc)
    crc = zlib.crc32(data[12:12 + 13 * 64], crc) ^ 0xffffffff
    require(crc == stored, "Stock PAK CRC mismatch")
    records = []
    for index in range(13):
        name, version, start, size = struct.unpack_from("<32s24sII", data, 12 + 64 * index)
        if name.rstrip(b"\0"):
            require(0x728 <= start <= start + size <= len(data), "PAK section out of bounds")
            records.append((name.rstrip(b"\0"), start, size))
    rootfs = [r for r in records if r[0] == b"rootfs"]
    require(len(rootfs) == 1, "Expected exactly one rootfs section")
    _, start, size = rootfs[0]
    return data[start:start + size], {"path": str(Path(path).resolve()),
            "sha256": sha256(data), "rootfs_offset": start, "rootfs_bytes": size}


def parse_ubi(data):
    """Validate the supported image shape; reject other UBI variants explicitly."""
    require(len(data) % PEB_SIZE == 0, "UBI image is not a whole number of PEBs")
    count = len(data) // PEB_SIZE
    require(3 <= count <= PARTITION_BYTES // PEB_SIZE - MIN_SPARE_PEBS,
            "UBI image exceeds conservative partition/spare-PEB bound")
    pebs = [data[i:i + PEB_SIZE] for i in range(0, len(data), PEB_SIZE)]
    image_seq = None
    for i, peb in enumerate(pebs):
        ec, vid = peb[:64], peb[VID_OFFSET:VID_OFFSET + 64]
        require(ec[:8] == b"UBI#\x01\0\0\0", f"PEB {i}: unsupported EC header")
        check_crc(ec, f"PEB {i} EC")
        require(struct.unpack_from(">II", ec, 16) == (VID_OFFSET, DATA_OFFSET),
                f"PEB {i}: unexpected VID/data offsets")
        require(ec[28:60] == bytes(32), f"PEB {i}: nonzero EC reserved bytes")
        seq = struct.unpack_from(">I", ec, 24)[0]
        image_seq = seq if image_seq is None else image_seq
        require(seq == image_seq, f"PEB {i}: inconsistent image_seq")
        require(peb[64:VID_OFFSET] == b"\xff" * (VID_OFFSET - 64),
                f"PEB {i}: unsupported EC-to-VID gap")
        require(peb[VID_OFFSET + 64:DATA_OFFSET] == b"\xff" * (DATA_OFFSET - VID_OFFSET - 64),
                f"PEB {i}: unsupported VID-to-data gap")
        check_crc(vid, f"PEB {i} VID")
        require(vid[:8] == (b"UBI!\x01\x01\0\x05" if i < 2 else b"UBI!\x01\x01\0\0"),
                f"PEB {i}: unsupported VID type/flags")
        require(struct.unpack_from(">II", vid, 8) ==
                ((LAYOUT_VOLUME_ID, i) if i < 2 else (0, i - 2)),
                f"PEB {i}: unexpected volume/logical numbering")
        require(vid[16:40] == bytes(24) and vid[48:60] == bytes(12),
                f"PEB {i}: unsupported dynamic VID fields")
    tables = [pebs[i][DATA_OFFSET:DATA_OFFSET + VTBL_RECORDS * VTBL_RECORD_SIZE] for i in (0, 1)]
    require(tables[0] == tables[1], "Redundant volume tables differ")
    for i in range(VTBL_RECORDS):
        record = tables[0][i * VTBL_RECORD_SIZE:(i + 1) * VTBL_RECORD_SIZE]
        check_crc(record, f"volume table record {i}")
        if i:
            require(record[:-4] == bytes(VTBL_RECORD_SIZE - 4), "Unexpected additional volume")
    record = tables[0][:VTBL_RECORD_SIZE]
    reserved, alignment, data_pad, vol_type, upd_marker, name_len = struct.unpack_from(">IIIBBH", record)
    require((reserved, alignment, data_pad, vol_type, upd_marker, name_len) ==
            (count - 2, 1, 0, 1, 0, 6), "Unsupported rootfs volume metadata")
    require(record[16:22] == b"rootfs" and record[22:168] == bytes(146),
            "Unexpected rootfs name, flags, or reserved fields")
    for i in (0, 1):
        require(pebs[i][DATA_OFFSET + len(tables[i]):] == b"\xff" *
                (LEB_SIZE - len(tables[i])), "Unexpected data after volume table")
    payload = b"".join(peb[DATA_OFFSET:] for peb in pebs[2:])
    require(payload[:4] == b"hsqs", "Rootfs volume does not begin with SquashFS")
    used = struct.unpack_from("<Q", payload, 40)[0]
    require(96 <= used <= len(payload), "Invalid embedded SquashFS bytes_used")
    padded = ((used + 4095) // 4096) * 4096
    require(payload[used:padded] == bytes(padded - used), "Unexpected SquashFS zero padding")
    require(payload[padded:] == b"\xff" * (len(payload) - padded), "Unexpected rootfs tail padding")
    squashfs_payload(payload[:used])
    return {"pebs": pebs, "payload": payload[:used], "image_seq": image_seq,
            "peb_count": count, "data_pebs": count - 2, "reserved_pebs": reserved,
            "erase_counters": [struct.unpack_from(">Q", p, 8)[0] for p in pebs],
            "vid_sequence_numbers": [struct.unpack_from(">Q", p, VID_OFFSET + 40)[0] for p in pebs]}


def repack(template, replacement):
    stock = parse_ubi(template)
    payload = squashfs_payload(replacement)
    data_pebs = max(stock["data_pebs"], (len(payload) + LEB_SIZE - 1) // LEB_SIZE)
    total_pebs = data_pebs + 2
    require(total_pebs <= PARTITION_BYTES // PEB_SIZE - MIN_SPARE_PEBS,
            "Replacement exceeds conservative partition/spare-PEB bound")
    padded_len = ((len(payload) + 4095) // 4096) * 4096
    require(padded_len <= data_pebs * LEB_SIZE, "SquashFS padding exceeds allocated LEBs")
    padded = payload + bytes(padded_len - len(payload))
    padded += b"\xff" * (data_pebs * LEB_SIZE - len(padded))
    pebs = [bytearray(p) for p in stock["pebs"]]
    for _ in range(total_pebs - len(pebs)):
        # For this stock image all EC and VID sqnum values are zero. Never infer
        # how to increment sequence state for a different runtime-generated UBI.
        require(set(stock["erase_counters"]) == {0} and
                set(stock["vid_sequence_numbers"]) == {0},
                "Growth requires stock zero EC/VID sequence numbers")
        pebs.append(bytearray(stock["pebs"][-1]))
    for i, peb in enumerate(pebs):
        ec = bytearray(peb[:64])
        refresh_crc(ec)
        peb[:64] = ec
        vid = bytearray(peb[VID_OFFSET:VID_OFFSET + 64])
        if i >= 2:
            struct.pack_into(">I", vid, 12, i - 2)
            peb[DATA_OFFSET:] = padded[(i - 2) * LEB_SIZE:(i - 1) * LEB_SIZE]
        else:
            record = bytearray(peb[DATA_OFFSET:DATA_OFFSET + VTBL_RECORD_SIZE])
            struct.pack_into(">I", record, 0, data_pebs)
            refresh_crc(record)
            peb[DATA_OFFSET:DATA_OFFSET + VTBL_RECORD_SIZE] = record
            # Recalculate every redundant volume-table record, including empties.
            for recnum in range(1, VTBL_RECORDS):
                off = DATA_OFFSET + recnum * VTBL_RECORD_SIZE
                record = bytearray(peb[off:off + VTBL_RECORD_SIZE])
                refresh_crc(record)
                peb[off:off + VTBL_RECORD_SIZE] = record
        refresh_crc(vid)
        peb[VID_OFFSET:VID_OFFSET + 64] = vid
    result = b"".join(pebs)
    verified = parse_ubi(result)
    require(verified["payload"] == payload, "Reconstructed payload differs from input")
    changed_pebs = [i for i in range(total_pebs)
                    if i >= stock["peb_count"] or bytes(pebs[i]) != stock["pebs"][i]]
    manifest = {
        "schema_version": 1, "offline_only": True,
        "template_ubi_sha256": sha256(template), "output_ubi_sha256": sha256(result),
        "replacement_input_sha256": sha256(replacement), "squashfs_sha256": sha256(payload),
        "squashfs_bytes_used": len(payload), "input_zero_padding_bytes_removed": len(replacement) - len(payload),
        "geometry": {"peb_bytes": PEB_SIZE, "vid_offset": VID_OFFSET,
                     "data_offset": DATA_OFFSET, "leb_bytes": LEB_SIZE,
                     "partition_bytes": PARTITION_BYTES, "minimum_spare_pebs_guard": MIN_SPARE_PEBS},
        "stock": {"ubi_bytes": len(template), "data_pebs": stock["data_pebs"],
                  "reserved_pebs": stock["reserved_pebs"], "squashfs_bytes_used": len(stock["payload"])},
        "output": {"ubi_bytes": len(result), "data_pebs": data_pebs,
                   "reserved_pebs": data_pebs, "partition_spare_pebs": PARTITION_BYTES // PEB_SIZE - total_pebs,
                   "volume_slack_bytes": data_pebs * LEB_SIZE - len(payload)},
        "preserved": {"image_seq": stock["image_seq"], "volume_name": "rootfs",
                      "volume_id": 0, "volume_type": "dynamic", "alignment": 1,
                      "data_pad": 0, "flags": 0, "redundant_layout_tables": 2,
                      "existing_ec_headers_and_vid_sequence_numbers": True},
        "changes": {"changed_peb_indices": changed_pebs,
                    "added_data_pebs": data_pebs - stock["data_pebs"],
                    "reserved_pebs_delta": data_pebs - stock["reserved_pebs"],
                    "identical_to_template": result == template},
        "validation": {"ec_vid_and_all_vtbl_crcs": True, "redundant_tables_identical": True,
                       "logical_numbering_contiguous": True, "payload_matches_input": True,
                       "preserves_stock_squashfs_roundtrip": repack_roundtrip(template, stock)},
        "limits": ["Does not build a PAK, sign, upload, flash, or execute anything on the camera.",
                   "SquashFS superblock/padding and UBI structure checked; filesystem contents not decompressed here.",
                   "No proof of updater acceptance, bootability, recovery, or sufficient spare good NAND blocks."]}
    return result, manifest


def repack_roundtrip(template, stock):
    # Independent identity check on the exact stock payload and padding pattern.
    payload = stock["payload"]
    padded_len = ((len(payload) + 4095) // 4096) * 4096
    padded = payload + bytes(padded_len - len(payload))
    padded += b"\xff" * (stock["data_pebs"] * LEB_SIZE - len(padded))
    reconstructed = b"".join(stock["pebs"][:2]) + b"".join(
        peb[:DATA_OFFSET] + padded[i * LEB_SIZE:(i + 1) * LEB_SIZE]
        for i, peb in enumerate(stock["pebs"][2:]))
    require(reconstructed == template, "Stock SquashFS roundtrip is not byte-identical")
    return True
