#!/usr/bin/env python3
"""Actual projected writer through isolated syscall edges; never opens camera paths."""
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import unittest
import mtd_writer

HERE=Path(__file__).resolve().parent
DEFAULT_SOURCE=HERE.parents[3]/"tools/mi-360/camera-mtd-write.c"

class WriterPolicy(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp=tempfile.TemporaryDirectory(prefix="mi360-writer-test-")
        cls.root=Path(cls.temp.name)
        cls.original=Path(os.environ.get("MI360_WRITER_SOURCE",DEFAULT_SOURCE)).read_bytes()
        cls.projected=mtd_writer.project(cls.original)
        (cls.root/"camera-mtd-write.c").write_bytes(cls.projected)
        cls.binary=cls.root/"writer-test"
        subprocess.run([*shlex.split(os.environ.get("CC","clang")),"-std=c11","-Wall","-Wextra","-Werror",
                        "-fsanitize=address,undefined","-fno-omit-frame-pointer","-g","-I",str(cls.root),
                        str(HERE/"test_mtd_writer.c"),"-o",str(cls.binary)],check=True)

    @classmethod
    def tearDownClass(cls):cls.temp.cleanup()

    def invoke(self,scenario,operation="write",kind="kernel"):
        result=subprocess.run([str(self.binary),scenario,operation,kind],capture_output=True,text=True,timeout=5)
        records=[line.removeprefix("TEST_COUNTERS ") for line in result.stderr.splitlines() if line.startswith("TEST_COUNTERS ")]
        self.assertEqual(len(records),1,result.stderr)
        return result,json.loads(records[0])

    def test_projection_requires_the_original_source_bytes(self):
        with self.assertRaisesRegex(ValueError,"hash differs"):mtd_writer.project(self.original+b"\n")
        self.assertIn(b"Complete partition readback matches, including FF padding",self.projected)

    def test_info_and_plan_are_readonly_on_both_inactive_slot_directions(self):
        for scenario in ("normal","active_a"):
            for operation in ("info","plan"):
                for kind in ("kernel","usrfs"):
                    with self.subTest(scenario=scenario,operation=operation,kind=kind):
                        result,count=self.invoke(scenario,operation,kind)
                        self.assertEqual(result.returncode,0,result.stderr)
                        self.assertTrue(count["readonly"])
                        self.assertEqual(count["image_opens"],int(operation=="plan"))
                        for key in ("erase_calls","write_calls","sync_calls","verify_bytes"):self.assertEqual(count[key],0)
                        info=json.loads(result.stdout.splitlines()[0])
                        self.assertTrue(info["qualified"])
                        self.assertEqual((info["type"],info["flags"],info["erasesize"],info["writesize"],info["oobsize"]),(3,0xc00,32768,256,0))
                        self.assertEqual(info["partition"],("B" if scenario=="active_a" else "A")+("_KERNEL" if kind=="kernel" else "_USRFS"))

    def test_geometry_layout_privilege_and_image_fail_before_any_erase(self):
        for scenario in ("wrong_type","wrong_flags","extra_flags","wrong_size","wrong_erase","wrong_write1",
                         "wrong_write512","wrong_oob","layout_wrong_erase","bad_mount","unprivileged","mtd_regular",
                         "mtd_open_fail","invalid_image"):
            for operation in ("plan","write"):
                with self.subTest(scenario=scenario,operation=operation):
                    result,count=self.invoke(scenario,operation)
                    self.assertNotEqual(result.returncode,0)
                    for key in ("erase_calls","write_calls","sync_calls","verify_bytes"):self.assertEqual(count[key],0,result.stderr)

    def test_exact_erases_and_full_padded_readback_for_each_partition(self):
        for kind,limit in (("kernel",0x1b0000),("usrfs",0x470000)):
            for scenario in ("normal","active_a","partial_pages","fsync_einval"):
                with self.subTest(kind=kind,scenario=scenario):
                    result,count=self.invoke(scenario,"write",kind)
                    self.assertEqual(result.returncode,0,result.stderr)
                    self.assertFalse(count["readonly"])
                    self.assertEqual(count["erase_calls"],limit//32768)
                    self.assertGreaterEqual(count["write_calls"],count["erase_calls"])
                    self.assertEqual(count["verify_bytes"],limit)
                    self.assertEqual(count["sync_calls"],1)
                    self.assertIn("Complete partition readback matches",result.stdout)

    def test_fsync_io_errors_and_readback_mismatch_never_report_success(self):
        for scenario in ("fsync_eio","fsync_enospc","verify_mismatch"):
            result,count=self.invoke(scenario)
            self.assertNotEqual(result.returncode,0)
            self.assertEqual(count["erase_calls"],0x1b0000//32768)
            self.assertNotIn("Complete partition readback matches",result.stdout)
            self.assertEqual(count["verify_bytes"],4096 if scenario=="verify_mismatch" else 0)

    def test_erase_plan_rejects_unaligned_overflow_and_wrong_limit(self):
        result=subprocess.run([str(self.binary),"guard_matrix","write","kernel"],capture_output=True,text=True,timeout=5)
        self.assertEqual(result.returncode,0,result.stderr)

if __name__=="__main__":unittest.main()
