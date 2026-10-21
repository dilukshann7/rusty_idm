"""Fetch pinned Windows media tools into an ignored local directory; no app build."""
import hashlib
import json
import shutil
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOCK = json.loads((ROOT / "tools/media_tools_lock.json").read_text(encoding="utf-8"))
CACHE = ROOT / "assets/media-tools-cache"
OUTPUT = ROOT / "assets/media-tools/windows-x64"
LIMIT = 600 * 1024 * 1024


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def fetch(entry):
    CACHE.mkdir(parents=True, exist_ok=True)
    name = entry["name"].split("/")[0].lower() + (".zip" if entry["archive"] == "zip" else ".exe")
    target = CACHE / name
    if target.is_file() and digest(target) == entry["archive_sha256"]:
        return target
    if target.exists():
        raise SystemExit(f"Cached tool hash differs from lock: {target}")
    temporary = target.with_suffix(target.suffix + ".partial")
    if temporary.is_file():
        temporary.unlink()
    request = urllib.request.Request(entry["url"], headers={"User-Agent": "RustyIDM-media-packager/0.2.1"})
    hash_value = hashlib.sha256()
    size = 0
    with urllib.request.urlopen(request, timeout=60) as response, temporary.open("xb") as file:
        while chunk := response.read(1024 * 1024):
            size += len(chunk)
            if size > LIMIT:
                raise SystemExit("Tool archive exceeds package limit")
            hash_value.update(chunk)
            file.write(chunk)
    if hash_value.hexdigest() != entry["archive_sha256"]:
        raise SystemExit(f"Published checksum mismatch: {entry['name']}")
    temporary.rename(target)
    return target


def extract(entry, archive):
    OUTPUT.mkdir(parents=True, exist_ok=True)
    if entry["archive"] == "raw":
        destination = OUTPUT / next(iter(entry["outputs"]))
        if destination.exists() and digest(destination) != entry["archive_sha256"]:
            raise SystemExit(f"Existing tool differs from pinned file: {destination}")
        if not destination.exists():
            shutil.copy2(archive, destination)
        return
    with zipfile.ZipFile(archive) as source:
        for name in entry["outputs"]:
            members = [item for item in source.infolist() if Path(item.filename).name.lower() == name.lower() and not item.is_dir()]
            if len(members) != 1 or members[0].file_size > LIMIT:
                raise SystemExit(f"Missing or ambiguous pinned tool: {name}")
            destination = OUTPUT / name
            if destination.exists():
                if digest(destination) != entry["outputs"][name]:
                    raise SystemExit(f"Existing tool differs from pinned file: {destination}")
                continue
            with source.open(members[0]) as input_file, destination.open("xb") as output_file:
                shutil.copyfileobj(input_file, output_file, length=1024 * 1024)
            if digest(destination) != entry["outputs"][name]:
                raise SystemExit(f"Extracted tool differs from pinned file: {destination}")


def main():
    for entry in LOCK["tools"]:
        if not all((OUTPUT / name).is_file() and digest(OUTPUT / name) == sha for name, sha in entry["outputs"].items()):
            extract(entry, fetch(entry))
    for entry in LOCK["tools"]:
        for name, sha in entry["outputs"].items():
            if digest(OUTPUT / name) != sha:
                raise SystemExit(f"Tool hash mismatch: {name}")
    manifest = {"schema_version": 1, "archive_sha256": {item["name"]: item["archive_sha256"] for item in LOCK["tools"]},
                "files": {path.name: digest(path) for path in sorted(OUTPUT.glob("*.exe"))}}
    (OUTPUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
