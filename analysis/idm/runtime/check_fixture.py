"""Exercise fixture branches, explicitly identified separately from IDM traffic."""
import hashlib
import http.client
import json
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

ROOT = Path(__file__).parent
BASE = "http://127.0.0.1:18765/"
SIZE = 1048576
results = []


def fetch(case, headers=None):
    req = Request(BASE + case + ".bin?size=" + str(SIZE), headers={"User-Agent": "FixtureSelfCheck/1", **(headers or {})})
    with urlopen(req, timeout=10) as response:
        return response.status, dict(response.headers), response.read()


def check(name, action):
    try:
        action()
        results.append({"name": name, "passed": True})
    except Exception as exc:
        results.append({"name": name, "passed": False, "error": repr(exc)})


def baseline():
    status, headers, body = fetch("baseline")
    expected = json.loads((ROOT / "expected-hashes.json").read_text())[f"{SIZE}:v1"]
    assert status == 200 and hashlib.sha256(body).hexdigest() == expected


def ranges():
    status, headers, body = fetch("ranges", {"Range": "bytes=100-199"})
    assert status == 206 and len(body) == 100 and headers["Content-Range"] == f"bytes 100-199/{SIZE}"


def ignored():
    status, _, body = fetch("ignore", {"Range": "bytes=100-199"})
    assert status == 200 and len(body) == SIZE


def unknown():
    status, headers, body = fetch("unknown")
    assert status == 200 and "Content-Length" not in headers and len(body) == SIZE


def redirect():
    status, _, body = fetch("redirect")
    assert status == 200 and len(body) == SIZE


def interruption():
    try:
        fetch("interrupt")
        raise AssertionError("expected incomplete response")
    except http.client.IncompleteRead as exc:
        assert len(exc.partial) == 262144
    assert fetch("interrupt", {"Range": "bytes=262144-"})[0] == 206


def temporary():
    try:
        fetch("temporary")
        raise AssertionError("expected 503")
    except HTTPError as exc:
        assert exc.code == 503 and exc.headers["Retry-After"] == "1"
    assert fetch("temporary")[0] == 200


def validators():
    status, _, body = fetch("ranges", {"Range": "bytes=100-", "If-Range": '"outdated"'})
    assert status == 200 and len(body) == SIZE


def unsatisfiable():
    try:
        fetch("ranges", {"Range": f"bytes={SIZE}-"})
        raise AssertionError("expected 416")
    except HTTPError as exc:
        assert exc.code == 416


def mutable():
    version_file = ROOT / "mutable-version.txt"
    previous = version_file.read_bytes() if version_file.exists() else None
    try:
        version_file.write_text("2")
        _, headers, body = fetch("mutable")
        expected = json.loads((ROOT / "expected-hashes.json").read_text())[f"{SIZE}:v2"]
        assert headers["ETag"] == '"fixture-v2"' and hashlib.sha256(body).hexdigest() == expected
    finally:
        if previous is None:
            version_file.unlink(missing_ok=True)
        else:
            version_file.write_bytes(previous)


for name, action in [("baseline-hash", baseline), ("valid-range", ranges), ("ignored-range", ignored),
                     ("unknown-length", unknown), ("redirect", redirect), ("interruption", interruption),
                     ("temporary-failure", temporary), ("changed-validator", validators),
                     ("unsatisfiable-range", unsatisfiable), ("mutable-content", mutable)]:
    check(name, action)
(ROOT / "fixture-checks.json").write_text(json.dumps({"source": "fixture-self-check-not-IDM", "results": results}, indent=2))
print(json.dumps(results, indent=2))
raise SystemExit(0 if all(item["passed"] for item in results) else 1)
