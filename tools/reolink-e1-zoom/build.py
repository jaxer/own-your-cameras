#!/usr/bin/env python3
"""Build an offline-only, rootfs-only SSH candidate for the exact 4417 PAK.

Uses SquashFS pseudo files to preserve stock symlinks, modes, owners, times,
devices and file bytes without mounting or executing the firmware. No network,
camera or flash operations exist in this script.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import struct
import subprocess
import zlib

from ubi import read_stock_pak, parse_ubi, repack, STOCK_SHA256

BASE = Path(__file__).resolve().parent
WORKER_SHA256 = "8610bab73426103d9e046de8a47ede7fd4a4b04c0927cda13da37a1822be6334"
# Official pinned Zig and the historical distributor build differ only in nonloaded compiler comments.
WORKER_SHA256S = frozenset({WORKER_SHA256, "1e9c27dc78633446af3620057d171c21a93445bf614d3da9fb312921bd1109e3"})
TIMESTAMP = 1792627200  # 2026-10-22 00:00:00 UTC, fixed for reproducibility
MARKER = b"# START OF DATA - DO NOT MODIFY\n#\n"
ADDED = {"root/.ssh", "root/.ssh/authorized_keys", "usr/sbin/dropbear",
         "usr/bin/dropbearkey", "etc/init.d/S99_zzRootSSH"}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def run(*args):
    return subprocess.run(args, check=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE).stdout


def pseudo_manifest(path):
    header, data = Path(path).read_bytes().split(MARKER, 1)
    entries = {}
    for line in header.decode().splitlines():
        if not line or line.startswith("#"):
            continue
        fields = line.split()
        name, kind = fields[:2]
        if kind not in ("R", "D", "S", "C", "B", "I"):
            raise ValueError(f"Unimplemented pseudo type: {kind} at {name}")
        entry = {"type": kind, "mtime": int(fields[2]),
                 "mode": fields[3], "uid": int(fields[4]), "gid": int(fields[5])}
        if kind == "R":
            size, offset, sparse = map(int, fields[6:9])
            if sparse != 0 or offset + size > len(data):
                raise ValueError("Unsupported sparse file or truncated pseudo payload")
            entry.update(bytes=size, sha256=digest(data[offset:offset + size]))
        elif kind == "S":
            entry["target"] = " ".join(fields[6:])
        else:
            entry["extra"] = fields[6:]
        if name in entries:
            raise ValueError(f"Duplicate path {name}")
        entries[name] = entry
    return entries


def pak_sections(data):
    result = []
    end = 0x728
    for index in range(13):
        name, ver, start, size = struct.unpack_from("<32s24sII", data, 12 + index * 64)
        if not name.rstrip(b"\0"):
            continue
        if start != end or start + size > len(data):
            raise ValueError("Non-contiguous section extents")
        result.append({"index": index, "name": name.rstrip(b"\0").decode(),
                       "offset": start, "bytes": size,
                       "sha256": digest(data[start:start + size])})
        end = start + size
    if end != len(data):
        raise ValueError("Trailing PAK bytes")
    return result


def make_pak(stock, rootfs):
    before = pak_sections(stock)
    header = bytearray(stock[:0x728])
    chunks = []
    offset = 0x728
    for section in before:
        start, size = section["offset"], section["bytes"]
        payload = rootfs if section["name"] == "rootfs" else stock[start:start + size]
        struct.pack_into("<II", header, 12 + section["index"] * 64 + 56,
                         offset, len(payload))
        chunks.append(payload)
        offset += len(payload)
    data = b"".join(chunks)
    crc = zlib.crc32(data, 0xffffffff)
    crc = zlib.crc32(struct.pack("<I", 2), crc)
    crc = zlib.crc32(header[12:0x34c], crc) ^ 0xffffffff
    struct.pack_into("<I", header, 4, crc)
    candidate = bytes(header) + data
    after = pak_sections(candidate)
    assert stock[0x34c:0x728] == candidate[0x34c:0x728], "MTD table changed"
    assert stock[:4] == candidate[:4] and stock[8:12] == candidate[8:12]
    preserved = []
    for original, new in zip(before, after, strict=True):
        if original["name"] != "rootfs":
            assert original["sha256"] == new["sha256"], "Preserved payload changed"
            preserved.append(original["name"])
    return candidate, {"stored_crc": f"0x{crc:08x}", "sections": after,
                       "byte_identical_sections": preserved,
                       "mtd_table_byte_identical": True,
                       "mtd_table_sha256": digest(stock[0x34c:0x728]),
                       "header_changed_byte_offsets": [i for i in range(0x728)
                           if stock[i] != candidate[i]]}


def main():
    if not __debug__:
        raise SystemExit("Run without -O: integrity checks must remain enabled")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stock_pak", type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--ssh-binary", required=True, type=Path,
                        help="Explicit CPU-verified ARM binary; no implicit CPU target")
    parser.add_argument("--filename-build", type=int, default=4419)
    parser.add_argument("--authorized-key", required=True, type=Path)
    args = parser.parse_args()
    out = args.output_dir.resolve()
    if out.exists():
        raise SystemExit("Output directory must be new")
    stock = args.stock_pak.read_bytes()
    if digest(stock) != STOCK_SHA256:
        raise SystemExit("Unexpected stock artifact")
    key = args.authorized_key.read_text().strip()
    if len(key.splitlines()) != 1 or not key.startswith("ssh-ed25519 "):
        raise SystemExit("Expected one dedicated Ed25519 public key")
    binary = args.ssh_binary.resolve()
    elf = binary.read_bytes()
    if digest(elf) not in WORKER_SHA256S:
        raise SystemExit("SSH worker differs from the tested Cortex-A9 build; use the pinned recipe")
    if len(elf) < 100000 or elf[:7] != b"\x7fELF\x01\x01\x01" or struct.unpack_from("<H", elf, 18)[0] != 40:
        raise SystemExit("Expected a built ARM32 little-endian ELF")
    # PT_INTERP would defeat the intended static-runtime isolation.
    phoff = struct.unpack_from("<I", elf, 28)[0]
    phsize, phnum = struct.unpack_from("<HH", elf, 42)
    if any(struct.unpack_from("<I", elf, phoff + i * phsize)[0] == 3 for i in range(phnum)):
        raise SystemExit("SSH executable has a dynamic interpreter")
    template, provenance = read_stock_pak(args.stock_pak)
    stock_sqsh = parse_ubi(template)["payload"]
    out.mkdir(parents=True, mode=0o700)
    sqsh_input = out / "stock.squashfs"
    sqsh_input.write_bytes(stock_sqsh)
    empty = out / "empty"
    empty.mkdir()
    original = out / "stock.pseudo"
    run("unsquashfs", "-pf", str(original), str(sqsh_input))
    stock_manifest = pseudo_manifest(original)
    header, payload = original.read_bytes().split(MARKER, 1)
    old = b"root D 1724642682 777 1050 1050\n"
    if header.count(old) != 1:
        raise SystemExit("Unexpected stock root directory metadata")
    header = header.replace(old, b"root D 1724642682 700 0 0\n")
    modified = out / "base.pseudo"
    modified.write_bytes(header + MARKER + payload)
    public = out / "authorized_keys"
    public.write_text(key + "\n")
    hook = BASE / "S99_zzRootSSH"
    overlay = out / "overlay.pseudo"
    overlay.write_text(
        f"root/.ssh D {TIMESTAMP} 700 0 0\n"
        f"root/.ssh/authorized_keys F {TIMESTAMP} 600 0 0 cat {shlex.quote(str(public))}\n"
        f"usr/sbin/dropbear F {TIMESTAMP} 755 0 0 cat {shlex.quote(str(binary))}\n"
        f"usr/bin/dropbearkey S {TIMESTAMP} 777 0 0 ../sbin/dropbear\n"
        f"etc/init.d/S99_zzRootSSH F {TIMESTAMP} 755 0 0 cat {shlex.quote(str(hook))}\n")
    sqsh = out / "rootfs.squashfs"
    log = run("mksquashfs", str(empty), str(sqsh), "-pf", str(modified),
              "-pf", str(overlay), "-comp", "xz", "-b", "262144", "-noappend",
              "-processors", "2", "-mem", "256M", "-no-progress",
              "-mkfs-time", "1733998130", "-no-xattrs",
              "-root-mode", "777", "-root-uid", "1050", "-root-gid", "1050",
              "-root-time", "1724642682")
    (out / "mksquashfs.log").write_bytes(log)
    reread = out / "candidate.pseudo"
    run("unsquashfs", "-pf", str(reread), str(sqsh))
    result = pseudo_manifest(reread)
    assert set(result) - set(stock_manifest) == ADDED, "Unexpected added paths"
    assert not set(stock_manifest) - set(result), "Stock paths removed"
    for name, entry in stock_manifest.items():
        expected = dict(entry)
        if name == "root":
            expected.update(mode="700", uid=0, gid=0)
        assert result[name] == expected, f"Stock path changed: {name}"
    for name in ADDED:
        assert result[name]["uid"] == result[name]["gid"] == 0
    ubi, ubi_manifest = repack(template, sqsh.read_bytes())
    (out / "rootfs.ubi").write_bytes(ubi)
    candidate, pak_manifest = make_pak(stock, ubi)
    if args.filename_build < 4418:
        raise SystemExit("Filename build must be above the stock 4417")
    name = f"IPC_566SD664M5MP.{args.filename_build}_2412122178.E1-Zoom.5MP.WIFI7.PTZ.REOLINK_patched.pak"
    (out / name).write_bytes(candidate)
    manifest = {"offline_only": True, "flash_ready": False,
        "stock": provenance, "candidate": {"path": str(out / name),
            "sha256": digest(candidate), "bytes": len(candidate)},
        "ssh_binary_sha256": digest(elf),
        "ssh_binary_path": str(binary),
        "authorized_key_sha256": digest(public.read_bytes()),
        "new_paths": {p: result[p] for p in sorted(ADDED)},
        "changed_existing_paths": {"root": {"before": stock_manifest["root"], "after": result["root"]}},
        "verified_stock_paths": len(stock_manifest),
        "pak": pak_manifest, "ubi": ubi_manifest,
        "limits": ["Offline structure/content checks do not establish hardware boot or updater acceptance",
            "Filename build increment follows upstream precedent; internal version files remain unchanged",
            "Unchanged PAK sections might still be erased/rewritten by the updater",
            "Device bad-block state and physical recovery remain unverified"]}
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"candidate": manifest["candidate"], "preserved": pak_manifest["byte_identical_sections"],
                      "verified_stock_paths": len(stock_manifest), "manifest": str(out / "manifest.json")}, indent=2))


if __name__ == "__main__":
    main()
