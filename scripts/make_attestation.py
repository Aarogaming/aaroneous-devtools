#!/usr/bin/env python3
"""Build a valid, head-bound attestation block for a pull request (lane C100). Prints it; posts nothing.

Reviewers have been hand-writing these blocks, and the same mistakes keep costing a round trip: YAML instead of JSON,
an upper-case verdict, a short or stale head, a guessed base_sha, a missing implementer. This reads the facts from
the clone and GitHub instead:

  make_attestation.py --repo OWNER/NAME --repo-dir CLONE --pr 110 --reviewer gemini --verdict accept [--out block.md]

head is the PR's current head, base_sha is `git merge-base origin/<base> <head>`, implementers come from the branch
prefix and the co-author trailers in base..head (add seats that pushed without a trailer with `--implementer`, which
repeats). The block is checked with the same `validate` the coverage tool uses, and it refuses (exit 2) to build a
verdict for a reviewer whose seat is among the implementers: that verdict would never count, so it should not be
posted. Paste the file's contents as a PR comment; the verdict counts only at this head.
"""

import argparse
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import collect_state as cs  # noqa: E402
import review_coverage as rc  # noqa: E402

VERDICTS = ("accept", "needs_revision", "changes_requested")


def build_record(repo, pr, head, base_sha, reviewer, implementers, verdict, scope="whole", frm=None):
    """(record, error). The error is a sentence, never a traceback; the record is built only when it would be counted."""
    if verdict not in VERDICTS:
        return None, f"verdict must be one of {VERDICTS}, got {verdict!r}"
    if len(head) != 40 or any(c not in "0123456789abcdef" for c in head):
        return None, f"head must be the full 40-character lower-case SHA, got {head!r}"
    record = {"repo": repo, "pr": pr, "head": head, "base_sha": base_sha, "scope": scope, "reviewer": reviewer,
              "implementers": list(implementers), "verdict": verdict}
    if scope == "delta":
        if not frm:
            return None, "a delta attestation needs --from <the head already reviewed>"
        record["from"] = frm
    problem = rc.validate(record)
    if problem:
        return None, problem
    if verdict == "accept" and not rc.independent(record):
        return None, (f"reviewer {reviewer!r} is among the implementers {sorted(implementers)}: an accept from this seat "
                      "can never count; another seat has to review")
    return record, None


def render_block(record):
    return "```attestation\n" + json.dumps(record, indent=2) + "\n```\n"


def gather(repo, repo_dir, pr, extra_implementers, run=None):
    """(head, base_sha, implementers, error) from the clone and GitHub. `run(*gh args)` returns parsed JSON."""
    run = run or rc.gh_json
    remote = rc.git(repo_dir, "remote", "get-url", "origin")
    if remote.returncode != 0 or not rc.checkout_matches(remote.stdout, repo):
        return None, None, None, f"--repo-dir {repo_dir!r} is not a clone of {repo}"
    info = run("pr", "view", str(pr), "-R", repo, "--json", "headRefOid,baseRefName,headRefName")
    head, base = info["headRefOid"], f"origin/{info['baseRefName']}"
    if rc.git(repo_dir, "cat-file", "-e", head + "^{commit}").returncode != 0:
        return None, None, None, (f"head {head} is not in the clone; fetch it first: "
                                  f"git fetch origin refs/pull/{pr}/head")
    merge_base = rc.git(repo_dir, "merge-base", base, head).stdout.strip()
    if not merge_base:
        return None, None, None, f"no merge base between {base} and {head}; fetch origin first"
    log = rc.git(repo_dir, "log", "--format=%B", f"{base}..{head}").stdout
    implementers = list(cs.seats_from_trailers(log, info["headRefName"]))
    for name in extra_implementers:
        if name not in implementers:
            implementers.append(name)
    if not implementers:
        return None, None, None, ("no implementing seat found in the branch name or trailers; name them with "
                                  "--implementer (an empty list is not a valid attestation)")
    return head, merge_base, implementers, None


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", required=True)
    parser.add_argument("--repo-dir", required=True)
    parser.add_argument("--pr", required=True, type=int)
    parser.add_argument("--reviewer", required=True, help="your seat: claude, codex, gemini or mimo")
    parser.add_argument("--verdict", required=True, choices=VERDICTS)
    parser.add_argument("--scope", default="whole", choices=("whole", "delta"))
    parser.add_argument("--from", dest="frm", help="for --scope delta: the head you already reviewed")
    parser.add_argument("--implementer", action="append", default=[], help="a seat that pushed without a trailer")
    parser.add_argument("--out", help="write the block to this file instead of stdout")
    args = parser.parse_args(argv)
    head, base_sha, implementers, error = gather(args.repo, args.repo_dir, args.pr, args.implementer)
    if error:
        print(error, file=sys.stderr)
        return 3
    record, error = build_record(args.repo, args.pr, head, base_sha, args.reviewer, implementers, args.verdict,
                                 args.scope, args.frm)
    if error:
        print(error, file=sys.stderr)
        return 2
    block = render_block(record)
    if args.out:
        with open(args.out, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(block)
    else:
        sys.stdout.write(block)
    print(f"built for head {head[:12]}, base_sha {base_sha[:12]}, implementers {implementers}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
