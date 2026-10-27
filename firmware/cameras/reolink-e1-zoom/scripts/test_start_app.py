"""start_app assembly: our wrapper plus the two stock blocks taken from the reader's own stock rootfs."""
import hashlib
import os
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).parent))
import start_app  # noqa: E402

TEMPLATE = Path(__file__).resolve().parents[1] / "image/start_app"
# image/start_app as shipped before the stock blocks left the tree.
ASSEMBLED_SHA256 = "98a2d98433bb9ddb84551dd94fcf7eab9a69ef49f35d1b54230f137e56bea8a7"


class AssemblyTests(unittest.TestCase):
    def test_template_carries_placeholders_not_stock_bytes(self):
        text = TEMPLATE.read_bytes()
        self.assertEqual(text.count(b"\n@STOCK_INIT@\n"), 1)
        self.assertEqual(text.count(b"\n@STOCK_VENDOR@\n"), 1)
        self.assertNotIn(b"./router &", text)

    def test_assemble_splices_both_blocks_in_place(self):
        out = start_app.assemble(b"a\n@STOCK_INIT@\nb\n@STOCK_VENDOR@\nc\n", b"init\n", b"vendor\n")
        self.assertEqual(out, b"a\ninit\nb\nvendor\nc\n")

    def test_assemble_refuses_a_template_without_exactly_one_of_each_placeholder(self):
        for template in (b"@STOCK_INIT@\n", b"@STOCK_INIT@\n@STOCK_VENDOR@\n@STOCK_VENDOR@\n"):
            with self.subTest(template=template), self.assertRaisesRegex(ValueError, "placeholder"):
                start_app.assemble(template, b"i\n", b"v\n")

    def test_stock_blocks_refuse_a_start_app_that_is_not_4417(self):
        with self.assertRaisesRegex(ValueError, "4417"):
            start_app.stock_blocks(b"echo init\n./router &\necho vendor\n")
        with self.assertRaisesRegex(ValueError, "4417"):
            start_app.stock_blocks(b"no vendor boundary\n")

    @unittest.skipUnless(os.environ.get("STOCK_PAK") and os.environ.get("OWN_YOUR_CAMERAS"),
                         "set STOCK_PAK and OWN_YOUR_CAMERAS for the real stock blocks")
    def test_real_stock_blocks_assemble_the_shipped_start_app(self):
        stock = start_app.stock_start_app(Path(os.environ["STOCK_PAK"]), Path(os.environ["OWN_YOUR_CAMERAS"]))
        assembled = start_app.assemble(TEMPLATE.read_bytes(), *start_app.stock_blocks(stock))
        self.assertEqual(hashlib.sha256(assembled).hexdigest(), ASSEMBLED_SHA256)


if __name__ == "__main__":
    unittest.main()
