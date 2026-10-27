"""Assemble /etc/init.d/start_app: our wrapper (image/start_app) around the two blocks of the stock 4417 one.

The stock blocks are Reolink's and never live in this repository; the image builder takes them from the
reader's own stock PAK and refuses anything but the exact 4417 bytes.
"""
import hashlib
import subprocess
import tempfile
from pathlib import Path

import own_your_cameras

INIT_SHA256 = "b54689ec4b6809eab6cc156c10f18f806c1abd6a1f91f81c5fdc67cb3707a787"
VENDOR_SHA256 = "c178f01a93f2b2c5428cbf8ba1b3e5b111e5ced16ce79825c2850f477a8a2166"
# The stock file is the init block followed by the vendor block, which starts by launching the router.
VENDOR_START = b"\n./router &\n"


def stock_blocks(stock):
    """Splits the stock start_app into (init, vendor); refuses anything but the 4417 file."""
    if stock.count(VENDOR_START) != 1:
        raise ValueError("stock start_app has no single vendor boundary: only the 4417 firmware is supported")
    cut = stock.index(VENDOR_START) + 1
    init, vendor = stock[:cut], stock[cut:]
    for name, block, expected in (("init", init, INIT_SHA256), ("vendor", vendor, VENDOR_SHA256)):
        actual = hashlib.sha256(block).hexdigest()
        if actual != expected:
            raise ValueError(f"stock start_app {name} block has SHA-256 {actual}, expected {expected}: "
                             "only the 4417 firmware is supported")
    return init, vendor


def assemble(template, init, vendor):
    for placeholder in (b"\n@STOCK_INIT@\n", b"\n@STOCK_VENDOR@\n"):
        if template.count(placeholder) != 1:
            raise ValueError(f"template needs exactly one {placeholder.strip().decode()} placeholder line")
    return (template.replace(b"\n@STOCK_INIT@\n", b"\n" + init)
            .replace(b"\n@STOCK_VENDOR@\n", b"\n" + vendor))


def stock_start_app(stock_pak, checkout):
    """Reads /etc/init.d/start_app out of the stock PAK's rootfs."""
    ubi = own_your_cameras.load(checkout, "ubi")
    template, _ = ubi.read_stock_pak(stock_pak)
    with tempfile.TemporaryDirectory() as temp:
        squashfs = Path(temp) / "rootfs.squashfs"
        squashfs.write_bytes(ubi.parse_ubi(template)["payload"])
        return from_squashfs(squashfs)


def from_squashfs(squashfs):
    return subprocess.run(["unsquashfs", "-cat", str(squashfs), "etc/init.d/start_app"],
                          check=True, capture_output=True).stdout
