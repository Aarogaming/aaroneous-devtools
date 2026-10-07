#!/usr/bin/env python3
"""Report lane rows that more than one open PR adds with different content (lane C100).

Every queue check looks at one PR against `main`. Nothing compared open PRs with each other, so two PRs that add the
same lane ID with different text or a different status (scout sample 2026-10-07: C100 in four PRs with three versions,
C101 in four with three, C99 in four with two) looked fine one by one and were settled by merge order alone: the one
that landed last conflicted or silently won. This reads each open PR's patch of the queue file from the GitHub files
API (no local clone state), keeps the lane rows the PR adds, and names every lane that has more than one distinct row.
A PR whose patch GitHub did not return is reported as unreadable, never skipped. Read-only; it never posts or edits.

Usage:
  queue_collisions.py --repo OWNER/NAME [--json]
"""

import argparse
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import review_coverage as rc  # noqa: E402

QUEUE = "governance/COORDINATION_QUEUE.md"
ADDED = re.compile(r"^\+\|\s*(C\d+[a-z]?)\s*\|")
STATUS_COLUMN = 2  # | ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |


def split_cells(line):
    """The cells of a table row, honouring `\\|` as a literal pipe inside a cell."""
    cells, current, i = [], "", 0
    while i < len(line):
        if line[i] == "\\" and i + 1 < len(line) and line[i + 1] == "|":
            current += "|"
            i += 2
            continue
        if line[i] == "|":
            cells.append(current)
            current = ""
        else:
            current += line[i]
        i += 1
    cells.append(current)
    return [cell.strip() for cell in cells[1:-1]] if len(cells) > 2 else []


def added_rows(patch):
    """{lane id: [row text, ...]} for the lane rows a unified-diff patch of the queue file adds."""
    rows = {}
    for line in (patch or "").replace("\r\n", "\n").split("\n"):
        match = ADDED.match(line)
        if match:
            rows.setdefault(match.group(1), []).append(line[1:].rstrip())
    return rows


def status_of(row):
    cells = split_cells(row)
    return cells[STATUS_COLUMN] if len(cells) > STATUS_COLUMN else "?"


def lane_key(lane):
    digits = re.match(r"C(\d+)", lane)
    return (int(digits.group(1)) if digits else 0, lane)


def collisions(by_pr):
    """Lanes with more than one distinct added row across ``by_pr`` ({pr number: {lane: [rows]}}).

    Each entry is ``{"lane": ..., "versions": [{"prs": [...], "status": ..., "row": ...}, ...]}``, versions ordered by
    their lowest PR number. Identical rows added by several PRs are one version, not a collision, and a single PR that
    adds two different rows for one lane collides with itself."""
    seen = {}
    for pr, lanes in by_pr.items():
        for lane, rows in lanes.items():
            for row in rows:
                seen.setdefault(lane, {}).setdefault(row, set()).add(pr)
    found = []
    for lane in sorted(seen, key=lane_key):
        if len(seen[lane]) < 2:
            continue
        versions = [{"prs": sorted(prs), "status": status_of(row), "row": row} for row, prs in seen[lane].items()]
        versions.sort(key=lambda v: v["prs"][0])
        found.append({"lane": lane, "versions": versions})
    return found


def live(repo):
    """({pr: {lane: [rows]}}, [PRs whose queue patch could not be read]) from the GitHub files API."""
    by_pr, unreadable = {}, []
    for item in rc.gh_json(*rc.pr_list_args(repo)):
        files = rc.flatten_pages(rc.gh_json("api", f"repos/{repo}/pulls/{item['number']}/files", "--paginate", "--slurp"))
        touched = [f for f in files if f.get("filename") == QUEUE]
        if not touched:
            continue
        if any("patch" not in f for f in touched):
            unreadable.append(item["number"])
            continue
        by_pr[item["number"]] = added_rows("\n".join(f["patch"] for f in touched))
    return by_pr, sorted(unreadable)


def render(found, unreadable):
    lines = []
    for entry in found:
        lines.append(f"{entry['lane']}: {len(entry['versions'])} different rows")
        for version in entry["versions"]:
            lines.append("  " + ", ".join(f"#{n}" for n in version["prs"]) + f"  status={version['status']}")
    for pr in unreadable:
        lines.append(f"#{pr}: queue patch not returned by GitHub (diff too large?); NOT CHECKED")
    return "\n".join(lines) if lines else "no lane row is added with different content by more than one open PR"


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", required=True)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    by_pr, unreadable = live(args.repo)
    found = collisions(by_pr)
    if args.json:
        print(json.dumps({"collisions": found, "unreadable": unreadable}, indent=2))
    else:
        print(render(found, unreadable))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
