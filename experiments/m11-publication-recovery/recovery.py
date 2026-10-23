"""Staged board recovery adapter. Explicit stopped-root recovery only; no replay."""
import argparse
import hashlib
import json
import math
import os
import re
from contextlib import contextmanager
from pathlib import Path

BOUND = 32768
NAME = re.compile(r"^\.([0-9]{4,}-[0-9a-f]{12}\.json)\.pending$")
FIELDS = {"actor", "kind", "text", "evidence", "id", "received_at_epoch",
          "attribution", "accepted", "status"}
ACTORS = {"Owner", "Codex host", "Codex compliance", "Codex coordination",
          "Codex reuse research", "Codex architecture research", "Claude",
          "MiMo (OpenCode)", "Gemini", "Local Qwen", "Guest"}

def digest(raw):
    return hashlib.sha256(raw).hexdigest()

def bounded_bytes(path):
    if path.is_symlink() or not path.is_file():
        raise ValueError("nonregular or linked evidence")
    with path.open("rb") as handle:
        raw = handle.read(BOUND + 1)
    if len(raw) > BOUND:
        raise ValueError("entry exceeds byte bound")
    return raw

def validated_record(raw, basename):
    def unique(pairs):
        out = {}
        for key, value in pairs:
            if key in out:
                raise ValueError("duplicate JSON field")
            out[key] = value
        return out
    obj = json.loads(raw.decode("utf-8"), object_pairs_hook=unique)
    if not isinstance(obj, dict) or set(obj) != FIELDS:
        raise ValueError("unexpected record fields")
    identity = obj["id"]
    if type(identity) is not int or identity < 1 or basename.split("-", 1)[0] != f"{identity:04d}":
        raise ValueError("record identity mismatch")
    stamp = obj["received_at_epoch"]
    if type(stamp) not in (int, float) or not math.isfinite(stamp):
        raise ValueError("invalid timestamp")
    if (obj["actor"] not in ACTORS or
            obj["kind"] not in {"idea", "question", "concern", "evidence", "synthesis"} or
            not isinstance(obj["text"], str) or not 1 <= len(obj["text"].strip()) <= 10000 or
            not isinstance(obj["evidence"], str) or len(obj["evidence"]) > 3000 or
            obj["accepted"] is not False or obj["status"] != "advisory-unverified" or
            obj["attribution"] != "self-reported discussion label, not authenticated review identity"):
        raise ValueError("invalid advisory record")
    return obj

def inspect_pending(root):
    """Read-only classification. No absence is interpreted as permission to replay."""
    root = Path(root)
    entries = root / "entries"
    if root.is_symlink() or entries.is_symlink():
        raise ValueError("linked root/entries unsupported")
    result = []
    for pending in sorted(entries.glob(".*.pending")):
        row = {"pending": pending.name, "classification": "conflicting-or-invalid"}
        try:
            match = NAME.fullmatch(pending.name)
            if not match:
                raise ValueError("invalid pending filename")
            basename = match[1]
            raw = bounded_bytes(pending)
            row["sha256"] = digest(raw)
            obj = validated_record(raw, basename)
            row.update(id=obj["id"], published=basename)
            prefix = f"{obj['id']:04d}-"
            siblings = list(entries.glob(prefix + "*.json"))
            same_id_pending = list(entries.glob("." + prefix + "*.json.pending"))
            if len(same_id_pending) != 1:
                raise ValueError("multiple pending records for one identity")
            destination = entries / basename
            if destination.is_symlink():
                raise ValueError("linked published record")
            if not destination.exists():
                if siblings:
                    raise ValueError("identity already published at another filename")
                row["classification"] = "unpublished"
            else:
                if siblings != [destination]:
                    raise ValueError("duplicate published identity")
                published = bounded_bytes(destination)
                validated_record(published, basename)
                if raw != published:
                    raise ValueError("published and pending bytes differ")
                row["classification"] = "published-matching"
        except (OSError, ValueError, TypeError, UnicodeError) as exc:
            row["reason"] = str(exc)
        result.append(row)
    return {"schema_version": 1, "records": result, "retry_allowed": False}

def plan_digest(plan):
    return digest(json.dumps(plan, sort_keys=True, separators=(",", ":")).encode())

@contextmanager
def stopped_lease(root):
    """Use the ordinary host's existing Windows byte lock; never stop it here."""
    import msvcrt
    root = Path(root)
    if not root.is_absolute() or ".." in root.parts or root.is_symlink():
        raise ValueError("explicit absolute unlinked root required")
    control = root / "board-control.json"
    lock = root / "board-host.lock"
    if control.is_symlink() or lock.is_symlink():
        raise ValueError("linked control/lease unsupported")
    # Require an existing host lock; recovery must not fabricate runtime ownership.
    with lock.open("r+b") as handle:
        handle.seek(0)
        msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
        try:
            if json.loads(control.read_text(encoding="utf-8-sig")).get("state") != "stop":
                raise ValueError("explicitly stopped board required")
            yield
        finally:
            handle.seek(0)
            msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)

def reconcile_matching(root, expected_plan_digest):
    """Retain pending bytes before cleanup, under an exclusive stopped-host lease.

    Unpublished and conflicting records block the entire plan. No replay,
    published replacement, record deletion or intake reopening is performed.
    """
    root = Path(root)
    with stopped_lease(root):
        plan = inspect_pending(root)
        if plan_digest(plan) != expected_plan_digest:
            raise ValueError("stale recovery plan")
        if any(row["classification"] != "published-matching" for row in plan["records"]):
            raise ValueError("unpublished/conflicting evidence requires owner resolution")
        archive = root / "recovery-evidence"
        if archive.is_symlink():
            raise ValueError("linked recovery archive unsupported")
        archive.mkdir(exist_ok=True)
        for row in plan["records"]:
            pending = root / "entries" / row["pending"]
            published = root / "entries" / row["published"]
            raw = bounded_bytes(pending)
            if digest(raw) != row["sha256"] or bounded_bytes(published) != raw:
                raise ValueError("evidence changed since classification")
            retained = archive / (row["pending"] + "." + row["sha256"] + ".retained")
            # Copy with exclusive creation and fsync. Never overwrite prior evidence.
            if retained.exists() or retained.is_symlink():
                if bounded_bytes(retained) != raw:
                    raise ValueError("conflicting archive evidence")
                # An earlier attempt could have failed on its archive flush.
                # Re-establish the durability precondition before unlinking.
                with retained.open("r+b") as handle:
                    os.fsync(handle.fileno())
            else:
                with retained.open("xb") as handle:
                    handle.write(raw)
                    handle.flush()
                    os.fsync(handle.fileno())
            if bounded_bytes(retained) != raw or bounded_bytes(published) != raw:
                raise ValueError("retention verification failed")
            pending.unlink()
        return {"schema_version": 1, "reconciled": plan["records"],
                "published_records_changed": False, "control_changed": False,
                "retry_allowed": False}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True)
    parser.add_argument("--reconcile-matching", metavar="EXPECTED_PLAN_SHA256")
    args = parser.parse_args()
    root = Path(args.root)
    if not root.is_absolute() or ".." in root.parts:
        raise ValueError("explicit absolute root required")
    if args.reconcile_matching:
        report = reconcile_matching(root, args.reconcile_matching)
    else:
        report = inspect_pending(root)
        report["plan_sha256"] = plan_digest(report)
    print(json.dumps(report, indent=2))

if __name__ == "__main__":
    main()
