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

MARKER = re.compile(r"\[card:([a-z_]+:\d+:[0-9a-f]{8})\]")
BRIEF = re.compile(r"OWNER MERGE BRIEF", re.I)
KNOWN_SEATS = ("claude", "codex", "gemini", "mimo", "qwen")
SEAT_PATTERN = re.compile(r"\b(" + "|".join(KNOWN_SEATS) + r")\b", re.I)


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


def done_keys(snapshot):
    """Keys of cards already completed: explicit [card:...] markers in comments, plus merge briefs that
    name the PR head (so briefs posted before markers existed still count)."""
    keys = set()
    for p in snapshot["prs"]:
        head8 = p["head"][:8]
        for comment in p.get("comments", []):
            body = comment.get("body", "") if isinstance(comment, dict) else ""
            keys.update(MARKER.findall(body))
            if BRIEF.search(body) and head8 in body:
                keys.add(f"ready_to_merge:{p['number']}:{head8}")
    return sorted(keys)


def build_state(snapshot, coverage_rows, meta, unresolved, seats, agents, backlog, now, default_branch="main"):
    covered = {r["pr"]: r["covered"] for r in coverage_rows}
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
                            "number,title,isDraft,mergeStateStatus,updatedAt,headRefName")
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
