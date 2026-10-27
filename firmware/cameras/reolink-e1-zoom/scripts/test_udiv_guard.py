"""The Cortex-A9 instruction gate must fail closed on tool failures."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class InstructionGuardTests(unittest.TestCase):
    def check_guard(self, output, status=0):
        with tempfile.TemporaryDirectory() as temp:
            tool = Path(temp) / "objdump"
            tool.write_text("#!/bin/sh\ncat <<'ASM'\n" + output + "\nASM\nexit " + str(status) + "\n")
            tool.chmod(0o700)
            return subprocess.run(
                [str(Path(__file__).with_name("udiv-guard")), "unused-binary"],
                env={**os.environ, "LLVM_OBJDUMP": str(tool)},
                capture_output=True, text=True,
            )

    def test_valid_instructions_pass(self):
        self.assertEqual(self.check_guard("  0: e1a00000  nop").returncode, 0)

    def test_unsigned_division_fails(self):
        self.assertNotEqual(self.check_guard("  0: e730f211  udiv r0, r1, r2").returncode, 0)

    def test_signed_division_fails(self):
        self.assertNotEqual(self.check_guard("  0: e710f211  sdiv r0, r1, r2").returncode, 0)

    def test_failed_disassembler_fails_even_with_partial_output(self):
        self.assertNotEqual(self.check_guard("  0: e1a00000  nop", 2).returncode, 0)


if __name__ == "__main__":
    unittest.main()
