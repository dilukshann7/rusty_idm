"""Open a disposable Edge profile for the explicitly requested integration test."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent
profile = ROOT / "edge-profile"
default = profile / "Default"
default.mkdir(parents=True, exist_ok=True)
preferences = default / "Preferences"
if not preferences.exists():
    preferences.write_text(json.dumps({"download": {"default_directory": str(ROOT / "downloads"), "prompt_for_download": False},
                                       "browser": {"check_default_browser": False}}))
extension = ROOT.parent / "browser" / "extension"
args = [r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        "--user-data-dir=" + str(profile), "--no-first-run", "--no-default-browser-check", "--disable-sync",
        "--disable-background-networking", "--load-extension=" + str(extension), "edge://extensions/"]
process = subprocess.Popen(args, creationflags=subprocess.CREATE_NO_WINDOW)
(ROOT / "browser-launch.json").write_text(json.dumps({"profile": str(profile), "extension": str(extension),
    "arguments": args, "launched_pid": process.pid, "purpose": "disposable-profile integration test", "extension_load_verified": False}, indent=2))
print("Opened disposable Edge profile at edge://extensions/. Verify IDM Integration Module is loaded before testing.")
