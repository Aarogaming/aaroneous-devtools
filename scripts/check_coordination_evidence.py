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
import os
import re
import subprocess
import sys

TARGET_FILE = "governance/COORDINATION_QUEUE.md"
VOCABULARY_FILE = "governance/QUEUE_STATUSES.json"
RUST_PARSER = "foundation/crates/foundation_cli/src/multithreader.rs"
# `cols[3]` or `cols[3].as_str()`: the parser's cells became owned Strings when it learned to honour an escaped pipe (PR #120);
# a pattern that only knew the first spelling would stop finding the list and silently switch the comparison off.
RUST_STATUS_LIST = re.compile(r'matches!\(\s*cols\[3\](?:\.as_str\(\))?\s*,((?:\s*"[A-Za-z ]+"\s*\|?)+)\)')
LANE_ID = re.compile(r"C\d+")
PROGRAM_FILE = "governance/MULTITHREAD_PROGRAM.md"
PROGRAM_TIERS = ("P0", "P1", "P2", "P3", "Done", "Retired")

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


def split_cells(row: str):
    """The cells of a table row split the way GitHub renders it: a pipe written as backslash-pipe stays inside its cell.
    The live C89 row holds kernel, control and tooling joined by escaped pipes; a plain split read its status as
    "control" plus a backslash (found with the Rust schedule-review, which dropped the row)."""
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


def lane_status(row: str):
    """(lane id, status) for a lane row, read as multithreader.rs queue_ids reads it since PR #120 (and as GitHub renders the
    table): the row must start with `|`, is split on unescaped `|` only, needs at least five columns, the id is column 1
    and the status column 3. None for anything that is not a lane row (headers, separators, prose)."""
    if not row.startswith("|"):
        return None
    cols = [c.strip() for c in split_cells(row)]
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


LANE_ROW = re.compile(r"^\|\s*(C\d+)\s*\|")


def split_cells(row: str):
    """The cells of a table row split the way GitHub renders it: a pipe written `\\|` stays inside its cell. The live C89 row
    holds `kernel\\|control\\|tooling`; a plain split read its status as `control\\` (found with the Rust schedule-review,
    which dropped the row)."""
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


def lane_rows(text: str):
    """({lane ID: status}, [IDs that appear more than once]) for the lane rows of a queue file. The status is the
    fourth `|`-separated field, exactly as the native parser reads it."""
    rows, repeats = {}, []
    for line in text.splitlines():
        match = LANE_ROW.match(line)
        if not match:
            continue
        lane, fields = match.group(1), split_cells(line)
        if lane in rows:
            repeats.append(lane)
        rows[lane] = fields[3].strip() if len(fields) > 3 else ""
    return rows, repeats


def lane_row_findings(base_text: str, head_text: str):
    """(failures, warnings) for what a change does to existing lanes. This check only looked at ADDED lines, so a PR
    could delete a Claimed lane, add a second row with an existing ID, or turn a Done row back into Ready and pass
    (probe on main: all three exit 0). Failures: a lane ID that appears more than once at the head and did not at the
    base; a removed lane that was not Done (a live lane lost); a Done lane whose status changed. Warning: a removed Done
    lane (retiring finished work to an archive is legitimate but should be a visible decision)."""
    base, base_repeats = lane_rows(base_text)
    head, head_repeats = lane_rows(head_text)
    failures, warnings = [], []
    for lane in sorted(set(head_repeats) - set(base_repeats)):
        failures.append(f"{lane}: lane ID appears more than once")
    for lane, status in sorted(base.items()):
        if lane not in head:
            (warnings if status == "Done" else failures).append(f"{lane}: row removed (it was {status or 'status unreadable'})")
        elif status == "Done" and head[lane] != "Done":
            failures.append(f"{lane}: Done row changed to {head[lane] or 'status unreadable'}")
    return failures, warnings


def show(ref: str, path: str = TARGET_FILE):
    """A tracked file at a ref (the queue by default), or None when it does not exist there."""
    result = subprocess.run(["git", "show", f"{ref}:{path}"], capture_output=True, text=True, encoding="utf-8",
                            errors="replace")
    return result.stdout if result.returncode == 0 else None


def program_ids(text: str):
    """The IDs of the worklist rows: a table row whose first cell is a tier (P0..P3, Done, Retired) names its ID in
    the second cell."""
    ids = set()
    for line in text.splitlines():
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in split_cells(line)]
        if len(cells) > 3 and cells[1] in PROGRAM_TIERS and cells[2]:
            ids.add(cells[2])
    return ids


def id_overlap_findings(base_queue, head_queue, base_program, head_program):
    """(failures, warnings) for IDs that appear in both the queue and the worklist. MULTITHREAD_PROGRAM.md says the
    worklist holds future work and the queue only work ready to own, so one ID in both files is two answers to "is
    this ready"; the Rust readers merge the two ID sets and no check noticed (live: C106 is a queue row and a P2
    worklist row). An overlap this change introduces fails; one that was already there is a warning."""
    now = set(lane_rows(head_queue)[0]) & program_ids(head_program)
    before = set(lane_rows(base_queue)[0]) & program_ids(base_program)
    failures = [f"{i}: the ID is now in both {TARGET_FILE} and {PROGRAM_FILE}" for i in sorted(now - before)]
    warnings = [f"{i}: the ID is in both {TARGET_FILE} and {PROGRAM_FILE} (already so before this change)"
                for i in sorted(now & before)]
    return failures, warnings


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: check_coordination_evidence.py <base-ref> <head-ref>", file=sys.stderr)
        return 2

    base, head = sys.argv[1], sys.argv[2]
    merge_base = subprocess.run(["git", "merge-base", base, head], capture_output=True, text=True).stdout.strip()
    needed = (show(merge_base) if merge_base else None, show(head), show(merge_base, PROGRAM_FILE) if merge_base else None,
              show(head, PROGRAM_FILE))
    if all(text is not None for text in needed):
        overlap_failures, overlap_warnings = id_overlap_findings(*needed)
        for warning in overlap_warnings:
            print(f"WARNING (not a failure): {warning}", file=sys.stderr)
        if overlap_failures:
            print(f"ID overlap check failed: {PROGRAM_FILE} is the worklist of future work and {TARGET_FILE} holds "
                  "only work ready to own:\n")
            for failure in overlap_failures:
                print(f"  {failure}")
            print("\nKeep each ID in one file: move the row, do not copy it.")
            return 1
    # Validate gap register unconditionally when present
    gap_register_path = "governance/FOUNDRY_GAP_REGISTER.md"
    if os.path.isfile(gap_register_path) and os.path.isfile(TARGET_FILE):
        try:
            import check_gap_register
            gap_errors = check_gap_register.check(gap_register_path, TARGET_FILE)
            if gap_errors:
                print(f"\nGap register check failed for {gap_register_path}:", file=sys.stderr)
                for err in gap_errors:
                    print(f"  {err}", file=sys.stderr)
                return 1
        except ImportError:
            pass

    diff = get_diff(base, head)
    if not diff:
        print("No changes to governance/COORDINATION_QUEUE.md; gap register validation passed.")
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
    base_text, head_text = (show(merge_base) if merge_base else None), show(head)
    lane_failures, lane_warnings = ([], []) if base_text is None or head_text is None else lane_row_findings(
        base_text, head_text)
    for warning in lane_warnings:
        print(f"WARNING (not a failure): {warning}", file=sys.stderr)
    if lane_failures:
        print(f"Lane-row check failed: this change damages existing lanes in {TARGET_FILE}:\n")
        for failure in lane_failures:
            print(f"  {failure}")
        print("\nRestore the row (or change its status through the normal Ready, Claimed, Review, Done order), or move a "
              "Done row to an archive file in the same change.")
        return 1

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
            cells = len(split_cells(row)) - 2
            hint = (f" (the row has {cells} cells, 7 expected: an unescaped `|` inside a cell shifts the columns)"
                    if cells != 7 else "")
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

    gap_register_path = "governance/FOUNDRY_GAP_REGISTER.md"
    if os.path.isfile(gap_register_path) and os.path.isfile(TARGET_FILE):
        try:
            import check_gap_register
            gap_errors = check_gap_register.check(gap_register_path, TARGET_FILE)
            if gap_errors:
                print(f"\nGap register check failed for {gap_register_path}:", file=sys.stderr)
                for err in gap_errors:
                    print(f"  {err}", file=sys.stderr)
                return 1
        except ImportError:
            pass

    print("Evidence-retention check passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
