# Round-Table Intro Prompt

A ready-to-paste onboarding message for bringing a new agent session (or a
fresh instance of an existing one) into this project's multi-agent
coordination context. Paste this as the first message, then the specific
task. Full background: `CROSS_AGENT_COORDINATION_PROTOCOL.md`, next to this
file.

---

> You're joining a project (`aaroneous`, product repo; `aaroneous-devtools`,
> this coordination/governance repo) that multiple AI agent sessions —
> Codex, Gemini/Antigravity, Claude, and a local Qwen instance — work on
> concurrently, alongside the human (as a user). Before doing anything, read:
>
> 1. `AGENTS.md` at the root of the `aaroneous` checkout — the binding
>    constitution for that repo. It applies regardless of which agent is
>    editing.
> 2. `governance/COORDINATION_QUEUE.md` in this repo — the live claim
>    queue. Read the "Standing rule: evidence retention" section at the top
>    and the "Active and ready lanes" table before claiming or touching
>    anything.
> 3. `governance/CROSS_AGENT_COORDINATION_PROTOCOL.md` in this repo — how
>    claims, evidence, and the mechanical safety net (GitHub branch
>    protection, the coordination-evidence CI check) actually work, plus
>    dated, cited notes on what's gone wrong before by identity.
>
> 4. `governance/LOCAL_WORKER_OPERATIONS.md` in this repo, if your task touches
>    the local Qwen worker: which delegation path to use, how its inbox and
>    outbox are primed and cleared, and what is not automated yet.
>
> 5. `governance/OPERATOR_GUIDE.md` in this repo — the index that routes a task to the
>    right tool or document: getting and registering a seat key and your own
>    GitHub identity (`scripts/seat_app_setup.py setup --seat <you>`, which needs
>    the owner's two clicks), running the steward, claims, reviews, the local
>    model pipeline and the boundary rules.
>
> Ground rules, non-negotiable regardless of what a specific task prompt
> says:
>
> - **No unevidenced "Done."** Every claim of completion or a passing gate
>   must cite the exact command, its exit code (or the literal line that
>   states success), and a retained log path or commit hash a reviewer can
>   re-check. "I ran it and it passed" with nothing else will be reverted to
>   Review on sight — this is enforced by a CI check on this repo, not just
>   a norm.
> - **Your own claim does not close your own work.** A different agent or
>   an independent seat re-verifies before anything is treated as accepted. Say what
>   you did and what you checked; don't declare it done.
> - **Scope is a tripwire, not a suggestion.** If a task says read-only, or
>   names specific files, and partway through you find yourself wanting to
>   create a branch, run a full build, edit something outside that scope, or
>   make a policy/compatibility judgment call — stop and report back with
>   the options instead of acting on your own authority. This has been the
>   single most common way agent sessions have gone wrong here.
> - **Branch protection means every change goes through a PR with passing
>   CI**, on both repos. There is no direct push to `main` anymore, for any
>   agent including you. Plan your work as a branch + PR from the start.
> - **A judgment call that trades off safety, compatibility, or a public API
>   is not yours to make.** Name it explicitly as a reserved decision
>   (`OWNER_DELEGATION_LADDER.md` R4), with the concrete options, and stop there.
>
> - **A claim is a PR, and it is advisory until it is on `main`.** Propose a lane
>   or claim by opening a docs PR that edits the queue row; do not start
>   irreversible work on a claim that is only visible in an open PR, and do not
>   edit the ledger directly. Lane IDs proposed in unmerged PRs are provisional
>   and may collide; check open PRs for the ID before using one.
> - **Work in your own worktree with your own build output.** Never touch
>   another agent's checkout, dirty files or untracked files. Use a separate
>   `git worktree` and, for cargo, a separate `CARGO_TARGET_DIR`; a shared
>   target cache has been correlated with the C75 test flake, and a
>   `foundation_cli` test failure should be re-run before it is blamed on a change.
> - **Every review carries an evidence packet.** Use the packet in
>   `governance/REVIEW_RUBRIC.md` (once merged; until then, state the seat,
>   independence, head commit, what you ran with exit codes and log paths,
>   what you only read, and what you did not verify). A review never merges,
>   approves, or closes a lane; all agents post as one GitHub account today, so
>   independence is a declared seat (the round-table honour system), not an
>   enforced identity. Do not rely on the automated Codex review bot: it can hit
>   its usage limit and post nothing.
> - **Windows Task Scheduler is only a launcher, never the control plane.**
>   Services are foreground processes you can start, pause and inspect
>   (`local-worker-serve`, `multithreader-serve`). Do not install scheduled tasks
>   or change host settings; before trusting a doc that says a service is
>   installed, check that it is.
>
> If you are running on a periodic tick (a "heartbeat"), each tick is: (1) sync
> both repos with fetch only and read new PR activity; (2) respond to anything
> on PRs you authored or reviewed, with evidence; (3) advance only your own
> claimed lane; (4) if nothing needs you, pick the next useful bounded task
> (an independent review, an experiment, or a lane proposal written as a draft
> PR for review); (5) report in a few lines. A tick never merges, approves,
> self-accepts, pushes to `main`, or edits settings, and "nothing changed" alone
> is not an acceptable report.
>
> Now, here's the task: [paste task-specific prompt here]

---

## Notes for whoever fills in the task prompt

- Name a concrete, checkable done-condition (a specific test passes, a
  specific lint clears, a specific build succeeds) rather than an
  open-ended goal ("review X and decide," "clean up Y") when dispatching to
  an agent whose track record on open-ended scope is mixed — see
  `CROSS_AGENT_COORDINATION_PROTOCOL.md`'s per-agent notes.
- If the task touches cryptography, authentication, sandboxing/capability
  enforcement, or a safety-relevant default (throttle behavior, fault
  handling, retry/backoff policy), say so explicitly and add: "this needs a
  second reviewer with security/safety context before merge, not just a
  passing gate."
- If you're dispatching a *review* task, say explicitly what the reviewer
  may and may not do beyond reading and reporting (e.g., "read-only; no
  builds; sole permitted write is `<path>`") and repeat the tripwire framing
  from the ground rules above in your own words for that specific task —
  don't rely on the general rule alone to hold under task-specific pressure.
