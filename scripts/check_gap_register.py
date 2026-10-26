#!/usr/bin/env python3
"""Deterministic policy checker for governance/FOUNDRY_GAP_REGISTER.md (C83).

Mechanizes the governance gap-to-lane rule (G-2026-05, C83):
1. Every open register entry must cite concrete evidence (source path, commit,
   command result, or committed review).
2. Every entry must declare an affected boundary and a stated risk.
3. Every entry must declare a valid disposition (a corrective queue lane,
   a blocked owner decision, or an explicit rejection).
4. Verifies queue lane existence and prevents false Done linkages (claiming a lane
   is done/accepted when the queue row is not Done, or leaving an entry marked
   Ready/Claimed/Review after the corrective lane is marked Done).
5. Does not infer task ownership or auto-create claims.

Usage:
    check_gap_register.py [--register PATH] [--queue PATH]
"""

import argparse
import os
import re
import sys

DEFAULT_REGISTER_PATH = "governance/FOUNDRY_GAP_REGISTER.md"
DEFAULT_QUEUE_PATH = "governance/COORDINATION_QUEUE.md"

GAP_HEADING_RE = re.compile(r"^##\s+(G-\d{4}-\d{2,})\s+[—–-]\s*(.+)$")
LANE_ID_RE = re.compile(r"\b(C\d+)\b")
BULLET_RE = re.compile(r"^\s*-\s*([A-Za-z]+):\s*(.*)$")


def split_cells(row: str) -> list[str]:
    """Split markdown table row honouring backslash-escaped pipes."""
    cells, current, i = [], [], 0
    while i < len(row):
        char = row[i]
        following = row[i + 1] if i + 1 < len(row) else ""
        if char == "\\" and following in ("|", "\\"):
            current.append("|" if following == "|" else "\\\\")
            i += 2
            continue
        if char == "|":
            cells.append("".join(current))
            current = []
        else:
            current.append(char)
        i += 1
    cells.append("".join(current))
    return cells


def parse_queue(queue_path: str) -> dict[str, str]:
    """Extract {lane_id: status} from the coordination queue markdown table."""
    if not os.path.isfile(queue_path):
        raise FileNotFoundError(f"Coordination queue not found: {queue_path}")

    queue_lanes = {}
    with open(queue_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line.startswith("|"):
                continue
            cols = [c.strip() for c in split_cells(line)]
            # Needs at least 5 cols: ['', ID, Scope, Status, Owner, ...]
            if len(cols) >= 5 and re.fullmatch(r"C\d+", cols[1]):
                queue_lanes[cols[1]] = cols[3]
    return queue_lanes


def parse_register(register_path: str) -> list[dict]:
    """Parse FOUNDRY_GAP_REGISTER.md into structured gap entry dicts."""
    if not os.path.isfile(register_path):
        raise FileNotFoundError(f"Foundry gap register not found: {register_path}")

    entries = []
    current_entry = None
    current_field = None

    with open(register_path, "r", encoding="utf-8") as f:
        for line_num, line in enumerate(f, start=1):
            stripped = line.strip()
            heading_match = GAP_HEADING_RE.match(stripped)
            if heading_match:
                if current_entry:
                    entries.append(current_entry)
                current_entry = {
                    "id": heading_match.group(1),
                    "title": heading_match.group(2).strip(),
                    "line": line_num,
                    "boundary": "",
                    "evidence": "",
                    "risk": "",
                    "disposition": "",
                }
                current_field = None
                continue

            if current_entry is None:
                continue

            bullet_match = BULLET_RE.match(line)
            if bullet_match:
                field_name = bullet_match.group(1).lower()
                field_value = bullet_match.group(2).strip()
                if field_name in ("boundary", "evidence", "risk", "disposition"):
                    current_field = field_name
                    current_entry[current_field] = field_value
                else:
                    current_field = None
            elif current_field and stripped:
                # Continuation line for the current bullet
                current_entry[current_field] += " " + stripped

    if current_entry:
        entries.append(current_entry)

    return entries


def has_concrete_evidence(text: str) -> bool:
    """Evidence must cite a source path, commit SHA, command result, or review."""
    if not text:
        return False
    stripped = text.strip()
    if len(stripped) < 8 or stripped.lower() in ("tbd", "none", "todo", "n/a", "unspecified"):
        return False
    # Look for paths, backticks, line ranges, commits, or command markers
    evidence_indicators = re.compile(
        r"(`[^`]+`|[a-zA-Z0-9_\-\.\/]+\.[a-zA-Z0-9]{1,4}(?::\d+(?:-\d+)?)?|"
        r"\b[0-9a-f]{7,40}\b|\bexit\s*\d+\b|\bPR\s*#?\d+\b|\b[A-Z]\d+\b|"
        r"\b(?:(?:[a-zA-Z]+-)?(?:component|code|architecture|peer|steward|guild)\s+review|review\s+(?:by|from|thread|#\d+|at|comment|finding|established)|command\s+(?:exit|output|result|run|`)|observed\s+(?:in|at|by|exit|output))\b)",
        re.IGNORECASE,
    )
    return bool(evidence_indicators.search(stripped))


def validate_entry(entry: dict, queue_lanes: dict[str, str]) -> list[str]:
    """Validate a single gap register entry against policy and coordination queue."""
    errors = []
    gap_id = entry.get("id", "UNKNOWN")

    # 1. Boundary check
    boundary = entry.get("boundary", "").strip()
    if not boundary:
        errors.append(f"{gap_id}: missing boundary")

    # 2. Evidence check
    evidence = entry.get("evidence", "").strip()
    if not evidence or not has_concrete_evidence(evidence):
        errors.append(f"{gap_id}: missing evidence: must cite source path, commit, command result, or review")

    # 3. Risk check
    risk = entry.get("risk", "").strip()
    if not risk:
        errors.append(f"{gap_id}: missing risk")

    # 4. Disposition check
    disposition = entry.get("disposition", "").strip()
    if not disposition:
        errors.append(f"{gap_id}: missing disposition")
        return errors

    # Check for valid disposition categories:
    # - Corrective lane (C\d+)
    # - Blocked on owner / decision
    # - Explicit rejection (Rejected)
    # - Resolved / Accepted
    has_lane = bool(LANE_ID_RE.search(disposition))
    has_blocked = bool(re.search(r"\bblocked\b", disposition, re.IGNORECASE))
    has_rejected = bool(re.search(r"\brejected\b", disposition, re.IGNORECASE))
    has_resolved = bool(re.search(r"\b(accepted|resolved|done|superseded)\b", disposition, re.IGNORECASE))

    if not (has_lane or has_blocked or has_rejected or has_resolved):
        errors.append(
            f"{gap_id}: invalid disposition: must cite a corrective queue lane (e.g. C83), a blocked decision, or an explicit rejection"
        )

    # Cross-reference referenced queue lanes
    referenced_lanes = LANE_ID_RE.findall(disposition)
    for lane_id in referenced_lanes:
        if lane_id not in queue_lanes:
            errors.append(f"{gap_id}: disposition references lane {lane_id} not found in coordination queue")
            continue

        q_status = queue_lanes[lane_id]

        # False Done linkage check:
        # If disposition claims lane is Done/Accepted/Resolved, but queue is not Done:
        done_claim_re = re.compile(
            rf"\b{lane_id}\s+(?:Done|Accepted|Resolved)\b|(?:\b(?:resolved|accepted|closed|fixed)\s+by\s+{lane_id}\b)",
            re.IGNORECASE,
        )
        if done_claim_re.search(disposition) and q_status != "Done":
            errors.append(
                f"{gap_id}: false Done linkage: disposition claims {lane_id} is completed/accepted, but queue status is '{q_status}'"
            )

        # Unupdated entry check:
        # If queue status is Done, but disposition still marks that lane as Ready or Claimed:
        pending_claim_re = re.compile(rf"\b{lane_id}\s+(?:Ready|Claimed|Review)\b", re.IGNORECASE)
        if pending_claim_re.search(disposition) and q_status == "Done":
            errors.append(
                f"{gap_id}: unupdated entry: corrective lane {lane_id} is Done in queue, but disposition still marks it as Ready/Claimed/Review"
            )

    return errors


def check(register_path: str = DEFAULT_REGISTER_PATH, queue_path: str = DEFAULT_QUEUE_PATH) -> list[str]:
    """Execute all gap register validations. Returns list of error messages."""
    try:
        queue_lanes = parse_queue(queue_path)
    except Exception as exc:
        return [f"Failed to read coordination queue ({queue_path}): {exc}"]

    try:
        entries = parse_register(register_path)
    except Exception as exc:
        return [f"Failed to read gap register ({register_path}): {exc}"]

    if not entries:
        return [f"{register_path}: no gap entries found (register must not be empty)"]

    all_errors = []
    seen_ids = set()

    for entry in entries:
        gap_id = entry.get("id")
        if gap_id in seen_ids:
            all_errors.append(f"{gap_id}: duplicate gap identifier in register")
        seen_ids.add(gap_id)

        all_errors.extend(validate_entry(entry, queue_lanes))

    return all_errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--register", default=DEFAULT_REGISTER_PATH, help="Path to FOUNDRY_GAP_REGISTER.md")
    parser.add_argument("--queue", default=DEFAULT_QUEUE_PATH, help="Path to COORDINATION_QUEUE.md")
    args = parser.parse_args()

    errors = check(args.register, args.queue)
    if errors:
        print("ERROR: Foundry gap register consistency check failed:", file=sys.stderr)
        for err in errors:
            print(f"  {err}", file=sys.stderr)
        return 1

    entries = parse_register(args.register)
    print(f"{args.register}: OK ({len(entries)} gap entry/entries verified against queue)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
