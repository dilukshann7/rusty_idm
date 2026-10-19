"""Verify retained identities, report links, syntax, output hashes and redaction."""
import ast
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def main():
    errors = []
    index = read_json(ROOT / "evidence-index.json")
    for entry in index["files"]:
        path = REPO / entry["path"]
        if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
            errors.append("identity mismatch: " + entry["path"])
    for path in ROOT.rglob("*.py"):
        if "edge-profile" not in path.parts:
            ast.parse(path.read_text(encoding="utf-8-sig"), filename=str(path))
    for path in [REPO / "IDM_REVERSE_ENGINEERING.md", *(REPO / "docs").rglob("*.md")]:
        for target in re.findall(r"\]\(([^)]+)\)", path.read_text(encoding="utf-8")):
            if not target.startswith(("http:", "https:", "#")) and not (path.parent / target.split("#")[0]).exists():
                errors.append("missing local link: " + str(path.relative_to(REPO)) + " -> " + target)
    bundles = [read_json(ROOT / "native" / name) for name in ("engine-evidence.json", "host-evidence.json")]
    bundles.append(read_json(ROOT / "browser" / "rea-javascript-summary.json"))
    evidence_ids = set()

    def collect(value):
        if isinstance(value, dict):
            if "evidence_id" in value:
                evidence_ids.add(value["evidence_id"])
            for item in value.values():
                collect(item)
        elif isinstance(value, list):
            for item in value:
                collect(item)

    for bundle in bundles:
        collect(bundle)
    for finding in read_json(ROOT / "finding-ledger.json"):
        identity = finding.get("evidence_id")
        if identity and identity not in evidence_ids:
            errors.append("ledger evidence ID absent from bundles: " + identity)
    runtime = read_json(ROOT / "runtime" / "runtime-results.json")
    log = ROOT / "runtime" / "requests.jsonl"
    if hashlib.sha256(log.read_bytes()).hexdigest() != runtime["log_snapshot_sha256"]:
        errors.append("runtime report references a different log snapshot")
    file_results = runtime["download_files"]
    if not file_results:
        errors.append("no runtime file results")
    for entry in file_results:
        if entry.get("classification") not in {"matches-v1", "matches-v2"}:
            errors.append("unverified output: " + entry["name"])
        path = ROOT / "runtime" / "downloads" / entry["name"]
        if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
            errors.append("download changed after report: " + entry["name"])
    for line in log.read_text(encoding="utf-8").splitlines():
        row = json.loads(line)
        if row.get("cookie") not in (None, "[present; value not retained]"):
            errors.append("unredacted cookie field")
            break
    fixture_checks = read_json(ROOT / "runtime" / "fixture-checks.json")["results"]
    if not all(row["passed"] for row in fixture_checks):
        errors.append("fixture self-check failed")
    result = dict(indexed_files=len(index["files"]), ledger_evidence_ids_verified=len(evidence_ids),
                  completed_files_verified=len(file_results), fixture_checks_passed=sum(row["passed"] for row in fixture_checks),
                  errors=errors, passed=not errors)
    (ROOT / "verification.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if not errors else 1)


if __name__ == "__main__":
    main()
