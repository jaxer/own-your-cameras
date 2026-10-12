#!/usr/bin/env python3
"""Compatibility entry point for the consolidated G3/G5 decoder."""

import runpy
from pathlib import Path

if __name__ == "__main__":
    runpy.run_path(
        str(Path(__file__).resolve().parent.parent / "camera-firmware" / "decode.py"),
        run_name="__main__",
    )
