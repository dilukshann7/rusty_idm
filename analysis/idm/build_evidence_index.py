"""Rebuild the finding ledger and hashes of retained investigation evidence."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
ENGINE = "c7c80b420579d774f445b6b3f9282c0850760ef790373c0ece0f856a0a05d1d5"
HOST = "dba25a49adb88f675db26d2dc7a0fa9d4a5db2326858cb9d2515f6f34b8e0b65"
LOCAL_ONLY_RUNTIME = {
    "browser-launch.json",
    "browser-submissions.jsonl",
    "cli-submissions.jsonl",
    "fixture-lifecycle.jsonl",
}


def finding(identifier, question, digest, evidence, location, conclusion, limits, unknowns):
    return dict(id=identifier, question=question, artifact_sha256=digest, evidence_id=evidence,
                location=location, evidence_class="static-analysis", conclusion=conclusion,
                limitations=limits, unknowns=unknowns)


def main():
    ledger = [
        finding("engine-range", "How are HTTP ranges built?", ENGINE,
                "ev_d7aca5af011cb3debb8cba2457f8315b6bec33bb7a174c06c1a7d9c924b5c566", "0x589430 / native/0x589430.c",
                "Request construction contains bounded/open-ended Range formats and a send call; caller 0x5abda0.",
                "Decompiler types/offsets are uncertain; indirect flows may be incomplete.", "Exact partition allocation/reassignment policy."),
        finding("engine-headers", "Are range metadata and validators parsed?", ENGINE,
                "ev_6a698bc609ab4404ddaa417540036e53334d5c3c707a261178074fa8829e521f", "0x58a880 / native/0x58a880.c",
                "Response handler branches on Content-Range, ETag, Last-Modified, Location and transfer/content encoding.",
                "Parsing presence does not prove validation policy. Dossier delivered through retained view.", "Full malformed-response and resume decision paths."),
        finding("incoming-filter", "Does an If-Range string mean conditional resumes are sent?", ENGINE,
                "ev_73fc19b173c0231cbf94b2358541fe8865f5abe6ccbd11126f61dfb4a4182987", "0x5d99a0 / native/0x5d99a0.c",
                "This occurrence filters incoming request-context headers; no outgoing If-Range claim follows.",
                "Only inspected path covered; runtime logger records three validator headers.", "Other conditional-header construction paths."),
        finding("temp-path", "Where are temporary download paths represented?", ENGINE,
                "ev_6667a925440caa2f9e8fc26803a3d403d92654f2bae6da9de2dbdd2741d23951", "0x46e6e0 / native/0x46e6e0.c",
                "Queries TempPath and constructs DwnlData/user paths.", "Path handling is not a recovered checkpoint format.", "Exact test checkpoint identity, encoding and write durability."),
        finding("queue-store", "Is queue state persisted?", ENGINE,
                "ev_4260206edd9526ef507ad37671e34e2015d66acbe5be12a6d75a7d59df5f4da6", "0x49d300 / native/0x49d300.c",
                "Writes Queue under HKCU Software/DownloadManager/Queue.", "No private registry history values copied.", "Scheduler policy and interaction with per-server limits."),
        finding("limiter-store", "How is the limiter setting stored?", ENGINE,
                "ev_72edfd2acc00b355adedd60e01c6155a077aca40ecf60dd5456c202d9c02571b", "0x5b7ef0 / native/0x5b7ef0.c",
                "Writes/deletes bUseSpeedLimiter and psnSpeedLimit through an object registry handle.",
                "Units and algorithm are not established by names or storage lengths.", "Achieved bandwidth, aggregation and limiter algorithm."),
        finding("host-bridge", "What transport follows browser native messaging?", HOST,
                "ev_a0f115b1e65bb889cc2727d8657bc1ec542fee061f5f98fd87480246adbee8b3", "0x402830 / native/host-0x402830.c",
                "Opens session-specific IDMNetworkMonitor named pipe and converts quoted browser string framing to inner records and back.",
                "Static path only; no live host frames captured; pointer types recovered imperfectly.", "Manager-side dispatch and origin validation."),
        finding("host-init", "Is the host initial record separate from extension handshake?", HOST,
                "ev_864c4fcc246ee1eaf48477b482d3ef3575e93308db3594f9bad33b0fc4f2d922", "0x403530 / native/host-0x403530.c",
                "Constructs MSG#0#17#1#0:9:<process-id>; with a length prefix.",
                "Process ID is derived using peer/parent process logic; acceptance not observed.", "Complete semantics of type17 and manager response."),
    ]
    browser_ledger = ROOT / "browser" / "finding-ledger.json"
    if browser_ledger.exists():
        ledger.append({"id": "browser-source-findings", "question": "How do browser events reach IDM?",
                       "evidence_class": "static-source-analysis", "ledger": "browser/finding-ledger.json",
                       "artifact_sha256": "8ac82b1abdfa5d1fac79fdf8511f1d86a7e7fb7a02d32e4a01e52b64d85fa016",
                       "evidence_id": "ev_5708179b68eba5330e262acdc6b86c567988c96e749c23f39bfca2ac19add676",
                       "location": "browser/extension/background.js:11-16,37-41,45,99-121",
                       "conclusion": "WebSocket preference and native messaging fallback carry a custom textual inner protocol with request context.",
                       "limitations": "REA semantic analysis partial after worker heap exhaustion; detailed trace supported by preserved source.",
                       "unknowns": "Selected live transport and many negotiated flags."})
    ledger.append({"id": "runtime-findings", "question": "Which behavior was exercised and which outputs matched?",
                   "artifact_sha256": ENGINE, "evidence_class": "endpoint-file-process-observations-plus-user-report",
                   "location": "runtime/runtime-results.json; runtime/user-observations.json; docs/idm-runtime-results.md",
                   "evidence_id": None, "conclusion": "See per-case request details and per-file hashes; no runtime record is assigned a fabricated REA ID.",
                   "limitations": "Periodic payload, buffered writes, manual retries and incomplete transport attribution.",
                   "unknowns": "Precise retry policy, exact throttle enforcement and clean two-ready-job queue scheduling."})
    (ROOT / "finding-ledger.json").write_text(json.dumps(ledger, indent=2), encoding="utf-8")
    paths = [ROOT / "artifacts.json", ROOT / "finding-ledger.json"]
    paths += list(ROOT.glob("*.py"))
    for folder in ("native", "browser", "runtime"):
        paths += [p for p in (ROOT / folder).glob("*") if p.is_file() and p.name not in LOCAL_ONLY_RUNTIME and p.suffix in {".json", ".jsonl", ".py", ".c"}]
    paths += [p for p in (ROOT / "browser" / "extension").rglob("*") if p.is_file()]
    entries = []
    for path in sorted(set(paths)):
        data = path.read_bytes()
        entries.append(dict(path=path.relative_to(REPO).as_posix(), bytes=len(data), sha256=hashlib.sha256(data).hexdigest()))
    result = dict(format="rusty-idm-investigation-evidence-index-v1", files=entries,
                  exclusions=["disposable browser profile", "downloaded payloads (hashes in runtime-results.json)", "temporary PID and process logs", "machine-specific browser launch and submission records"],
                  bundles=["native/engine-evidence.json", "native/host-evidence.json"],
                  snapshots=["native/engine-snapshot.json", "native/host-snapshot.json"],
                  interpretation="File identity does not authenticate publisher or prove runtime execution.")
    (ROOT / "evidence-index.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps({"findings": len(ledger), "indexed_files": len(entries)}))


if __name__ == "__main__":
    main()
