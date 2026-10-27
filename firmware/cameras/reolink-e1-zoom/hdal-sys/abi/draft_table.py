#!/usr/bin/env python3 -I
"""Produce an UNREVIEWED inventory, never a verified id-to-payload-size table.

Header byte counts and instruction-scan candidates are separate columns. A count may
refer to a temporary, another pointer or an unreachable path. Every get/set is emitted,
including missing switches, shared targets and parameterless/unknown declarations.
Legacy hand pins are leads to inspect, not upgraded evidence.

usage: draft_table.py libhdal.asm header_sizes.rs vendor/hdal-include > candidates.tsv
"""
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
import disasm_cases as d  # noqa: E402

UNITS = {
    "videocap": "hd_videocapture.h",
    "videoproc": "hd_videoprocess.h",
    "videoenc": "hd_videoenc.h",
    "audiocap": "hd_audiocapture.h",
    "audioenc": "hd_audioenc.h",
    "audioout": "hd_audioout.h",
}


def enum_rows(path, unit):
    text = Path(path).read_text(encoding="utf-8")
    enum = f"HD_{unit.upper()}_PARAM_ID"
    match = re.search(rf"typedef enum _{enum}\s*\{{(.*?)\}}", text, re.S)
    if not match:
        raise ValueError(f"missing enum {enum} in {path}")
    body = match.group(1)
    rows, i = [], 0
    for line in body.splitlines():
        code = line.split("//", 1)[0].strip()
        if not code or code == f"ENUM_DUMMY4WORD({enum})":
            continue
        m = re.fullmatch(rf"(HD_{unit.upper()}_PARAM_\w+)\s*,", code)
        if not m:
            # This is deliberately not a C evaluator/preprocessor. Never guess ids.
            raise ValueError(f"unsupported enum syntax in {path}: {code}")
        if m.group(1).endswith("_MAX"):
            continue
        using = re.search(r"using (\w+) struct", line)
        rows.append((i, m.group(1), using.group(1) if using else ""))
        i += 1
    if not rows:
        raise ValueError(f"empty enum {enum} in {path}")
    return rows


# Historical (unit, op, id) -> (struct, claimed bytes, note). Re-review before use.
PINS = {
    ("audiocap", "set", 3): ("HD_AUDIOCAP_DEV_CONFIG", 172, "def8/df10 memcpy 172"),
    ("audiocap", "set", 4): ("HD_AUDIOCAP_DRV_CONFIG", 148, "de30/de44 memcpy 148"),
    ("audioenc", "set", 6): ("HD_AUDIOENC_PATH_CONFIG2", 20, "150a4/150ac memcpy 20"),
    ("videocap", "get", 7): ("HD_VIDEOCAP_CROP", 52, "1b5d4 memcpy 52, same case as IN_CROP"),
    ("videocap", "set", 7): ("HD_VIDEOCAP_CROP", 52, "1c154 memcpy 52, same case as IN_CROP"),
    ("videocap", "get", 12): ("HD_VIDEOCAP_IN", 24, "1b930 memcpy 24"),
    ("videoproc", "set", 8): ("HD_VIDEOPROC_CROP", 28, "390d8 reads user bytes 0..27, stores zeros (set is a no-op)"),
    # audioout set is an if-chain on the id, not a jump table
    ("audioout", "set", 3): ("HD_AUDIOOUT_DEV_CONFIG", 24, "1829c ldm/stm 16+8"),
    ("audioout", "set", 4): ("HD_AUDIOOUT_DRV_CONFIG", 56, "18474 ldm/stm 3x16+8"),
    ("audioout", "set", 5): ("HD_AUDIOOUT_OUT", 12, "185a4 ldm/stm 12"),
    ("audioout", "set", 6): ("HD_AUDIOOUT_IN", 4, "181f8 one word"),
    ("audioout", "set", 7): ("HD_AUDIOOUT_VOLUME", 4, "18418 one word"),
    # the user buffer is codec_type + union{h26x, jpeg}: 2.00 copies 8 or 40 bytes to offset 4
    ("videoenc", "get", 5): ("HD_VIDEOENC_OUT", 44, "2ce38 memset 44, then codec + memcpy(p+4, 8|40)"),
    ("videoenc", "set", 5): ("HD_VIDEOENC_OUT", 44, "2df18 memset 44"),
    ("videoenc", "get", 27): ("HD_VIDEOENC_OUT2", 44, "2c638 memcpy 44"),
    ("videoenc", "set", 27): ("HD_VIDEOENC_OUT2", 44, "2d97c memcpy 44"),
    # rate control: the 80-byte memset is a temp; the user struct is rc_mode + union{28,32,12,60}
    ("videoenc", "get", 8): ("HD_H26XENC_RATE_CONTROL", 64, "2ccb0: copies rc_mode then 28|32|12|60 into the union (2d2e4..2d2a8)"),
}


def main():
    funcs = d.load(sys.argv[1])
    sizes = dict(re.findall(r'\("(\w+)", (\d+)\)', Path(sys.argv[2]).read_text(encoding="utf-8")))
    if not sizes:
        raise ValueError("empty or unrecognized header size table")
    enums_by_unit = {unit: enum_rows(f"{sys.argv[3]}/{header}", unit) for unit, header in UNITS.items()}
    output = ["# unit\top\tid\tname\tstruct\theader_bytes\tstatus\tdispatch_candidates\tcopy_candidates\tlegacy_pin"]
    for unit, enums in enums_by_unit.items():
        seen = {}
        for op in ("get", "set"):
            insns = funcs[f"hd_{unit}_{op}"]
            by_addr = {a: i for i, (a, _, _) in enumerate(insns)}
            tabs = list(d.tables(insns))
            stop = {t for _, _, _, _, ts in tabs for t in ts}
            for addr, reg, bias, _, targets in tabs:
                for idx, t in enumerate(targets):
                    counts = d.case_sizes(insns, by_addr, t, stop - {t})
                    # These are index guesses, not a proof the switch is on the API's id.
                    seen.setdefault((op, idx + bias), []).append((addr, reg, t, counts))
        for i, name, using in enums:
            short = name.replace(f"HD_{unit.upper()}_PARAM_", "")
            for op in ("get", "set"):
                cases = seen.get((op, i), [])
                where = ";".join(f"{a:x}:{reg}->{t:x}" for a, reg, t, _ in cases)
                counts = ";".join(
                    f"{t:x}:{kind}={n}" for _, _, t, cs in cases for kind, n in cs
                )
                pin = PINS.get((unit, op, i))
                note = f"{pin[0]}:{pin[1]}:{pin[2]}" if pin else ""
                output.append("\t".join([unit, op, str(i), short, using, sizes.get(using, "?"),
                                         "unreviewed", where, counts, note]))
    # Do not publish a plausible prefix if a later unit/input cannot be parsed.
    print("\n".join(output))


if __name__ == "__main__":
    main()
