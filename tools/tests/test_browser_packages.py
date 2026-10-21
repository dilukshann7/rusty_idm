"""Verify the browser payloads used by release staging without building binaries."""

import json
import tempfile
import unittest
import zipfile
from pathlib import Path

from tools.package_browser_extensions import stage
from tools.package_release import VERSION, archive


ROOT = Path(__file__).resolve().parents[2]


class BrowserPackageTests(unittest.TestCase):
    def test_each_profile_stages_a_versioned_nonempty_archive(self):
        base = ROOT / "target" / "tmp"
        base.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=base) as temporary:
            scratch = Path(temporary).resolve()
            self.assertTrue(scratch.is_relative_to(base.resolve()))
            payload = scratch / "payload"
            payload.mkdir()
            stage(payload)
            for browser in ("chrome", "brave", "edge", "firefox"):
                with self.subTest(browser=browser):
                    folder = payload / f"{browser}-extension"
                    manifest = json.loads((folder / "manifest.json").read_text(encoding="utf-8"))
                    self.assertEqual(manifest["version"], VERSION)
                    output = scratch / f"RustyDownloadManager-{VERSION}-{browser}-extension.zip"
                    archive(folder, output)
                    with zipfile.ZipFile(output) as package:
                        names = set(package.namelist())
                        self.assertIn("manifest.json", names)
                        self.assertIn("service-worker.js", names)
                        self.assertFalse(any(name.startswith("tests/") for name in names))
                        self.assertEqual(json.loads(package.read("manifest.json"))["version"], VERSION)


if __name__ == "__main__":
    unittest.main()
