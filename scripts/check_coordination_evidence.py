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
    result = subprocess.run(
        ["git", "diff", f"{base}...{head}", "--", TARGET_FILE],
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout


def check_table_rows(diff_lines: list[str]) -> list[str]:
    offenders = []
    for line in diff_lines:
        if CLAIM_TABLE_ROW.match(line) and not EVIDENCE_MARKERS.search(line):
            offenders.append(line)
    return offenders


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
