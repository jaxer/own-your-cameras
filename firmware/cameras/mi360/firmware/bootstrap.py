"""Compose the public SSH bootstrap from owner inputs and qualified stock components."""
from __future__ import annotations
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
PUBLIC_ROOT = HERE.parents[3]
TOOLS = PUBLIC_ROOT / "tools/mi-360"


def load_tools():
    # The bootstrap builder imports its adjacent format module by its original name.
    # Bind that name only while loading; do not overwrite another tool's module.
    modules = []
    previous = sys.modules.get("firmware")
    try:
        for name, filename in (("firmware", "firmware.py"), ("mi360_ssh_builder", "build.py")):
            spec = importlib.util.spec_from_file_location(name, TOOLS / filename)
            module = importlib.util.module_from_spec(spec)
            if name == "firmware":
                sys.modules[name] = module
            spec.loader.exec_module(module)
            modules.append(module)
    finally:
        if previous is None:
            sys.modules.pop("firmware", None)
        else:
            sys.modules["firmware"] = previous
    return tuple(modules)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def check_original(data, expected):
    formats, _ = load_tools()
    formats.check_dump(data, expected)
    lock = json.loads((HERE / "stock.lock.json").read_text())
    if (digest(data[0x50000:0x2d0000]) != lock["rootfs_sha256"]
            or digest(data[0x8f0000:0xaa0000]) != lock["b_kernel_sha256"]):
        raise ValueError("stock ROOTFS/kernel differs from the qualified firmware")
    return lock


def prepare(args, output):
    original = args.original_dump.read_bytes()
    lock = check_original(original, args.expected_original_sha256)
    _, builder = load_tools()
    # Reuse the same public recipe and preserve the owner's exact key/factory data.
    # It performs its own static-ELF, filesystem and protected-byte checks.
    report = builder.build(args.original_dump, args.expected_original_sha256,
                           args.ssh_binary, args.bootstrap_writer, args.authorized_key,
                           output / "bootstrap")
    source = output / "bootstrap/stock-usrfs.bin"
    for name, expected in lock["files"].items():
        result = subprocess.run(["unsquashfs", "-cat", str(source), name],
                                check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        if digest(result.stdout) != expected:
            raise ValueError("stock SDK/module/tuning differs: " + name)
    base = (output / "bootstrap/camera-ssh-16MiB.bin").read_bytes()
    if digest(base) != report["image_sha256"]:
        raise ValueError("bootstrap output changed after verification")
    return base, report
