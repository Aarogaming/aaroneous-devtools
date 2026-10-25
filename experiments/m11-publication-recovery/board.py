"""Local discussion transport; no model dispatch, source edits or approval."""
import argparse
import hashlib
import heapq
import json
import math
import os
import secrets
import time
from recovery import inspect_pending
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

LIMIT = 16384
PAGE_MAX = 128
KINDS = {"idea", "question", "concern", "evidence", "synthesis"}
ACTORS = {"Owner", "Codex host", "Codex compliance", "Codex coordination",
          "Codex reuse research", "Codex architecture research",
          "Claude", "MiMo (OpenCode)", "Gemini", "Local Qwen", "Guest"}

def load_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))

def atomic_json(path, data):
    temporary = path.with_name("." + path.name + "." + secrets.token_hex(6) + ".tmp")
    with temporary.open("x", encoding="utf-8", newline="\n") as handle:
        json.dump(data, handle, ensure_ascii=False, indent=2, allow_nan=False)
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temporary, path)

class PublicationFailure(OSError):
    def __init__(self, report):
        super().__init__("publication requires recovery; inspect retained evidence")
        self.publication = report

class Board:
    def __init__(self, root, clock, config, legacy_deadline=None):
        self.root, self.clock, self.config = root, clock, config
        self.legacy_deadline = legacy_deadline
        self.token = secrets.token_hex(24)
        self.entries = root / "entries"
        self.entries.mkdir(exist_ok=True)
        self.count = self.used_bytes = self.last_id = 0
        self.publication_failed = False
        self.pending_recovery = inspect_pending(root)
        self.publication_failed = bool(self.pending_recovery["records"])
        self.recount_published()

    def recount_published(self):
        self.count = self.used_bytes = self.last_id = 0
        for path in self.entries.glob("*.json"):
            size = path.stat().st_size
            if size > LIMIT * 2:
                raise ValueError("stored entry exceeds bound")
            record = load_json(path)
            identity = record.get("id")
            if type(identity) is not int or identity < 1:
                raise ValueError("stored entry identity invalid")
            prefix = path.name.split("-", 1)[0]
            if prefix != ("%04d" % identity):
                raise ValueError("entry filename/identity mismatch")
            if sum(1 for _ in self.entries.glob(prefix + "-*.json")) != 1:
                raise ValueError("duplicate stored identity; owner recovery required")
            self.count += 1
            self.used_bytes += size
            self.last_id = max(self.last_id, identity)
        if self.used_bytes > self.config["max_storage_bytes"]:
            raise ValueError("retained data exceeds configured storage budget")

    def session_state(self):
        data = load_json(self.root / "session.json")
        lifecycle = data.get("lifecycle", {"state": "open", "closes_at": data.get("deadline_epoch")})
        state, deadline = lifecycle.get("state"), lifecycle.get("closes_at")
        if state not in {"open", "paused", "closed"}:
            raise ValueError("unsupported session state")
        if self.legacy_deadline is not None:
            deadline = self.legacy_deadline
        if deadline is not None and (isinstance(deadline, bool) or
                not isinstance(deadline, (int, float)) or not math.isfinite(deadline)):
            raise ValueError("invalid session deadline")
        closed = state != "open" or (deadline is not None and self.clock() >= deadline)
        return data, state, deadline, closed

    def intake_flags(self, closed):
        # Eligibility is a known precondition, never a guarantee of durable append.
        return {"lifecycle_open": not closed,
                "publication_hold": self.publication_failed,
                "intake_open": not closed and not self.publication_failed,
                "publication_recovery": self.pending_recovery}

    def snapshot(self, before=None, limit=64):
        data, state, deadline, closed = self.session_state()
        def candidates():
            for path in self.entries.glob("*.json"):
                identity = int(path.name.split("-", 1)[0])
                if before is None or identity < before:
                    yield identity, path
        selected = heapq.nlargest(limit + 1, candidates(), key=lambda item: item[0])
        more = len(selected) > limit
        selected = selected[:limit]
        records = [load_json(path) for _, path in reversed(selected)]
        data.update(entries=records, closed=closed, session_state=state,
                    token=self.token, deadline_epoch=deadline, contract_version=4,
                    total_entries=self.count,
                    pagination={"limit": limit, "has_older": more,
                                "older_cursor": min((i for i, _ in selected), default=None) if more else None},
                    storage={"used_bytes": self.used_bytes,
                             "budget_bytes": self.config["max_storage_bytes"]},
                    **self.intake_flags(closed))
        return data

    def append(self, data):
        if self.publication_failed:
            raise OSError("publication held after prior failure; inspect retained evidence")
        if self.session_state()[3]:
            raise ValueError("session intake paused or closed; history remains readable")
        if not isinstance(data, dict) or set(data) != {"actor", "kind", "text", "evidence"}:
            raise ValueError("exact contribution fields required")
        if not isinstance(data["actor"], str) or data["actor"] not in ACTORS:
            raise ValueError("unknown participant")
        if not isinstance(data["kind"], str) or data["kind"] not in KINDS:
            raise ValueError("unknown contribution kind")
        if not isinstance(data["text"], str) or not 1 <= len(data["text"].strip()) <= 10000:
            raise ValueError("contribution requires 1-10000 characters")
        if not isinstance(data["evidence"], str) or len(data["evidence"]) > 3000:
            raise ValueError("evidence references exceed bound")
        number = self.last_id + 1
        record = dict(data, id=number, received_at_epoch=self.clock(),
                      attribution="self-reported discussion label, not authenticated review identity",
                      accepted=False, status="advisory-unverified")
        payload = json.dumps(record, ensure_ascii=False, indent=2, allow_nan=False).encode("utf-8")
        if len(payload) > LIMIT * 2:
            raise ValueError("entry byte bound exceeded")
        if self.used_bytes + len(payload) > self.config["max_storage_bytes"]:
            raise ValueError("storage budget reached; history retained, no automatic eviction")
        path = self.entries / ("%04d-%s.json" % (number, secrets.token_hex(6)))
        temporary = self.entries / ("." + path.name + ".pending")
        try:
            with temporary.open("xb") as handle:
                handle.write(payload)
                handle.flush()
                os.fsync(handle.fileno())
            # Exclusive publication: an existing destination is never replaced.
            os.link(temporary, path)
            temporary.unlink()
        except OSError as exc:
            self.publication_failed = True
            self.pending_recovery = inspect_pending(self.root)
            # Publication may have succeeded even when cleanup failed.
            # Counters describe retained published records, not HTTP success.
            try:
                self.recount_published()
            except (OSError, ValueError, TypeError):
                pass  # Keep the hold and original failure; never reopen intake.
            raise PublicationFailure(self.pending_recovery) from exc
        self.count += 1
        self.last_id = number
        self.used_bytes += len(payload)
        return {"id": number, "sha256": hashlib.sha256(payload).hexdigest(),
                "accepted": False, "contract_version": 4}

def handler_for(board, origin):
    class Handler(BaseHTTPRequestHandler):
        def setup(self):
            super().setup()
            self.connection.settimeout(10)

        def respond(self, status, body, mime="application/json"):
            raw = body if isinstance(body, bytes) else json.dumps(body, allow_nan=False).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", mime + "; charset=utf-8")
            self.send_header("Content-Length", str(len(raw)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.send_header("Content-Security-Policy",
                             "default-src 'self'; script-src 'self'; style-src 'self'; "
                             "connect-src 'self'; frame-ancestors 'none'; base-uri 'none'")
            self.end_headers()
            self.wfile.write(raw)

        def valid_host(self):
            return self.headers.get("Host") == origin.removeprefix("http://")

        def do_GET(self):
            if not self.valid_host():
                return self.respond(403, {"error": "loopback host required"})
            parsed = urlsplit(self.path)
            try:
                if parsed.path == "/api/session":
                    query = parse_qs(parsed.query, keep_blank_values=True, strict_parsing=True)
                    if set(query) - {"before", "limit"} or any(len(v) != 1 for v in query.values()):
                        raise ValueError("unsupported pagination fields")
                    before = int(query["before"][0]) if "before" in query else None
                    limit = int(query.get("limit", ["64"])[0])
                    if not 1 <= limit <= PAGE_MAX or (before is not None and before < 1):
                        raise ValueError("pagination out of bounds")
                    return self.respond(200, board.snapshot(before, limit))
                if parsed.path == "/api/health":
                    _, state, deadline, closed = board.session_state()
                    return self.respond(200, {"ready": not board.publication_failed, "model_dispatch": False,
                                             "contract_version": 4, "total_entries": board.count,
                                             "session_state": state, "deadline_epoch": deadline,
                                             **board.intake_flags(closed)})
                files = {"/": ("board.html", "text/html"),
                         "/board.js": ("board.js", "text/javascript"),
                         "/board.css": ("board.css", "text/css"),
                         "/invite": ("INVITATION.md", "text/plain")}
                if parsed.path in files:
                    name, mime = files[parsed.path]
                    return self.respond(200, (board.root / name).read_bytes(), mime)
                return self.respond(404, {"error": "route not found"})
            except (ValueError, TypeError) as exc:
                return self.respond(400, {"error": str(exc)})
            except OSError:
                return self.respond(503, {"error": "retained data unavailable; owner recovery required"})

        def do_POST(self):
            if not self.valid_host() or self.path != "/api/entries":
                return self.respond(403, {"error": "contribution route required"})
            if self.headers.get("Origin") not in (None, origin):
                return self.respond(403, {"error": "same-origin contribution required"})
            if not secrets.compare_digest(self.headers.get("X-Board-Token", ""), board.token):
                return self.respond(403, {"error": "local board token required"})
            try:
                length = int(self.headers.get("Content-Length", "0"))
                if not 0 < length <= LIMIT:
                    return self.respond(413, {"error": "contribution byte bound exceeded"})
                if self.headers.get_content_type() != "application/json":
                    return self.respond(415, {"error": "JSON contribution required"})
                raw = self.rfile.read(length)
                if len(raw) != length:
                    raise ValueError("incomplete contribution")
                return self.respond(201, board.append(json.loads(raw)))
            except (ValueError, TypeError) as exc:
                return self.respond(400, {"error": str(exc)})
            except OSError as exc:
                return self.respond(503, {
                    "error": "publication held; inspect recovery evidence; do not blindly retry",
                    "retry_allowed": False, "contract_version": 4,
                    "publication_recovery": getattr(exc, "publication", board.pending_recovery)})

    return Handler

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    parser.add_argument("--support-directory", required=True)
    parser.add_argument("--port", type=int)
    parser.add_argument("--deadline-epoch", type=float)
    args = parser.parse_args()
    root = Path(args.root)
    authorized = Path(args.support_directory)
    if not authorized.is_absolute() or ".." in authorized.parts:
        raise ValueError("explicit absolute support directory required")
    if not root.is_absolute() or ".." in root.parts or not root.is_relative_to(authorized):
        raise ValueError("explicit runtime discussion root required")
    config = load_json(root / "board-config.json")
    if config.get("schema_version") != 2 or type(config.get("max_storage_bytes")) is not int:
        raise ValueError("unsupported server configuration")
    if not 32768 <= config["max_storage_bytes"] <= 1024 * 1024 * 1024:
        raise ValueError("storage budget out of bounds")
    port = args.port if args.port is not None else config["port"]
    if type(port) is not int or not 1024 <= port <= 65535:
        raise ValueError("explicit local port required")
    board = Board(root, time.time, config, args.deadline_epoch)
    http = HTTPServer(("127.0.0.1", port), BaseHTTPRequestHandler)
    http.timeout = 1
    origin = "http://127.0.0.1:%d" % port
    http.RequestHandlerClass = handler_for(board, origin)
    atomic_json(root / "board-runtime.json", {"url": origin, "worker_pid": os.getpid(),
                "ordinary_process": True, "model_dispatch": False, "contract_version": 4,
                "deadline_epoch": board.session_state()[2]})
    print("Local discussion board: " + origin, flush=True)
    try:
        while load_json(root / "board-control.json").get("state") == "run":
            http.handle_request()
    finally:
        http.server_close()

if __name__ == "__main__":
    main()
