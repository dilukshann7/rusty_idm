"""Source/package checks; does not build Rusty or run its media tools."""
import hashlib
import json
import re
import unittest
from pathlib import Path

from package_release import inspect_pe

ROOT = Path(__file__).resolve().parents[1]
LOCK = json.loads((ROOT / "tools/media_tools_lock.json").read_text(encoding="utf-8"))


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


class MediaPackageTest(unittest.TestCase):
    def test_runtime_verification_hashes_match_the_package_lock(self):
        source = (ROOT / "crates/engine/src/media.rs").read_text(encoding="utf-8")
        constants = dict(re.findall(r'const (\w+_HASH): &str = "([0-9a-f]{64})";', source))
        names = {"yt-dlp.exe": "YT_DLP_HASH", "ffmpeg.exe": "FFMPEG_HASH", "ffprobe.exe": "FFPROBE_HASH", "deno.exe": "DENO_HASH"}
        for tool in LOCK["tools"]:
            for executable, expected in tool["outputs"].items():
                self.assertEqual(constants[names[executable]], expected)

    def test_pinned_tools_have_absolute_package_paths_and_expected_hashes(self):
        location = ROOT / "assets/media-tools/windows-x64"
        for entry in LOCK["tools"]:
            self.assertTrue(entry["url"].startswith("https://"))
            self.assertRegex(entry["archive_sha256"], r"^[0-9a-f]{64}$")
            for name, expected in entry["outputs"].items():
                self.assertEqual(Path(name).name, name)
                file = location / name
                self.assertTrue(file.is_file(), name)
                self.assertEqual(digest(file), expected)
                self.assertEqual(inspect_pe(file, allow_system_crt=True)["machine"], "x64")

    def test_required_upstream_notices_are_pinned(self):
        for name, expected in LOCK["notices"].items():
            file = ROOT / "assets/media-tool-notices" / name
            self.assertTrue(file.is_file(), name)
            self.assertEqual(digest(file), expected)


if __name__ == "__main__":
    unittest.main()
