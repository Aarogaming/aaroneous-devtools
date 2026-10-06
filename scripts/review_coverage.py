#!/usr/bin/env python3
"""Shadow (report-only) review-coverage check, lane C97, per governance/PLATFORM_SPEC.md section 3.

A PR is review-covered at its current head when accepted scopes, chained from a whole-diff accept to
that head, cover base...head and no changes-requested stands at that head.

Inputs are attestation records, never prose:

* Structured (authoritative): a fenced block in a PR comment, tagged ``attestation``, holding one JSON
  object with the fields in the spec (``repo``, ``pr``, ``head``, ``base_sha``, ``scope``, ``from``,
  ``reviewer``, ``implementers`` (non-empty list), ``verdict``). A record counts only for the repository and PR it
  names: ``repo`` and ``pr`` must equal the subject, otherwise it is reported as malformed.
* Legacy (heuristic, reported separately, never counted as covered): existing REVIEW PACKET prose.

Usage:
  review_coverage.py --snapshot FILE.json                       offline, deterministic (the tests)
  review_coverage.py --repo OWNER/NAME --repo-dir CHECKOUT      live read via the gh CLI (read only);
                                                               CHECKOUT must be a clone of OWNER/NAME

Exit status is 0 for a produced report: this check never blocks anything.
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
REQUIRED = ("repo", "pr", "head", "base_sha", "scope", "reviewer", "implementers", "verdict")


def validate(record):
    """Return a problem string, or None when the record is a usable attestation."""
    if not isinstance(record, dict):
        return f"not a JSON object: {type(record).__name__}"
    missing = [key for key in REQUIRED if key not in record]
    if record.get("scope") == "delta" and "from" not in record:
        missing.append("from")
    if missing:
        return f"missing fields: {missing}"
    implementers = record["implementers"]
    if not isinstance(implementers, list) or not implementers:
        return "implementers must be a non-empty list"
    if record["scope"] not in ("whole", "delta"):
        return f"unknown scope: {record['scope']!r}"
    return None


def parse_attestations(comments):
    """Return (structured, problems, legacy). Malformed records are reported, never counted."""
    structured, problems, legacy = [], [], []
    for index, comment in enumerate(comments):
        body = comment.get("body", "") if isinstance(comment, dict) else ""
        found = False
        for raw in FENCE.findall(body):
            found = True
            try:
                record = json.loads(raw)
            except ValueError as error:
                problems.append({"comment": index, "problem": f"invalid JSON: {error}"})
                continue
            problem = validate(record)
            if problem:
                problems.append({"comment": index, "problem": problem})
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
    return record["reviewer"] not in record["implementers"]


def evaluate(head, merge_base, records, is_ancestor):
    """Pure coverage decision for one PR head.

    ``is_ancestor(a, b)`` says whether commit a is an ancestor of commit b. A delta accept extends
    coverage only when its ``from`` is a covered head AND an ancestor of the delta's own head, so a
    rebased or flattened head cannot inherit coverage by naming an old SHA.
    """
    at_head = [r for r in records if r["head"] == head]
    blockers = [r for r in at_head if r["verdict"] in REJECT and r["base_sha"] == merge_base]
    reasons = []
    if blockers:
        reasons.append(f"standing {blockers[0]['verdict']} at head by {blockers[0]['reviewer']}")

    valid = [
        r
        for r in records
        if r["verdict"] in ACCEPT and independent(r) and r["base_sha"] == merge_base
    ]
    # A delta whose `from` is the merge base is a review of the whole PR diff, so it seeds coverage like a whole accept.
    # Stacked PRs make this the normal case: reviewing "the delta over the base branch" IS reviewing the PR.
    covered = {r["head"] for r in valid if r["scope"] == "whole" or (r["scope"] == "delta" and r["from"] == merge_base)}
    changed = True
    while changed:
        changed = False
        for r in valid:
            if (
                r["scope"] == "delta"
                and r["from"] in covered
                and r["head"] not in covered
                and is_ancestor(r["from"], r["head"])
            ):
                covered.add(r["head"])
                changed = True
    ok = head in covered and not blockers
    if head not in covered:
        if any(r["verdict"] in ACCEPT and r["head"] == head for r in records):
            reasons.append("accept at head exists but is not chained to a whole accept at this base")
        else:
            reasons.append("no accept at the current head")
    return {"covered": ok, "reasons": reasons, "accepts_at_head": len(at_head)}


def pairs_ancestry(pairs):
    known = {(a, b) for a, b in pairs}
    return lambda a, b: a == b or (a, b) in known


def split_subject(records, repo, number):
    """Keep only records that name this repository and PR; the rest are reported, never counted."""
    mine, other = [], []
    for record in records:
        same = record["repo"] == repo and str(record["pr"]) == str(number)
        (mine if same else other).append(record)
    return mine, other


def report_snapshot(snapshot, is_ancestor=None):
    repo = snapshot.get("repo")
    if not repo:
        raise ValueError("snapshot needs a top-level 'repo' (OWNER/NAME) naming the subject repository")
    rows = []
    for pr in snapshot["prs"]:
        parsed, problems, legacy = parse_attestations(pr.get("comments", []))
        structured, foreign = split_subject(parsed, repo, pr["number"])
        for record in foreign:
            problems.append(
                {
                    "comment": record["comment"],
                    "problem": f"attestation subject {record['repo']}#{record['pr']} "
                    f"does not match {repo}#{pr['number']}",
                }
            )
        ancestry = is_ancestor or pairs_ancestry(pr.get("ancestry", []))
        decision = evaluate(pr["head"], pr["merge_base"], structured, ancestry)
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


def flatten_pages(pages):
    """gh api --paginate --slurp returns a list of pages; each page is a list of items."""
    items = []
    for page in pages:
        items.extend(page if isinstance(page, list) else [page])
    return items


def checkout_matches(remote_url, repo):
    """True when a clone's origin URL names OWNER/NAME (https or ssh form)."""
    cleaned = remote_url.strip().removesuffix(".git").rstrip("/")
    return cleaned.endswith("/" + repo) or cleaned.endswith(":" + repo)


def pr_list_args(repo):
    return ["pr", "list", "-R", repo, "--state", "open", "--limit", "1000", "--json",
            "number,headRefOid,baseRefName"]


def gh_json(*args):
    out = subprocess.run(["gh", *args], capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def git(repo_dir, *args):
    return subprocess.run(["git", "-C", repo_dir, *args], capture_output=True, text=True, check=False)


def live_snapshot(repo, repo_dir):
    remote = git(repo_dir, "remote", "get-url", "origin")
    if remote.returncode != 0 or not checkout_matches(remote.stdout, repo):
        raise SystemExit(
            f"--repo-dir {repo_dir!r} is not a clone of {repo} (origin: {remote.stdout.strip()!r})"
        )
    prs = []
    for item in gh_json(*pr_list_args(repo)):
        pages = gh_json("api", f"repos/{repo}/issues/{item['number']}/comments", "--paginate", "--slurp")
        base = f"origin/{item['baseRefName']}"
        mb = git(repo_dir, "merge-base", base, item["headRefOid"]).stdout.strip() or "unknown"
        prs.append(
            {
                "number": item["number"],
                "head": item["headRefOid"],
                "base": item["baseRefName"],
                "merge_base": mb,
                "comments": flatten_pages(pages),
            }
        )
    return {"repo": repo, "prs": prs}


def live_ancestry(repo_dir):
    return lambda a, b: a == b or git(repo_dir, "merge-base", "--is-ancestor", a, b).returncode == 0


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
    parser.add_argument("--repo-dir", help="clone of --repo; required with --repo")
    parser.add_argument("--pr", type=int, help="only report this PR")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    if bool(args.snapshot) == bool(args.repo):
        parser.error("give exactly one of --snapshot or --repo")
    if args.repo and not args.repo_dir:
        parser.error("--repo requires an explicit --repo-dir (a clone of that repository)")
    if args.snapshot:
        with open(args.snapshot, encoding="utf-8") as handle:
            snapshot = json.load(handle)
        ancestry = None
    else:
        snapshot = live_snapshot(args.repo, args.repo_dir)
        ancestry = live_ancestry(args.repo_dir)
    if args.pr:
        snapshot["prs"] = [p for p in snapshot["prs"] if p["number"] == args.pr]
    rows = report_snapshot(snapshot, ancestry)
    print(json.dumps(rows, indent=2) if args.json else render(rows))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
