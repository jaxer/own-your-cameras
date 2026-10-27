"""check-links on throwaway git repositories: relative links in public Markdown must reach public files."""
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("check-links")
ROOT = Path(__file__).resolve().parents[1]


class CheckLinksTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.repo = Path(temp.name)
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        self.add("README.md", "# Readme\n")
        self.add("docs/hardware/ptz.md", "# PTZ\n\n## Move sequence\n\nSee [the overview](../../README.md).\n")

    def add(self, name, text):
        path = self.repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        subprocess.run(["git", "-C", str(self.repo), "add", "-f", name], check=True)

    def check(self):
        return subprocess.run(["python3", "-I", str(SCRIPT), str(self.repo)], capture_output=True, text=True)

    def assert_passes(self):
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stderr)

    def assert_refused(self, *needles):
        result = self.check()
        self.assertNotEqual(result.returncode, 0)
        for needle in needles:
            self.assertIn(needle, result.stderr)

    def test_resolving_links_pass(self):
        self.add("README.md", "[PTZ](docs/hardware/ptz.md), [moves](docs/hardware/ptz.md#move-sequence), "
                              "[docs](docs/hardware/), [web](https://example.com/x.md), [top](#readme)\n\n# Readme\n")
        self.assert_passes()

    def test_a_missing_target_is_refused(self):
        self.add("README.md", "See [recovery](docs/recovery.md).\n")
        self.assert_refused("README.md", "docs/recovery.md")

    def test_an_untracked_candidate_target_is_checked(self):
        (self.repo / "docs/untracked.md").write_text("# Untracked\n")
        self.add("README.md", "[x](docs/untracked.md)\n")
        self.assert_passes()

    def test_a_missing_anchor_is_refused(self):
        self.add("README.md", "[x](docs/hardware/ptz.md#homing)\n")
        self.assert_refused("README.md", "#homing")

    def test_anchors_follow_github_slugs(self):
        self.add("docs/a.md", "# Speed, halt and hold-to-zoom\n\n## `cam ptz` / `cam led`\n\n"
                              "[a](#speed-halt-and-hold-to-zoom) [b](#cam-ptz--cam-led)\n")
        self.assert_passes()

    def test_a_link_into_private_files_is_refused(self):
        self.add("docs/decisions/2026-01-01-x.md", "# X\n")
        self.add("README.md", "[record](docs/decisions/2026-01-01-x.md)\n")
        self.assert_refused("README.md", "private")

    def test_private_files_are_not_checked(self):
        self.add("CLAUDE.md", "[gone](nowhere.md)\n")
        self.add("docs/decisions/x.md", "[gone](../../nowhere.md)\n")
        self.add("docs/agents/y.md", "[gone](nowhere.md)\n")
        self.assert_passes()

    def test_links_in_code_are_ignored(self):
        self.add("README.md", "```md\n[x](missing.md)\n```\n\nInline `[y](missing.md)` too.\n")
        self.assert_passes()

    def test_this_repository_passes(self):
        result = subprocess.run(["python3", "-I", str(SCRIPT), str(ROOT)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
