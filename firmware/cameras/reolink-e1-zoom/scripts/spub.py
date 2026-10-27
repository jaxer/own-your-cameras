#!/usr/bin/env python3
"""Build/check kernel, rootfs and app SPUB restores for the E1 Zoom (device recovery unverified).

The layout is what the vendor `upgrade` daemon writes (docs/recovery.md has the derivation):
descriptor at 0x3f188, staging offsets at 0x45630, partition table at .data 0x9c37c.

    spub.py build STOCK.pak -o stock.spub
    spub.py --check stock.spub

Bootloader sections are always omitted; settings/calibration sections are rejected.
There is no full-vendor SPUB build/check mode. A successful check verifies the host
layout and CRCs only; it does not authorize flashing (see docs/recovery.md).
"""
import argparse
import hashlib
import struct
import sys
import zlib
from collections import namedtuple

MAGIC = 0x42555053  # "SPUB"
PAK_MAGICS = (0x32725913, 0x32725923)
PAK_HEADER = 0x728
BLOCK = 0x20000  # NAND erase block; staging is block-aligned
DOWNLOAD_START = 0x2480000  # added to every entry's `start`
STAGING_BLOCKS = 8  # descriptor + reserved blocks before the first section
HEADER_BYTES = 24
ENTRY_BYTES = 60

# name -> (flash start, partition size): upgrade's static table, matches /proc/mtd.
PARTITIONS = {
    "loader": (0x0, 0x40000),
    "fdt": (0x40000, 0x40000),
    "uboot": (0x80000, 0x100000),
    "kernel": (0x180000, 0x400000),
    "rootfs": (0x580000, 0xF00000),
    "app": (0x1480000, 0xB00000),
    "para": (0x1F80000, 0x400000),
    "sp": (0x2380000, 0x80000),
    "ep": (0x2400000, 0x80000),
}
DOWNLOAD_SIZE = 0x1B80000
SAFE_PARTITIONS = frozenset(("kernel", "rootfs", "app"))
OMITTED_PARTITIONS = frozenset(("loader", "fdt", "uboot"))

# Recorded kernel/rootfs/app-only stock 4417 digest (formerly stock-nobl.spub).
STOCK_SPUB_SHA256 = "7795d52e0a8a43fae1b34ae9bbb862cd98e7d7978226dadd9dc242c2e9fb3901"

Entry = namedtuple("Entry", "name size start crc addr space ext_type ext_len")
Report = namedtuple("Report", "entries image_bytes")


class SpubError(Exception):
    pass


def crc(data):
    """Vendor bc_gen_crc(0, data): CRC-32 with the register starting at 0 and no final xor."""
    return zlib.crc32(data, 0xFFFFFFFF) ^ 0xFFFFFFFF


def align(size):
    return -(-size // BLOCK) * BLOCK


def section_name(raw):
    """Decode an unambiguous NUL-terminated, zero-padded ASCII name."""
    name, terminator, padding = raw.partition(b"\0")
    if not terminator or any(padding):
        raise SpubError("malformed section name (expected NUL termination and zero padding)")
    try:
        return name.decode("ascii")
    except UnicodeDecodeError as err:
        raise SpubError("non-ASCII section name") from err


def parse_pak(pak):
    """Validate all PAK records before selection; return [(name, bytes)].

    Only the payload records are imported. PAK metadata after them (including
    the vendor partition table) is never used for SPUB destinations or flags.
    """
    if len(pak) < PAK_HEADER:
        raise SpubError("PAK shorter than its header")
    magic, stored_crc, format_word = struct.unpack_from("<III", pak)
    if magic not in PAK_MAGICS:
        raise SpubError(f"PAK magic {magic:#x} unknown")
    if format_word != 0x4102:
        raise SpubError(f"unsupported PAK format {format_word:#x}")
    records = pak[12:12 + 13 * 64]
    computed = zlib.crc32(pak[PAK_HEADER:], 0xFFFFFFFF)
    computed = zlib.crc32(struct.pack("<I", 2), computed)
    computed = zlib.crc32(records, computed) ^ 0xFFFFFFFF
    if computed != stored_crc:
        raise SpubError(f"PAK CRC {computed:#010x} != stored {stored_crc:#010x}")
    sections, names, end = [], set(), PAK_HEADER
    for index in range(13):
        name, version, start, length = struct.unpack_from("<32s24sII", records, index * 64)
        name = section_name(name)
        if not name:
            # Stock unused slots have start=EOF; zeroed fixture slots are also valid.
            if any(version) or length or start not in (0, len(pak)):
                raise SpubError(f"malformed unused PAK record {index}")
            continue
        if name not in SAFE_PARTITIONS | OMITTED_PARTITIONS:
            raise SpubError(f"{name!r}: unknown or protected PAK section")
        if name in names:
            raise SpubError(f"{name}: duplicate PAK section")
        names.add(name)
        if not 0 < length <= PARTITIONS[name][1]:
            raise SpubError(f"{name}: PAK payload must be positive and fit its partition")
        if start != end or start + length > len(pak):
            raise SpubError(f"section {name} lies outside the contiguous PAK body")
        sections.append((name, pak[start:start + length]))
        end = start + length
    if end != len(pak):
        raise SpubError("trailing bytes after the last PAK section")
    if not sections:
        raise SpubError("empty PAK")
    return sections


def build_spub(pak, skip=()):
    """Build a kernel/rootfs/app-only restore; boot sections are always omitted."""
    skip = set(skip)
    if skip - (SAFE_PARTITIONS | OMITTED_PARTITIONS):
        raise SpubError("unknown or protected skip name")
    sections = parse_pak(pak)
    sections = [(n, d) for n, d in sections if n in SAFE_PARTITIONS and n not in skip]
    if not sections:
        raise SpubError("empty safe section selection")
    entries, body, offset = [], b"", STAGING_BLOCKS * BLOCK
    for name, data in sections:
        addr, space = PARTITIONS[name]
        entries.append(struct.pack("<32s7I", name.encode(), len(data), DOWNLOAD_START + offset,
                                   crc(data), addr, space, 0, 0))
        body += data.ljust(align(len(data)), b"\xff")
        offset += align(len(data))
    if offset > DOWNLOAD_SIZE:
        raise SpubError("staged image does not fit the download partition")
    table = b"".join(entries)
    body_of_header = struct.pack("<IIII", 1, len(entries), 0, 0) + table
    descriptor = struct.pack("<II", MAGIC, crc(body_of_header)) + body_of_header
    return descriptor.ljust(STAGING_BLOCKS * BLOCK, b"\xff") + body


def check_spub(image):
    """Check the safe restore layout and CRCs, not device readiness or recovery success."""
    if len(image) < HEADER_BYTES:
        raise SpubError("truncated: shorter than the SPUB header")
    magic, header_crc, update, num, reserved, ext_len = struct.unpack_from("<6I", image)
    if magic != MAGIC:
        raise SpubError(f"bad magic {magic:#010x} (want {MAGIC:#010x})")
    if not 1 <= num <= 19:
        raise SpubError(f"entry count {num} outside 1..19")
    if ext_len:
        raise SpubError("header extension is not produced by upgrade; refusing to guess its layout")
    if update != 1 or reserved:
        raise SpubError("unsupported update or reserved header field")
    end = HEADER_BYTES + num * ENTRY_BYTES
    if len(image) < end:
        raise SpubError("truncated: descriptor cut short")
    if crc(image[8:end]) != header_crc:
        raise SpubError("header CRC mismatch")
    if len(image) > DOWNLOAD_SIZE:
        raise SpubError("image exceeds download partition capacity")
    if len(image) % BLOCK:
        raise SpubError("truncated: image must contain whole staging blocks")
    entries, names, ranges = [], set(), []
    for i in range(num):
        fields = struct.unpack_from("<32s7I", image, HEADER_BYTES + i * ENTRY_BYTES)
        entry = Entry(section_name(fields[0]), *fields[1:])
        if entry.name not in SAFE_PARTITIONS:
            raise SpubError(f"{entry.name!r}: not a safe destination (kernel, rootfs, app only)")
        if (entry.addr, entry.space) != PARTITIONS[entry.name]:
            raise SpubError(f"{entry.name}: destination does not match the verified partition table")
        if not 0 < entry.size <= entry.space:
            raise SpubError(f"{entry.name}: payload must be positive and fit its partition")
        if entry.ext_type or entry.ext_len:
            raise SpubError(f"{entry.name}: unsupported entry extension")
        if entry.name in names:
            raise SpubError(f"{entry.name}: duplicate destination")
        names.add(entry.name)
        offset = entry.start - DOWNLOAD_START
        if offset < STAGING_BLOCKS * BLOCK or offset % BLOCK:
            raise SpubError(f"{entry.name}: start {entry.start:#x} not block-aligned inside download")
        stop = offset + align(entry.size)
        if stop > DOWNLOAD_SIZE:
            raise SpubError(f"{entry.name}: staged range exceeds download partition capacity")
        if len(image) < stop:
            raise SpubError(f"truncated: {entry.name} data cut short")
        if any(offset < other_stop and other_start < stop for other_start, other_stop in ranges):
            raise SpubError(f"{entry.name}: overlapping staging ranges")
        ranges.append((offset, stop))
        if crc(image[offset:offset + entry.size]) != entry.crc:
            raise SpubError(f"{entry.name}: data CRC mismatch")
        entries.append(entry)
    return Report(entries, len(image))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", metavar="SPUB", help="verify safe layout/CRCs and print entries; no device checks")
    sub = parser.add_subparsers(dest="cmd")
    build = sub.add_parser("build", help="build a kernel/rootfs/app-only container",
                           description="Always omits loader/fdt/uboot; rejects settings/calibration sections.")
    build.add_argument("pak")
    build.add_argument("-o", "--output", required=True)
    build.add_argument("--skip", default="", help="omit kernel/rootfs/app sections (comma-separated); "
                       "loader,fdt,uboot accepted for compatibility and always omitted")
    args = parser.parse_args(argv)
    if args.check and args.cmd:
        parser.error("--check cannot be combined with build")
    try:
        if args.check:
            with open(args.check, "rb") as f:
                image = f.read()
            report = check_spub(image)
            for e in report.entries:
                print(f"{e.name:8} size={e.size:#09x} start={e.start:#010x} crc={e.crc:#010x} "
                      f"addr={e.addr:#09x} space={e.space:#09x}")
            print(f"OK safe layout/CRCs; device recovery unverified. "
                  f"{len(image)} bytes sha256={hashlib.sha256(image).hexdigest()}")
        elif args.cmd == "build":
            with open(args.pak, "rb") as f:
                pak = f.read()
            image = build_spub(pak, skip=args.skip.split(",") if args.skip else ())
            with open(args.output, "wb") as f:
                f.write(image)
            print(f"wrote {args.output}: {len(image)} bytes sha256={hashlib.sha256(image).hexdigest()}")
        else:
            parser.error("give `build PAK -o OUT` or `--check SPUB`")
    except (SpubError, OSError) as err:
        print(f"spub: {err}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
