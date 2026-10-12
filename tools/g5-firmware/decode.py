#!/usr/bin/env python3
"""Decode tested G5 HK20/HK30 packages offline; never flash or execute payloads."""

import argparse
import hashlib
import json
import re
import struct
import zlib
from pathlib import Path

from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
from elftools.common.exceptions import ELFError
from elftools.elf.elffile import ELFFile

# Vendor header-obfuscation constant, not a device credential.
KEY_XOR = bytes.fromhex("BACDBCFE D6CADDD3 BAB9A3AB BFCBB5BE".replace(" ", ""))


# Shared vendor firmware-decryption seeds recovered from G5 V5.5.113 build 210317.
# These are not per-camera credentials or firmware-signing keys.
G5_SEEDS = {
    "header": bytes.fromhex(
        "96b51bee73eb817907c0dbf57fbd280c8eb6201553cef3058054859eee6038da"
    ),
    "file": bytes.fromhex(
        "018f84c3fc13b3e6af0ab81e525bedbb7bca9290a00ddc93c193c0b4e50cfe36"
    ),
}


def xor16(buf):
    return bytes(buf[i] ^ KEY_XOR[(i + (i >> 4)) & 0xF] for i in range(len(buf)))


def gen_aes_key(seed, length=32):
    out = bytearray(length)
    for i in range(length):
        ch = seed[i]
        v = (
            ((i * i) & 0xFF)
            + ((ch * ch) & 0xFF)
            + (ch % length)
            + ((ch * length * i) & 0xFF)
        )
        out[i] = (v & 0xFF) ^ ch
    return bytes(out)


def aes_ecb_dec(data, key):
    c = Cipher(algorithms.AES(key), modes.ECB(), backend=default_backend())
    d = c.decryptor()
    return d.update(data) + d.finalize()


def crc_raw(data):
    """mtd_crc32: standard CRC32 (init 0xFFFFFFFF) without the final XOR."""
    return zlib.crc32(data) ^ 0xFFFFFFFF


def read_keys(path=None):
    if path is None:
        return gen_aes_key(G5_SEEDS["header"]), gen_aes_key(G5_SEEDS["file"])
    with path.open("rb") as stream:
        elf = ELFFile(stream)
        symbols = elf.get_section_by_name(".dynsym")
        matches = symbols.get_symbol_by_name("tFirmAesKey") if symbols else None
        if not matches or len(matches) != 1:
            raise ValueError("Expected exported tFirmAesKey symbol")
        symbol = matches[0]
        if symbol["st_size"] != 72 or not isinstance(symbol["st_shndx"], int):
            raise ValueError("Unsupported AES seed table")
        section = elf.get_section(symbol["st_shndx"])
        offset = symbol["st_value"] - section["sh_addr"]
        seeds = section.data()[offset : offset + 72]
    if (
        len(seeds) != 72
        or struct.unpack_from("<I", seeds)[0] != 0x00F00000
        or struct.unpack_from("<I", seeds, 36)[0] != 0x00F00001
    ):
        raise ValueError("Unexpected G5 seed records")
    return gen_aes_key(seeds[4:36]), gen_aes_key(seeds[40:72])


def unpack(data, k1, k2):
    head = xor16(data[:108])
    if len(head) != 108 or head[:4] != b"02KH":
        raise ValueError("Not an HK20 G5 package")
    hlen = struct.unpack_from("<I", head, 8)[0]
    if hlen != 108:
        raise ValueError("Unsupported outer header length")
    section = xor16(data[hlen : hlen + 16])
    if len(section) != 16 or section[:4] != b"03KH":
        raise ValueError("Not an HK30 section")
    size = struct.unpack_from("<I", section, 8)[0]
    if size < 1024 or size % 256 or hlen + size > len(data):
        raise ValueError("Invalid section length")
    table = aes_ecb_dec(data[hlen + 16 : hlen + size], k1)
    count = struct.unpack_from("<I", table, 0x1F0)[0]
    if size != (count + 4) * 256:
        raise ValueError("Inconsistent file count / section length")
    entries = []
    cursor = hlen + size
    names = set()
    # Each file record is 256 bytes. Scanning every 64 bytes also scans opaque
    # metadata and can mistake random printable bytes for filenames.
    for i in range(count):
        pos = 0x3F0 + i * 256
        name = table[pos : pos + 32].split(b"\0", 1)[0]
        if not re.fullmatch(rb"[A-Za-z0-9_.-]{1,31}", name):
            raise ValueError(f"Invalid file name at record {i}")
        name = name.decode("ascii")
        if name in (".", "..") or name in names:
            raise ValueError("Unsafe or duplicate file name")
        names.add(name)
        offset, length, crc = struct.unpack_from("<III", table, pos + 32)
        start = hlen + offset
        if start != cursor or length == 0 or start + length > len(data):
            raise ValueError(f"Invalid/noncontiguous payload: {name}")
        full = length // 16 * 16
        plain = (
            aes_ecb_dec(data[start : start + full], k2)
            + data[start + full : start + length]
        )
        if crc_raw(plain) != crc:
            raise ValueError(f"Payload CRC mismatch: {name}")
        expected_sha512 = table[pos + 44 : pos + 108]
        actual_sha512 = hashlib.sha512(plain).digest()
        if actual_sha512 != expected_sha512:
            raise ValueError(
                f"Payload SHA-512 mismatch: {name}; CRC preservation is insufficient"
            )
        entries.append(
            {
                "name": name,
                "record": pos,
                "start": start,
                "length": length,
                "crc": crc,
                "plain": plain,
                "expected_sha512": expected_sha512.hex(),
                "actual_sha512": actual_sha512.hex(),
            }
        )
        cursor += length
    if cursor != len(data):
        raise ValueError("Unaccounted trailing package data")
    if struct.unpack_from("<I", section, 4)[0] != crc_raw((section + table)[12:]):
        raise ValueError("HK30 section checksum mismatch")
    if struct.unpack_from("<I", head, 16)[0] != len(data):
        raise ValueError("HK20 package length mismatch")
    if struct.unpack_from("<I", head, 100)[0] != len(data) - hlen:
        raise ValueError("HK20 body length mismatch")
    if struct.unpack_from("<I", head, 104)[0] != sum(data[hlen:]) & 0xFFFFFFFF:
        raise ValueError("HK20 body checksum mismatch")
    if struct.unpack_from("<I", head, 4)[0] != sum(head[12:]) & 0xFFFFFFFF:
        raise ValueError("HK20 header checksum mismatch")
    return head, section, table, entries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("firmware", type=Path)
    parser.add_argument(
        "--davinci",
        type=Path,
        help="Optional vendor ELF seed source; defaults to built-in G5 seeds",
    )
    parser.add_argument(
        "--output", type=Path, required=True, help="New output directory"
    )
    args = parser.parse_args()
    if args.output.exists():
        parser.error("Output directory must not already exist")
    try:
        data = args.firmware.read_bytes()
        k1, k2 = read_keys(args.davinci)
        _, _, _, entries = unpack(data, k1, k2)
        if any(e["name"] == "manifest.json" for e in entries):
            raise ValueError("Reserved output filename: manifest.json")
        args.output.mkdir(parents=True, mode=0o700)
        for entry in entries:
            with (args.output / entry["name"]).open("xb") as stream:
                stream.write(entry["plain"])
        manifest = {
            "firmware_sha256": hashlib.sha256(data).hexdigest(),
            "integrity_checks": "passed",
            "vendor_signature": "not verified",
            "payloads": [
                {"name": e["name"], "bytes": e["length"], "sha512": e["actual_sha512"]}
                for e in entries
            ],
        }
        with (args.output / "manifest.json").open("x") as stream:
            stream.write(json.dumps(manifest, indent=2) + "\n")
        print(f"Decoded {len(entries)} payloads to {args.output}")
        print("Integrity checks passed; vendor signature NOT verified.")
    except (ValueError, OSError, struct.error, ELFError) as error:
        parser.exit(1, f"Decode failed: {error}\n")


if __name__ == "__main__":
    main()
