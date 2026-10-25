# Git Workflow

## Branches

`main` is the integrated, releasable foundation branch. Use short-lived `feat/`, `fix/`, `docs/`,
or `chore/` branches for material changes. Keep migration history separate from active tooling.

## Pull requests

A pull request states its purpose, contract impact, verification, and any external support
requirements. Prefer one reviewer for code that executes commands or handles paths.

**Branch protection on `main` is enabled as of 2026-09-27** (both this repo and Product
`aaroneous`), following the 2026-09-26 documentation/process audit's Finding 5 (the same
unevidenced "Done"/"gate PASS" self-certification pattern recurring across sessions). Settings:
`enforce_admins: true` (applies to every credential, not just non-admin ones — every agent on
this machine authenticates as the same GitHub account), a pull request is required to reach
`main` (`required_approving_review_count: 0`, so a PR can be opened and merged without waiting
for a second reviewer — this isn't a human-approval gate, it's a "you cannot silently push
straight to main" gate), and force-pushes/branch deletion are disabled. Product `aaroneous`
additionally requires its `Check & Test` CI job (both OS legs) to pass before a PR can merge.
This repo additionally runs `.github/workflows/coordination-evidence.yml` on any PR touching
`governance/COORDINATION_QUEUE.md`, which fails the check if a newly added row or entry claims
Done/PASS/"gate green" without a commit hash, exit code, log path, or command block nearby (see
`scripts/check_coordination_evidence.py` and the "Standing rule: evidence retention" section at
the top of that file).

If this is blocking urgent work and needs to be relaxed or removed, that's a reserved-decision
request (OWNER_DELEGATION_LADDER.md R4; automation-debt: branch-protection admin) — an account
administrator applies it in GitHub's branch protection settings — don't route around it by
force-pushing
or disabling it from inside an agent session.

## Hobby cadence

Keep maintenance sustainable: make small changes, merge completed work often, and use the daily
automation only for evidence changes. A weekly review should reconcile queue ownership and
Dependabot updates; a monthly review should retire stale experiments and reconsider scope.
