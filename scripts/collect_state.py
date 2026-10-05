#!/usr/bin/env python3
"""Read-only state collector for next_task.py (lane C99).

Builds the snapshot next_task.py consumes from live GitHub data and a local clone, so the dispatcher
needs no hand-made input. It only reads: `gh pr list`, `gh api` (comments, GraphQL review threads) and
`git merge-base`. It writes the snapshot JSON to stdout and never posts, edits or merges.

Usage:
  collect_state.py --repo OWNER/NAME --repo-dir CLONE [--agents claude,mimo,codex,gemini]
                   [--backlog FILE.json] > state.json
"""

import argparse
import json
import os
import re
import sys
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(__file__))
import review_coverage as rc  # noqa: E402

# A completion receipt is a comment that STARTS with the card marker. A marker quoted or mentioned further
# down a comment is a reference, not a receipt. "[card:KEY blocked]" reports a block and completes nothing.
MARKER_START = re.compile(r"\A\s*\[card:([a-z_]+):(\d+):([0-9a-f]{8})(?:\s+(done|blocked))?\]", re.I)
SEAT_WORD = re.compile(r"\b(Claude|Mimo|Codex|Gemini|Antigravity)\s+seat\b", re.I)
KNOWN_SEATS = ("claude", "codex", "gemini", "mimo", "qwen")
SEAT_PATTERN = re.compile(r"\b(" + "|".join(KNOWN_SEATS) + r")\b", re.I)


SEAT_IN_TEXT = re.compile(
    r"(?:Seat\s*/\s*agent|Reviewer(?:\s+seat)?)\W{0,6}(Claude|Mimo|Codex|Gemini|Antigravity)"
    r"|^\s*(Claude|Codex|Gemini|Mimo|Antigravity)\b",
    re.I | re.M,
)
SEAT_DECLARED = re.compile(r"(?:Seat\s*/\s*agent|Reviewer(?:\s+seat)?)\W{0,6}(Claude|Mimo|Codex|Gemini|Antigravity)", re.I)
ALIASES = {"antigravity": "gemini"}
MAX_REJECTED_PER_SEAT = 2


def comment_seat(body, strict=False):
    """The seat a comment says it is from ('' when it does not say). Antigravity is the Gemini seat.

    strict=True accepts only an explicit declaration ("Seat / agent: X", "Reviewer: X") or a seat name that opens
    the comment, so a mention of a seat further down a comment never attributes it."""
    body = body or ""
    if strict:
        first = next((line for line in body.splitlines() if line.strip()), "")
        found = SEAT_IN_TEXT.search(first) or SEAT_WORD.search(first) or SEAT_DECLARED.search(body)
    else:
        found = SEAT_IN_TEXT.search(body)
    if not found:
        return ""
    name = next(g for g in found.groups() if g).lower()
    return ALIASES.get(name, name)


def explain_problem(raw):
    """The coverage tool's rejection reason in words an author can act on."""
    raw = raw or ""
    if raw.startswith("invalid JSON") or raw.startswith("not a JSON object"):
        return "the attestation block is not valid JSON; it must be one JSON object, not YAML or prose"
    if raw.startswith("missing fields"):
        return "the attestation block is " + raw
    if raw.startswith("implementers must be"):
        return 'implementers must be a non-empty list such as ["Claude"]'
    if raw.startswith("unknown scope"):
        return 'scope must be "whole" or "delta"'
    if "does not match" in raw:
        return "the block names a different repository or PR than the one it was posted on"
    if raw.startswith("unsigned or unverifiable"):
        return "signatures are required and this block is unsigned or does not verify against the registered key"
    return raw


LEGACY_REASON = "the comment reads like a verdict but has no fenced attestation block, so nothing was counted"


HEX_TOKEN = re.compile(r"\b[0-9a-f]{8,40}\b")


def cites_head(body, head):
    """True when the comment cites this PR head: a hex token of 8 to 40 characters that is a prefix of the full head.
    A 40-character token must equal the head, so a comment citing a different head that merely shares the first
    eight characters is not mistaken for one that cites this head."""
    head = (head or "").lower()
    if not head:
        return False
    return any(head.startswith(token) for token in HEX_TOKEN.findall((body or "").lower()))


def rejected_verdicts(pr, row):
    """Verdict comments the coverage tool did not count at the PR's CURRENT head, each attributed to a seat where
    the text explicitly says which. A comment that does not cite the current head is history, not a pending
    repair, and at most the latest MAX_REJECTED_PER_SEAT per seat are kept so a card stays short."""
    comments = pr.get("comments", [])
    head = pr.get("head") or ""
    found = []

    def entry(index, reason):
        if not isinstance(index, int) or not 0 <= index < len(comments):
            return
        comment = comments[index] if isinstance(comments[index], dict) else {}
        body = comment.get("body", "")
        if not cites_head(body, head):
            return
        found.append({"comment_id": comment.get("id"), "seat": comment_seat(body, strict=True), "reason": reason,
                      "_index": index})

    for problem in row.get("malformed", []):
        if "comment" in problem:
            entry(problem["comment"], explain_problem(problem.get("problem", "")))
    for packet in row.get("legacy", []):
        entry(packet.get("comment"), LEGACY_REASON)
    found.sort(key=lambda item: item["_index"])
    kept, per_seat = [], {}
    for item in reversed(found):
        count = per_seat.get(item["seat"], 0)
        if count < MAX_REJECTED_PER_SEAT:
            per_seat[item["seat"]] = count + 1
            kept.append(item)
    kept.reverse()
    return [{k: v for k, v in item.items() if k != "_index"} for item in kept]


def seats_from_trailers(text, branch=""):
    """Every implementing seat found, deduplicated in order: a seat-named branch prefix first (the
    branch owner), then every seat named in a co-author or contribution trailer anywhere in the range.
    Independence needs the whole list, not just the first match."""
    seats = []

    def add(name):
        name = name.lower()
        if name not in seats:
            seats.append(name)

    prefix = SEAT_PATTERN.match(branch.split("/", 1)[0]) if branch else None
    if prefix:
        add(prefix.group(1))
    for line in text.splitlines():
        if line.lower().startswith(("co-authored-by:", "contribution-id:")):
            for found in SEAT_PATTERN.finditer(line):
                add(found.group(1))
    return seats


def seat_from_trailers(text, branch=""):
    """Primary implementing seat (the branch owner when the branch names one, else the first trailer
    seat); '' when unknown."""
    seats = seats_from_trailers(text, branch)
    return seats[0] if seats else ""


THREADS_QUERY = (
    'query($owner:String!,$name:String!){repository(owner:$owner,name:$name){'
    "pullRequests(states:OPEN,first:100){nodes{number reviewThreads(first:100){nodes{isResolved}}}}}}"
)


def unresolved_by_pr(graphql):
    nodes = graphql["data"]["repository"]["pullRequests"]["nodes"]
    return {
        n["number"]: sum(1 for t in n["reviewThreads"]["nodes"] if not t["isResolved"]) for n in nodes
    }


def eligible_completer(kind, seat, implementers):
    """Whether this seat may complete a card of this kind. An unattributed comment never completes a card, an
    implementer cannot complete a review of its own work, and only the author seat completes its own thread fixes."""
    if not seat:
        return False
    if kind == "needs_review":
        return seat not in implementers
    if kind == "fix_threads":
        return bool(implementers) and seat == implementers[0]
    return True


def done_keys(snapshot):
    """Keys of cards completed by an explicit receipt: a comment on that PR that starts with the card marker, names
    the PR's current head, is not marked blocked, and comes from a seat allowed to complete that kind of card.
    Quoted or mid-text markers, a marker for another PR or head, an unattributed comment and a blocked report
    never complete a card."""
    keys = set()
    for p in snapshot["prs"]:
        head8 = p["head"][:8].lower()
        implementers = p.get("implementers") or []
        for comment in p.get("comments", []):
            body = comment.get("body", "") if isinstance(comment, dict) else ""
            match = MARKER_START.match(body)
            if not match:
                continue
            kind, number, marker_head, status = match.group(1), int(match.group(2)), match.group(3).lower(), match.group(4)
            if number != p["number"] or marker_head != head8 or (status or "").lower() == "blocked":
                continue
            if not eligible_completer(kind, comment_seat(body, strict=True), implementers):
                continue
            keys.add(f"{kind}:{number}:{marker_head}")
    return sorted(keys)


def build_state(snapshot, coverage_rows, meta, unresolved, seats, agents, backlog, now, default_branch="main"):
    covered = {r["pr"]: r["covered"] for r in coverage_rows}
    rows = {r["pr"]: r for r in coverage_rows}
    prs = []
    for p in snapshot["prs"]:
        n = p["number"]
        m = meta[n]
        prs.append(
            {
                "number": n,
                "title": m["title"],
                "head": p["head"],
                "base": p["base"],
                "draft": m["isDraft"],
                "mergeable_state": m["mergeStateStatus"],
                "updated_at": m["updatedAt"],
                "seat": seats.get(n, ""),
                "base_sha": p.get("merge_base") or "",
                "rejected_verdicts": rejected_verdicts(p, rows.get(n, {})),
                "size": f"+{m.get('additions', 0)}/-{m.get('deletions', 0)} in {m.get('changedFiles', 0)} file(s)",
                "implementers": list(p.get("implementers") or ([seats[n]] if seats.get(n) else [])),
                "threads_unresolved": unresolved.get(n, 0),
                "covered": covered.get(n, False),
            }
        )
    return {"now": now, "agents": [{"name": a} for a in agents], "prs": prs, "backlog": backlog,
            "done": done_keys(snapshot), "default_branch": default_branch}


def collect(repo, repo_dir, agents, backlog, identities=None):
    snapshot = rc.live_snapshot(repo, repo_dir)
    meta = {
        p["number"]: p
        for p in rc.gh_json("pr", "list", "-R", repo, "--state", "open", "--limit", "1000", "--json",
                            "number,title,isDraft,mergeStateStatus,updatedAt,headRefName,additions,deletions,changedFiles")
    }
    owner, name = repo.split("/", 1)
    graphql = rc.gh_json("api", "graphql", "-f", f"query={THREADS_QUERY}", "-F", f"owner={owner}", "-F", f"name={name}")
    seats = {}
    for p in snapshot["prs"]:
        log = rc.git(repo_dir, "log", "--format=%B", f"origin/{p['base']}..{p['head']}").stdout
        every = seats_from_trailers(log, meta[p["number"]].get("headRefName", ""))
        seats[p["number"]] = every[0] if every else ""
        p["implementers"] = every
    rows = rc.report_snapshot(snapshot, rc.live_ancestry(repo_dir), identities)
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    default = rc.gh_json("repo", "view", repo, "--json", "defaultBranchRef")["defaultBranchRef"]["name"]
    return build_state(snapshot, rows, meta, unresolved_by_pr(graphql), seats, agents, backlog, now, default)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", required=True)
    parser.add_argument("--repo-dir", required=True)
    parser.add_argument("--agents", default="claude,mimo,codex,gemini")
    parser.add_argument("--backlog", help="JSON file: list of {id, task, acceptance}")
    parser.add_argument("--identities", help="identity map JSON (governance/identities.json)")
    args = parser.parse_args(argv)
    remote = rc.git(args.repo_dir, "remote", "get-url", "origin")
    if remote.returncode != 0 or not rc.checkout_matches(remote.stdout, args.repo):
        raise SystemExit(f"--repo-dir {args.repo_dir!r} is not a clone of {args.repo}")
    backlog = []
    if args.backlog:
        with open(args.backlog, encoding="utf-8") as handle:
            backlog = json.load(handle)
    identities = rc.load_identities(args.identities) if args.identities else None
    state = collect(args.repo, args.repo_dir, [a for a in args.agents.split(",") if a], backlog, identities)
    print(json.dumps(state, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
