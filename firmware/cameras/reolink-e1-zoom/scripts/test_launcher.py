"""Expose the real-artifact matrix to ordinary unittest discovery as well as Cargo."""
import os
import json
from pathlib import Path
import runpy
import subprocess


def load_tests(loader, tests, pattern):
    repo = Path(__file__).resolve().parents[1]
    if "CAM_LAUNCH_BIN" not in os.environ:
        subprocess.run(["cargo", "build", "--locked", "--quiet", "-p", "cam", "--bin", "cam-launch"],
                       cwd=repo, check=True)
        metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"], cwd=repo))
        os.environ["CAM_LAUNCH_BIN"] = str(Path(metadata["target_directory"]) / "debug/cam-launch")
    fixture = runpy.run_path(str(repo / "scripts/launcher_tests.py"))["Launcher"]
    return loader.loadTestsFromTestCase(fixture)
