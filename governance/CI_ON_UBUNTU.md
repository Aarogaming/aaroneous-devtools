# CI and merging on the Ubuntu machine (lane C128)

Why: the repository is private, so every GitHub-hosted Actions minute is billed, and the two heaviest workflows run on events we produce constantly (`auto-merge.yml` on every PR event, comment and review plus a 20-minute cron; Foundation CI on Ubuntu and Windows for every push to every PR). Throughput is going to grow, and on 2026-10-11 the account's Actions jobs stopped starting at all ("recent account payments have failed or your spending limit needs to be increased"). This moves the work to the Ubuntu machine, in two steps, without weakening the gate.

## The bridge: unblock merging today, before any runner exists

Actions jobs stop starting when a private repository's hosted minutes run out (the account's free allowance resets monthly) or a payment fails. Nothing can merge until a green check exists on the exact head, so this blocks everything, and I cannot change billing or push workflow files. Two owner actions unblock it, in this order:

1. **Raise the Actions spending limit** (GitHub, Settings, Billing and plans, Spending limits; or fix the failed payment). A small cap such as $10 to $20 is enough for the rest of the month if step 2 is applied, and hosted jobs start again immediately.
2. **Apply the bridge workflows** so the minutes last: `ci/bridge/*.yml` are the current workflows with the waste removed.
   - Foundation CI on Linux only for pull requests, superseded pushes cancelled, and a fast path: a pull request that touches nothing under `foundation/`, `stacks/`, `ci/` or the Foundation workflow exits in seconds with a green check (so the gate still sees one) instead of rebuilding the workspace. Pushes to main always build.
   - Windows moves to its own workflow on main pushes only. A skipped or missing check fails the gate, so it must not appear on pull requests.
   - Auto-merge loses its `issue_comment` trigger (every PR comment started a run) and its 20-minute cron becomes hourly; it keeps `pull_request_target` and `check_suite`, so a merge is evaluated when CI finishes. Attestation comments are picked up by the next hourly run.
   - Dashboard CI on Linux only.

   From a checkout, with the owner's login (the seat Apps cannot push workflow files):

   ```bash
   git fetch origin proposal/c128-ubuntu-ci
   git checkout -b ci-bridge origin/main
   git checkout origin/proposal/c128-ubuntu-ci -- ci/bridge
   cp ci/bridge/*.yml .github/workflows/
   git add .github/workflows && git commit -m "ci: bridge workflows, fewer hosted minutes" && git push origin ci-bridge
   ```

   Then merge that branch. Open PRs keep their old workflow definitions until they are rebased, except `auto-merge`, which always runs from main.

The bridge is a stopgap with the same gate rules. The real fix is the self-hosted runners below, which cost no hosted minutes at all.

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
4. **Every job leaves nothing behind, or the runner takes itself out of service.** The `job-completed.sh` hook wipes all of `_work` (including the downloaded actions and the tool cache, which would otherwise be reused by the next pull request), checks that it is empty and that no process of the account survived the job, and refuses to delete outside a devtools runner's `_work`. If any of that fails it writes `_diag/QUARANTINE`, stops the listener and exits non-zero; the service will not start while the marker exists. Cleanup failing is never reported as success. `ci/ubuntu/test-job-completed.sh` exercises all of this without a runner (14 cases; five deliberate regressions of the hook each fail it).
5. **A job cannot change the runner or its toolchain.** The runner's program, configuration and credentials are root-owned and read-only to the job account (self-update is off); the Rust toolchain is installed by root under `/opt/devtools-toolchain`; the unit's only writable paths are `_work`, `_diag` and a cargo cache; stopping the service ends every process the jobs started (`KillMode=control-group`). `setup-runner.sh` checks the writes it forbids and `ci-smoke.yml` checks them again from inside a job.
6. **Seat credentials stay out of reach.** If `ci-smoke.yml` ever reports a readable path, stop the service and fix the permissions before anything else runs.
7. **Only this repository.** `setup-runner.sh` refuses any repository but `Aarogaming/aaroneous-devtools`. The product repo keeps its own gate.

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

2. Push **only** `ci/proposed-workflows/ci-smoke.yml` to `.github/workflows/ci-smoke.yml` and run it **twice in a row** (Actions, "CI smoke", Run workflow). Expect green jobs both times. The first run leaves sentinel files in the runner's tool and temp directories; the first step of the second run fails if the cleanup hook did not remove them, which is the only way to prove the wipe from inside a job. This also settles whether the failed-payment lock reaches self-hosted runners: self-hosted jobs are not billed, so they should start, but the smoke run is the proof.
3. Copy the rest of `ci/proposed-workflows/*.yml` over `.github/workflows/` and push. Delete `ci-smoke.yml` or keep it; it only runs by hand.
4. Optional, for the Windows leg: register a runner on the Windows machine with the label `devtools-windows` (the stock runner install) so `foundation-ci-windows.yml` has somewhere to run. Without it that workflow just queues on main and nightly; it is not part of the pull-request gate.

## What changes for the gate

Nothing it reads. The checks still appear as check runs on the head commit and `ci_state` still requires every one to be a success. Two deliberate differences:

- **The Windows check is not required before merge in the proposed workflows, and that is a weaker gate than the one it replaces.** A skipped or missing check would fail the gate, so the Windows leg was moved to main and nightly. Codex's review is right that this contradicts "without weakening the gate" and the native-host objective. `ci/platform-policy.toml` already says which changes need Windows (everything under `foundation/`, `stacks/` and `ci/`); the intended end state is a required Windows result for the exact candidate, pending (not absent) while the Windows host is offline. Until a Windows runner exists, the hosted `windows-latest` job on pull requests (behind the same fast path, so documents and queue rows skip the build) is the way to keep that requirement; switching the bridge back to it is a deliberate choice for the owner, not something this document decides.
- `auto-merge.yml` reacts to a formal GitHub review within 10 minutes instead of at once.

## Capacity and cost

Self-hosted jobs cost no hosted minutes. The cost moves to the machine: a full Foundation CI build is CPU- and disk-heavy, and the seats also work on that machine, so the units cap the runner at 70% of memory and 4,096 tasks, and `cancel-in-progress` stops superseded pushes. If CI starts to crowd out the seats, give the machine a second runner instance of kind `ci` (a different `--name`) on separate hardware, or move to Step 2.

## Rollback

`sudo systemctl disable --now devtools-runner-ci devtools-runner-merge`, then restore the previous `.github/workflows/*.yml` from git history (they use hosted runners). Nothing else on the machine is touched. `sudo userdel -r devtools-ci devtools-merge` removes the accounts.

## Step 2: take the gate off Actions entirely

A `foundation` service (the Rust `guild` daemon already exists) polls the open PRs, runs the same checks in a sandbox, posts the result as a check run, and runs the merge decision from a trusted checkout of main. That needs a CI GitHub App with `checks: write` (no seat App has it) and is the C127 phase 2-3 work: measure the Rust path against the Python gate on real PR states first, then shadow, then switch. Until then Step 1 keeps every rule in force and costs no minutes.

## Known limits

- **This is not an isolation boundary for arbitrary pull-request code.** A job runs as the same account as the runner's listener, so it can read the runner's own credential (the files are read-only to it, not unreadable), and systemd sandboxing is not a container or a virtual machine. Separating the `ci` and `merge` accounts keeps PR code away from the merge-capable token, which matters, but it is not cross-job isolation. The hook, the read-only runner and toolchain, and the control-group teardown narrow what a job can leave behind and detect when cleanup fails; they do not make a hostile job harmless. Until a disposable executor exists (a container or VM per job with the runner credential outside its reach, or the verification nodes of C129), use this runner only for a private repository whose pull requests come from the owner's own seats, and keep hosted isolation for anything else. Ephemeral registration (a new runner credential per job) is the standard way to remove the credential risk and needs a token source that the hall, not a job, controls.
- A persistent runner still reuses the cargo registry cache between jobs (`cargo-home`); cargo checks every crate against the checksum in `Cargo.lock`, so a poisoned cache fails the build rather than changing it, but a build that is not locked would not be protected.
- The unit's quarantine check (`ExecStartPre`) and the hook's marker live in `_diag`, which the job account can write; a job that deletes the marker after a failed cleanup could restart a runner that should stay down. That is the same trust boundary as above.
- Not run: `setup-runner.sh` and the units were syntax-checked (`bash -n`) and the workflows parse as YAML, but nothing here has been executed on the Ubuntu machine, and `shellcheck` was not available.
