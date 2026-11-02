#!/usr/bin/env python3
"""Report lane rows that more than one open PR adds with different content (lane C165).

Every queue check looks at one PR against `main`. Nothing compared open PRs with each other, so two PRs that add the
same lane ID with different text or a different status (scout sample 2026-10-07: C100 in four PRs with three versions,
C101 in four with three, C99 in four with two) looked fine one by one and were settled by merge order alone: the one
that landed last conflicted or silently won. This reads each open PR's patch of the queue file from the GitHub files
API (no local clone state), keeps the lane rows the PR adds, and names every lane that has more than one distinct row.
It provides two complementary views:
1. Direct same-base row collisions: PRs targeting the same base that propose differing rows.
2. Cross-stack claim namespace view: short lane IDs claimed by multiple PRs across different base branches with
   distinct Contribution-IDs (distinguishing legitimate parent/child status updates from competing allocations).
3. Description-declared claims: a `Contribution-ID: Cnnn/slug` line in an open PR's description counts as a claim on
   that short ID even when the PR's patch adds no queue row for it (scout 2026-10-31: draft #256 declared
   C176/codex-... in its description while #250 added a C176/gemini-... row; views 1 and 2 saw only the rows).
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
CONTRIBUTION_ID_RE = re.compile(r"Contribution-ID:?\s*([A-Za-z0-9_./-]+)", re.IGNORECASE)
LANE_OF_CID = re.compile(r"^(C\d+[a-z]?)(?:/|$)")


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


def contribution_id_of(row):
    """Extract Contribution-ID token from row text if present."""
    m = CONTRIBUTION_ID_RE.search(row)
    return m.group(1).rstrip(".,; ") if m else None


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


def collisions_by_base(by_base):
    """``collisions`` run separately for each base branch ({base: {pr: {lane: [rows]}}}), each entry naming its base.

    Only PRs aimed at the same base can compete directly: a stacked PR targets its predecessor's branch and intentionally
    updates the lane that predecessor added, which is ordered by the stack and not a direct collision."""
    found = []
    for base in sorted(by_base):
        found.extend(dict(entry, base=base) for entry in collisions(by_base[base]))
    return found


def namespace_collisions(by_pr):
    """Detect cross-stack claim collisions on the same short lane ID across all open PRs.

    Two PRs adding rows for the same short ID are NOT a namespace collision if:
    1. They add the exact same row text, or
    2. They carry the same non-None Contribution-ID (ordered parent/child status update across a stack).

    They ARE a collision if they declare different Contribution-IDs, or if any differing row lacks a
    Contribution-ID (unreconciled identity)."""
    seen = {}
    for pr, lanes in by_pr.items():
        for lane, rows in lanes.items():
            for row in rows:
                seen.setdefault(lane, []).append((pr, row))

    found = []
    for lane in sorted(seen, key=lane_key):
        entries = seen[lane]
        first_row = entries[0][1]
        if all(r == first_row for _, r in entries):
            continue

        cids = {}
        for pr, row in entries:
            cid = contribution_id_of(row)
            cids.setdefault(cid, []).append((pr, row))

        if len(cids) == 1 and None not in cids:
            continue

        claims = []
        for cid, pr_rows in cids.items():
            prs = sorted(set(p for p, _ in pr_rows))
            sample_row = pr_rows[0][1]
            claims.append({
                "prs": prs,
                "contribution_id": cid or "(missing)",
                "status": status_of(sample_row),
                "row": sample_row,
            })
        claims.sort(key=lambda c: c["prs"][0])
        found.append({"lane": lane, "claims": claims})
    return found


def lane_of_contribution_id(cid):
    """The short lane ID a Contribution-ID such as ``C176/slug-v1`` claims, or None when it does not start with one."""
    match = LANE_OF_CID.match(cid or "")
    return match.group(1) if match else None


def description_claims(bodies):
    """{lane: {contribution id: [pr, ...]}} for the ``Contribution-ID:`` declarations in PR descriptions ({pr: body})."""
    claims = {}
    for pr, body in bodies.items():
        for token in CONTRIBUTION_ID_RE.findall((body or "").replace("\r\n", "\n")):
            cid = token.rstrip(".,; ")
            lane = lane_of_contribution_id(cid)
            if lane and pr not in claims.setdefault(lane, {}).setdefault(cid, []):
                claims[lane][cid].append(pr)
    return claims


def description_collisions(by_pr, bodies):
    """Lanes claimed with more than one distinct Contribution-ID when PR descriptions are counted as claims.

    A claim is a Contribution-ID found in a queue row an open PR adds or in an open PR's description. A lane is
    reported only when at least one of its claims comes from a description (row-versus-row cases are the other two
    views' business) and the claims are not all the same ID: the same ID in a row and a description, or repeated
    across a stack, is one claim. Each entry is ``{"lane": ..., "claims": [{"contribution_id", "prs", "sources"}]}``."""
    claims = {}
    for pr, lanes in by_pr.items():
        for lane, rows in lanes.items():
            for row in rows:
                cid = contribution_id_of(row)
                if cid and lane_of_contribution_id(cid) == lane:
                    entry = claims.setdefault(lane, {}).setdefault(cid, {"prs": set(), "sources": set()})
                    entry["prs"].add(pr)
                    entry["sources"].add("queue row")
    for lane, by_cid in description_claims(bodies).items():
        for cid, prs in by_cid.items():
            entry = claims.setdefault(lane, {}).setdefault(cid, {"prs": set(), "sources": set()})
            entry["prs"].update(prs)
            entry["sources"].add("description")
    found = []
    for lane in sorted(claims, key=lane_key):
        by_cid = claims[lane]
        if len(by_cid) < 2 or not any("description" in c["sources"] for c in by_cid.values()):
            continue
        listed = [{"contribution_id": cid, "prs": sorted(c["prs"]), "sources": sorted(c["sources"])}
                  for cid, c in by_cid.items()]
        listed.sort(key=lambda c: (c["prs"][0], c["contribution_id"]))
        found.append({"lane": lane, "claims": listed})
    return found


def live(repo):
    """({base: {pr: {lane: [rows]}}}, [PRs whose queue patch could not be read]) from the GitHub files API."""
    by_base, unreadable = {}, []
    for item in rc.gh_json(*rc.pr_list_args(repo)):
        files = rc.flatten_pages(rc.gh_json("api", f"repos/{repo}/pulls/{item['number']}/files", "--paginate", "--slurp"))
        touched = [f for f in files if f.get("filename") == QUEUE]
        if not touched:
            continue
        if any("patch" not in f for f in touched):
            unreadable.append(item["number"])
            continue
        rows = added_rows("\n".join(f["patch"] for f in touched))
        by_base.setdefault(item.get("baseRefName", "?"), {})[item["number"]] = rows
    return by_base, sorted(unreadable)


def declared_bodies(repo):
    """{pr number: description} for the open PRs, so ``Contribution-ID:`` declarations outside queue rows are visible."""
    listing = rc.gh_json("pr", "list", "-R", repo, "--state", "open", "--limit", "1000", "--json", "number,body")
    return {item["number"]: item.get("body") or "" for item in listing}


def render(by_base_collisions, ns_collisions=None, unreadable=None, desc_collisions=None):
    if unreadable is None and isinstance(ns_collisions, list) and (not ns_collisions or isinstance(ns_collisions[0], int)):
        unreadable = ns_collisions
        ns_collisions = None
    if unreadable is None:
        unreadable = []

    lines = []
    if by_base_collisions:
        lines.append("Direct same-base collisions:")
        for entry in by_base_collisions:
            lines.append(f"  {entry['lane']}: {len(entry['versions'])} different rows (PRs into {entry.get('base', '?')})")
            for version in entry["versions"]:
                lines.append("    " + ", ".join(f"#{n}" for n in version["prs"]) + f"  status={version['status']}")
    else:
        lines.append("no lane row is added with different content by more than one open PR (direct same-base)")

    if ns_collisions:
        lines.append("\nCross-stack claim namespace collisions:")
        for entry in ns_collisions:
            lines.append(f"  {entry['lane']}: {len(entry['claims'])} competing claims across stacks")
            for claim in entry["claims"]:
                lines.append(f"    " + ", ".join(f"#{n}" for n in claim["prs"]) + f"  Contribution-ID={claim['contribution_id']}  status={claim['status']}")
    elif ns_collisions is not None:
        lines.append("no cross-stack claim namespace collision (unique Contribution-ID per short ID across open PRs)")

    if desc_collisions:
        lines.append("\nDescription-declared claim collisions (queue rows plus PR descriptions):")
        for entry in desc_collisions:
            lines.append(f"  {entry['lane']}: {len(entry['claims'])} competing Contribution-IDs")
            for claim in entry["claims"]:
                lines.append("    " + ", ".join(f"#{n}" for n in claim["prs"])
                             + f"  Contribution-ID={claim['contribution_id']}  from={'+'.join(claim['sources'])}")
    elif desc_collisions is not None:
        lines.append("no description-declared claim collision (no short ID claimed by different Contribution-IDs "
                     "once PR descriptions are counted)")

    for pr in unreadable:
        lines.append(f"#{pr}: queue patch not returned by GitHub (diff too large?); NOT CHECKED")
    return "\n".join(lines)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", required=True)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    by_base, unreadable = live(args.repo)
    by_base_found = collisions_by_base(by_base)

    all_by_pr = {}
    for base, pr_map in by_base.items():
        for pr, lanes in pr_map.items():
            all_by_pr[pr] = lanes
    ns_found = namespace_collisions(all_by_pr)
    desc_found = description_collisions(all_by_pr, declared_bodies(args.repo))

    if args.json:
        print(json.dumps({"collisions": by_base_found, "namespace_collisions": ns_found,
                          "description_collisions": desc_found, "unreadable": unreadable}, indent=2))
    else:
        print(render(by_base_found, ns_found, unreadable, desc_found))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
