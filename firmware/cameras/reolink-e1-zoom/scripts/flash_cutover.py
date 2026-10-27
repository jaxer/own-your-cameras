#!/usr/bin/env python3
"""Flash a cutover image built by cutover_image.py through own-your-cameras' upload.py. Read-only by default.

Without --execute-approved-flash it only checks the image and prints the plan; nothing contacts the camera.
With it, it logs in to the camera's stock web API (the vendor stack must be running, i.e. cam.enabled absent),
uploads once with configuration retained, and never retries. The camera installs and reboots by itself.
"""
import argparse
import getpass
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cutover_image as image  # noqa: E402
import own_your_cameras  # noqa: E402


def prepare(checkout, directory, expect=None):
    manifest = json.loads((directory / "manifest.json").read_text())
    image.require(manifest["independent_filesystem_validation"] is True,
                  "Missing independent filesystem verification")
    image.require(set(manifest["artifacts"]) == image.ARTIFACT_PATHS, "Wrong artifact set")
    for entry in manifest["artifacts"].values():
        image.require(entry["mode"] == 0o100755 and entry["uid"] == entry["gid"] == 0, "Unexpected artifact mode")
    expected = manifest["hashes"]["candidate"]
    if expect is not None:
        image.check_expectations(manifest["hashes"], expect)
    path = Path(manifest["candidate"]["path"])
    image.require(path.resolve().parent == directory, "Candidate is outside the selected image directory")
    upload = own_your_cameras.load(checkout, "upload")
    try:
        data = upload.checked_image(path, expected)  # exact hash, PAK magic, sections, CRC, basename
    except ValueError as error:
        raise ValueError(f"{path.name}: {error}; the image differs from the one its manifest records") from None
    plan = {"upload_started": False, "candidate": str(path), "candidate_sha256": image.digest(data),
            "bytes": len(data), "chunks": -(-len(data) // upload.CHUNK_BYTES),
            "uploader": {"own_your_cameras": own_your_cameras.COMMIT,
                         "upload_sha256": own_your_cameras.SHA256["upload.py"]},
            "adapter_sha256": image.digest(Path(__file__).read_bytes()),
            "scope": "One normal UpgradePrepare/Upgrade upload; the camera installs and reboots",
            "restoreCfg": 0, "clearConfig": 0, "no_write_retry": True,
            "excluded_sections": manifest["excluded_sections"], "expected_device": upload.EXPECTED}
    return upload, path, data, plan


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--own-your-cameras", type=Path, required=True,
                        help="root of this own-your-cameras checkout")
    parser.add_argument("--image-dir", type=Path, required=True)
    parser.add_argument("--expect", type=Path, help="JSON of expected hashes; refuse on any difference")
    parser.add_argument("--execute-approved-flash", action="store_true")
    parser.add_argument("--host", help="camera hostname or address; required to flash")
    parser.add_argument("--username", default="admin")
    parser.add_argument("--insecure", action="store_true", help="accept the camera's self-signed certificate")
    parser.add_argument("--operation-dir", type=Path, help="new directory for the flash record; required to flash")
    args = parser.parse_args()
    expect = json.loads(args.expect.read_text()) if args.expect else None
    upload, path, data, plan = prepare(args.own_your_cameras.resolve(), args.image_dir.resolve(), expect)
    print(json.dumps(plan, indent=2), flush=True)
    if not args.execute_approved_flash:
        return
    image.require(bool(args.host), "--host is required to flash")
    image.require(args.operation_dir is not None, "Choose a new --operation-dir")
    operation = args.operation_dir.resolve()
    operation.mkdir(parents=True, exist_ok=False)
    (operation / "plan.json").write_text(json.dumps({**plan, "host": args.host}, indent=2) + "\n")
    password = getpass.getpass("Camera administrator password: ")
    session = upload.Session(args.host, args.username, password, args.insecure)
    (operation / "upload-started").touch()
    upload.upload(session, path, data)
    (operation / "upload-acknowledged").touch()


if __name__ == "__main__":
    try:
        main()
    except (ValueError, RuntimeError) as error:
        raise SystemExit(str(error)) from None
