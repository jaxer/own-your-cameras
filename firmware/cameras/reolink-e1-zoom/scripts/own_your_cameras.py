"""The public own-your-cameras E1 Zoom tools this repo builds and flashes images with, pinned by hash.

The caller supplies this repository root. The shared tools must match the hashes below.
COMMIT records the upstream revision where those unchanged tool bytes were qualified;
it is provenance, not a requirement to use an older checkout.
"""
import hashlib
import importlib.util
from pathlib import Path
import sys

COMMIT = "b06aad8d5fa80aa19dd07e00c9bd65513a4879f8"
TOOLS = Path("tools/reolink-e1-zoom")
SHA256 = {
    "ubi.py": "e0fa51572a902c68851dac13177429f1b77212e6879b6b1ad9af4cee773ca95e",
    "build.py": "155e9b732ff4abc77cdafc939348021ad7c39796b73e557c55272f4fc8c774d2",
    "upload.py": "7d0f13dab24b1eb7d772e26cda5f7bd1b8e470c0207ec7e691a6a0dbc65fd81d",
    "S99_zzRootSSH": "1f458ad049d865cfc7dd1a13be81e3410b73ac531dc5c26bc70e125656704e90",
}
# Import order: build.py imports ubi, upload.py imports both, by their plain module names.
MODULES = ("ubi", "build", "upload")


def path(checkout, name):
    """The verified path of one tool file in the reader's checkout."""
    file = Path(checkout) / TOOLS / name
    if not file.is_file():
        raise ValueError(f"{file} not found: pass the root of this own-your-cameras checkout")
    actual = hashlib.sha256(file.read_bytes()).hexdigest()
    if actual != SHA256[name]:
        raise ValueError(f"{file} has SHA-256 {actual}, expected {SHA256[name]}: "
                         "review the shared tool change and requalify its hash before building")
    return file


def load(checkout, name):
    """Imports one pinned tool module (and the ones it imports) from the reader's checkout."""
    for module in MODULES[:MODULES.index(name) + 1]:
        if module not in sys.modules:
            spec = importlib.util.spec_from_file_location(module, path(checkout, module + ".py"))
            sys.modules[module] = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(sys.modules[module])
    return sys.modules[name]
