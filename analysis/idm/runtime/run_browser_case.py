"""Navigate only the disposable Edge profile to a loopback test download."""
import argparse
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument("case", choices=["baseline", "ranges", "ignore", "unknown", "redirect", "interrupt", "temporary", "slow", "queue-a", "queue-b"])
args = parser.parse_args()
run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%f")
url = f"http://127.0.0.1:18765/browser-cases-{run_id}/{args.case}.bin?size=1048576"
command = [r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
           "--user-data-dir=" + str(ROOT / "edge-profile"), url]
process = subprocess.Popen(command, creationflags=subprocess.CREATE_NO_WINDOW)
with (ROOT / "browser-submissions.jsonl").open("a", encoding="utf-8") as stream:
    stream.write(json.dumps({"utc": datetime.now(timezone.utc).isoformat(), "case": args.case,
                             "url": url, "command": command, "pid": process.pid,
                             "source": "disposable-browser-navigation-not-proof-of-IDM-completion"}) + "\n")
print("Submitted " + args.case + " to the disposable Edge profile")
