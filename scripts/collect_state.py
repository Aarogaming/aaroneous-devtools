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

# A completion RECEIPT is an explicit, positive statement bound to the exact full head:
# "[done:KIND:PR:<40-hex head>]" as the first thing in a comment. The 8-character "[card:...]" marker a card shows
# is only a reference to the task and never completes it. A receipt quoted further down a comment, one for another
# PR or head, one whose first line says the work was not done, or one in a comment that also names a different
# full head with the same first eight characters (contradictory provenance) completes nothing.
RECEIPT_START = re.compile(r"\A\s*\[done:([a-z_]+):(\d+):([0-9a-f]{40})\]", re.I)
NEGATIVE = re.compile(r"\b(not done|not complete[d]?|blocked|could not|cannot|unable|failed)\b", re.I)
HEX40 = re.compile(r"\b[0-9a-f]{40}\b")
RECEIPT_PREFIX = re.compile(r"\A\s*\[done:[a-z_]+:\d+:[0-9a-f]{40}\]\s*[:\u2013\u2014-]?\s*", re.I)
SEAT_WORD = re.compile(r"\b(Claude|Mimo|Codex|Gemini|Antigravity)\s+seat\b", re.I)
KNOWN_SEATS = ("claude", "codex", "gemini", "mimo", "qwen")
SEAT_PATTERN = re.compile(r"\b(" + "|".join(KNOWN_SEATS) + r")\b", re.I)


SEAT_IN_TEXT = re.compile(
    r"(?:Seat\s*/\s*agent|Reviewer(?:\s*/\s*Seat|\s+seat)?)\W{0,6}(Claude|Mimo|Codex|Gemini|Antigravity)"
    # A seat name that opens a line, optionally after a markdown heading mark and the word "Independent" ("### Codex
    # bounded review", "Independent Codex verdict: ACCEPT"), which is how the seats title their reviews.
    r"|^\s*(?:#{1,6}\s*)?(?:Independent\s+)?(Claude|Codex|Gemini|Mimo|Antigravity)\b",
    re.I | re.M,
)
SEAT_DECLARED = re.compile(r"(?:Seat\s*/\s*agent|Reviewer(?:\s*/\s*Seat|\s+seat)?)\W{0,6}(Claude|Mimo|Codex|Gemini|Antigravity)", re.I)
ALIASES = {"antigravity": "gemini"}
MAX_REJECTED_PER_SEAT = 2


def comment_seat(body, strict=False):
    """The seat a comment says it is from ('' when it does not say). Antigravity is the Gemini seat.

    strict=True accepts only an explicit declaration ("Seat / agent: X", "Reviewer: X") or a seat name that opens
    the comment, so a mention of a seat further down a comment never attributes it. A leading completion receipt
    ("[done:KIND:PR:HEAD]") is not part of the prose, so a seat name right after it opens the comment: the card says
    "start with the receipt and name your seat on the first line", and "[done:...] Claude Sonnet 5.5 - done" is how
    that reads."""
    body = body or ""
    if strict:
        first = next((line for line in body.splitlines() if line.strip()), "")
        first = RECEIPT_PREFIX.sub("", first, count=1)
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


NEAR_MISS_PREFIX = 7


def git_commit_exists(repo_dir):
    """A `commit_exists` callback backed by a clone: True when the commit is present, False when git reports it
    absent (exit 1), and None when the lookup itself failed (git missing, not a repository, any other exit code).
    Absent means absent from THIS clone, not from the repository: an unfetched historical head also reads False."""
    def exists(token):
        try:
            result = rc.git(repo_dir, "rev-parse", "--verify", "--quiet", f"{token}^{{commit}}")
        except OSError:
            return None
        return {0: True, 1: False}.get(result.returncode)
    return exists


def phantom_head(body, head, commit_exists):
    """A full 40-character token in the comment that agrees with the PR's current head in its first
    NEAR_MISS_PREFIX characters, differs after that, and is reported absent: a probably mistyped or invented head,
    which is not the same thing as a genuine older head (two real commits agreeing in seven hex characters is
    about one in 2.7e8). Returns that token, or None. `commit_exists` returns True, False, or None for a failed
    lookup; only False flags, so a git failure never turns into an accusation. Without it nothing is flagged."""
    head = (head or "").lower()
    if commit_exists is None or not head:
        return None
    for token in HEX40.findall((body or "").lower()):
        agree = 0
        while agree < len(head) and token[agree] == head[agree]:
            agree += 1
        if token != head and agree >= NEAR_MISS_PREFIX and commit_exists(token) is False:
            return token
    return None


def rejected_verdicts(pr, row, commit_exists=None):
    """Verdict comments the coverage tool did not count at the PR's CURRENT head, each attributed to a seat where
    the text explicitly says which. A comment that does not cite the current head is history, not a pending
    repair, and at most the latest MAX_REJECTED_PER_SEAT per seat are kept so a card stays short. The exception
    is a comment citing a head that does not exist but almost matches the current one (see `phantom_head`): that
    is a mistyped head, not history, so its author is told."""
    comments = pr.get("comments", [])
    head = pr.get("head") or ""
    found = []

    def entry(index, reason):
        if not isinstance(index, int) or not 0 <= index < len(comments):
            return
        comment = comments[index] if isinstance(comments[index], dict) else {}
        body = comment.get("body", "")
        if not cites_head(body, head):
            phantom = phantom_head(body, head, commit_exists)
            if not phantom:
                return
            reason = (f"it cites {phantom}, which was not found as a commit in the steward's clone (a suspected head "
                      f"mismatch, not proof); the PR's current head is {head} and only the first characters match, so it "
                      f"looks mistyped. Copy the head from the card. Also: {reason}")
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
    "pullRequests(states:OPEN,first:100){pageInfo{hasNextPage} nodes{number "
    "reviewThreads(first:100){pageInfo{hasNextPage} nodes{isResolved comments(last:1){totalCount nodes{author{login} body}}}}}}}}"
)


def unresolved_by_pr(graphql):
    """Unresolved review threads per open PR. The query reads the first 100 PRs and the first 100 threads of each, so a
    larger answer is truncated and must not be read as "no more unresolved threads": more than 100 PRs stops with a
    message (a missing PR would read as zero threads and look ready to merge), and a PR with a further page of threads
    counts one extra, so it is never reported as having none. `merge_gate.open_threads` already fails closed the same way.
    A response without `pageInfo` is read as complete, as older fixtures are."""
    pull_requests = graphql["data"]["repository"]["pullRequests"]
    if (pull_requests.get("pageInfo") or {}).get("hasNextPage"):
        raise SystemExit("more than 100 open pull requests: unresolved-thread counts would be missing for the rest; "
                         "paginate the threads query before trusting them")
    counts = {}
    for n in pull_requests["nodes"]:
        threads = n["reviewThreads"]
        counts[n["number"]] = sum(1 for t in threads["nodes"] if not t["isResolved"])
        if (threads.get("pageInfo") or {}).get("hasNextPage"):
            counts[n["number"]] += 1  # at least one more thread exists that this page did not show
    return counts


STANDING_REASON = re.compile(r"^standing (\w+) at head by (.+)$")


def standing_blocker(reasons):
    """The standing rejecting verdict at the PR's current head, from the coverage tool's reasons: {"verdict", "by"}, or
    None. "standing needs_revision at head by Claude" is a reviewer waiting for the author to change something; no
    further review of the same head can help."""
    for reason in reasons or []:
        match = STANDING_REASON.match(reason)
        if match:
            return {"verdict": match.group(1), "by": match.group(2)}
    return None


def thread_unanswered(thread, author_seats=()):
    """True unless the PR's own side has the last word in the thread. Every seat and the owner post from one shared
    login, so a person-versus-bot test cannot tell an author's answer from a reviewer's "still missing". The last comment
    is therefore attributed to a seat by its text (`comment_seat`, strict), and the thread counts as answered only when
    that seat is one of the PR's implementers (`author_seats`). A single comment, a bot, a reviewer seat, a comment that
    names no seat, unknown comment data, or no known implementers all count as unanswered: the author keeps the fix card,
    which is the old behaviour and the safe direction."""
    comments = thread.get("comments") or {}
    if "totalCount" not in comments or "nodes" not in comments:
        return True
    if comments["totalCount"] <= 1 or not comments["nodes"]:
        return True
    last = comments["nodes"][-1]
    login = (last.get("author") or {}).get("login", "")
    if login == "" or login.endswith("[bot]"):
        return True
    seat = comment_seat(last.get("body") or "", strict=True)
    return not (seat and seat in {s.lower() for s in author_seats})


def unanswered_by_pr(graphql, implementers_by_pr=None):
    """Unresolved review threads the PR's side has not answered, per open PR (see `thread_unanswered`; the PR's
    implementers come from `implementers_by_pr`). The same truncation rules as `unresolved_by_pr`: a further page of
    threads counts one extra, so it is never reported as none."""
    implementers_by_pr = implementers_by_pr or {}
    pull_requests = graphql["data"]["repository"]["pullRequests"]
    if (pull_requests.get("pageInfo") or {}).get("hasNextPage"):
        raise SystemExit("more than 100 open pull requests: unresolved-thread counts would be missing for the rest; "
                         "paginate the threads query before trusting them")
    counts = {}
    for n in pull_requests["nodes"]:
        threads = n["reviewThreads"]
        authors = implementers_by_pr.get(n["number"], ())
        counts[n["number"]] = sum(1 for t in threads["nodes"] if not t["isResolved"] and thread_unanswered(t, authors))
        if (threads.get("pageInfo") or {}).get("hasNextPage"):
            counts[n["number"]] += 1
    return counts


def eligible_completer(kind, seat, implementers, repair_owners=()):
    """Whether this seat may complete a card of this kind. It fails closed: an unattributed comment never completes
    a card; a review needs KNOWN implementers and a seat that is not one of them (unknown ownership must not make
    every seat, including the author, look independent); only the author seat completes its own thread fixes; and
    only a seat whose verdict was rejected completes that repair."""
    if not seat:
        return False
    if kind == "needs_review":
        return bool(implementers) and seat not in implementers
    if kind == "fix_threads":
        return bool(implementers) and seat == implementers[0]
    if kind == "resolve_threads":
        return bool(implementers) and seat not in implementers
    if kind == "address_review":
        return bool(implementers) and seat == implementers[0]
    if kind == "repair_verdict":
        return seat in repair_owners
    return True


def done_keys(snapshot, repair_owners=None, implementers_by_pr=None):
    """Keys of cards completed by an explicit receipt (see RECEIPT_START): a comment on that PR that starts with
    [done:KIND:PR:FULL_HEAD] for the PR's current head, says nothing negative on its first line, names no
    conflicting full head, and comes from a seat allowed to complete that kind of card.
    repair_owners maps a PR number to the seats whose verdicts were rejected there."""
    return sorted(done_receipts(snapshot, repair_owners, implementers_by_pr))


# Cards whose completion is visible on GitHub itself (a thread is resolved, a rejecting verdict stops standing). A
# receipt for one of these would complete the lease while the item is still open, and the dispatcher would lease the same
# key again on the next tick, so receipts for them are not read.
# identify_implementers belongs here too: its completion is the implementers becoming known (a trailer or a seat-named
# branch), which a comment cannot establish. A receipt would otherwise remove the item while the implementers are still
# unknown, and every review, fix and resolve card for that PR stays withheld until its head moves.
# not_mergeable too: its end state (the conflict is gone) is on GitHub, and any seat's receipt at the same head closed the
# card while the PR was still DIRTY (eligible_completer returns True for kinds it does not list).
OBSERVABLE_KINDS = ("resolve_threads", "address_review", "identify_implementers", "not_mergeable")


def done_receipts(snapshot, repair_owners=None, implementers_by_pr=None):
    """The same completions as `done_keys`, as {key: the seat whose receipt completed it}. The first eligible receipt
    for a key names the seat. A lease holder is credited with a completion only when it is that seat."""
    repair_owners = repair_owners or {}
    implementers_by_pr = implementers_by_pr or {}
    keys = {}
    for p in snapshot["prs"]:
        head = p["head"].lower()
        implementers = implementers_by_pr.get(p["number"]) or p.get("implementers") or []
        for comment in p.get("comments", []):
            body = comment.get("body", "") if isinstance(comment, dict) else ""
            match = RECEIPT_START.match(body)
            if not match:
                continue
            kind, number, receipt_head = match.group(1).lower(), int(match.group(2)), match.group(3).lower()
            if kind in OBSERVABLE_KINDS:
                continue
            if number != p["number"] or receipt_head != head:
                continue
            first = next((line for line in body.splitlines() if line.strip()), "")
            if NEGATIVE.search(first):
                continue
            if any(token != head and token[:8] == head[:8] for token in HEX40.findall(body.lower())):
                continue
            seat = comment_seat(body, strict=True)
            if not eligible_completer(kind, seat, implementers, repair_owners.get(p["number"], ())):
                continue
            keys.setdefault(f"{kind}:{number}:{head[:8]}", seat)
    return keys


def build_state(snapshot, coverage_rows, meta, unresolved, seats, agents, backlog, now, default_branch="main",
                commit_exists=None, unanswered=None):
    covered = {r["pr"]: r["covered"] for r in coverage_rows}
    rows = {r["pr"]: r for r in coverage_rows}
    implementers_by_pr = {
        p["number"]: list(p.get("implementers") or ([seats[p["number"]]] if seats.get(p["number"]) else []))
        for p in snapshot["prs"]
    }
    rejected_by_pr = {p["number"]: rejected_verdicts(p, rows.get(p["number"], {}), commit_exists) for p in snapshot["prs"]}
    repair_owners = {n: {r["seat"] for r in found if r["seat"]} for n, found in rejected_by_pr.items()}
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
                "rejected_verdicts": rejected_by_pr[n],
                "size": f"+{m.get('additions', 0)}/-{m.get('deletions', 0)} in {m.get('changedFiles', 0)} file(s)",
                "implementers": implementers_by_pr[n],
                "threads_unresolved": unresolved.get(n, 0),
                "threads_unanswered": (unanswered if unanswered is not None else unresolved).get(n, 0),
                "covered": covered.get(n, False),
                "standing_blocker": standing_blocker(rows.get(n, {}).get("reasons")),
            }
        )
    return {"now": now, "agents": [{"name": a} for a in agents], "prs": prs, "backlog": backlog,
            "done": done_keys(snapshot, repair_owners, implementers_by_pr),
            "done_by": done_receipts(snapshot, repair_owners, implementers_by_pr), "default_branch": default_branch}


def collect(repo, repo_dir, agents, backlog, identities=None, keys=None, require_signatures=False):
    """Read the live state. ``keys`` (the identity registry) and ``require_signatures`` reach `report_snapshot`, so once
    the owner registers seat keys the steward can stop counting unsigned, spoofable attestation blocks as coverage
    (Codex P2 on #99: the steward loaded the keys and dropped them). Enforcement without keys fails closed."""
    if require_signatures and not (keys and rc.attest_sig.allowed_signers(keys).strip()):
        raise SystemExit("--require-signatures needs --identities with registered seat keys: nothing could verify a signature")
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
    rows = rc.report_snapshot(snapshot, rc.live_ancestry(repo_dir), identities, keys, require_signatures)
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    default = rc.gh_json("repo", "view", repo, "--json", "defaultBranchRef")["defaultBranchRef"]["name"]
    return build_state(snapshot, rows, meta, unresolved_by_pr(graphql), seats, agents, backlog, now, default,
                       git_commit_exists(repo_dir),
                       unanswered_by_pr(graphql, {p["number"]: p["implementers"] for p in snapshot["prs"]}))


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", required=True)
    parser.add_argument("--repo-dir", required=True)
    parser.add_argument("--agents", default="claude,mimo,codex,gemini")
    parser.add_argument("--backlog", help="JSON file: list of {id, task, acceptance}")
    parser.add_argument("--identities", help="identity map JSON (governance/identities.json)")
    parser.add_argument("--require-signatures", action="store_true",
                        help="count only attestations whose signature verifies against the registered seat keys")
    args = parser.parse_args(argv)
    remote = rc.git(args.repo_dir, "remote", "get-url", "origin")
    if remote.returncode != 0 or not rc.checkout_matches(remote.stdout, args.repo):
        raise SystemExit(f"--repo-dir {args.repo_dir!r} is not a clone of {args.repo}")
    backlog = []
    if args.backlog:
        with open(args.backlog, encoding="utf-8") as handle:
            backlog = json.load(handle)
    identities = rc.load_identities(args.identities) if args.identities else None
    keys = rc.load_keys(args.identities) if args.identities else None
    state = collect(args.repo, args.repo_dir, [a for a in args.agents.split(",") if a], backlog, identities, keys,
                    args.require_signatures)
    print(json.dumps(state, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
