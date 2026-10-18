#!/usr/bin/env python3
"""Local merge gate (lane C100): our rules, enforced by us, decided offline.

Decides whether a PR may merge, from signed attestations plus checks run in a clean local worktree.
GitHub is only the transport; its review state and branch protection are not consulted.

This version is a dry-run decision tool: it prints the decision, the check plan and the evidence, and
never merges or pushes. Applying a merge is a separate step, run by whichever identity (human or
agent) holds push access — see governance/AUTOMATED_MERGE_POLICY.md's Scope section: within
aaroneous-devtools nothing here is owner-gated by path; aaroneous (the product) stays entirely
owner-gated, but that boundary is enforced by scripts/auto_merge.py's repository guard, not by this
module.

Conditions (mirroring the routine's step 3, with local equivalents):
  a. covered: a signed, independent, whole-diff accept chained to the current head and merge-base
  b. mergeable: a trial merge into the base has no conflicts
  c. checks: the path-based plan passes in a clean worktree (only when --run-checks is given)
  d. base is `main`
  e. no standing changes_requested at the head (part of coverage)

The default output path runs gather_facts() + checklist() for every PR and reconciles both
verdicts through reconcile(): decide()'s reasons (a/b/d) are necessary, the checklist (CI state,
unresolved threads, circuit breaker) is sufficient, and neither alone can
print an eligibility verdict. --run-checks remains the extra local-gate layer (condition c) on top.

Trust-building (a per-category clean-merge counter) was dropped by owner decision 2026-10-07;
see governance/RECURSIVE_AUDIT_CADENCE.md for the periodic-audit mechanism that replaced it.

Owner-only path gating (the former Tier A / "protected paths" concept, including this file's own
former `TIER_A_PATHS`/`PROTECTED`/`touches_protected`/`tier_of` and checklist box 0, and decide()'s
former condition f) was dropped by owner decision 2026-10-09: "I DO NOT want this system dependent
on me, and I do not want to manually review ANYTHING going on in devtools" — the mechanical
conditions below (especially >=2 independent accept seats) are now the only gate for any path
within aaroneous-devtools, workflow files, identities.json and this policy document included. See
governance/AUTOMATED_MERGE_POLICY.md and governance/DECISIONS.md (2026-10-09 entry).

Usage:
  merge_gate.py --repo OWNER/NAME --repo-dir CLONE --identities governance/identities.json [--pr N]
                [--run-checks] [--json]
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tomllib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import review_coverage as rc  # noqa: E402

DOCS_ALLOWLIST = ("governance/", "docs/", "architecture/")
IDENTITIES_PATH = "governance/identities.json"
# `git diff --check` with no revision inspects only uncommitted changes, and run_checks runs it in a clean detached worktree,
# so it always exited 0 whatever whitespace errors the PR committed (Codex P2 on #98). The range form checks the PR itself.
FULL_GATE = [
    ["git", "diff", "--check", "{base}...HEAD"],
    ["python", "scripts/check_coordination_evidence.py", "{base}", "HEAD"],
    ["cargo", "fmt", "--all", "--", "--check"],
    ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    ["cargo", "test", "-p", "foundation_cli"],
]
LIGHT_GATE = [
    ["git", "diff", "--check", "{base}...HEAD"],
    ["python", "scripts/check_coordination_evidence.py", "{base}", "HEAD"],
]


def gate_plan(paths):
    """Path-based plan: only an allowlist of docs/ledger paths runs the light gate; all else is full."""
    docs_only = bool(paths) and all(p.endswith((".md", ".json")) and p.startswith(DOCS_ALLOWLIST) for p in paths)
    return {"level": "light" if docs_only else "full", "commands": LIGHT_GATE if docs_only else FULL_GATE}


def box(ok, label, detail=""):
    """Render one checklist line: [x], [ ] or [blocked: reason] (ok=None)."""
    if ok is True:
        mark = "[x]"
    elif ok is False:
        mark = "[ ]"
    else:
        mark = f"[blocked: {detail}]"
        detail = ""
    return f"{mark} {label}" + (f" ({detail})" if detail else "")


def checklist(facts):
    """The policy's literal checklist (governance/AUTOMATED_MERGE_POLICY.md, #53). Report-only: returns
    (lines, would_merge, first_blocker). A box the checker cannot evaluate is blocked and fails the gate
    like an unchecked box; nothing is rounded up to green."""
    head = facts["head"][:12]
    ci = facts.get("ci") or {"state": "missing", "detail": "no CI status found"}
    # Every fact is read strictly: a missing key or a value of the wrong type blocks its box, it never reads as "fine".
    # Probe (scout/skeptic tick): with the old defaults a facts dict without
    # `standing_changes_requested` had no standing rejection, without `verdicts_missing_evidence` had complete
    # evidence, and `threads_open=False`, `breaker_tripped=0`, `accept_seats=["a", "a"]` or `"ab"` all passed.
    raw_seats = facts.get("accept_seats")
    seats_known = isinstance(raw_seats, list) and all(isinstance(s, str) for s in raw_seats)
    seats = raw_seats if seats_known else []
    distinct_seats = {s.strip() for s in seats if s.strip()}
    covered_fact = facts.get("covered")
    missing_evidence = facts.get("verdicts_missing_evidence")
    counted = facts.get("counted_verdicts")
    threads_open = facts.get("threads_open")
    standing = facts.get("standing_changes_requested")
    tripped = facts.get("breaker_tripped")
    # Owner decision 2026-10-11 (DECISIONS.md): a seat registering its own already-provisioned GitHub
    # App login or SSH attestation key is paperwork ("a job application"), not a judgement about trust,
    # so it is exempt from condition 3's >=2-accept bar (and, since no verdict is required, box 5's
    # per-verdict evidence check too). The exemption itself stays strict: only True, never inferred from
    # an absent or wrongly-typed fact, grants it — anything else falls through to the ordinary bar.
    identity_exempt = facts.get("identity_registration_exempt") is True
    exempt_suffix = " — exempt: additive-only identities.json registration (owner decision 2026-10-11)"

    def is_count(value):
        return isinstance(value, int) and not isinstance(value, bool)
    checks = [
        ("1", None if ci["state"] == "missing" else ci["state"] == "success",
         f"CI green on head {head}", ci.get("detail", ""), ci.get("detail") or "no CI status found"),
        ("2", facts["trial_merge_ok"], f"No merge conflict vs. {facts['base_ref']} tip {facts.get('base_tip', '?')[:12]}", "", ""),
        ("2a", facts["base_ref"] == facts["default_branch"], "Base branch is the repository default branch",
         f"base={facts['base_ref']}", ""),
        ("2b", None if facts.get("is_draft") is None else not facts["is_draft"], "PR is not a draft", "",
         "draft status unavailable (fail closed)"),
        ("3", True if identity_exempt else
         (None if not seats_known or not isinstance(covered_fact, bool) else len(distinct_seats) >= 2 and covered_fact),
         f">=2 independent accept verdicts at {head}, seats: {', '.join(seats) or 'none'}"
         + (exempt_suffix if identity_exempt else ""), "",
         "accepting seats or coverage unavailable"),
        ("4", None if not is_count(threads_open) or not isinstance(standing, bool) else threads_open == 0 and not standing,
         "Zero open blocking threads", f"count checked: {threads_open}",
         "thread count unavailable" if not is_count(threads_open) else "standing-rejection status unavailable"),
        ("5", True if identity_exempt else
         (None if not is_count(counted) or not isinstance(missing_evidence, list) else counted > 0 and not missing_evidence),
         "Each counted verdict cites command + exit code"
         + (exempt_suffix if identity_exempt else ""),
         f"verdicts missing evidence: {missing_evidence}" if missing_evidence else f"counted: {counted}",
         "verdict evidence facts unavailable"),
        ("6", facts.get("revertible"), "Revertible via plain `git revert` (no migration/external side effect)", "",
         "needs a human classification"),
        ("7", None if not isinstance(tripped, bool) else not tripped,
         "circuit_breaker_override.toml: tripped = false", "", "circuit breaker file not found or unparseable (fail closed)"),
    ]
    lines = []
    first = None
    for number, ok, label, detail, blocked_reason in checks:
        lines.append(box(ok, f"{number}. {label}", blocked_reason if ok is None else detail))
        if ok is not True and first is None:
            first = number
    return lines, first is None, first


def reconcile(decide_ok, facts):
    """One verdict: decide()'s reasons are necessary, the checklist is sufficient; both must pass.

    Returns (lines, eligible, first_blocker). eligible is True only when decide() said eligible
    AND every checklist box (CI, conflict, base, draft status, accepts, threads, evidence,
    revertible, breaker) is green — so the default path can never print ELIGIBLE with red CI, a
    draft PR, open threads, or a tripped breaker, which checklist() would block."""
    lines, would_merge, first = checklist(facts)
    return lines, bool(decide_ok) and would_merge, first


def decide(pr_row, paths, base_ref, trial_merge_ok):
    reasons = []
    if not pr_row["covered"]:
        reasons.append("a: not covered by a signed independent whole-diff accept at the current head")
    if not trial_merge_ok:
        reasons.append("b: trial merge into the base conflicts")
    if base_ref != "main":
        reasons.append(f"d: base is {base_ref}, not main")
    return {"eligible": not reasons, "reasons": reasons, "plan": gate_plan(paths)}


def changed_paths(repo_dir, base_ref, head):
    """Every path the PR touches, both sides of a rename."""
    # -z: without it git quotes a path holding a non-ASCII character or a quote, which broke downstream
    # consumers that matched on raw path strings (Codex P1 on #91, skeptic probe).
    out = rc.git(repo_dir, "diff", "--name-only", "-z", "--no-renames", f"origin/{base_ref}...{head}").stdout
    return [path for path in out.split("\0") if path]


def additive_identity_registration(repo_dir, base_ref, head, paths):
    """True only for a PR whose sole changed path is governance/identities.json and whose only edits are
    new entries appended to an *existing* seat's logins/keys lists: the set of seat names is unchanged
    (admitting a new seat is a trust decision, not paperwork), the owner list is unchanged, and no login
    or key present at the base is missing at the head (removing one is a credential change, not additive
    registration, and still needs the ordinary >=2-accept review). Malformed or unreadable JSON on
    either side is not exempt — the ordinary bar is the fail-safe default, not this one."""
    if paths != [IDENTITIES_PATH]:
        return False
    try:
        base_doc = json.loads(rc.git(repo_dir, "show", f"origin/{base_ref}:{IDENTITIES_PATH}").stdout)
        head_doc = json.loads(rc.git(repo_dir, "show", f"{head}:{IDENTITIES_PATH}").stdout)
    except ValueError:
        return False
    if not isinstance(base_doc, dict) or not isinstance(head_doc, dict):
        return False
    if base_doc.get("owner") != head_doc.get("owner"):
        return False
    base_seats, head_seats = base_doc.get("seats"), head_doc.get("seats")
    if not isinstance(base_seats, dict) or not isinstance(head_seats, dict):
        return False
    if set(base_seats) != set(head_seats):
        return False
    for name, base_seat in base_seats.items():
        head_seat = head_seats[name]
        if not isinstance(base_seat, dict) or not isinstance(head_seat, dict):
            return False
        for field in ("logins", "keys"):
            base_values, head_values = base_seat.get(field), head_seat.get(field)
            if not isinstance(base_values, list) or not isinstance(head_values, list):
                return False
            if not set(base_values) <= set(head_values):
                return False
    return True


def breaker_files(repo_dir, default_branch):
    """Paths of every circuit_breaker_override.toml on the default branch. NUL-separated: splitting the quoted,
    newline-separated listing on whitespace lost a breaker file under a directory with a space in its name, so a tripped
    breaker there was never read."""
    out = rc.git(repo_dir, "ls-tree", "-r", "--name-only", "-z", "origin/" + default_branch).stdout
    return [path for path in out.split("\0") if path.endswith("circuit_breaker_override.toml")]


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


def latest_checks(rollup):
    """The newest run of each check. GitHub keeps every run on a commit in the rollup, so a failure that was later re-run
    successfully (a re-run, a close and reopen, a flaky job) stays listed beside its replacement; only the newest run of a
    given workflow and name counts. Runs are ordered by `startedAt` (else `completedAt`); on a tie the worse result is
    kept (a failed or unfinished run outranks a successful one). An entry with no name, or with no timestamp at all (a
    queued re-run has not started), cannot be placed in that order, so it can never be discarded as older and always counts
    (fail closed)."""
    newest, unplaced = {}, []
    for item in rollup:
        name = item.get("name") or item.get("context") or ""
        stamp = item.get("startedAt") or item.get("completedAt") or ""
        if not name or not stamp:
            unplaced.append(item)
            continue
        key = (item.get("workflowName") or "", name)
        good = (item.get("conclusion") or "").upper() == "SUCCESS" and (item.get("status") or "").upper() == "COMPLETED"
        rank = (stamp, 0 if good else 1)
        if key not in newest or rank > newest[key][0]:
            newest[key] = (rank, item)
    return [entry[1] for entry in newest.values()] + unplaced


def ci_state(rollup):
    """Fail-closed CI state from a statusCheckRollup list; pending or absent is never success."""
    if not rollup:
        return {"state": "missing", "detail": "no CI status found"}
    states = []
    for item in latest_checks(rollup):
        conclusion = (item.get("conclusion") or "").upper()
        status = (item.get("status") or "").upper()
        states.append(conclusion if status == "COMPLETED" or conclusion else "PENDING")
    if any(s in ("", "PENDING", "QUEUED", "IN_PROGRESS") for s in states):
        return {"state": "pending", "detail": "checks still running"}
    ok = all(s == "SUCCESS" for s in states)
    return {"state": "success" if ok else "failure", "detail": f"{len(states)} checks: {', '.join(sorted(set(states)))}"}


REFRESH_REFSPECS = ("+refs/heads/*:refs/remotes/origin/*", "+refs/pull/*/head:refs/remotes/origin-pr/*")


def refresh_refs(repo_dir, run=None):
    """Bring the clone's view of the remote up to date: every branch and every pull request head, pruning what is gone.
    GitHub supplies the current head SHA, but the merge base, changed paths, trial merge, base tip and circuit-breaker
    contents are all read from the clone's own refs; a stale clone gave facts for a different base, or no head at all
    (Codex P2 on #99). Pull heads live under refs/remotes/origin-pr/* so they never collide with a remote branch named 'pr'.
    Returns (ok, message)."""
    run = run or rc.git
    result = run(repo_dir, "fetch", "--prune", "origin", *REFRESH_REFSPECS)
    return result.returncode == 0, (result.stderr or "").strip()


def missing_heads(repo_dir, prs, run=None):
    """PR numbers whose head commit is not in the clone, which no fact about them can be trusted."""
    run = run or rc.git
    return [pr["number"] for pr in prs if run(repo_dir, "cat-file", "-e", pr["head"] + "^{commit}").returncode != 0]


def evaluate_all(repo, repo_dir, identities_path, only_pr, do_checks, fetch=False):
    if fetch:
        ok, message = refresh_refs(repo_dir)
        if not ok:
            raise SystemExit(f"--fetch failed, so no gate fact can be trusted: {message}")
    snapshot = rc.live_snapshot(repo, repo_dir)
    absent = set(missing_heads(repo_dir, snapshot["prs"]))
    keys = rc.load_keys(identities_path)
    rows = {r["pr"]: r for r in rc.report_snapshot(
        snapshot, rc.live_ancestry(repo_dir), rc.load_identities(identities_path), keys, True)}
    default = rc.gh_json("repo", "view", repo, "--json", "defaultBranchRef")["defaultBranchRef"]["name"]
    decisions = []
    for pr in snapshot["prs"]:
        if only_pr and pr["number"] != only_pr:
            continue
        paths = changed_paths(repo_dir, pr["base"], pr["head"])
        trial = trial_merge(repo_dir, pr["base"], pr["head"])
        decision = decide(rows[pr["number"]], paths, pr["base"], trial)
        # One verdict: decide() reasons AND the full checklist (gather_facts is
        # read-only; anything it cannot read fails closed inside checklist()).
        facts = gather_facts(repo, repo_dir, pr, rows[pr["number"]], paths, trial, default)
        lines, eligible, first = reconcile(decision["eligible"], facts)
        decision["eligible"] = eligible
        decision["checklist"] = lines
        if first is not None:
            decision["reasons"].append(f"blocked on checklist box {first}")
        decision.update({"pr": pr["number"], "head": pr["head"], "paths": paths})
        if pr["number"] in absent:
            decision["eligible"] = False
            decision["reasons"].append("refs: the PR head is not in this clone; run with --fetch")
        if do_checks and decision["eligible"]:
            decision["checks"] = run_checks(repo_dir, pr["head"], pr["base"], decision["plan"])
            if any(c["rc"] != 0 for c in decision["checks"]):
                decision["eligible"] = False
                decision["reasons"].append("c: a gate command failed in the clean worktree")
        decisions.append(decision)
    return decisions


def print_checklists(args):
    if args.fetch:
        ok, message = refresh_refs(args.repo_dir)
        if not ok:
            raise SystemExit(f"--fetch failed, so no gate fact can be trusted: {message}")
    snapshot = rc.live_snapshot(args.repo, args.repo_dir)
    keys = rc.load_keys(args.identities)
    rows = {r["pr"]: r for r in rc.report_snapshot(
        snapshot, rc.live_ancestry(args.repo_dir), rc.load_identities(args.identities), keys, True)}
    default = rc.gh_json("repo", "view", args.repo, "--json", "defaultBranchRef")["defaultBranchRef"]["name"]
    for pr in snapshot["prs"]:
        if args.pr and pr["number"] != args.pr:
            continue
        paths = changed_paths(args.repo_dir, pr["base"], pr["head"])
        facts = gather_facts(args.repo, args.repo_dir, pr, rows[pr["number"]], paths,
                             trial_merge(args.repo_dir, pr["base"], pr["head"]), default)
        lines, would, first = checklist(facts)
        print(f"PR #{pr['number']} head {pr['head'][:12]}")
        for line in lines:
            print("  " + line)
        print("  => " + ("would merge (report-only; nothing merged)" if would else f"would merge, blocked on box {first}") + "\n")
    return 0


def category_of(paths):
    """A coarse file-pattern category for reporting, e.g. governance/*.md."""
    if not paths:
        return "?"
    dirs = {os.path.dirname(p) or "." for p in paths}
    exts = {os.path.splitext(p)[1] or "*" for p in paths}
    if len(dirs) == 1 and len(exts) == 1:
        return f"{dirs.pop()}/*{exts.pop()}"
    return "mixed"


def breaker_state(text):
    """True/False from a real TOML boolean `tripped`; None (blocked, fail closed) when the file does not
    parse or the value is missing or not a boolean. A comment such as `# tripped = false` never counts."""
    try:
        value = tomllib.loads(text).get("tripped")
    except (tomllib.TOMLDecodeError, TypeError, ValueError):
        return None
    return value if isinstance(value, bool) else None


def open_threads(data):
    """Unresolved review-thread count from a GraphQL reviewThreads response, or None when the count cannot
    be trusted: more pages exist (hasNextPage) or the response lacks pagination info. Never counts a
    truncated first page as complete."""
    try:
        threads = data["data"]["repository"]["pullRequest"]["reviewThreads"]
        if threads["pageInfo"]["hasNextPage"] is not False:
            return None
        return sum(1 for n in threads["nodes"] if not n["isResolved"])
    except (KeyError, TypeError):
        return None


def registry_version_checksum_update(before_text, after_text):
    """Prove the narrow lockfile shape; this says nothing about dependency safety or CI."""
    if max(len(before_text), len(after_text)) > 2 * 1024 * 1024:
        return False
    try:
        before, after = tomllib.loads(before_text), tomllib.loads(after_text)
    except (tomllib.TOMLDecodeError, TypeError):
        return False
    before_packages = before.pop("package", None)
    after_packages = after.pop("package", None)
    if (before != after or type(before.get("version")) is not int or before["version"] not in (3, 4)
            or not isinstance(before_packages, list) or not isinstance(after_packages, list)
            or not before_packages or len(before_packages) != len(after_packages)):
        return False
    changed = False
    for old, new in zip(before_packages, after_packages):
        if not isinstance(old, dict) or not isinstance(new, dict):
            return False
        if old == new:
            continue
        # Preserve package identity, provenance, dependencies and every other field.
        if ({k: v for k, v in old.items() if k not in ("version", "checksum")}
                != {k: v for k, v in new.items() if k not in ("version", "checksum")}):
            return False
        if old.get("source") != "registry+https://github.com/rust-lang/crates.io-index":
            return False
        if not isinstance(old.get("name"), str) or not old["name"]:
            return False
        for record in (old, new):
            if (not isinstance(record.get("version"), str)
                    or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?", record["version"])
                    or not isinstance(record.get("checksum"), str)
                    or not re.fullmatch(r"[0-9a-f]{64}", record["checksum"])):
                return False
        if old["version"] == new["version"] or old["checksum"] == new["checksum"]:
            return False
        changed = True
    return changed


def repository_revertibility(repo_dir, base, head, paths):
    """Classify tracked content only. Unknown shapes remain blocked by checklist box 6."""
    def documentation(path):
        return path.endswith((".md", ".json")) and path.startswith(DOCS_ALLOWLIST)

    if paths and all(documentation(path) for path in paths):
        return True  # Preserve the existing documentation classification.
    lock = "foundation/Cargo.lock"
    if not paths or lock not in paths or any(p != lock and not documentation(p) for p in paths):
        return None
    # Resolve immutable commits and the actual diff base, not the caller's worktree files.
    try:
        common = rc.git(repo_dir, "--no-replace-objects", "merge-base", base, head)
        if common.returncode or not re.fullmatch(r"[0-9a-f]{40}", common.stdout.strip()):
            return None
        revisions = (common.stdout.strip(), head)
        texts = []
        for revision in revisions:
            if not re.fullmatch(r"[0-9a-f]{40}", revision):
                return None
            entry = rc.git(repo_dir, "--no-replace-objects", "ls-tree", revision, "--", lock)
            if entry.returncode or not re.fullmatch(r"100644 blob [0-9a-f]{40}\tfoundation/Cargo.lock\n?", entry.stdout):
                return None
            contents = rc.git(repo_dir, "--no-replace-objects", "show", f"{revision}:{lock}")
            if contents.returncode:
                return None
            texts.append(contents.stdout)
        return True if registry_version_checksum_update(*texts) else None
    except (OSError, ValueError, TypeError):
        return None


def gather_facts(repo, repo_dir, pr, row, paths, trial_ok, default_branch):
    """Live facts for one PR, read-only. Anything that cannot be read stays None (blocked, fail closed)."""
    facts = {
        "head": pr["head"], "base_ref": pr["base"], "default_branch": default_branch,
        "trial_merge_ok": trial_ok, "covered": row["covered"], "accept_seats": row["accept_seats"],
        "counted_verdicts": row["counted_verdicts"], "verdicts_missing_evidence": row["verdicts_missing_evidence"],
        "standing_changes_requested": any("changes_requested" in r or "needs_revision" in r for r in row["reasons"]),
        "category": category_of(paths), "is_draft": pr.get("draft"),
        "breaker_tripped": None, "threads_open": None, "ci": None,
    }
    tip = rc.git(repo_dir, "rev-parse", f"origin/{pr['base']}").stdout.strip()
    facts["base_tip"] = tip or "?"
    facts["revertible"] = repository_revertibility(repo_dir, tip, pr["head"], paths)
    try:
        facts["identity_registration_exempt"] = additive_identity_registration(repo_dir, pr["base"], pr["head"], paths)
    except Exception:
        facts["identity_registration_exempt"] = False
    try:
        view = rc.gh_json("pr", "view", str(pr["number"]), "-R", repo, "--json", "statusCheckRollup")
        facts["ci"] = ci_state(view.get("statusCheckRollup", []))
        owner, name = repo.split("/", 1)
        query = ("query($o:String!,$n:String!,$num:Int!){repository(owner:$o,name:$n){pullRequest(number:$num)"
                 "{reviewThreads(first:100){pageInfo{hasNextPage} nodes{isResolved}}}}}")
        data = rc.gh_json("api", "graphql", "-f", f"query={query}", "-F", f"o={owner}", "-F", f"n={name}",
                          "-F", f"num={pr['number']}")
        facts["threads_open"] = open_threads(data)
    except Exception:
        pass
    breaker = breaker_files(repo_dir, default_branch)
    if breaker:
        text = rc.git(repo_dir, "show", f"origin/{default_branch}:{breaker[0]}").stdout
        facts["breaker_tripped"] = breaker_state(text)
    return facts


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", required=True)
    parser.add_argument("--repo-dir", required=True)
    parser.add_argument("--identities", required=True)
    parser.add_argument("--pr", type=int)
    parser.add_argument("--run-checks", action="store_true")
    parser.add_argument("--fetch", action="store_true",
                        help="fetch every branch and PR head from origin first; fail closed if the fetch fails")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--checklist", action="store_true",
                        help="print the policy's literal checklist per PR (report-only 'would merge')")
    args = parser.parse_args(argv)
    remote = rc.git(args.repo_dir, "remote", "get-url", "origin")
    if remote.returncode != 0 or not rc.checkout_matches(remote.stdout, args.repo):
        raise SystemExit(f"--repo-dir {args.repo_dir!r} is not a clone of {args.repo}")
    if args.checklist:
        return print_checklists(args)
    decisions = evaluate_all(args.repo, args.repo_dir, args.identities, args.pr, args.run_checks, args.fetch)
    if args.json:
        print(json.dumps(decisions, indent=2))
    else:
        for d in decisions:
            verdict = "ELIGIBLE" if d["eligible"] else "BLOCKED"
            print(f"#{d['pr']} {d['head'][:8]} {verdict} plan={d['plan']['level']} " + ("; ".join(d["reasons"]) or "-"))
            for line in d.get("checklist", []):
                print("    " + line)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
