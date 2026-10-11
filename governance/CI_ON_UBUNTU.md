# CI and merging on the Ubuntu machine (lane C128)

Why: the repository is private, so every GitHub-hosted Actions minute is billed, and the two heaviest workflows run on events we produce constantly (`auto-merge.yml` on every PR event, comment and review plus a 20-minute cron; Foundation CI on Ubuntu and Windows for every push to every PR). Throughput is going to grow, and on 2026-10-11 the account's Actions jobs stopped starting at all ("recent account payments have failed or your spending limit needs to be increased"). This moves the work to the Ubuntu machine, in two steps, without weakening the gate.

## The design in one page

| | Step 1 (this lane) | Step 2 (later lane) |
|---|---|---|
| Where jobs run | Two self-hosted runners on the Ubuntu machine | A Rust service on the same machine |
| How GitHub sees results | Ordinary check runs, as now | Check runs posted by a CI App |
| Gate change | None: `merge_gate.ci_state` already reads check runs | `foundation` gate subcommands (C127 phase 2-3); `ci_state` extended to read commit statuses, Rust first |
| Hosted minutes used | None | None |

Two runners, two operating-system users, on purpose:

| Runner | OS user | Label | Runs | Holds |
|---|---|---|---|---|
| ci | `devtools-ci` | `devtools-ci` | what a pull request brings: `cargo fmt/test/clippy`, `npm build`, the evidence check | a read-only job token |
| merge | `devtools-merge` | `devtools-merge` | only `auto_merge.py`, from main's own copy of the scripts | the token that can merge |

A process can read another process's environment when both run as one user, so the job that executes pull-request code must not share a user with the job that holds the merge-capable token. Neither user has a login shell, a password or sudo, and neither can read another account's home (the seats' `~/.steward` credentials live there). `setup-runner.sh` checks that and refuses to leave the service enabled if it fails.

## Invariants (do not weaken)

1. **The merge runner never executes pull-request code.** `auto_merge.py` calls the gate with `do_checks=False`; never pass `--run-checks` on that runner. The workflow checks out the default branch only.
2. **The merge workflow is main's.** It triggers on `pull_request_target`, `issue_comment`, `check_suite`, `schedule` and `workflow_dispatch`, all of which load the workflow definition from the default branch. A pull request cannot rewrite the job that merges it. (`pull_request_review` is gone; the 10-minute schedule catches a formal review.)
3. **The evidence check is judged by main's copy of the checker**, not the copy the pull request brings.
4. **Every job leaves nothing behind.** The `job-completed.sh` hook wipes the workspace; it refuses to delete outside a devtools runner's `_work`.
5. **Seat credentials stay out of reach.** If `ci-smoke.yml` ever reports a readable path, stop the service and fix the permissions before anything else runs.
6. **Only this repository.** `setup-runner.sh` refuses any repository but `Aarogaming/aaroneous-devtools`. The product repo keeps its own gate.

## Install (one-time admin steps; the seats cannot do these)

Registering a runner is an administrative action on the repository, and pushing workflow files needs the owner's login (the seat Apps deliberately have no `workflows` permission). Everything else is scripted.

1. On the Ubuntu machine, as a user with sudo, from a checkout of this repository:

   ```bash
   TOKEN_CI=$(gh api -X POST repos/Aarogaming/aaroneous-devtools/actions/runners/registration-token --jq .token)
   sudo ci/ubuntu/setup-runner.sh --repo Aarogaming/aaroneous-devtools --kind ci --token "$TOKEN_CI"
   TOKEN_MERGE=$(gh api -X POST repos/Aarogaming/aaroneous-devtools/actions/runners/registration-token --jq .token)
   sudo ci/ubuntu/setup-runner.sh --repo Aarogaming/aaroneous-devtools --kind merge --token "$TOKEN_MERGE"
   ```

   (`gh` must be logged in as someone with admin on the repository. Each token lives an hour; the script stores neither.)

2. Push **only** `ci/proposed-workflows/ci-smoke.yml` to `.github/workflows/ci-smoke.yml` and run it (Actions, "CI smoke", Run workflow). Expect two green jobs. This also settles whether the failed-payment lock reaches self-hosted runners: self-hosted jobs are not billed, so they should start, but the smoke run is the proof.
3. Copy the rest of `ci/proposed-workflows/*.yml` over `.github/workflows/` and push. Delete `ci-smoke.yml` or keep it; it only runs by hand.
4. Optional, for the Windows leg: register a runner on the Windows machine with the label `devtools-windows` (the stock runner install) so `foundation-ci-windows.yml` has somewhere to run. Without it that workflow just queues on main and nightly; it is not part of the pull-request gate.

## What changes for the gate

Nothing it reads. The checks still appear as check runs on the head commit and `ci_state` still requires every one to be a success. Two deliberate differences:

- The Windows check no longer appears on pull requests. A skipped or missing check would fail the gate, so the Windows leg lives in its own workflow that runs on main and nightly. A Windows-only regression is caught after merge, not before. If the Windows machine is always on, add `pull_request` to that workflow and it becomes a required check again.
- `auto-merge.yml` reacts to a formal GitHub review within 10 minutes instead of at once.

## Capacity and cost

Self-hosted jobs cost no hosted minutes. The cost moves to the machine: a full Foundation CI build is CPU- and disk-heavy, and the seats also work on that machine, so the units cap the runner at 70% of memory and 4,096 tasks, and `cancel-in-progress` stops superseded pushes. If CI starts to crowd out the seats, give the machine a second runner instance of kind `ci` (a different `--name`) on separate hardware, or move to Step 2.

## Rollback

`sudo systemctl disable --now devtools-runner-ci devtools-runner-merge`, then restore the previous `.github/workflows/*.yml` from git history (they use hosted runners). Nothing else on the machine is touched. `sudo userdel -r devtools-ci devtools-merge` removes the accounts.

## Step 2: take the gate off Actions entirely

A `foundation` service (the Rust `guild` daemon already exists) polls the open PRs, runs the same checks in a sandbox, posts the result as a check run, and runs the merge decision from a trusted checkout of main. That needs a CI GitHub App with `checks: write` (no seat App has it) and is the C127 phase 2-3 work: measure the Rust path against the Python gate on real PR states first, then shadow, then switch. Until then Step 1 keeps every rule in force and costs no minutes.

## Known limits

- Pull-request code runs as an unprivileged user under systemd sandboxing, not in a container or virtual machine. That is sufficient for a private repository whose contributors are the owner's own seats; it is not sufficient for outside contributors. Containerising the `ci` runner is the next hardening step if that ever changes.
- A persistent runner reuses its toolchain and cargo cache between jobs, which is the point (fast builds) and also means a poisoned cache could persist. The workspace is wiped after every job; the cargo registry cache is not.
- Not run: `setup-runner.sh` and the units were syntax-checked (`bash -n`) and the workflows parse as YAML, but nothing here has been executed on the Ubuntu machine, and `shellcheck` was not available.
