#!/usr/bin/env python3
"""Shadow (report-only) review-coverage check, lane C97, per governance/PLATFORM_SPEC.md section 3.

A PR is review-covered at its current head when accepted scopes, chained from a whole-diff accept to
that head, cover base...head and no standing changes-requested exists at that head.

Inputs are attestation records, never prose:

* Structured (authoritative): a fenced block in a PR comment, tagged ``attestation``, holding one JSON
  object with the fields in the spec (``repo``, ``pr``, ``head``, ``base_sha``, ``scope``, ``from``,
  ``reviewer``, ``implementers``, ``verdict``).
* Legacy (heuristic, reported separately, never counted as covered): existing REVIEW PACKET prose.

Usage:
  review_coverage.py --snapshot FILE.json          offline, deterministic (used by the tests)
  review_coverage.py --repo OWNER/NAME [--pr N]    live read via the gh CLI (read only)

Exit status is always 0 for a successful report: this check never blocks anything.
"""

import argparse
import json
import re
import subprocess
import sys

FENCE = re.compile(r"```attestation\s*\n(.*?)\n```", re.DOTALL)
FULL_SHA = re.compile(r"\b[0-9a-f]{40}\b")
ACCEPT = {"accept"}
REJECT = {"changes_requested", "needs_revision"}


def parse_attestations(comments):
    """Return (structured, legacy). Structured records are validated; malformed ones are reported."""
    structured, problems, legacy = [], [], []
    for index, comment in enumerate(comments):
        body = comment.get("body", "")
        found = False
        for raw in FENCE.findall(body):
            found = True
            try:
                record = json.loads(raw)
            except ValueError as error:
                problems.append({"comment": index, "problem": f"invalid JSON: {error}"})
                continue
            missing = [
                key
                for key in ("repo", "pr", "head", "base_sha", "scope", "reviewer", "verdict")
                if key not in record
            ]
            if record.get("scope") == "delta" and "from" not in record:
                missing.append("from")
            if missing:
                problems.append({"comment": index, "problem": f"missing fields: {missing}"})
                continue
            record["comment"] = index
            structured.append(record)
        if not found and re.search(r"\bVERDICT\b", body):
            legacy.append(
                {
                    "comment": index,
                    "heads_mentioned": sorted(set(FULL_SHA.findall(body))),
                    "looks_whole": bool(re.search(r"whole[- ]diff|entire diff", body, re.I)),
                }
            )
    return structured, problems, legacy


def independent(record):
    return record["reviewer"] not in record.get("implementers", [])


def evaluate(head, merge_base, records):
    """Pure coverage decision for one PR head. Returns a dict with a boolean and the reasons."""
    at_head = [r for r in records if r["head"] == head]
    blockers = [
        r for r in at_head if r["verdict"] in REJECT and r["base_sha"] == merge_base
    ]
    reasons = []
    if blockers:
        reasons.append(f"standing {blockers[0]['verdict']} at head by {blockers[0]['reviewer']}")

    # A whole accept is valid only for the merge-base it was computed against.
    valid = [
        r
        for r in records
        if r["verdict"] in ACCEPT and independent(r) and r["base_sha"] == merge_base
    ]
    whole_heads = {r["head"] for r in valid if r["scope"] == "whole"}
    covered = set(whole_heads)
    # Delta accepts extend coverage only from an already covered head (chain to a whole accept).
    changed = True
    while changed:
        changed = False
        for r in valid:
            if r["scope"] == "delta" and r["from"] in covered and r["head"] not in covered:
                covered.add(r["head"])
                changed = True
    ok = head in covered and not blockers
    if head not in covered:
        if any(r["verdict"] in ACCEPT and r["head"] == head for r in records):
            reasons.append("accept at head exists but is not chained to a whole accept at this base")
        else:
            reasons.append("no accept at the current head")
    return {"covered": ok, "reasons": reasons, "accepts_at_head": len(at_head)}


def report_snapshot(snapshot):
    rows = []
    for pr in snapshot["prs"]:
        structured, problems, legacy = parse_attestations(pr.get("comments", []))
        decision = evaluate(pr["head"], pr["merge_base"], structured)
        rows.append(
            {
                "pr": pr["number"],
                "head": pr["head"],
                "base": pr.get("base", "main"),
                "covered": decision["covered"],
                "reasons": decision["reasons"],
                "structured_attestations": len(structured),
                "malformed": problems,
                "legacy_packets_not_counted": len(legacy),
            }
        )
    return rows


def gh_json(*args):
    out = subprocess.run(["gh", *args], capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def merge_base(repo_dir, base_ref, head):
    out = subprocess.run(
        ["git", "-C", repo_dir, "merge-base", base_ref, head],
        capture_output=True,
        text=True,
        check=False,
    )
    return out.stdout.strip() or "unknown"


def live_snapshot(repo, only_pr, repo_dir):
    prs = []
    for item in gh_json("pr", "list", "-R", repo, "--state", "open", "--json", "number,headRefOid,baseRefName"):
        if only_pr and item["number"] != only_pr:
            continue
        comments = gh_json("api", f"repos/{repo}/issues/{item['number']}/comments", "--paginate")
        base = f"origin/{item['baseRefName']}"
        prs.append(
            {
                "number": item["number"],
                "head": item["headRefOid"],
                "base": item["baseRefName"],
                "merge_base": merge_base(repo_dir, base, item["headRefOid"]),
                "comments": comments,
            }
        )
    return {"prs": prs}


def render(rows):
    lines = ["PR    head      covered  structured  legacy(not counted)  reasons"]
    for r in rows:
        reasons = "; ".join(r["reasons"]) or "-"
        lines.append(
            f"#{r['pr']:<4} {r['head'][:8]}  {str(r['covered']):<7}  {r['structured_attestations']:<10}"
            f"  {r['legacy_packets_not_counted']:<19}  {reasons}"
        )
    return "\n".join(lines)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--snapshot")
    parser.add_argument("--repo")
    parser.add_argument("--pr", type=int)
    parser.add_argument("--repo-dir", default=".")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    if bool(args.snapshot) == bool(args.repo):
        parser.error("give exactly one of --snapshot or --repo")
    if args.snapshot:
        with open(args.snapshot, encoding="utf-8") as handle:
            snapshot = json.load(handle)
    else:
        snapshot = live_snapshot(args.repo, args.pr, args.repo_dir)
    rows = report_snapshot(snapshot)
    print(json.dumps(rows, indent=2) if args.json else render(rows))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
