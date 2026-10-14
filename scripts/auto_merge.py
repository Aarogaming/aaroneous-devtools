#!/usr/bin/env python3
"""Auto-merge pipeline (lane C103): the one place that actually presses merge for this repo.

Runs merge_gate.evaluate_all() against the live repo and, for every PR it marks eligible,
merges it with `gh pr merge`. Wired into .github/workflows/auto-merge.yml, this is meant to be
the *only* path that merges a PR here automatically -- an agent with repo write access still
goes through this script rather than calling the merge API directly, per the owner's "take the
agent out of the operation" decision (2026-10-07, see AGENTS.md's parent conversation and
governance/AUTOMATED_MERGE_POLICY.md): the checklist only means something if nothing can reach
the merge button except through it.

Hard-refuses to run against any repository other than aaroneous-devtools. Aarogaming/aaroneous
(the product) is never eligible for automatic merging under this pipeline, regardless of tier --
governance/AUTOMATED_MERGE_POLICY.md's Scope section, and governance/RECURSIVE_AUDIT_CADENCE.md's
Scope section, both say so; this is the mechanical enforcement of that sentence, not a new rule.

Usage:
  auto_merge.py --repo OWNER/NAME --repo-dir CLONE --identities governance/identities.json
                [--pr N] [--dry-run]
"""

import argparse
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import merge_gate as mg  # noqa: E402

ALLOWED_REPO = "Aarogaming/aaroneous-devtools"


def merge(repo, pr_number, expected_head):
    return subprocess.run(
        ["gh", "pr", "merge", str(pr_number), "--merge", "-R", repo,
         "--match-head-commit", expected_head],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
    )


def run(repo, repo_dir, identities_path, only_pr, dry_run, merge_fn=merge):
    """Returns (exit_code, report_lines). Never calls merge_fn when dry_run is True."""
    if repo != ALLOWED_REPO:
        return 1, [f"refusing: this pipeline only merges in {ALLOWED_REPO}, got {repo!r}"]
    lines = []
    exit_code = 0
    decisions = mg.evaluate_all(repo, repo_dir, identities_path, only_pr, False)
    for d in decisions:
        head = d["head"][:8]
        if not d["eligible"]:
            lines.append(f"#{d['pr']} {head} BLOCKED: {'; '.join(d['reasons']) or '-'}")
            continue
        lines.append(f"#{d['pr']} {head} ELIGIBLE")
        if dry_run:
            lines.append(f"  (dry run: would merge #{d['pr']})")
            continue
        result = merge_fn(repo, d["pr"], d["head"])
        if result.returncode != 0:
            lines.append(f"  merge failed: {result.stderr.strip()}")
            exit_code = 1
        else:
            lines.append(f"  merged #{d['pr']}")
    return exit_code, lines


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", required=True)
    parser.add_argument("--repo-dir", required=True)
    parser.add_argument("--identities", required=True)
    parser.add_argument("--pr", type=int)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)
    exit_code, lines = run(args.repo, args.repo_dir, args.identities, args.pr, args.dry_run)
    for line in lines:
        print(line)
    return exit_code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
