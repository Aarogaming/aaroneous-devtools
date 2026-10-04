#!/usr/bin/env python3
"""Local merge gate (lane C100): our rules, enforced by us, decided offline.

Decides whether a PR may merge, from signed attestations plus checks run in a clean local worktree.
GitHub is only the transport; its review state and branch protection are not consulted.

This version is a dry-run decision tool: it prints the decision, the check plan and the evidence, and
never merges or pushes. Applying a merge is a separate, owner-run step (the gate holds the only
credential that can push `main`; agents do not).

Conditions (mirroring the routine's step 3, with local equivalents):
  a. covered: a signed, independent, whole-diff accept chained to the current head and merge-base
  b. mergeable: a trial merge into the base has no conflicts
  c. checks: the path-based plan passes in a clean worktree (only when --run-checks is given)
  d. base is `main`
  e. no standing changes_requested at the head (part of coverage)
  f. protected paths need explicit owner confirmation: AGENTS.md, .github/workflows, settings

Usage:
  merge_gate.py --repo OWNER/NAME --repo-dir CLONE --identities governance/identities.json [--pr N]
                [--run-checks] [--json]
"""

import argparse
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import review_coverage as rc  # noqa: E402

PROTECTED = ("AGENTS.md", ".github/workflows/", ".github/settings", "SECURITY.md")
DOCS_ALLOWLIST = ("governance/", "docs/", "architecture/")
FULL_GATE = [
    ["git", "diff", "--check"],
    ["python", "scripts/check_coordination_evidence.py", "{base}", "HEAD"],
    ["cargo", "fmt", "--all", "--", "--check"],
    ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    ["cargo", "test", "-p", "foundation_cli"],
]
LIGHT_GATE = [
    ["git", "diff", "--check"],
    ["python", "scripts/check_coordination_evidence.py", "{base}", "HEAD"],
]


def touches_protected(paths):
    return [p for p in paths if p == "AGENTS.md" or any(p.startswith(x) for x in PROTECTED if x != "AGENTS.md")]


def gate_plan(paths):
    """Path-based plan: only an allowlist of docs/ledger paths runs the light gate; all else is full."""
    docs_only = bool(paths) and all(p.endswith((".md", ".json")) and p.startswith(DOCS_ALLOWLIST) for p in paths)
    return {"level": "light" if docs_only else "full", "commands": LIGHT_GATE if docs_only else FULL_GATE}


def decide(pr_row, paths, base_ref, trial_merge_ok, owner_confirmed_protected=False):
    reasons = []
    if not pr_row["covered"]:
        reasons.append("a: not covered by a signed independent whole-diff accept at the current head")
    if not trial_merge_ok:
        reasons.append("b: trial merge into the base conflicts")
    if base_ref != "main":
        reasons.append(f"d: base is {base_ref}, not main")
    protected = touches_protected(paths)
    if protected and not owner_confirmed_protected:
        reasons.append(f"f: touches protected paths without owner confirmation: {protected}")
    return {"eligible": not reasons, "reasons": reasons, "plan": gate_plan(paths)}


def changed_paths(repo_dir, base_ref, head):
    out = rc.git(repo_dir, "diff", "--name-only", f"origin/{base_ref}...{head}").stdout
    return [line for line in out.splitlines() if line]


def trial_merge(repo_dir, base_ref, head):
    return rc.git(repo_dir, "merge-tree", "--write-tree", f"origin/{base_ref}", head).returncode == 0


def run_checks(repo_dir, head, base_ref, plan):
    """Run the plan in a throwaway worktree of the head; returns [{cmd, rc}] and always cleans up."""
    results = []
    tmp = os.path.join(os.path.abspath(repo_dir), "..", "merge-gate-" + head[:8])
    rc.git(repo_dir, "worktree", "add", "--detach", tmp, head)
    try:
        for cmd in plan["commands"]:
            cmd = [c.replace("{base}", f"origin/{base_ref}") for c in cmd]
            workdir = os.path.join(tmp, "foundation") if cmd[0] == "cargo" else tmp
            done = subprocess.run(cmd, cwd=workdir, capture_output=True, text=True, encoding="utf-8", errors="replace")
            results.append({"cmd": " ".join(cmd), "rc": done.returncode})
            if done.returncode != 0:
                break
    finally:
        rc.git(repo_dir, "worktree", "remove", "--force", tmp)
    return results


def evaluate_all(repo, repo_dir, identities_path, only_pr, do_checks):
    snapshot = rc.live_snapshot(repo, repo_dir)
    keys = rc.load_keys(identities_path)
    rows = {r["pr"]: r for r in rc.report_snapshot(
        snapshot, rc.live_ancestry(repo_dir), rc.load_identities(identities_path), keys, True)}
    decisions = []
    for pr in snapshot["prs"]:
        if only_pr and pr["number"] != only_pr:
            continue
        paths = changed_paths(repo_dir, pr["base"], pr["head"])
        decision = decide(rows[pr["number"]], paths, pr["base"], trial_merge(repo_dir, pr["base"], pr["head"]))
        decision.update({"pr": pr["number"], "head": pr["head"], "paths": paths})
        if do_checks and decision["eligible"]:
            decision["checks"] = run_checks(repo_dir, pr["head"], pr["base"], decision["plan"])
            if any(c["rc"] != 0 for c in decision["checks"]):
                decision["eligible"] = False
                decision["reasons"].append("c: a gate command failed in the clean worktree")
        decisions.append(decision)
    return decisions


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", required=True)
    parser.add_argument("--repo-dir", required=True)
    parser.add_argument("--identities", required=True)
    parser.add_argument("--pr", type=int)
    parser.add_argument("--run-checks", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    remote = rc.git(args.repo_dir, "remote", "get-url", "origin")
    if remote.returncode != 0 or not rc.checkout_matches(remote.stdout, args.repo):
        raise SystemExit(f"--repo-dir {args.repo_dir!r} is not a clone of {args.repo}")
    decisions = evaluate_all(args.repo, args.repo_dir, args.identities, args.pr, args.run_checks)
    if args.json:
        print(json.dumps(decisions, indent=2))
    else:
        for d in decisions:
            verdict = "ELIGIBLE" if d["eligible"] else "BLOCKED"
            print(f"#{d['pr']} {d['head'][:8]} {verdict} plan={d['plan']['level']} " + ("; ".join(d["reasons"]) or "-"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
