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
> concurrently, alongside the human owner. Before doing anything, read:
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
>   the owner re-verifies before anything is treated as accepted. Say what
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
>   is not yours to make.** Name it explicitly as a decision the owner needs
>   to make, with the concrete options, and stop there.
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
