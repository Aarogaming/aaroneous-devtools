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

Usage: check_coordination_evidence.py <base-ref> <head-ref>
Exits 1 and prints each offending line/block if a claim lacks evidence.
"""

import re
import subprocess
import sys

TARGET_FILE = "governance/COORDINATION_QUEUE.md"

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


def lane_rows(text: str):
    """({lane ID: status}, [IDs that appear more than once]) for the lane rows of a queue file. The status is the
    fourth `|`-separated field, exactly as the native parser reads it."""
    rows, repeats = {}, []
    for line in text.splitlines():
        match = LANE_ROW.match(line)
        if not match:
            continue
        lane, fields = match.group(1), line.split("|")
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


def show(ref: str):
    """The queue file at a ref, or None when it does not exist there."""
    result = subprocess.run(["git", "show", f"{ref}:{TARGET_FILE}"], capture_output=True, text=True, encoding="utf-8",
                            errors="replace")
    return result.stdout if result.returncode == 0 else None


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

    merge_base = subprocess.run(["git", "merge-base", base, head], capture_output=True, text=True).stdout.strip()
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
