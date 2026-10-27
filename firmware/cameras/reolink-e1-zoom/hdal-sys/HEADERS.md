# HDAL headers

The Novatek HDAL headers are not in this repository. `make headers` (`scripts/fetch-hdal-headers`) downloads them
from the one public source below, checks each file against `hdal-include.lock`, applies `hdal-include.patch`, checks
the result against the lock again and moves the set into `vendor/hdal-include/` (git-ignored). The lock is the only
place their hashes are kept.

| | |
|---|---|
| Repository | `yanggungg/trunk_gtk4_dev` on GitHub |
| Commit | `69386cae2e39480b3969165bac3bd4a86425335a` (other public HDAL copies are different versions) |
| Path | `src/include/novatek.nt9833x/` (16 headers) plus `vendor/vendor_common.h`, `vendor/vendor_videoout.h` |
| Version | `HDAL_VERSION 0x302000` (HDAL 3.02.000, NT9833x); the camera runs 2.00.000 |
| License | The repository has no license file; the headers keep Novatek's copyright and all-rights-reserved notices. They are fetched for interoperability with the camera's own `libhdal.so` and never redistributed. |
| Patch | `hdal-include.patch` changes `hd_videoenc.h` (`HD_VIDEOENC_OUT` and `HD_VIDEOENC_OUT2` get the union the 2.00 library uses), `hd_type.h` and `vendor/vendor_common.h` (four SDK defects). It modifies Novatek text and is provided for interoperability only. |

Why each change is needed, and how the bindings are checked against the 2.00 library:
[../docs/hdal-abi.md](../docs/hdal-abi.md).

The vendor file names are the long forms: `hd_videocapture.h`, `hd_videoprocess.h`, `hd_audiocapture.h`.
