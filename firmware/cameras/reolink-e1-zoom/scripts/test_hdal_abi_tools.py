"""Public CLI regressions. Fixtures are isolated parser inputs, never camera data."""

import csv
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
ABI = ROOT / "hdal-sys/abi"
HEADERS = {
    "videocap": "hd_videocapture.h",
    "videoproc": "hd_videoprocess.h",
    "videoenc": "hd_videoenc.h",
    "audiocap": "hd_audiocapture.h",
    "audioenc": "hd_audioenc.h",
    "audioout": "hd_audioout.h",
}


class AbiTools(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.asm = self.root / "input.asm"
        self.sizes = self.root / "header_sizes.rs"
        self.sizes.write_text('pub const HEADER_SIZES: &[(&str, usize)] = &[("HD_SAMPLE", 4)];\n')
        asm = []
        for unit, header in HEADERS.items():
            (self.root / header).write_text(
                f"typedef enum _HD_{unit.upper()}_PARAM_ID {{\n"
                f"HD_{unit.upper()}_PARAM_FIRST, ///< using HD_SAMPLE struct\n"
                f"HD_{unit.upper()}_PARAM_MAX,\n"
                f"ENUM_DUMMY4WORD(HD_{unit.upper()}_PARAM_ID)\n}} ID;\n"
            )
            for op in ("get", "set"):
                # A case with no user access. A four-byte header cannot prove a copy.
                asm.append(f"00000100 <hd_{unit}_{op}@@Base>:\n" + self.switch())
        self.asm.write_text("".join(asm))

    @staticmethod
    def switch():
        return (
            " 100:\te3510000 \tcmp\tr1, #0\n"
            " 104:\t908ff101 \taddls\tpc, pc, r1, lsl #2\n"
            " 108:\tea000000 \tb\t120 <error>\n"
            " 10c:\tea000000 \tb\t120 <error>\n"
            " 120:\te3e00021 \tmvn\tr0, #33\n"
            " 124:\te12fff1e \tbx\tlr\n"
        )

    def draft(self):
        return subprocess.run(
            [sys.executable, "-I", str(ABI / "draft_table.py"), str(self.asm),
             str(self.sizes), str(self.root)], capture_output=True, text=True, check=False,
        )

    def rows(self):
        result = self.draft()
        self.assertEqual(result.returncode, 0, result.stderr)
        return list(csv.reader(io.StringIO(result.stdout), delimiter="\t"))[1:]

    def test_header_size_is_not_binary_evidence(self):
        row = next(r for r in self.rows() if r[:3] == ["videocap", "set", "0"])
        self.assertEqual(row[6], "unreviewed", "no copy was observed; a header is not binary evidence")

    def test_unknown_enum_syntax_cannot_silently_renumber_ids(self):
        header = self.root / HEADERS["videocap"]
        original = header.read_text()
        for entry in ("HD_VIDEOCAP_PARAM_FIRST = 7,", "#if FEATURE\nHD_VIDEOCAP_PARAM_FIRST,"):
            with self.subTest(entry=entry):
                header.write_text(original.replace("HD_VIDEOCAP_PARAM_FIRST,", entry))
                result = self.draft()
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, "", "invalid input must not leave a plausible partial table")
                self.assertIn("unsupported enum", result.stderr)

    def test_copy_count_does_not_survive_clobbers_or_conditional_assignments(self):
        cases = [
            " 120:\te3a0202c \tmov\tr2, #44\n 124:\teb000000 \tbl\t900 <helper>\n",
            " 120:\te3a0202c \tmov\tr2, #44\n 124:\te5962004 \tldr\tr2, [r6, #4]\n",
            " 120:\t13a0202c \tmovne\tr2, #44\n",
        ]
        for body in cases:
            with self.subTest(body=body):
                self.asm.write_text("00000100 <sample>:\n" + self.switch().split(" 120:")[0] + body
                                    + " 128:\teb000000 \tbl\t800 <memcpy@plt>\n"
                                    + " 12c:\te12fff1e \tbx\tlr\n")
                result = subprocess.run([sys.executable, "-I", str(ABI / "disasm_cases.py"),
                                         str(self.asm), "sample"], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertNotIn("('memcpy', 44)", result.stdout)

    def test_show_preserves_the_complete_instruction_evidence(self):
        line = " 120:\teb000000 \tbl\t800 <" + "long_symbol_" * 12 + "> @ evidence\n"
        self.asm.write_text(line + " 124:\te12fff1e \tbx\tlr\n")
        result = subprocess.run([sys.executable, "-I", str(ABI / "show.py"),
                                 str(self.asm), "120", "124"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, line)

    def test_incomplete_inputs_do_not_publish_a_partial_inventory(self):
        original = self.asm.read_text()
        self.asm.write_text(original.replace("hd_audioout_set@@Base", "missing_function@@Base"))
        result = self.draft()
        with self.subTest(input="missing function"):
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "")
        self.asm.write_text(original)
        self.sizes.write_text("pub const HEADER_SIZES: &[(&str, usize)] = &[];\n")
        result = self.draft()
        with self.subTest(input="empty size table"):
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "")

    def test_known_copy_candidate_keeps_its_symbol_and_literal_count(self):
        self.asm.write_text("00000100 <sample>:\n" + self.switch().split(" 120:")[0]
                            + " 120:\te3a0202c \tmov\tr2, #44 @ 0x2c\n"
                            + " 124:\teb000000 \tbl\t800 <memcpy@plt>\n"
                            + " 128:\te12fff1e \tbx\tlr\n")
        result = subprocess.run([sys.executable, "-I", str(ABI / "disasm_cases.py"),
                                 str(self.asm), "sample"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("('memcpy', 44)", result.stdout)

    def test_shared_targets_and_no_table_cases_remain_visible(self):
        for header in HEADERS.values():
            p = self.root / header
            text = p.read_text()
            line = text.splitlines()[1]
            p.write_text(text.replace(line, "\n".join(line.replace("FIRST", name)
                                                    for name in ("FIRST", "SECOND", "THIRD"))))
        original = self.asm.read_text()
        shared = original.replace("r1, #0", "r1, #2").replace(
            " 120:", " 110:\tea000000 \tb\t120 <error>\n"
            " 114:\tea000000 \tb\t120 <error>\n 120:")
        for asm in (shared, original.replace(self.switch(), " 100:\te12fff1e \tbx\tlr\n")):
            with self.subTest(asm=asm[:100]):
                self.asm.write_text(asm)
                rows = [r for r in self.rows() if r[0] == "videocap"]
                self.assertEqual([(r[1], r[2]) for r in rows],
                                 [(op, str(i)) for i in range(3) for op in ("get", "set")])
                self.assertTrue(all(r[6] == "unreviewed" for r in rows))

    def test_copy_candidates_are_independent_of_the_expected_header_size(self):
        self.asm.write_text(self.asm.read_text().replace(
            " 120:\te3e00021 \tmvn\tr0, #33\n",
            " 120:\te880003f \tstm\tr0, {r0, r1, r2, r3, r4, r5}\n"))
        for size in (4, 24, 28, 32, 64):
            with self.subTest(size=size):
                self.sizes.write_text(f'[("HD_SAMPLE", {size})]')
                row = next(r for r in self.rows() if r[:3] == ["videocap", "set", "0"])
                self.assertEqual(row[5:9], [str(size), "unreviewed", "104:r1->120", "120:stm-total=24"])


if __name__ == "__main__":
    unittest.main()
