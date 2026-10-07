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

* Formal GitHub reviews (authenticated, lane C100): an APPROVED or CHANGES_REQUESTED review whose
  commit equals the PR head, by a login mapped to a seat in an identity map, counts like a whole-diff
  record. The map never lists the shared owner login as a seat, so shared-account reviews never count.

Usage:
  review_coverage.py --snapshot FILE.json                       offline, deterministic (the tests)
  review_coverage.py --repo OWNER/NAME --repo-dir CHECKOUT      live read via the gh CLI (read only);
                                                               CHECKOUT must be a clone of OWNER/NAME

Exit status is 0 for a produced report: this check never blocks anything.
"""

import argparse
import json
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import attest_sig  # noqa: E402

FENCE = re.compile(r"```attestation\s*\n(.*?)\n```", re.DOTALL)
FULL_SHA = re.compile(r"\b[0-9a-f]{40}\b")
# A prose verdict: the upper-case VERDICT label the old packets used, or the word "verdict" in any case followed within
# a few words by an accept or reject word ("Independent Codex verdict: ACCEPT ..."). Without the second form a seat
# that writes the label in lower case is invisible: not counted, not malformed, not legacy, so nobody tells it why.
PROSE_VERDICT = re.compile(
    r"\bVERDICT\b|(?i:\bverdict\b[^.\n]{0,40}?\b(?:accept(?:ed)?|reject(?:ed)?|approve[ds]?|changes[ _-]?requested|needs[ _-]?revision)\b)")
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
        for raw, signature in attest_sig.pairs(body):
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
            record["_canon"] = attest_sig.canonical(record)
            record["_sig"] = signature
            record["comment"] = index
            structured.append(record)
        if not found and PROSE_VERDICT.search(body):
            legacy.append(
                {
                    "comment": index,
                    "heads_mentioned": sorted(set(FULL_SHA.findall(body))),
                    "looks_whole": bool(re.search(r"whole[- ]diff|entire diff", body, re.I)),
                }
            )
    return structured, problems, legacy


# One seat is one seat however its name is spelled: "Gemini", "gemini" and "Antigravity" are the same reviewer. The merge gate's
# "at least two independent seats" box counts len(accept_seats), so two comments from one seat spelled differently (live on
# #96: ['Gemini', 'gemini']) looked like two seats.
SEAT_NAME_ALIASES = {"antigravity": "gemini", "qwen": "mimo"}


# A seat is recognised by a word that STARTS with its name: "claude-sonnet-5-5", "gemini-3.8-flash", "mimo-v2.6-flash-free",
# "Claude\u200b" and "CLAUDE." all name a seat. Taking only the first space-separated word missed every model-id and
# punctuation form (skeptic probe on #99: reviewer "Claude" with implementers ["claude-sonnet-5-5"] was independent).
KNOWN_SEATS = ("claude", "codex", "gemini", "antigravity", "mimo", "qwen")


def seat_names(name):
    """The set of seats a reviewer or implementer string names. A string may name several ("Claude, Mimo",
    "claude+mimo", "Gemini/Antigravity"); a string that names none keeps its first alphanumeric token, so an unknown
    seat still compares with itself."""
    tokens = [t for t in re.split(r"[^a-z0-9]+", str(name).strip().lower()) if t]
    found = set()
    for token in tokens:
        for seat in KNOWN_SEATS:
            if token.startswith(seat):
                found.add(SEAT_NAME_ALIASES.get(seat, seat))
    if not found and tokens:
        found.add(tokens[0])
    return found


def seat_name(name):
    """One seat for display and de-duplication: the first of the sorted seats `seat_names` finds."""
    seats = sorted(seat_names(name))
    return seats[0] if seats else ""


def independent(record):
    """The reviewer is not among the implementers, comparing seats (see `seat_name`): reviewer "gemini" with implementers
    ["Gemini"], or "Gemini/Antigravity" with ["gemini"], is the same seat and never independent (Codex P1 on #98: the
    raw comparison let an implementer's own verdict count under a spelling variant)."""
    implementers = set().union(*(seat_names(i) for i in record["implementers"])) if record["implementers"] else set()
    return not (seat_names(record["reviewer"]) & implementers)


def evaluate(head, merge_base, records, is_ancestor):
    """Pure coverage decision for one PR head.

    ``is_ancestor(a, b)`` says whether commit a is an ancestor of commit b. A delta accept extends
    coverage only when its ``from`` is a covered head AND an ancestor of the delta's own head, so a
    rebased or flattened head cannot inherit coverage by naming an old SHA.
    """
    if merge_base in (None, "", "unknown"):
        return {"covered": False, "reasons": ["merge-base unresolved: coverage cannot be judged"], "accepts_at_head": 0,
                "counted": [], "accept_seats": []}
    at_head = [r for r in records if r["head"] == head]
    # A rejection stands whatever merge base it names: it was written about this exact head, and a base_sha typed or
    # copied wrong (a hand-written attestation) must not let an accept through over it. Accepts stay strict.
    blockers = [r for r in at_head if r["verdict"] in REJECT]
    reasons = []
    if blockers:
        reasons.append(f"standing {blockers[0]['verdict']} at head by {blockers[0]['reviewer']}")
        for r in blockers:
            if r["base_sha"] != merge_base:
                reasons.append(f"the rejection by {r['reviewer']} names base_sha {str(r['base_sha'])[:12]}, not the merge "
                               f"base {merge_base[:12]}; it still stands until that reviewer re-attests or withdraws it")

    valid = [
        r
        for r in records
        if r["verdict"] in ACCEPT and independent(r) and r["base_sha"] == merge_base
    ]
    covered = {r["head"] for r in valid if r["scope"] == "whole"}
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
            for r in at_head:
                if r["verdict"] not in ACCEPT or r in valid:
                    continue
                if r["base_sha"] != merge_base:
                    reasons.append(f"the accept by {r['reviewer']} names base_sha {str(r['base_sha'])[:12]}, but the merge "
                                   f"base is {merge_base[:12]}: re-attest against the current merge base")
                elif not independent(r):
                    reasons.append(f"the accept by {r['reviewer']} is not independent of its implementers "
                                   f"{sorted(r['implementers'])}")
        else:
            reasons.append("no accept at the current head")
    counted = [r for r in valid if r["head"] == head]
    return {
        "covered": ok,
        "reasons": reasons,
        "accepts_at_head": len(at_head),
        "counted": counted,
        "accept_seats": sorted(set().union(*(seat_names(r["reviewer"]) for r in counted)) if counted else []),
    }


def load_keys(path):
    """Return the raw identities data (seat -> logins/keys) for signature verification."""
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def load_identities(path):
    """Return {login_lower: seat} from governance/identities.json ({"seats": {seat: [logins]}})."""
    with open(path, encoding="utf-8") as handle:
        data = json.load(handle)
    mapping = {}
    for seat, entry in data.get("seats", {}).items():
        logins = entry.get("logins", []) if isinstance(entry, dict) else entry
        for login in logins:
            mapping[login.lower()] = seat
    return mapping


def reviews_to_records(reviews, repo, number, head, merge_base, implementers, login_to_seat, base_events=()):
    """Turn GitHub PR reviews into attestation-shaped records, plus problems explaining skips.

    Only reviews at the current head by a mapped seat are used. Approval is whole-diff by GitHub's own
    binding of a review to a commit. Implementers must be known, or independence cannot be shown.
    A review submitted before a ``base_events`` timestamp (base_ref_changed / base_ref_force_pushed) is
    skipped: the diff it reviewed is no longer base...head, so only an attestation may restate it.
    """
    records, problems = [], []
    if merge_base in (None, "", "unknown"):
        return records, [{"review": None, "problem": "merge-base unresolved (checkout lacks the PR head?); formal reviews cannot be bound"}]
    # GitHub lists every review chronologically. Only each reviewer's latest decision stands: a later
    # APPROVED supersedes an earlier CHANGES_REQUESTED, and a DISMISSED review is no decision at all.
    latest = {}
    for review in sorted(reviews, key=lambda r: (r.get("submitted_at", ""), r.get("id", 0))):
        if review.get("state") in ("APPROVED", "CHANGES_REQUESTED", "DISMISSED"):
            latest[(review.get("user") or {}).get("login", "").lower()] = review
    for review in latest.values():
        login = (review.get("user") or {}).get("login", "")
        state = review.get("state", "")
        if state not in ("APPROVED", "CHANGES_REQUESTED") or review.get("commit_id") != head:
            continue
        submitted = review.get("submitted_at", "")
        if submitted and any(event > submitted for event in base_events):
            problems.append({"review": review.get("id"), "problem": "base changed after this review was submitted"})
            continue
        seat = login_to_seat.get(login.lower())
        if not seat:
            problems.append({"review": review.get("id"), "problem": f"login {login!r} is not a mapped seat"})
            continue
        if not implementers:
            problems.append({"review": review.get("id"), "problem": "implementers unknown; independence unproven"})
            continue
        records.append(
            {
                "repo": repo,
                "pr": number,
                "head": head,
                "base_sha": merge_base,
                "scope": "whole",
                "reviewer": seat,
                "implementers": list(implementers),
                "verdict": "accept" if state == "APPROVED" else "changes_requested",
                "source": "github_review",
            }
        )
    return records, problems


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


def report_snapshot(snapshot, is_ancestor=None, identities=None, keys=None, require_signatures=False):
    repo = snapshot.get("repo")
    if not repo:
        raise ValueError("snapshot needs a top-level 'repo' (OWNER/NAME) naming the subject repository")
    rows = []
    for pr in snapshot["prs"]:
        parsed, problems, legacy = parse_attestations(pr.get("comments", []))
        allowed = attest_sig.allowed_signers(keys) if keys else ""
        for record in parsed:
            record["verified"] = bool(record.get("_sig")) and attest_sig.verify(
                record["_canon"], record["_sig"], record["reviewer"], allowed
            )
        if require_signatures:
            for record in [r for r in parsed if not r["verified"]]:
                problems.append(
                    {"comment": record["comment"], "problem": "unsigned or unverifiable attestation (signatures required)"}
                )
            parsed = [r for r in parsed if r["verified"]]
        structured, foreign = split_subject(parsed, repo, pr["number"])
        for record in foreign:
            problems.append(
                {
                    "comment": record["comment"],
                    "problem": f"attestation subject {record['repo']}#{record['pr']} "
                    f"does not match {repo}#{pr['number']}",
                }
            )
        formal, formal_problems = reviews_to_records(
            pr.get("reviews", []), repo, pr["number"], pr["head"], pr["merge_base"],
            pr.get("implementers", []), identities or {}, pr.get("base_events", []),
        )
        problems = problems + formal_problems
        ancestry = is_ancestor or pairs_ancestry(pr.get("ancestry", []))
        decision = evaluate(pr["head"], pr["merge_base"], structured + formal, ancestry)
        rows.append(
            {
                "pr": pr["number"],
                "head": pr["head"],
                "base": pr.get("base", "main"),
                "covered": decision["covered"],
                "reasons": decision["reasons"],
                "accept_seats": decision["accept_seats"],
                "counted_verdicts": len(decision["counted"]),
                "verdicts_missing_evidence": [r.get("comment") for r in decision["counted"] if not r.get("commands")],
                "structured_attestations": len(structured),
                "authenticated_reviews": len(formal),
                "signed_attestations": sum(1 for r in structured if r.get("verified")),
                "trust": (
                    "signed"
                    if any(r.get("verified") and r["verdict"] == "accept" and r["head"] == pr["head"] for r in structured)
                    else "authenticated"
                    if any(r["verdict"] == "accept" for r in formal)
                    else "attested-only"
                ),
                "malformed": problems,
                "legacy_packets_not_counted": len(legacy),
                "legacy": legacy,
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
    out = subprocess.run(
        ["gh", *args], capture_output=True, text=True, encoding="utf-8", errors="replace", check=True
    )
    return json.loads(out.stdout)


def git(repo_dir, *args):
    return subprocess.run(
        ["git", "-C", repo_dir, *args], capture_output=True, text=True, encoding="utf-8", errors="replace",
        check=False,
    )


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
                "reviews": flatten_pages(
                    gh_json("api", f"repos/{repo}/pulls/{item['number']}/reviews", "--paginate", "--slurp")
                ),
                "base_events": sorted(
                    e.get("created_at", "")
                    for e in flatten_pages(
                        gh_json("api", f"repos/{repo}/issues/{item['number']}/timeline", "--paginate", "--slurp")
                    )
                    if e.get("event") in ("base_ref_changed", "base_ref_force_pushed")
                ),
            }
        )
    return {"repo": repo, "prs": prs}


def live_ancestry(repo_dir):
    return lambda a, b: a == b or git(repo_dir, "merge-base", "--is-ancestor", a, b).returncode == 0


def render(rows):
    lines = ["PR    head      covered  structured  auth  legacy(not counted)  reasons"]
    for r in rows:
        reasons = "; ".join(r["reasons"]) or "-"
        lines.append(
            f"#{r['pr']:<4} {r['head'][:8]}  {str(r['covered']):<7}  {r['structured_attestations']:<10}"
            f"  {r.get('authenticated_reviews', 0):<4}  {r['legacy_packets_not_counted']:<19}  {reasons}"
        )
    return "\n".join(lines)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--snapshot")
    parser.add_argument("--repo")
    parser.add_argument("--repo-dir", help="clone of --repo; required with --repo")
    parser.add_argument("--pr", type=int, help="only report this PR")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--identities", help="identity map JSON (seat -> logins/keys); enables formal reviews and signature checks")
    parser.add_argument("--require-signatures", action="store_true",
                        help="enforcement mode: only attestations signed by the reviewer seat's key count")
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
    identities = load_identities(args.identities) if args.identities else None
    keys = load_keys(args.identities) if args.identities else None
    if args.require_signatures and not keys:
        parser.error("--require-signatures needs --identities")
    rows = report_snapshot(snapshot, ancestry, identities, keys, args.require_signatures)
    print(json.dumps(rows, indent=2) if args.json else render(rows))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
