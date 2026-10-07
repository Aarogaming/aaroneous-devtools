#!/usr/bin/env python3
"""Evidence-retention gate for governance/COORDINATION_QUEUE.md.

Enforces the "Standing rule: evidence retention" section of that file: any
newly added line or block that claims Done/PASS/"gate green" must also carry
something that looks like real evidence (a commit hash, an exit code, a log
path, or a fenced command block) in the same row or block. This is a
heuristic nudge, not a semantic judge -- it exists to make the exact failure
pattern documented in AUDIT_2026-09-26_DOCUMENTATION_AND_PROCESS.md (Finding
5: repeated unevidenced "Done"/"gate PASS" claims) fail a CI check instead of
waiting for the next reviewer to notice by hand.

It also rejects an added or edited lane row whose status is not in governance/QUEUE_STATUSES.json. The native
queue parser (foundation_cli multithreader.rs queue_ids) silently skips a row with any other status, so such a row
never enters its existing-ID set; the check reads the rule the same way the parser does (split on every `|`).

Usage: check_coordination_evidence.py <base-ref> <head-ref>
Exits 1 and prints each offending line/block if a claim lacks evidence or a lane row uses an unrecognised status.
"""

import json
import re
import subprocess
import sys

TARGET_FILE = "governance/COORDINATION_QUEUE.md"
VOCABULARY_FILE = "governance/QUEUE_STATUSES.json"
RUST_PARSER = "foundation/crates/foundation_cli/src/multithreader.rs"
RUST_STATUS_LIST = re.compile(r'matches!\(\s*cols\[3\]\s*,((?:\s*"[A-Za-z ]+"\s*\|?)+)\)')
LANE_ID = re.compile(r"C\d+")

CLAIM_TABLE_ROW = re.compile(r"^\+\s*\|.*\|\s*Done\s*\|", re.IGNORECASE)
CLAIM_PHRASES = re.compile(r"\b(Done|PASS(ED)?|gate green|100%\s*green)\b", re.IGNORECASE)
EVIDENCE_MARKERS = re.compile(
    r"(\b[0-9a-f]{7,40}\b|exit code|\.log\b|```|`cargo )", re.IGNORECASE
)
HEADING_RE = re.compile(r"^\+(#{2,4})\s")


def get_diff(base: str, head: str) -> str:
    # git prints UTF-8. Decoding with the platform default (cp1252 on Windows) failed on bytes such as the one in a
    # right double quote (U+201D), the read thread died, stdout came back empty and the check reported "nothing to
    # check" with exit code 0, so an unevidenced Done row containing that character passed.
    result = subprocess.run(
        ["git", "diff", f"{base}...{head}", "--", TARGET_FILE],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=True,
    )
    return result.stdout


def lane_status(row: str):
    """(lane id, status) for a lane row, read exactly as multithreader.rs queue_ids reads it: the row must start with
    `|`, is split on every `|`, needs at least five columns, the id is column 1 and the status column 3. None for
    anything that is not a lane row (headers, separators, prose)."""
    if not row.startswith("|"):
        return None
    cols = [c.strip() for c in row.split("|")]
    if len(cols) < 5 or not LANE_ID.fullmatch(cols[1]):
        return None
    return cols[1], cols[3]


def status_violations(diff_lines: list[str], statuses) -> list[tuple]:
    """Added lane rows whose status is not one of `statuses`: (lane id, status, row). Removed and context lines
    are ignored, so an untouched row is never flagged, but editing one means fixing its status (touch it, fix it)."""
    found = []
    for line in diff_lines:
        if not line.startswith("+") or line.startswith("+++"):
            continue
        parsed = lane_status(line[1:])
        if parsed and parsed[1] not in statuses:
            found.append((parsed[0], parsed[1], line[1:]))
    return found


def parse_rust_statuses(text: str):
    """The status words in the native parser's `matches!(cols[3], ...)`, or None when that shape is not found."""
    match = RUST_STATUS_LIST.search(text)
    return re.findall(r'"([A-Za-z ]+)"', match.group(1)) if match else None


def rust_statuses(head: str):
    """The parser's status list at `head`, or None when the file or the expected shape cannot be read."""
    result = subprocess.run(["git", "show", f"{head}:{RUST_PARSER}"], capture_output=True, text=True)
    return parse_rust_statuses(result.stdout) if result.returncode == 0 else None


def load_statuses(head: str):
    """The allowed status words at `head`, or None when that revision has no readable vocabulary file."""
    result = subprocess.run(["git", "show", f"{head}:{VOCABULARY_FILE}"], capture_output=True, text=True)
    if result.returncode != 0:
        return None
    try:
        statuses = json.loads(result.stdout)["statuses"]
    except (ValueError, KeyError, TypeError):
        return None
    return statuses if isinstance(statuses, list) and statuses else None


def existing_violations(head: str, statuses) -> list[tuple]:
    """Lane rows already at `head` that use an unrecognised status (information only)."""
    result = subprocess.run(["git", "show", f"{head}:{TARGET_FILE}"], capture_output=True, text=True)
    if result.returncode != 0:
        return []
    found = []
    for line in result.stdout.splitlines():
        parsed = lane_status(line)
        if parsed and parsed[1] not in statuses:
            found.append(parsed)
    return found


def check_table_rows(diff_lines: list[str]) -> list[str]:
    offenders = []
    for line in diff_lines:
        if CLAIM_TABLE_ROW.match(line) and not EVIDENCE_MARKERS.search(line):
            offenders.append(line)
    return offenders


REVIEW_ROW = re.compile(r"^\+\s*\|[^|]*\|[^|]*\|\s*Review\s*\|")
CONTRIBUTION_ID = re.compile(r"Contribution-ID", re.IGNORECASE)


def review_rows_without_contribution_id(diff_lines: list[str]) -> list[str]:
    """Added rows whose status cell is Review but that carry no Contribution-ID.

    Warning only (ratchet, AGENTS.md section 1): reported, never a CI failure, until the
    in-flight rows have been brought up to date.
    """
    return [
        line
        for line in diff_lines
        if REVIEW_ROW.match(line) and not CONTRIBUTION_ID.search(line)
    ]


def check_prose_blocks(diff_lines: list[str]) -> list[str]:
    """Group consecutive added lines under a new heading into blocks, and
    require evidence markers somewhere in a block that makes a claim."""
    offenders = []
    block: list[str] = []

    def flush(block_lines: list[str]):
        if not block_lines:
            return
        text = "\n".join(block_lines)
        if CLAIM_PHRASES.search(text) and not EVIDENCE_MARKERS.search(text):
            offenders.append(block_lines[0])

    for line in diff_lines:
        if not line.startswith("+") or line.startswith("+++"):
            flush(block)
            block = []
            continue
        if HEADING_RE.match(line):
            flush(block)
            block = [line]
        else:
            block.append(line)
    flush(block)
    return offenders


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: check_coordination_evidence.py <base-ref> <head-ref>", file=sys.stderr)
        return 2

    base, head = sys.argv[1], sys.argv[2]
    diff = get_diff(base, head)
    if not diff:
        print("No changes to governance/COORDINATION_QUEUE.md; nothing to check.")
        return 0

    diff_lines = diff.splitlines()
    offenders = check_table_rows(diff_lines) + check_prose_blocks(diff_lines)

    statuses = load_statuses(head)
    bad_status = []
    if statuses is None and load_statuses(base) is not None:
        print(f"Queue status check failed: {VOCABULARY_FILE} is readable at {base} but not at {head}. Deleting or breaking the "
              "vocabulary file would silently switch this check off; restore it, or change it in a reviewed commit that keeps it readable.")
        return 1
    if statuses is None:
        print(f"WARNING (not a failure): no readable {VOCABULARY_FILE} at {head} (and none at {base}); lane statuses were not checked.", file=sys.stderr)
    else:
        bad_status = status_violations(diff_lines, statuses)
        rust = rust_statuses(head)
        if rust is None and rust_statuses(base) is not None:
            print(f"Queue status check failed: the status list in {RUST_PARSER} was readable at {base} but not at {head}. A "
                  "refactor of the parser would silently stop the vocabulary comparison; update this checker's pattern in the same change.")
            return 1
        if rust is None:
            print(f"WARNING (not a failure): could not read the status list in {RUST_PARSER} at {head} (and none at {base}); the "
                  "vocabulary was not compared with the native parser.", file=sys.stderr)
        elif sorted(rust) != sorted(statuses):
            print(f"Queue status check failed: {VOCABULARY_FILE} and {RUST_PARSER} disagree. Only in the JSON: "
                  f"{sorted(set(statuses) - set(rust))}; only in the Rust parser: {sorted(set(rust) - set(statuses))}. "
                  "Change both together, or rows using the difference are skipped by the parser.")
            return 1
        old = existing_violations(head, statuses)
        if old:
            print("INFO: lane rows already at this revision with a status the queue parser skips (fixed when edited): "
                  + ", ".join(f"{i} ({s!r})" for i, s in old), file=sys.stderr)

    for row in review_rows_without_contribution_id(diff_lines):
        print(
            "WARNING (not a failure): added Review row has no Contribution-ID: "
            f"{row[:100]}",
            file=sys.stderr,
        )

    if bad_status:
        print(f"Queue status check failed: {len(bad_status)} added or edited lane row(s) in {TARGET_FILE} use a status "
              f"that is not in {VOCABULARY_FILE} (allowed: {', '.join(statuses)}). The native queue parser skips such "
              "a row, so its lane ID never enters the existing-ID set.\n")
        for lane, status, row in bad_status:
            hint = " (a `|` inside a cell shifts the columns; the parser does not honour `\\|`)" if status.endswith("\\") else ""
            print(f"  {lane}: status {status!r}{hint}")
        print(f"\nUse a listed status, or change {VOCABULARY_FILE} together with the Rust parser and a protocol amendment.")
        if not offenders:
            return 1

    if offenders:
        print(
            "Evidence-retention check failed: the following added line(s) in "
            f"{TARGET_FILE} claim Done/PASS/\"gate green\" without a commit "
            "hash, exit code, .log path, or fenced command block nearby:\n"
        )
        for o in offenders:
            print(f"  {o}")
        print(
            "\nSee the 'Standing rule: evidence retention' section at the top "
            f"of {TARGET_FILE}. Add the exact command, its exit code or the "
            "terminal line that states success, and a retained log path or "
            "commit hash a reviewer can re-check."
        )
        return 1

    print("Evidence-retention check passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
