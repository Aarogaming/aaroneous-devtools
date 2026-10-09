# Operator guide

One page that answers "I am a seat (an agent session) working on Aaroneous through devtools: what do I
run, what do I read, and who decides?" It is an index. Every fact has one home in another document; this
page links to that home and adds only the route between them. If this page and a linked document disagree,
the linked document wins and this page is wrong (fix it, section 12).

Written 2026-10-09 (lane C110). The status labels in section 1 are dated; re-check them before relying on
one.

## 0. The shape of the system

Two repositories and one direction of dependency:

- **Aaroneous** (the product repo, `aaroneous`) is the Rust component framework. Its `AGENTS.md` is the
  binding constitution for any edit there.
- **Devtools** (this repo, `aaroneous-devtools`) is the companion: maintenance tools, coordination, review
  and the local-model pipeline. It depends on Aaroneous. **Aaroneous never depends on devtools** for build,
  CI, release, crates or runtime. See [FOUNDATION_BOUNDARY.md](../FOUNDATION_BOUNDARY.md) and
  [PLATFORM_SPEC.md](PLATFORM_SPEC.md) section 2 (a proposal, see section 1 below).
- Things flow from devtools into Aaroneous only as user-approved patches on an isolated branch that pass
  Aaroneous's own gates, or by graduating a proven component under the `AGENTS.md` graduation path (classify,
  put it behind a workspace trait with contract tests, land it inert or in shadow mode, then delete the
  devtools copy so one implementation exists).

Inside devtools the layers are, from least to most authority:

| Layer | What it does | Authority |
| --- | --- | --- |
| Tier 0 gates | Compiler, clippy, tests, `cargo xtask gate`, CI | The real authority; nothing lands without them |
| Local model (Qwen, "Mimo") | Drafts and classifies bounded jobs | Output is a draft; software validates; a different seat accepts |
| Cloud seats (Claude, Codex, Gemini/Antigravity) | Implement, review, take over a local model's pushed branch | Open PRs and post independent reviews; never self-accept |
| Steward | Reads GitHub and the queue, hands each seat one task card, logs what happened | Advises; changes nothing by itself |
| Merge gate | Decides offline whether a PR is eligible | Report-only today; the owner presses merge |
| Owner | Owner-only paths, policy, merging | Final |

The end state the owner is working toward is the local model operating autonomously, with cloud agents
pinged to take over, wrapped into one operable application built on Aaroneous to build and enhance
Aaroneous. Sections 1 and 7 say how far each piece is from that today. Nothing here claims it is done.

## 1. What is real today

Status as of 2026-10-09. "Operating" means a command exists and was run; it does not mean installed as a
service. Anything marked proposal changes no behaviour until the owner accepts it.

| Piece | State | Where it is described |
| --- | --- | --- |
| Queue of claims and the evidence CI check | Operating | [COORDINATION_QUEUE.md](COORDINATION_QUEUE.md), [CROSS_AGENT_COORDINATION_PROTOCOL.md](CROSS_AGENT_COORDINATION_PROTOCOL.md) |
| Review coverage and attestations | The tool operates, report-only; the spec it follows is a proposal | [PLATFORM_SPEC.md](PLATFORM_SPEC.md) section 3, `scripts/review_coverage.py` |
| Signed attestations (seat keys) | Mechanism operating; keys not all registered | [IDENTITY_SETUP.md](IDENTITY_SETUP.md) |
| Steward (task cards, receipts, event log) | Operating as scripts, run per seat by a heartbeat | [routines/STEWARD_HEARTBEAT.md](routines/STEWARD_HEARTBEAT.md), section 4 |
| Merge gate and eligibility checklist | Operating, report-only | [AUTOMATED_MERGE_POLICY.md](AUTOMATED_MERGE_POLICY.md), `scripts/merge_gate.py` |
| Automatic merging | Not built; the workflow that would press merge is an open draft PR | [AUTOMATED_MERGE_POLICY.md](AUTOMATED_MERGE_POLICY.md) section "Mechanism" |
| Scheduler review (`foundation schedule-review`) | Operating | [SCHEDULER_MAINTAINER.md](SCHEDULER_MAINTAINER.md) |
| Local worker, multithreader, registrar | Operating as commands | [LOCAL_WORKER_OPERATIONS.md](LOCAL_WORKER_OPERATIONS.md), section 7 |
| Packet utility (readiness and research packets) | Operating, local | [UTILITY.md](UTILITY.md), [../architecture/JOB_PIPELINE.md](../architecture/JOB_PIPELINE.md) |
| Daily cloud primary-lane routine | Installed; first run had no GitHub access | [SCHEDULE.md](SCHEDULE.md) section "Cloud primary-lane routine" |
| Rust steward crate (the proposed Guild component) | Draft PR, golden vectors are the contract | `scripts/golden/steward/README.md`, section 9 |
| Platform spec v0 | **Proposal**, awaiting owner approval | [PLATFORM_SPEC.md](PLATFORM_SPEC.md) |
| One operable, autonomous application | Not yet | [DEVTOOLS_STATUS.md](DEVTOOLS_STATUS.md) (dated 2026-09-17) |

## 2. Quick start for a new seat

1. Read, in this order: the Aaroneous `AGENTS.md`; this page; the "Standing rule: evidence retention" section
   and the "Active and ready lanes" table at the top of [COORDINATION_QUEUE.md](COORDINATION_QUEUE.md);
   [CROSS_AGENT_COORDINATION_PROTOCOL.md](CROSS_AGENT_COORDINATION_PROTOCOL.md). To brief a fresh session,
   paste [ROUND_TABLE_INTRO_PROMPT.md](ROUND_TABLE_INTRO_PROMPT.md) first.
2. Pick your seat name: `claude`, `codex`, `gemini` (also written Antigravity) or `mimo` (the local Qwen).
   Use it exactly in every comment, attestation and receipt.
3. Get a signing key and register it (section 3).
4. Fetch, then run the steward against your own persistent log (section 4) and read your card.
5. Work on a branch in your own worktree. Never touch another seat's dirty checkout or untracked files, and
   never push to `main` (branch protection requires a PR for every credential).
6. Claim before you build: a lane exists only as a visible row in the queue (section 5).
7. Before saying done, run the checks and quote the command, exit code and a log path or commit hash.
8. Review someone else's PR with a head-bound attestation (section 6). Never accept your own work.

## 3. Get a seat key and register it

Why: every seat posts under one shared GitHub login, so a pasted verdict block cannot be told from a real
one. A signature proves the seat holding the private key made the verdict. The full procedure, trust
levels and the rule for switching enforcement on are in [IDENTITY_SETUP.md](IDENTITY_SETUP.md); the short
version:

```bash
ssh-keygen -t ed25519 -f ~/.ssh/aaroneous_attest_SEAT -C "SEAT attestation key" -N ""
cat ~/.ssh/aaroneous_attest_SEAT.pub        # the one line to register; the private file never leaves your machine
```

- Register the **public** line in that seat's `keys` list in `governance/identities.json`, by a PR the owner
  reviews and merges (that file is an owner-only path). Never put a private key, passphrase or token in the
  repository.
- Sign a verdict: `python scripts/make_attestation.py --repo OWNER/NAME --repo-dir . --pr N --reviewer SEAT
  --verdict accept --key ~/.ssh/aaroneous_attest_SEAT` prints the attestation block and the signature block;
  paste both into your review comment. The tool refuses to build a verdict for a seat that is among the
  implementers.
- Check how your verdict counts: `python scripts/review_coverage.py --repo OWNER/NAME --repo-dir . --identities
  governance/identities.json --pr N`. The trust level shows as `signed`, `authenticated` or `attested-only`.

Today (2026-10-09) the merge gate always requires signatures, so a seat with no registered key has no
verdict the gate counts. The claude seat's key is in an open PR; the other three seats have none yet.

## 4. The steward: your task card each tick

The steward is a set of read-only scripts plus a heartbeat routine. It reads GitHub and the queue, writes
one card per seat, and logs what it saw in a hash-chained event log. It never edits a PR or the queue.

- **Run one tick:** `python scripts/steward_run.py --repo OWNER/NAME --repo-dir CLONE --identities
  governance/identities.json --log LOG --outbox OUTBOX`. Use a log and outbox that persist outside the repo
  (a throwaway log erases the history that leases, stalls and receipts depend on). Your card is
  `OUTBOX/SEAT.md`. `python scripts/steward_loop.py -- <the same arguments>` repeats ticks on an interval;
  it installs nothing.
- **Complete a card:** start your comment with the receipt `[done:KIND:PR:FULL_HEAD]` using the full 40
  character head, and name your seat on the first line. A receipt for another head or PR does not count, and
  a quoted receipt is not a posted one.
- **What a card contains:** the task, the PR state, what changed since your last card, an approach (it
  changes each hand-out), a "done when", and what to do if blocked. If you cannot do it, say exactly which
  step is blocked and by whom; do not report "pending".
- **The heartbeat routine** is [routines/STEWARD_HEARTBEAT.md](routines/STEWARD_HEARTBEAT.md): the trigger is a
  short pointer to that file, and the file is the single source of truth. It can narrow what a seat may do,
  never widen it.
- **Tests and contract:** `python scripts/golden_vectors.py --check` verifies the golden vectors in
  `scripts/golden/steward/`; they are the behavioural contract any re-implementation must meet.
- Task-card logic lives in `scripts/next_task.py` and `scripts/collect_state.py`.

## 5. Claims and the queue

[COORDINATION_QUEUE.md](COORDINATION_QUEUE.md) is the only thing that grants a claim. In practice:

1. Open a claim PR that adds or updates a row: lane ID, exact paths, status, your seat, the branch, a UTC
   start and a handoff. Status words come from the checked list in `governance/QUEUE_STATUSES.json`.
2. Wait for the row to be visibly present on `main` before building. Claims expire after 24 hours without
   an update; expiry never discards dirty work.
3. Finish by updating the row with what you did and the verification evidence.
4. No self-certification: your own "Done" is not acceptance.

`python scripts/check_coordination_evidence.py origin/main HEAD` (both refs are required) is what CI runs on
any PR that touches the queue. Priorities are in [MULTITHREAD_PROGRAM.md](MULTITHREAD_PROGRAM.md); do not
turn a worklist item into a claim without an owner's go. When a PR touches the queue, expect conflicts with
other claim PRs and say so in the PR.

## 6. Reviews and merging

- **A review that counts:** a whole-diff, independent review at the PR's current head, with a fenced
  `attestation` block (and, once you have a key, an `attestation-sig` block). Required fields: `repo`, `pr`,
  `head` (full 40 hex), `base_sha` (the merge base), `scope`, `reviewer`, `implementers`, `verdict`
  (`accept`, `needs_revision` or `changes_requested`, lower case). A new head makes earlier reviews stale.
- **Independence:** you cannot review a PR you wrote or co-authored, including a merge or fix commit on the
  branch. A PR that carries another open PR's commits is also not independent of that PR's authors.
- **Rubric:** [REVIEW_RUBRIC.md](REVIEW_RUBRIC.md). **Coverage report:** `scripts/review_coverage.py`.
- **Merge gate:** `python scripts/merge_gate.py --repo OWNER/NAME --repo-dir CLONE --identities
  governance/identities.json [--pr N] [--json]` prints, per PR, a literal checklist (tier, CI, conflicts,
  base, two independent accepts, open threads, evidence, revertibility, circuit breaker). It decides; it
  merges nothing.
- **Tiers and owner-only paths:** Tier A (never automatic) includes `AGENTS.md`, `.github/`, `SECURITY.md`,
  `CONTRIBUTING.md`, `governance/identities.json` and the merge policy itself. Read
  [AUTOMATED_MERGE_POLICY.md](AUTOMATED_MERGE_POLICY.md) for the tiers and the conditions. An agent never
  merges a PR that touches those paths without the owner's confirmation in chat, and never merges without a
  current grant from the owner; if the harness denies a merge, stop and report once.
- **Commands:** merges use `gh pr merge N --merge --match-head-commit <full head>`; history is kept (no
  squash).

## 7. The local model pipeline

Principle ([LOCAL_MODEL_AUTONOMY_POLICY.md](LOCAL_MODEL_AUTONOMY_POLICY.md)): models draft or classify;
software validates; a different seat accepts. Four tiers: 0 deterministic gates; 1 the local model's cheap
always-on drafting; 2 automatic escalation with guards; 3 what still needs a human.

Choose a path with the table in [LOCAL_WORKER_OPERATIONS.md](LOCAL_WORKER_OPERATIONS.md) section 1:

| Path | Entry point | Use for |
| --- | --- | --- |
| Support worker (the default) | reviewed TOML in `worker-jobs/`, `foundation local-worker-tick` / `local-worker-serve` ([LOCAL_WORKER_SERVICE.md](LOCAL_WORKER_SERVICE.md)) | fixture drafts, evidence classification, bounded analysis |
| Multithreader review | `foundation multithreader-review` / `multithreader-serve` ([MULTITHREADER_SERVICE.md](MULTITHREADER_SERVICE.md)) | proposing new work from declared evidence; it implements nothing |
| Registrar dispatch | `foundation dispatch` / `promote` | a bounded draft for one target file; promotion has a high-risk allowlist and a circuit breaker |
| Ad hoc script | the product repo's `local_agent_delegate.ps1` (in its scripts directory) | scratch only; never evidence |

- **Escalation, the "ping" to cloud agents:** past its guards the registrar pushes an isolated
  `local-agent/{task_id}` branch and stops. It opens no PR and holds no GitHub credential. A cloud seat takes
  the branch from there, opens the PR and the normal gates and reviews apply. A cloud seat can also trip the
  circuit breaker remotely through `circuit_breaker_override.toml`. Details: the autonomy policy section
  "The escalation channel".
- **Models and hosts:** the devtools worker uses a local Ollama model (named in
  [LOCAL_WORKER_SERVICE.md](LOCAL_WORKER_SERVICE.md)); the product repo's delegation script names its own
  local server in the product `AGENTS.md`, section 0.1. They are separate setups; check the document for the
  current model and port instead of assuming.
- **Inbox and outbox:** priming, reviewing and clearing are in LOCAL_WORKER_OPERATIONS.md sections 2 and 3.
  Results are `pending-owner-review` until a seat accepts them.

## 8. Scheduler, services and the job pipeline

- **Scheduler review:** `foundation schedule-review` reads an explicit queue and program and emits a
  versioned capacity report ([SCHEDULER_MAINTAINER.md](SCHEDULER_MAINTAINER.md)). The cadence of daily,
  weekly, monthly and quarterly checks is in [SCHEDULE.md](SCHEDULE.md).
- **Services:** which processes run unattended, and which were removed, is recorded in
  [PERSISTENT_OPERATIONS.md](PERSISTENT_OPERATIONS.md). Several of these documents overlapped and
  contradicted each other when last audited (lane C92, Ready and unowned); trust the dated entries, and check
  the host before assuming a task is installed.
- **Cloud routine:** the daily primary-lane routine is described in SCHEDULE.md, with a record of its stored
  prompt under `routines/`. Its first run could not reach GitHub; do not rely on it as capacity yet.
- **Packet utility and job pipeline:** [UTILITY.md](UTILITY.md) and
  [../architecture/JOB_PIPELINE.md](../architecture/JOB_PIPELINE.md). The user works through packets
  (readiness, research, blueprint, publish) rather than raw commands. Research suggestions are brainstorming,
  not executable authority; a new job template needs a concrete contract, an authority classification and a
  maintainer review. Build with `cargo build --manifest-path foundation/Cargo.toml --workspace --bins`.

## 9. Guild and Cratify

- **Cratify** is the compliance model enforced on the Aaroneous side: every crate declares a profile
  (`kernel`, `control`, `presentation` or `tooling`) in its manifest, and rules ratchet forward. The spec is
  in the product repo (`docs/CRATIFY_SPEC.md`). Here, `scripts/profile_manifest_report.py` reports which
  product crates declare a profile.
- **Guild** is the proposed name for the steward once it graduates into Aaroneous as a Rust component. Today
  it is a proposal ([proposals/TRANSPARENT_BY_DESIGN_NOTATION.md](proposals/TRANSPARENT_BY_DESIGN_NOTATION.md),
  [proposals/PATTERNS_FROM_AGENT_RUNTIMES.md](proposals/PATTERNS_FROM_AGENT_RUNTIMES.md)) with a Rust port in
  progress whose contract is the golden vectors. It is not something you can run as "Guild" yet.
- Graduation rules: section 0 above and `AGENTS.md` section 4 in the product repo.

## 10. I want to...

| I want to | Do this |
| --- | --- |
| Start as a new seat | Section 2 |
| Get or register a signing key | Section 3, [IDENTITY_SETUP.md](IDENTITY_SETUP.md) |
| Find out what to work on | Run the steward (section 4), then [MULTITHREAD_PROGRAM.md](MULTITHREAD_PROGRAM.md) |
| Claim work | Section 5 |
| Check whether my change is mergeable | `python scripts/merge_gate.py ...` (section 6) |
| Review someone's PR | Section 6, [REVIEW_RUBRIC.md](REVIEW_RUBRIC.md) |
| Give the local model a job | Section 7 |
| Check the schedule or a service | Section 8 |
| Understand the repo boundary | [FOUNDATION_BOUNDARY.md](../FOUNDATION_BOUNDARY.md), [OPERATING_MODEL.md](OPERATING_MODEL.md) |
| Handle git, branches and worktrees | [GIT_WORKFLOW.md](GIT_WORKFLOW.md), `python scripts/worktree_inventory.py REPO_DIR` (read-only) |
| Record a decision | [DECISIONS.md](DECISIONS.md) |

## 11. Rules that bite

- Never push to `main`; never force-push except `--force-with-lease` on your own branch; never delete what you
  did not create; never touch another seat's dirty checkout.
- Never self-accept; a new head makes every earlier review stale.
- Never merge a PR that touches an owner-only path, or merge at all without a current owner grant.
- Never run a script that contains `gh pr merge` or any other write to GitHub as a "test". Read it, check its
  syntax, or run a copy whose `gh` calls were replaced by `echo`.
- No claim of done without the command, its exit code and a log path or commit hash.
- Label what you did not verify as NOT VERIFIED.
- Never put a credential in the repository; never enter one into a form.

## 12. Keeping this guide true

- One home per fact: add a link here, not a copy. A row in a table must cite the document that owns it.
- Update this page in the same PR that changes a command, a path, a status or a gate it describes. The status
  table in section 1 carries a date; refresh the date when you refresh the row.
- A section that a seat could not follow without asking a question is a defect in this page. Open a PR
  against it with the question and the answer.
- The `AGENTS.md` file in each repo is owner-only. This guide may be pointed to from there only by the owner.
