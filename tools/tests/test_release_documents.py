"""Check the documentation that ships inside the installer and portable ZIP."""
import tempfile
import unittest
from pathlib import Path

from tools import package_release


class ReleaseDocumentTests(unittest.TestCase):
    def test_shipped_acceptance_links_resolve_to_the_actual_record(self):
        with tempfile.TemporaryDirectory() as temporary:
            payload = Path(temporary)
            package_release.stage_documents(payload)
            acceptance = payload / "docs/windows-acceptance.md"
            self.assertEqual(
                acceptance.read_bytes(),
                (package_release.ROOT / "docs/windows-acceptance.md").read_bytes(),
            )
            for name in ("VALIDATION.md", "RELEASE_NOTES.md"):
                document = payload / "docs" / name
                self.assertIn(
                    "](windows-acceptance.md)", document.read_text(encoding="utf-8")
                )
                self.assertTrue((document.parent / "windows-acceptance.md").is_file())


if __name__ == "__main__":
    unittest.main()
