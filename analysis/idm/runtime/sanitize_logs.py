"""Remove cookie values inadvertently supplied by IDM; retain presence only."""
import json
from pathlib import Path

ROOT = Path(__file__).parent


def sanitize(value):
    if isinstance(value, dict):
        for key, item in value.items():
            if key.lower() in {"cookie", "authorization", "proxy-authorization"} and item:
                value[key] = "[present; value not retained]"
            else:
                sanitize(item)
    elif isinstance(value, list):
        for item in value:
            sanitize(item)
    return value


if __name__ == "__main__":
    path = ROOT / "requests.jsonl"
    records = [sanitize(json.loads(line)) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    path.write_text("".join(json.dumps(record) + "\n" for record in records), encoding="utf-8")
    path = ROOT / "runtime-results.json"
    if path.exists():
        path.write_text(json.dumps(sanitize(json.loads(path.read_text(encoding="utf-8"))), indent=2), encoding="utf-8")
    print("Retained cookie presence only; no cookie values retained in analysis logs/results")
