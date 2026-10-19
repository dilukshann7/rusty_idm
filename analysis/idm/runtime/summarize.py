"""Read-only snapshot analysis of fixture logs/downloads; writes only result artifacts."""
import hashlib
import importlib.util
import json
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlsplit


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
SCENARIOS = ["baseline", "ranges", "ignore", "unknown", "redirect", "interrupt", "temporary", "slow", "mutable", "queue-a", "queue-b", "browser"]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("idm_fixture_snapshot", ROOT / "fixture.py")
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)  # fixture's __main__ server is never invoked


def sha(data):
    return hashlib.sha256(data).hexdigest()


def concurrency(requests):
    points = []
    for row in requests:
        if "started_monotonic" in row:
            points.append((row["started_monotonic"], 1))
        if "finished_monotonic" in row:
            # Keep the request active for zero-duration rounded observations.
            finish = max(row["finished_monotonic"], row["started_monotonic"] + 1e-9)
            points.append((finish, -1))
    current = maximum = 0
    for _, change in sorted(points):
        current += change
        maximum = max(maximum, current)
    return maximum


def summarize_group(rows):
    return {
        "requests": len(rows),
        "methods": dict(Counter(r["method"] for r in rows)),
        "status_codes": dict(Counter(str(r.get("status", "unfinished")) for r in rows)),
        "versions": dict(Counter(r["version"] for r in rows)),
        "payload_sizes": sorted(set(r["total"] for r in rows)),
        "range_requests": sum(bool(r.get("range")) for r in rows),
        "max_overlapping_request_lifetimes": concurrency(rows),
        "bytes_sent_end_records": sum(r.get("bytes_sent", 0) for r in rows if r.get("phase") == "end"),
        "unfinished_request_ids": [r["id"] for r in rows if r.get("phase") != "end"],
        "validators": {name: [{"request_id": r["id"], "value": r[name]} for r in rows if r.get(name)]
                       for name in ("if_range", "if_match", "if_unmodified_since")},
        "user_agents": dict(Counter(r.get("user_agent") or "(none)" for r in rows)),
        "request_details": [{k: r.get(k) for k in ("id", "method", "path", "version", "total", "range", "status", "start", "end", "bytes_sent", "error", "started_utc", "finished_utc")}
                            for r in sorted(rows, key=lambda r: r["id"])],
    }


def main():
    raw = (ROOT / "requests.jsonl").read_bytes() if (ROOT / "requests.jsonl").exists() else b""
    records, skipped = [], []
    lines = raw.splitlines()
    for number, line in enumerate(lines, 1):
        try:
            records.append(json.loads(line))
        except (ValueError, UnicodeDecodeError):
            skipped.append(number)
    # Requests can outlive the snapshot. Merge phase records by request ID,
    # separating logger restarts when a repeated ID has a new start time.
    requests = {}
    for row in records:
        requests[(row["id"], row.get("started_monotonic"))] = row
    self_checks = [r for r in requests.values() if r.get("user_agent") == "FixtureSelfCheck/1"]
    candidates = [r for r in requests.values() if r.get("user_agent") != "FixtureSelfCheck/1"]
    expected = json.loads((ROOT / "expected-hashes.json").read_text())
    manifest_checks = []
    for key, digest in expected.items():
        size, version = key.split(":v")
        actual = sha(fixture.payload(int(size), version))
        manifest_checks.append({"key": key, "expected": digest, "actual": actual, "matches": actual == digest})
    file_results = []
    for path in sorted((ROOT / "downloads").glob("*")):
        if not path.is_file():
            continue
        before = path.stat()
        try:
            data = path.read_bytes()
        except (PermissionError, OSError) as exc:
            file_results.append({"name": path.name, "read_error": type(exc).__name__})
            continue
        after = path.stat()
        stable = before.st_size == after.st_size == len(data) and before.st_mtime_ns == after.st_mtime_ns
        versions = {v: fixture.payload(len(data), v) for v in ("1", "2")}
        matches = [v for v, payload in versions.items() if sha(payload) == sha(data)]
        distribution = Counter()
        if not matches:
            for actual, v1, v2 in zip(data, versions["1"], versions["2"]):
                distribution["v1" if actual == v1 else "v2" if actual == v2 else "neither"] += 1
        related = [r for r in candidates if urlsplit(r["path"]).path.rsplit("/", 1)[-1] == path.name]
        sizes = sorted(set(r["total"] for r in related))
        classification = "matches-v" + matches[0] if matches else "mixed-v1-v2-at-expected-offsets" if distribution["v1"] and distribution["v2"] and not distribution["neither"] else "does-not-match-either-version"
        file_results.append({"name": path.name, "size": len(data), "sha256": sha(data), "stable_during_read": stable,
                             "matching_fixture_versions": matches, "classification": classification,
                             "per_offset_version_byte_counts": dict(distribution),
                             "associated_logged_sizes": sizes, "matches_logged_total_size": len(data) in sizes,
                             "expected_hashes_for_actual_size": {v: sha(p) for v, p in versions.items()}})
    scenarios = {}
    for name in SCENARIOS:
        rows = [r for r in candidates if urlsplit(r["path"]).path.rsplit("/", 1)[-1].split(".")[0] == name]
        scenarios[name] = {"idm_test_status": "endpoint-requests-observed" if rows else "unrun", **summarize_group(rows)}
    result = {"generated_utc": datetime.now(timezone.utc).isoformat(),
              "log_snapshot_sha256": sha(raw), "log_snapshot_bytes": len(raw), "log_records": len(records),
              "skipped_malformed_line_numbers": skipped, "fixture_sha256": sha((ROOT / "fixture.py").read_bytes()),
              "fixture_manifest_checks": manifest_checks,
              "attribution": "Non-self-check requests are candidates correlated with user-assisted IDM actions; User-Agent alone does not identify IDM or prove browser integration.",
              "self_check": summarize_group(self_checks), "candidate_idm": summarize_group(candidates),
              "scenarios": scenarios, "download_files": file_results,
              "limitations": ["Snapshot may precede completion; rerun after final downloads settle.",
                              "User reports manually downloading ranges and pausing/resuming mutable after fixture switched versions; runtime logs do not independently prove the UI actions.",
                              "Overlapping request lifetimes measure request concurrency, not exact TCP connection count; requests can share or reuse connections.",
                              "bytes_sent is successful server writes and may be buffered or discarded; it is not durable client progress.",
                              "Only If-Range, If-Match and If-Unmodified-Since are logged; absence says nothing about unlogged validator headers or internal decisions.",
                              "Payload repeats every 256 bytes, so hashes cannot detect some permutations or mispositioning by multiples of 256. Version changes distinguish XOR patterns but do not establish a segmentation algorithm.",
                              "Download path association uses the original endpoint basename and may miss renamed files; stable read is not proof the UI considers a file complete."]}
    (ROOT / "runtime-results.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    text = ["# IDM runtime experiment snapshot", "", "Generated UTC: " + result["generated_utc"], "",
            "This report separates fixture self-checks from requests correlated with the user's IDM actions. It is a snapshot; rerun `python analysis/idm/runtime/summarize.py` after subsequent downloads. No application build or dev server is involved.", "",
            "## Attribution and coverage", "",
            f"The log snapshot contains {len(records)} records, {len(self_checks)} self-check requests (`FixtureSelfCheck/1`) and {len(candidates)} other requests. Log SHA-256: `{sha(raw)}`. Malformed/trailing lines skipped: {skipped}.", "",
            "The user assisted by downloading ranges and pausing/resuming the mutable test after the fixture switched from version 1 to 2. These human reports establish the action context. The endpoint records the request details; its legacy-browser User-Agent string alone does not identify IDM, its exact version, or the source of browser integration.", "",
            f"Candidate requests: methods `{dict(Counter(r['method'] for r in candidates))}`; maximum overlapping request lifetimes **{concurrency(candidates)}**. See `analysis/idm/runtime/runtime-results.json` for every interval, status, validator and user-agent value.", "",
            "## Experiment matrix", "", "| Scenario | Candidate requests | Methods | Statuses | Sizes | Range requests | Max request overlap | Result boundary |", "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for name, group in scenarios.items():
        text.append(f"| {name} | {group['requests']} | {group['methods']} | {group['status_codes']} | {group['payload_sizes']} | {group['range_requests']} | {group['max_overlapping_request_lifetimes']} | {group['idm_test_status']} |")
    text += ["", "User-reported actions and their verification limits are recorded in user-observations.json and docs/idm-investigation.md. Browser takeover and restart/resume were exercised; exact limiter throughput and clean two-ready-job queue scheduling remain unverified. Fixture self-check coverage does not count as IDM coverage.", "", "## Output hashes", "",
             "Expected hashes are generated by dynamically importing the fixture's `payload` function without invoking its server. Each output is compared at its actual byte length against both fixture versions; it need not appear in the fixed-size manifest.", "",
             "| File | Actual bytes | Stable read | SHA-256 | Pattern result | Logged-size match |", "| --- | --- | --- | --- | --- | --- |"]
    for f in file_results:
        if "read_error" in f:
            text.append(f"| {f['name']} | unknown | no | unavailable | {f['read_error']} | unknown |")
        else:
            text.append(f"| {f['name']} | {f['size']} | {f['stable_during_read']} | `{f['sha256']}` | {f['classification']} | {f['matches_logged_total_size']} |")
    text += ["", "The ranges URL was manually entered with `size=1677721`; report and hash that actual size rather than assuming the planned 16,777,216 bytes. A matching smaller file validates that request only.", "",
             "A mixed-v1-v2 result means bytes at their expected offsets include both fixture patterns, with none matching neither pattern. It describes the resulting file; it does not establish why IDM retained or replaced individual ranges. An absent final mutable file leaves completion and corruption outcomes unresolved.", "", "## Validators and measurement limits", ""]
    for name in ("if_range", "if_match", "if_unmodified_since"):
        values = result["candidate_idm"]["validators"][name]
        text.append(f"- `{name}`: {len(values)} nonempty values observed across candidate requests; values/request IDs are in JSON.")
    text += ["", *["- " + line for line in result["limitations"]], "",
             "Fixture manifest verification: " + ("all declared hashes match the imported payload function." if all(r["matches"] for r in manifest_checks) else "MISMATCH; inspect JSON."), ""]
    (REPO / "docs" / "idm-runtime-results.md").write_text("\n".join(text), encoding="utf-8")
    print(json.dumps({"requests": len(candidates), "self_checks": len(self_checks), "files": [{"name": f["name"], "classification": f.get("classification", f.get("read_error"))} for f in file_results]}))


if __name__ == "__main__":
    main()
