"""Loopback-only IDM test endpoint. No external packages or persistent IDM changes."""
import argparse
import hashlib
import json
import threading
import time
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

BLOCK = bytes(range(256)) * 256
LOCK = threading.Lock()
ATTEMPTS = {}


def payload(size, version):
    block = BLOCK if version == "1" else bytes(b ^ 0x5A for b in BLOCK)
    return (block * ((size + len(block) - 1) // len(block)))[:size]


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_):
        pass

    def do_HEAD(self):
        self.transfer(True)

    def do_GET(self):
        self.transfer(False)

    def transfer(self, head):
        parsed = urlsplit(self.path)
        args = parse_qs(parsed.query)
        case = parsed.path.rsplit("/", 1)[-1].split(".")[0]
        if case not in {"baseline", "ranges", "ignore", "unknown", "redirect", "interrupt", "temporary", "slow", "mutable", "queue-a", "queue-b", "browser"}:
            self.send_error(404)
            return
        size = min(max(int(args.get("size", [16777216])[0]), 1), 67108864)
        version = args.get("version", ["1"])[0]
        if case == "mutable":
            version = self.server.version_file.read_text().strip() if self.server.version_file.exists() else "1"
        data = payload(size, version)
        with LOCK:
            request_id = self.server.next_id
            self.server.next_id += 1
            key = (parsed.path, self.command)
            ATTEMPTS[key] = ATTEMPTS.get(key, 0) + 1
            attempt = ATTEMPTS[key]
        event = dict(id=request_id, source="endpoint-observation", started_utc=datetime.now(timezone.utc).isoformat(),
                     started_monotonic=time.monotonic(), method=self.command, path=self.path,
                     user_agent=self.headers.get("User-Agent"), range=self.headers.get("Range"),
                     if_range=self.headers.get("If-Range"), if_match=self.headers.get("If-Match"),
                     if_unmodified_since=self.headers.get("If-Unmodified-Since"),
                     referer=self.headers.get("Referer"), cookie="[present; value not retained]" if self.headers.get("Cookie") else None,
                     attempt=attempt, version=version, total=size, bytes_sent=0)
        status = 200
        start, end = 0, size - 1
        etag = '"fixture-v' + version + '"'
        modified = "Sun, 18 Oct 2026 00:00:00 GMT" if version == "1" else "Sun, 18 Oct 2026 00:01:00 GMT"
        self.record({**event, "phase": "start"})
        try:
            if case == "redirect":
                status = 302
                self.send_response(status)
                self.send_header("Location", "/ranges.bin?size=" + str(size))
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            if case == "temporary" and attempt == 1:
                status = 503
                self.send_response(status)
                self.send_header("Retry-After", "1")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            requested = self.headers.get("Range")
            if requested and case not in {"ignore", "unknown"} and self.headers.get("If-Range") in (None, etag, modified):
                import re
                match = re.fullmatch(r"bytes=(\d+)-(\d*)", requested)
                if not match or int(match[1]) >= size or (match[2] and int(match[2]) < int(match[1])):
                    status = 416
                    self.send_response(status)
                    self.send_header("Content-Range", f"bytes */{size}")
                    self.send_header("Content-Length", "0")
                    self.end_headers()
                    return
                start = int(match[1])
                end = min(int(match[2]), end) if match[2] else end
                status = 206
            self.send_response(status)
            self.send_header("Content-Type", "application/octet-stream")
            self.send_header("Content-Disposition", 'attachment; filename="' + case + '.bin"')
            self.send_header("ETag", etag)
            self.send_header("Last-Modified", modified)
            self.send_header("Accept-Ranges", "none" if case in {"ignore", "unknown"} else "bytes")
            if status == 206:
                self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
            if case == "unknown":
                self.send_header("Connection", "close")
                self.close_connection = True
            else:
                self.send_header("Content-Length", str(end - start + 1))
            self.end_headers()
            if head:
                return
            delay = float(args.get("delay", ["0.025" if case in {"slow", "mutable", "queue-a", "queue-b"} else "0.002"])[0])
            for offset in range(start, end + 1, 65536):
                chunk = data[offset:min(offset + 65536, end + 1)]
                self.wfile.write(chunk)
                self.wfile.flush()
                event["bytes_sent"] += len(chunk)
                if case == "interrupt" and attempt == 1 and event["bytes_sent"] >= 262144:
                    self.close_connection = True
                    break
                time.sleep(delay)
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError, TimeoutError) as exc:
            event["error"] = type(exc).__name__
            self.close_connection = True
        finally:
            self.record({**event, "phase": "end", "status": status, "start": start, "end": end,
                         "finished_monotonic": time.monotonic(), "finished_utc": datetime.now(timezone.utc).isoformat()})

    def record(self, event):
        with LOCK:
            with self.server.log_path.open("a", encoding="utf-8") as stream:
                stream.write(json.dumps(event) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=18765)
    parser.add_argument("--output", type=Path, default=Path(__file__).parent)
    options = parser.parse_args()
    options.output.mkdir(parents=True, exist_ok=True)
    manifest = {f"{size}:v{version}": hashlib.sha256(payload(size, version)).hexdigest()
                for size in (1048576, 16777216, 67108864) for version in ("1", "2")}
    (options.output / "expected-hashes.json").write_text(json.dumps(manifest, indent=2))
    server = ThreadingHTTPServer(("127.0.0.1", options.port), Handler)
    server.daemon_threads = True
    server.log_path = options.output / "requests.jsonl"
    previous_ids = []
    if server.log_path.exists():
        for line in server.log_path.read_text(encoding="utf-8").splitlines():
            try:
                previous_ids.append(json.loads(line)["id"])
            except (ValueError, KeyError):
                pass
    server.next_id = max(previous_ids, default=0) + 1
    server.version_file = options.output / "mutable-version.txt"
    print(f"IDM fixture listening on http://127.0.0.1:{options.port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
