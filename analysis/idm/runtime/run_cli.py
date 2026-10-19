"""Submit only disposable fixture downloads using IDM's documented CLI."""
import argparse
import hashlib
import json
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).parent
IDM = Path(r"C:\Program Files (x86)\Internet Download Manager\IDMan.exe")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("case", choices=["baseline", "ranges", "ignore", "unknown", "redirect", "interrupt", "temporary"])
    options = parser.parse_args()
    destination = ROOT / "downloads"
    destination.mkdir(exist_ok=True)
    filename = "cli-" + options.case + ".bin"
    target = destination / filename
    if target.exists():
        raise SystemExit("Refusing to overwrite an existing experiment output")
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%f")
    url = f"http://127.0.0.1:18765/idm-cli-{run_id}/{options.case}.bin?size=1048576"
    args = [str(IDM), "/d", url, "/p", str(destination.resolve()), "/f", filename, "/n"]
    event = dict(case=options.case, source="IDM-documented-CLI-submission", started_utc=datetime.now(timezone.utc).isoformat(),
                 url=url, destination=str(target.resolve()), command=args,
                 documentation="https://www.internetdownloadmanager.com/support/command_line.html")
    process = subprocess.Popen(args, creationflags=subprocess.CREATE_NO_WINDOW)
    deadline = time.monotonic() + 25
    while time.monotonic() < deadline:
        if target.exists():
            try:
                data = target.read_bytes()
                if len(data) == 1048576:
                    event.update(bytes=len(data), sha256=hashlib.sha256(data).hexdigest(), completed_file_observed=True)
                    break
            except PermissionError:
                pass
        time.sleep(0.2)
    else:
        event["completed_file_observed"] = False
    event["submission_exit_code"] = process.poll()
    event["finished_utc"] = datetime.now(timezone.utc).isoformat()
    with (ROOT / "cli-submissions.jsonl").open("a", encoding="utf-8") as stream:
        stream.write(json.dumps(event) + "\n")
    print(json.dumps(event, indent=2))
