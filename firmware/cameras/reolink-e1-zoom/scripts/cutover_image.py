#!/usr/bin/env python3
"""Offline cutover image builder and verifier from public inputs only; never uploads or flashes.

One PAK = the reader's stock 4417 PAK + the five root-SSH additions own-your-cameras makes + our three files
(start_app assembled from the stock blocks, cam, cam-launch), plus stock.spub and a manifest. Needs
dissect.squashfs and ubi-reader (own-your-cameras' requirements.txt) in the running Python's environment.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import shlex
import shutil
import struct
import subprocess
import sys
import zlib

sys.path.insert(0, str(Path(__file__).resolve().parent))
import own_your_cameras  # noqa: E402
import spub  # noqa: E402
import start_app  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
TIMESTAMP = 1792627200
RELEASE = ROOT / "target/armv7-unknown-linux-gnueabihf/release"
BINARIES = {"/usr/bin/cam": RELEASE / "cam", "/usr/bin/cam-launch": RELEASE / "cam-launch"}
ARTIFACT_PATHS = frozenset(("/etc/init.d/start_app", "/usr/bin/cam", "/usr/bin/cam-launch"))
CANDIDATE_NAME = "IPC_566SD664M5MP.4420_2412122178.E1-Zoom.5MP.WIFI7.PTZ.REOLINK_patched.pak"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(*args):
    return subprocess.run(args, check=True, capture_output=True).stdout


def file_entry(data, mode):
    return {"mode": mode, "uid": 0, "gid": 0, "mtime": TIMESTAMP, "bytes": len(data), "sha256": digest(data)}


def ssh_entries(dropbear, authorized_key, hook):
    """The five paths own-your-cameras' build.py adds, as the independent reader must see them."""
    return {"/root/.ssh": {"mode": 0o40700, "uid": 0, "gid": 0, "mtime": TIMESTAMP},
            "/root/.ssh/authorized_keys": file_entry(authorized_key, 0o100600),
            "/usr/sbin/dropbear": file_entry(dropbear, 0o100755),
            "/usr/bin/dropbearkey": {"mode": 0o120777, "uid": 0, "gid": 0, "mtime": TIMESTAMP,
                                     "target": "../sbin/dropbear"},
            "/etc/init.d/S99_zzRootSSH": file_entry(hook, 0o100755)}


def validate_inventory(stock, after, ssh, approved):
    """Accept the stock tree with /root made private, the SSH additions and exactly our three files."""
    require(set(approved) == ARTIFACT_PATHS, "Expected exactly the three cutover artifacts")
    require(ARTIFACT_PATHS & set(stock) == {"/etc/init.d/start_app"}, "Unexpected stock cutover paths")
    require(not set(ssh) & set(stock), "SSH additions collide with stock paths")
    expected = {**stock, "/root": {**stock["/root"], "mode": 0o40700, "uid": 0, "gid": 0}, **ssh, **approved}
    require(set(after) == set(expected), "Image added or removed unexpected paths: "
            + ", ".join(sorted(set(after) ^ set(expected))))
    for path, entry in expected.items():
        require(after[path] == entry, f"Image changed unapproved bytes or metadata: {path}")


def check_expectations(hashes, expect):
    """Refuses unless every expected hash is present and equal."""
    differences = [f"{key}: built {hashes.get(key)}, expected {value}"
                   for key, value in sorted(expect.items()) if hashes.get(key) != value]
    require(not differences, "Build differs from --expect:\n  " + "\n  ".join(differences))


def filesystem(payload):
    # Independent decompressor, not unsquashfs or the recipe's pseudo-file parser.
    from dissect.squashfs import SquashFS

    def walk(node, path="/"):
        entry = {"mode": node.mode, "uid": node.uid, "gid": node.gid,
                 "mtime": int(node.mtime.timestamp())}
        if node.is_file():
            with node.open() as stream:
                data = stream.read()
            entry.update(bytes=len(data), sha256=digest(data))
        elif node.is_symlink():
            entry["target"] = node.link
        elif not node.is_dir():
            raise ValueError(f"Unsupported inode: {path}")
        yield path, entry
        if node.is_dir():
            for child in node.iterdir():
                yield from walk(child, path.rstrip("/") + "/" + child.name)
    return dict(walk(SquashFS(io.BytesIO(payload)).get("/")))


def ubireader():
    beside = Path(sys.executable).parent / "ubireader_extract_images"
    found = str(beside) if beside.exists() else shutil.which("ubireader_extract_images")
    require(found, "ubireader_extract_images not found: install own-your-cameras' requirements.txt")
    return found


def read_key(path):
    key = Path(path).read_text().strip()
    require(len(key.splitlines()) == 1 and key.startswith("ssh-ed25519 "),
            "Expected one dedicated Ed25519 public key")
    return (key + "\n").encode()


def pseudo_base(stock_pseudo, marker):
    """Stock pseudo file with /root private (as build.py does) and the stock start_app left out."""
    header, contents = stock_pseudo.split(marker, 1)
    old = b"root D 1724642682 777 1050 1050\n"
    require(header.count(old) == 1, "Unexpected stock root directory metadata")
    lines = header.replace(old, b"root D 1724642682 700 0 0\n").decode().splitlines(keepends=True)
    removed = [line for line in lines if line.startswith("etc/init.d/start_app ")]
    require(len(removed) == 1, "Expected one stock start_app")
    return "".join(line for line in lines if line not in removed).encode() + marker + contents


def safe_pak(stock_pak, rootfs):
    """A normal PAK carrying only kernel, rootfs and app: bootloader records are omitted, since
    byte-identical bootloaders are still writes if the updater chooses to stage them."""
    header = bytearray(stock_pak[:0x728])
    header[12:12 + 13 * 64] = bytes(13 * 64)
    offset, chunks = 0x728, []
    for index, (name, data) in enumerate(spub.parse_pak(stock_pak)):
        if name not in spub.SAFE_PARTITIONS:
            continue
        data = rootfs if name == "rootfs" else data
        header[12 + index * 64:12 + (index + 1) * 64] = stock_pak[12 + index * 64:12 + (index + 1) * 64]
        struct.pack_into("<II", header, 12 + index * 64 + 56, offset, len(data))
        chunks.append(data)
        offset += len(data)
    body = b"".join(chunks)
    crc = zlib.crc32(body, 0xffffffff)
    crc = zlib.crc32(struct.pack("<I", 2), crc)
    crc = zlib.crc32(header[12:0x34c], crc) ^ 0xffffffff
    struct.pack_into("<I", header, 4, crc)
    pak = bytes(header) + body
    sections = dict(spub.parse_pak(pak))
    stock_sections = dict(spub.parse_pak(stock_pak))
    require(set(sections) == spub.SAFE_PARTITIONS, "Unexpected packaged partitions")
    require(pak[0x34c:0x728] == stock_pak[0x34c:0x728], "Partition table changed")
    require(all(sections[name] == stock_sections[name] for name in ("kernel", "app")), "Kernel/app changed")
    return pak, sections


def build(stock_path, checkout, dropbear_path, key_path, out, expect=None, binaries=None):
    binaries = BINARIES if binaries is None else binaries
    require(set(binaries) == set(BINARIES), "Expected exactly cam and cam-launch binary inputs")
    ubi = own_your_cameras.load(checkout, "ubi")
    recipe = own_your_cameras.load(checkout, "build")
    hook = own_your_cameras.path(checkout, "S99_zzRootSSH").read_bytes()
    stock_pak = Path(stock_path).read_bytes()
    template, stock_provenance = ubi.read_stock_pak(stock_path)  # refuses anything but the 4417 PAK
    dropbear = Path(dropbear_path).read_bytes()
    require(digest(dropbear) in recipe.WORKER_SHA256S,
            "SSH binary differs from own-your-cameras' Cortex-A9 Dropbear build (build_dropbear.py)")
    key = read_key(key_path)

    out.mkdir(parents=True, exist_ok=False)
    stock_sqsh = ubi.parse_ubi(template)["payload"]
    (out / "stock.squashfs").write_bytes(stock_sqsh)
    assembled = start_app.assemble((ROOT / "image/start_app").read_bytes(),
                                   *start_app.stock_blocks(start_app.from_squashfs(out / "stock.squashfs")))
    sources = {"/etc/init.d/start_app": out / "start_app", **binaries}
    (out / "start_app").write_bytes(assembled)
    for binary in binaries.values():
        run(str(ROOT / "scripts/udiv-guard"), str(binary))
    approved = {path: file_entry(source.read_bytes(), 0o100755) for path, source in sources.items()}
    (out / "authorized_keys").write_bytes(key)

    run("unsquashfs", "-pf", str(out / "stock.pseudo"), str(out / "stock.squashfs"))
    (out / "base.pseudo").write_bytes(pseudo_base((out / "stock.pseudo").read_bytes(), recipe.MARKER))
    q = shlex.quote
    (out / "overlay.pseudo").write_text(
        f"root/.ssh D {TIMESTAMP} 700 0 0\n"
        f"root/.ssh/authorized_keys F {TIMESTAMP} 600 0 0 cat {q(str(out / 'authorized_keys'))}\n"
        f"usr/sbin/dropbear F {TIMESTAMP} 755 0 0 cat {q(str(Path(dropbear_path).resolve()))}\n"
        f"usr/bin/dropbearkey S {TIMESTAMP} 777 0 0 ../sbin/dropbear\n"
        f"etc/init.d/S99_zzRootSSH F {TIMESTAMP} 755 0 0 cat "
        f"{q(str(own_your_cameras.path(checkout, 'S99_zzRootSSH')))}\n"
        + "".join(f"{path.lstrip('/')} F {TIMESTAMP} 755 0 0 cat {q(str(source))}\n"
                  for path, source in sources.items()))
    empty = out / "empty"
    empty.mkdir()
    sqsh = out / "rootfs.squashfs"
    log = run("mksquashfs", str(empty), str(sqsh), "-pf", str(out / "base.pseudo"),
              "-pf", str(out / "overlay.pseudo"), "-comp", "xz", "-b", "262144", "-noappend",
              "-processors", "2", "-mem", "256M", "-no-progress", "-mkfs-time", "1733998130",
              "-no-xattrs", "-root-mode", "777", "-root-uid", "1050", "-root-gid", "1050",
              "-root-time", "1724642682")
    (out / "mksquashfs.log").write_bytes(log)
    rootfs, ubi_manifest = ubi.repack(template, sqsh.read_bytes())
    (out / "rootfs.ubi").write_bytes(rootfs)

    run(ubireader(), "-o", str(out / "independent-ubi-extract"), str(out / "rootfs.ubi"))
    images = list((out / "independent-ubi-extract").rglob("*.ubifs"))
    require(len(images) == 1, "Independent UBI extraction did not produce exactly one volume")
    extracted = images[0].read_bytes()
    payload = sqsh.read_bytes()
    require(extracted[:len(payload)] == payload and all(b == 255 for b in extracted[len(payload):]),
            "Independent UBI extraction differs from input")
    require(set(recipe.ADDED) == {"root/.ssh", "root/.ssh/authorized_keys", "usr/sbin/dropbear",
                                  "usr/bin/dropbearkey", "etc/init.d/S99_zzRootSSH"},
            "own-your-cameras SSH additions changed")
    inventory = filesystem(extracted)
    validate_inventory(filesystem(stock_sqsh), inventory, ssh_entries(dropbear, key, hook), approved)
    (out / "inventory.json").write_text(json.dumps(inventory, indent=1, sort_keys=True) + "\n")

    pak, sections = safe_pak(stock_pak, rootfs)
    candidate = out / CANDIDATE_NAME
    candidate.write_bytes(pak)
    restore = spub.build_spub(stock_pak)
    require(digest(restore) == spub.STOCK_SPUB_SHA256, "Stock recovery digest changed")
    spub.check_spub(restore)
    (out / "stock.spub").write_bytes(restore)
    hashes = {**{path: entry["sha256"] for path, entry in approved.items()},
              "candidate": digest(pak), "rootfs.ubi": digest(rootfs), "stock.spub": digest(restore)}
    revision_file = Path(checkout) / "SOURCE_REVISION"
    revision = (revision_file.read_text().strip() if revision_file.is_file()
                else run("git", "-C", str(ROOT), "rev-parse", "HEAD").decode().strip())
    result = {"packaging_commit": revision,
              "hashes": hashes, "artifacts": approved,
              "own_your_cameras": {"commit": own_your_cameras.COMMIT, "sha256": own_your_cameras.SHA256},
              "ssh_binary_sha256": digest(dropbear), "authorized_key_sha256": digest(key),
              "builder_sha256": digest(Path(__file__).read_bytes()),
              "spub_tool_sha256": digest((ROOT / "scripts/spub.py").read_bytes()),
              "stock": stock_provenance, "independent_filesystem_validation": True,
              "rootfs_entries": len(inventory), "inventory_sha256": digest((out / "inventory.json").read_bytes()),
              "ubi": ubi_manifest,
              "candidate": {"path": str(candidate), "sha256": digest(pak), "bytes": len(pak)},
              "sections": {name: digest(data) for name, data in sections.items()},
              "excluded_sections": ["loader", "fdt", "uboot", "para", "sp", "ep"],
              "stock_spub_sha256": digest(restore)}
    (out / "manifest.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(hashes, indent=2))
    if expect is not None:
        check_expectations(hashes, expect)
        print("matches --expect")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stock-pak", type=Path, required=True, help="the stock 4417 PAK")
    parser.add_argument("--own-your-cameras", type=Path, required=True,
                        help="root of this own-your-cameras checkout")
    parser.add_argument("--ssh-binary", type=Path, required=True,
                        help="Dropbear built by own-your-cameras' build_dropbear.py")
    parser.add_argument("--authorized-key", type=Path, required=True, help="your Ed25519 public key")
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--expect", type=Path, help="JSON of expected hashes; refuse on any difference")
    parser.add_argument("--cam", type=Path, help="prebuilt camera binary; requires --launcher")
    parser.add_argument("--launcher", type=Path, help="prebuilt launcher; requires --cam")
    args = parser.parse_args()
    require(bool(args.cam) == bool(args.launcher), "Supply both --cam and --launcher")
    binaries = ({"/usr/bin/cam": args.cam.resolve(), "/usr/bin/cam-launch": args.launcher.resolve()}
                if args.cam else None)
    expect = json.loads(args.expect.read_text()) if args.expect else None
    build(args.stock_pak.resolve(), args.own_your_cameras.resolve(), args.ssh_binary, args.authorized_key,
          args.output_dir.resolve(), expect, binaries)


if __name__ == "__main__":
    try:
        main()
    except ValueError as error:
        raise SystemExit(str(error)) from None
