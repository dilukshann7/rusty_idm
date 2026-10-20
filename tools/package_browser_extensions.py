"""Stage four browser packages from shared source; never builds or installs."""
import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def stage(destination):
    destination = Path(destination)
    for browser in ("chrome", "brave", "edge", "firefox"):
        target = destination / (browser + "-extension")
        if target.exists():
            raise RuntimeError(f"Preserve the existing browser package first: {target}")
        target.mkdir(parents=True)
        for source in (ROOT / "extensions/chrome").rglob("*"):
            relative = source.relative_to(ROOT / "extensions/chrome")
            if source.is_file() and "tests" not in relative.parts and source.name != "package.json":
                (target / relative).parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, target / relative)
        if browser != "chrome":
            for name in ("manifest.json", "browser-config.js"):
                shutil.copyfile(ROOT / "extensions/profiles" / browser / name, target / name)
        manifest = json.loads((target / "manifest.json").read_text(encoding="utf-8"))
        if manifest.get("browser_specific_settings") and browser != "firefox":
            raise RuntimeError("Unexpected Firefox configuration")


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("destination", type=Path)
    stage(parser.parse_args().destination)
