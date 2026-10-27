"""Host tests for spub.py: synthetic PAK and no camera; the stock PAK only when STOCK_PAK names it."""
import hashlib
import os
import struct
import subprocess
import sys
import tempfile
import unittest
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import spub  # noqa: E402

BLOCK = 0x20000
STOCK_PAK_SHA256 = "ae1227a89eae4d801381490c95b3fbc38e5e0040d7c839d90f1993ec0b771d5c"


def raw_crc(data):
    """CRC-32 (poly 0xEDB88320), register starts at 0, no final xor: vendor bc_gen_crc(0, ...)."""
    return zlib.crc32(data, 0xFFFFFFFF) ^ 0xFFFFFFFF


def make_pak(sections, magic=0x32725913):
    """Build a PAK the way research/inspect_stock_pak.py parses it."""
    body = b"".join(data for _, data in sections)
    records = b""
    offset = 0x728
    for name, data in sections:
        records += struct.pack("<32s24sII", name.encode(), b"v1.0.0.1", offset, len(data))
        offset += len(data)
    records = records.ljust(13 * 64, b"\0")
    crc = zlib.crc32(body, 0xFFFFFFFF)
    crc = zlib.crc32(struct.pack("<I", 2), crc)
    crc = zlib.crc32(records, crc) ^ 0xFFFFFFFF
    header = struct.pack("<III", magic, crc, 0x4102) + records
    return header.ljust(0x728, b"\0") + body


SECTIONS = [
    ("loader", b"L" * 1000),
    ("kernel", b"K" * (BLOCK + 5)),
    ("rootfs", b"R" * (2 * BLOCK)),
]


def mutate_spub(image, changes):
    """Change wire fields, then repair the CRC so layout policy is exercised."""
    image = bytearray(image)
    for offset, value in changes:
        image[offset:offset + len(value)] = value
    count = struct.unpack_from("<I", image, 12)[0]
    struct.pack_into("<I", image, 4, raw_crc(image[8:24 + count * 60]))
    return bytes(image)


def mutate_pak(pak, changes):
    """Repair the PAK CRC after mutating its public wire records."""
    pak = bytearray(pak)
    for offset, value in changes:
        pak[offset:offset + len(value)] = value
    crc = zlib.crc32(pak[0x728:], 0xFFFFFFFF)
    crc = zlib.crc32(struct.pack("<I", 2), crc)
    crc = zlib.crc32(pak[12:12 + 13 * 64], crc) ^ 0xFFFFFFFF
    struct.pack_into("<I", pak, 4, crc)
    return bytes(pak)


class PakTest(unittest.TestCase):
    def test_rejects_malformed_records_even_when_skipped(self):
        pak = make_pak([("kernel", b"K"), ("app", b"A")])
        u32 = lambda value: struct.pack("<I", value)
        malformed = {
            "duplicate name": [(76, b"kernel".ljust(32, b"\0"))],
            "non-ascii name": [(12, b"\xff" + b"\0" * 31)],
            "unterminated name": [(12, b"k" * 32)],
            "hidden name suffix": [(12, b"kernel\0app".ljust(32, b"\0"))],
            "unnamed payload": [(76, b"\0" * 32)],
            "empty slot with version": [(140 + 32, b"x")],
            "empty slot with invalid offset": [(140 + 56, u32(1))],
            "overlapping body": [(76 + 56, u32(0x728))],
            "body gap": [(76 + 56, u32(0x72a))],
            "outside body": [(76 + 60, u32(2))],
            "unsupported format": [(8, u32(0))],
        }
        cases = [mutate_pak(pak, changes) for changes in malformed.values()]
        cases += [make_pak([]), make_pak([("kernel", b"")]),
                  make_pak([("loader", b""), ("kernel", b"K")]),
                  make_pak([("kernel", b"K" * (0x400000 + 1))]),
                  make_pak([("loader", b"L" * (0x40000 + 1)), ("kernel", b"K")])]
        reasons = list(malformed) + ["empty PAK", "empty payload", "empty boot payload",
                                     "oversize payload", "oversize boot payload"]
        for reason, bad in zip(reasons, cases):
            with self.subTest(reason=reason):
                with self.assertRaises(spub.SpubError):
                    spub.parse_pak(bad)
                with self.assertRaises(spub.SpubError):
                    spub.build_spub(bad, skip=("kernel",))


class DestinationTest(unittest.TestCase):
    def test_rejects_valid_crc_destinations_outside_the_safe_table(self):
        image = spub.build_spub(make_pak([("kernel", b"K")]))
        for field, value in [(44, 0), (44, 0x580000), (48, 0x40001),
                             (32, 0), (32, 0x400001)]:
            with self.subTest(field=field, value=value):
                bad = mutate_spub(image, [(24 + field, struct.pack("<I", value))])
                with self.assertRaises(spub.SpubError):
                    spub.check_spub(bad)
        for name in ["loader", "fdt", "uboot", "para", "sp", "ep", "ext_para", "unknown"]:
            with self.subTest(name=name):
                bad = mutate_spub(image, [(24, name.encode().ljust(32, b"\0"))])
                with self.assertRaises(spub.SpubError):
                    spub.check_spub(bad)


class BuildTest(unittest.TestCase):
    def setUp(self):
        self.pak = make_pak(SECTIONS)
        self.spub = spub.build_spub(self.pak)

    def test_header_fields(self):
        magic, crc, update, num, reserved, ext_len = struct.unpack_from("<6I", self.spub)
        self.assertEqual(magic, 0x42555053)
        self.assertEqual((update, num, reserved, ext_len), (1, 2, 0, 0))
        self.assertEqual(crc, raw_crc(self.spub[8:24 + 2 * 60]))

    def test_entries_use_partition_table_and_block_aligned_staging(self):
        entries = [struct.unpack_from("<32s7I", self.spub, 24 + i * 60) for i in range(2)]
        base = spub.DOWNLOAD_START + 8 * BLOCK
        expected = [
            (b"kernel", BLOCK + 5, base, raw_crc(b"K" * (BLOCK + 5)), 0x180000, 0x400000),
            (b"rootfs", 2 * BLOCK, base + 2 * BLOCK, raw_crc(b"R" * 2 * BLOCK), 0x580000, 0xF00000),
        ]
        for entry, want in zip(entries, expected):
            name, size, start, crc, addr, space, ext_type, ext_len = entry
            self.assertEqual((name.rstrip(b"\0"), size, start, crc, addr, space),
                             want)
            self.assertEqual((ext_type, ext_len), (0, 0))

    def test_image_layout_is_staged_in_whole_blocks(self):
        self.assertEqual(len(self.spub), 12 * BLOCK)
        self.assertEqual(self.spub[24 + 2 * 60:8 * BLOCK], b"\xff" * (8 * BLOCK - 24 - 2 * 60))
        self.assertEqual(self.spub[8 * BLOCK:9 * BLOCK + 5], b"K" * (BLOCK + 5))
        self.assertEqual(self.spub[9 * BLOCK + 5:10 * BLOCK], b"\xff" * (BLOCK - 5))

    def test_skip_drops_sections_but_keeps_offsets_dense(self):
        out = spub.build_spub(self.pak, skip=("kernel",))
        report = spub.check_spub(out)
        self.assertEqual([e.name for e in report.entries], ["rootfs"])
        self.assertEqual(report.entries[0].start, spub.DOWNLOAD_START + 8 * BLOCK)

    def test_build_is_deterministic(self):
        self.assertEqual(self.spub, spub.build_spub(self.pak))

    def test_rejects_corrupt_pak(self):
        bad = bytearray(self.pak)
        bad[-1] ^= 1
        with self.assertRaises(spub.SpubError):
            spub.build_spub(bytes(bad))

    def test_rejects_unknown_partition(self):
        with self.assertRaises(spub.SpubError):
            spub.build_spub(make_pak([("mystery", b"x")]))

    def test_rejects_total_download_overflow_even_when_payloads_fit_partitions(self):
        pak = make_pak([("kernel", b"K" * 0x400000), ("rootfs", b"R" * 0xF00000),
                        ("app", b"A" * 0xB00000)])
        with self.assertRaisesRegex(spub.SpubError, "download"):
            spub.build_spub(pak)

    def test_build_selection_never_stages_boot_or_settings(self):
        pak = make_pak([(n, b"x") for n in ["loader", "fdt", "uboot", "kernel", "rootfs", "app"]])
        self.assertEqual([e.name for e in spub.check_spub(spub.build_spub(pak)).entries],
                         ["kernel", "rootfs", "app"])
        for name in ["para", "sp", "ep", "ext_para", "unknown"]:
            with self.subTest(name=name):
                pak = make_pak([("kernel", b"K"), (name, b"x")])
                for skip in [(), (name,)]:
                    with self.assertRaises(spub.SpubError):
                        spub.build_spub(pak, skip=skip)
        for sections, skip in [([], ()), ([("loader", b"x")], ()),
                               ([("kernel", b"K")], ("kernel",)),
                               ([("kernel", b"K")], ("kernle",))]:
            with self.subTest(sections=sections, skip=skip):
                with self.assertRaises(spub.SpubError):
                    spub.build_spub(make_pak(sections), skip=skip)


class CheckTest(unittest.TestCase):
    def setUp(self):
        self.spub = spub.build_spub(make_pak(SECTIONS))

    def test_accepts_good_container(self):
        report = spub.check_spub(self.spub)
        self.assertEqual(len(report.entries), 2)

    def test_rejects_valid_crc_unsupported_or_ambiguous_descriptors(self):
        image = spub.build_spub(make_pak([("kernel", b"K"), ("rootfs", b"K")]))
        u32 = lambda value: struct.pack("<I", value)
        mutations = {
            "update disabled": [(8, u32(0))],
            "unknown update": [(8, u32(2))],
            "reserved": [(16, u32(1))],
            "header extension": [(20, u32(1))],
            "entry extension type": [(24 + 52, u32(1))],
            "entry extension length": [(24 + 56, u32(1))],
            "duplicate destination": [(84, image[24:56]), (84 + 44, image[24 + 44:24 + 52])],
            "overlapping sources": [(84 + 36, image[24 + 36:24 + 40])],
            "empty payload": [(24 + 32, u32(0)), (24 + 40, u32(0))],
            "empty name": [(24, b"\0" * 32)],
            "non-ascii name": [(24, b"\xff" + b"\0" * 31)],
            "unterminated name": [(24, b"k" * 32)],
            "hidden name suffix": [(24, b"kernel\0app".ljust(32, b"\0"))],
            "source in reserved area": [(24 + 36, u32(spub.DOWNLOAD_START + BLOCK)),
                                         (24 + 40, u32(raw_crc(b"\xff")))],
            "source in descriptor": [(24 + 36, u32(spub.DOWNLOAD_START))],
            "unaligned source": [(24 + 36, u32(spub.DOWNLOAD_START + 8 * BLOCK + 1))],
            "source outside download": [(24 + 36, u32(spub.DOWNLOAD_START + spub.DOWNLOAD_SIZE))],
            "empty descriptor": [(12, u32(0))],
            "excessive entry count": [(12, u32(0xffffffff))],
        }
        for reason, changes in mutations.items():
            with self.subTest(reason=reason):
                with self.assertRaises(spub.SpubError):
                    spub.check_spub(mutate_spub(image, changes))

    def test_rejects_oversized_or_incomplete_staging_image(self):
        image = spub.build_spub(make_pak([("kernel", b"K")]))
        for bad in [image.ljust(spub.DOWNLOAD_SIZE + BLOCK, b"\xff"), image[:-1]]:
            with self.subTest(size=len(bad)):
                with self.assertRaises(spub.SpubError):
                    spub.check_spub(bad)

    def test_rejects_flipped_data_byte(self):
        bad = bytearray(self.spub)
        bad[8 * BLOCK + 3] ^= 0xFF
        with self.assertRaisesRegex(spub.SpubError, "kernel"):
            spub.check_spub(bytes(bad))

    def test_rejects_flipped_descriptor_byte(self):
        bad = bytearray(self.spub)
        bad[30] ^= 0x01
        with self.assertRaisesRegex(spub.SpubError, "header CRC"):
            spub.check_spub(bytes(bad))

    def test_rejects_bad_magic_and_truncation(self):
        with self.assertRaisesRegex(spub.SpubError, "magic"):
            spub.check_spub(b"\xff" * 4096)
        with self.assertRaisesRegex(spub.SpubError, "truncated"):
            spub.check_spub(self.spub[:-BLOCK])
        for end in [0, 23, 24, 83, 143]:
            with self.subTest(end=end):
                with self.assertRaisesRegex(spub.SpubError, "truncated"):
                    spub.check_spub(self.spub[:end])


class CliTest(unittest.TestCase):
    def run_cli(self, *args):
        return subprocess.run([sys.executable, str(Path(spub.__file__)), *map(str, args)],
                              capture_output=True, text=True, check=False)

    def test_cli_build_check_and_rejections_leave_output_untouched(self):
        with tempfile.TemporaryDirectory() as directory:
            pak, output = Path(directory) / "input.pak", Path(directory) / "output.spub"
            pak.write_bytes(make_pak(SECTIONS))
            built = self.run_cli("build", pak, "-o", output)
            self.assertEqual(built.returncode, 0, built.stderr)
            original = output.read_bytes()
            checked = self.run_cli("--check", output)
            self.assertEqual(checked.returncode, 0, checked.stderr)
            self.assertIn("kernel", checked.stdout)
            self.assertIn("rootfs", checked.stdout)
            self.assertNotIn("loader", checked.stdout)
            mixed = self.run_cli("--check", output, "build", pak, "-o", output)
            self.assertNotEqual(mixed.returncode, 0)
            redirected = mutate_spub(original, [(24 + 44, struct.pack("<I", 0))])
            redirected_path = Path(directory) / "redirected.spub"
            redirected_path.write_bytes(redirected)
            rejected = self.run_cli("--check", redirected_path)
            self.assertEqual(rejected.returncode, 1)
            self.assertNotIn("OK", rejected.stdout)
            self.assertNotIn("Traceback", rejected.stderr)
            for skip in ["kernle", "kernel,", ",kernel", "kernel,,rootfs", "kernel,rootfs", "sp"]:
                with self.subTest(skip=skip):
                    result = self.run_cli("build", pak, "-o", output, "--skip", skip)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertNotIn("Traceback", result.stderr)
                    self.assertEqual(output.read_bytes(), original)

    def test_cli_malformed_inputs_and_io_errors_are_controlled(self):
        with tempfile.TemporaryDirectory() as directory:
            pak, image = Path(directory) / "input.pak", Path(directory) / "input.spub"
            output = Path(directory) / "output.spub"
            pak.write_bytes(mutate_pak(make_pak(SECTIONS), [(12, b"\xff")]))
            image.write_bytes(mutate_spub(spub.build_spub(make_pak([("kernel", b"K")])),
                                         [(24, b"\xff")]))
            for args in [("build", pak, "-o", output), ("--check", image),
                         ("--check", Path(directory) / "missing"),
                         ("build", Path(directory) / "missing", "-o", output),
                         ("--check", image, "build", pak, "-o", output)]:
                with self.subTest(args=args):
                    result = self.run_cli(*args)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertNotIn("Traceback", result.stderr)
                    self.assertFalse(output.exists())
            pak.write_bytes(make_pak(SECTIONS))
            result = self.run_cli("build", pak, "-o", directory)
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn("Traceback", result.stderr)


@unittest.skipUnless(os.environ.get("STOCK_PAK"), "set STOCK_PAK to the stock 4417 PAK")
class StockPakTest(unittest.TestCase):
    def test_stock_container_matches_recorded_digest(self):
        pak = Path(os.environ["STOCK_PAK"]).read_bytes()
        self.assertEqual(hashlib.sha256(pak).hexdigest(), STOCK_PAK_SHA256)
        out = spub.build_spub(pak)
        self.assertEqual(hashlib.sha256(out).hexdigest(), spub.STOCK_SPUB_SHA256)
        self.assertEqual(len(out), 26214400)
        self.assertEqual([e.name for e in spub.check_spub(out).entries], ["kernel", "rootfs", "app"])
        nobl = spub.build_spub(pak, skip=("loader", "fdt", "uboot"))
        self.assertEqual(out, nobl)
        self.assertEqual(out, spub.build_spub(pak))


if __name__ == "__main__":
    unittest.main()
