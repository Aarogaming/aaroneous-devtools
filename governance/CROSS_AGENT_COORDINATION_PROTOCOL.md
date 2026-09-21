# Cross-Agent Coordination Protocol

The formal name for what gets called "the round table" in conversation —
the set of mechanisms that let multiple independent AI agents (currently
Claude, Codex, and Antigravity/Gemini, each running in its own application,
plus the local Qwen worker) coordinate work across Aaroneous and devtools
without stepping on each other, without a live API bridge between products,
and without spending tokens polling large documents on a timer. "Round
table" is fine as shorthand in conversation; this document and its
sub-documents are the actual system it refers to.

This is a coordination layer, not an authority grant. Nothing here lets an
agent self-certify, self-merge, or skip review — see `EVIDENCE_STANDARD.md`
and `CHANGE_VERIFICATION_TIERS.md` for what still applies regardless of how
many agents are involved.

## The five pieces

| Piece | File | What it's for |
| --- | --- | --- |
| Claim ledger | [`COORDINATION_QUEUE.md`](COORDINATION_QUEUE.md) | Who owns what, right now. Claim a row before editing; leave a handoff note when done. The one piece every agent already uses. |
| Change signal | [`ROUND_TABLE_PING.md`](ROUND_TABLE_PING.md) | A near-zero-cost "come look" mechanism. Each agent's own scheduler diffs one small file's git hash on its own cadence; only reads real content when the hash moved. |
| Pairwise deliberation | `cross_verifier.rs` / `cloud_providers.rs` | Puts one agent's draft in front of another (author ≠ auditor) for a real Red-Team audit, across any of Claude/Gemini/Codex/local Ollama. Requires that provider's own API key via `from_env` — a separate budget from any app-level session credits. |
| Stop signal | `circuit_breaker_override.toml` | Any cloud agent can trip this to halt `promote()` after finding a problem in already-landed work. Enforced, unlike the ping — fails closed if unreadable. |
| Proposal format | `worker-jobs/*.toml` | The structured packet schema for handing a bounded task to the local worker and getting a `pending-owner-review` result back. |
| Brainstorm session | [`BRAINSTORM_PROTOCOL.md`](BRAINSTORM_PROTOCOL.md), `brainstorm/*.md` | Bounded, multi-agent ideation on one open question — parallel contribution to one file with an explicit close condition, not a live chat. Announced and closed via one `ROUND_TABLE_PING.md` row each, not a thread inside the ping itself. |

For a ready-to-paste onboarding message covering everything below in one
block, see [`ROUND_TABLE_INTRO_PROMPT.md`](ROUND_TABLE_INTRO_PROMPT.md) —
give it to a new agent session as its first message instead of retyping
this document's contents by hand.

## How a new agent joins

1. Read `MAINTENANCE_SYSTEM.md`, then this document, then
   `COORDINATION_QUEUE.md`'s current rows — know what's claimed before
   touching anything.
2. Check `ROUND_TABLE_PING.md` for anything flagged since you were last
   here.
3. Set up your own scheduled check against the ping mechanism (a shell
   command your background/scheduling feature runs, not a chat turn) —
   see `ROUND_TABLE_PING.md`'s "The check" section for the exact command.
   If your application can only schedule prompts, not raw shell, say so
   explicitly rather than approximating it with a full-file re-read every
   tick — that defeats the point and burns the exact budget this exists to
   protect.
4. Before editing a file, confirm no queue row claims it and it isn't
   showing up as locally modified/uncommitted in a shared working tree you
   don't own (see "Shared working tree" below).
5. Claim a row, do the work, leave a real handoff note — evidence, not a
   confidence claim. Nothing here is exempt from the existing review gates.
6. If you find something worth another agent's attention specifically
   (not just routine progress), add a row to `ROUND_TABLE_PING.md`. If you
   find something that should stop another agent's pipeline, use
   `circuit_breaker_override.toml` instead — the ping has no enforcement.

## Table etiquette (avoiding unintentional event chains)

The mechanisms above are cheap and low-friction by design, which is exactly
what makes them easy to misuse into a feedback loop. None of the following
are enforced in code — they're discipline, the same way "don't ping for
routine commits" already is.

- **No reply-pings.** Never add a `ROUND_TABLE_PING.md` row solely to
  confirm you read a previous one — that looks like new activity to every
  other agent's diff check and invites another reply. Acknowledgment is
  local state, never a write here.
- **Never widen what's diffed.** The check stays scoped to
  `ROUND_TABLE_PING.md` alone. Auto-regenerating files —
  `DISCOVERY_DIGEST.md`, `PORTFOLIO_TRIAGE_DIGEST.md`,
  `INVARIANT_AUDIT_STATUS.md`, anything the local worker rewrites every
  tick — must never be added to any agent's watched-paths list. Watching
  one of those turns "cheap, only on real change" into "changes every few
  minutes regardless."
- **The local worker doesn't post to the table.** Per
  `LOCAL_AGENT_CONTROL_PLANE.md`, Qwen proposes, it doesn't claim or edit
  governance docs. It never writes to `ROUND_TABLE_PING.md` or
  `COORDINATION_QUEUE.md` directly — a cloud agent decides, from its
  output, whether something is genuinely worth relaying to the table.
- **`urgent` is rare, on purpose.** Reserve it for something that should
  actually interrupt another agent's current work. Used for anything
  routine, it teaches every agent to eventually treat `urgent` the same as
  `notice`, which defeats its one purpose.
- **Deliberation belongs in `cross_verifier.rs`, not a growing queue
  cell.** If two agents genuinely need back-and-forth, use the pairwise
  author/auditor mechanism instead of turning one `COORDINATION_QUEUE.md`
  row into an extended thread. Keep the queue transactional: claim,
  evidence, handoff.
- **A review finding isn't automatically a new claim.** If reviewing one
  row surfaces something else worth doing, note it and let it be picked up
  deliberately — it gets the same evidence/scope bar as anything else, not
  a lower one for having been noticed mid-review.
- **Back off your own check cadence.** Don't poll every few seconds —
  that's needless traffic even at zero token cost. Mirror
  `local-worker-serve`'s own adaptive backoff: check tighter right after a
  real change, relax toward roughly 15–30 minutes when the table's been
  quiet.
- **A circuit breaker trip needs a recorded untrip condition, not just a
  trip.** State what has to be true for someone to clear it. An
  unconditioned trip just becomes indefinite ambient caution — the same
  self-certification problem this project already refuses, in reverse.

## Shared working tree (known gap, not yet fixed)

All agents currently operate against the same checkout of each repo, not
isolated per-agent worktrees. The queue tells you who *claimed* something;
nothing currently stops two agents editing the same file at the same
uncommitted moment. Recommended, not yet enforced: each agent works in its
own `git worktree` / branch and pushes, merging through the queue like a
normal PR, rather than live-editing one shared tree. Until that's adopted
by convention, check `git status` for local modifications before editing
anything, even if the queue shows it unclaimed — a row only gets claimed
once an agent chooses to log it, not the moment it starts editing.

## Relationship to other documents

- [`LOCAL_AGENT_CONTROL_PLANE.md`](LOCAL_AGENT_CONTROL_PLANE.md) governs
  the local Qwen worker specifically — narrower authority, proposal-only,
  never self-claims. This document governs the broader set of cloud agents
  plus the local worker coordinating together; it doesn't relax anything
  the control plane restricts.
- [`ROUND_TABLE_PING.md`](ROUND_TABLE_PING.md) is this protocol's one
  operational mechanism with its own detailed spec; this document is the
  index and onboarding path, not a replacement for reading it directly.
- [`COORDINATION_QUEUE.md`](COORDINATION_QUEUE.md) remains the live source
  of truth for claims; this document doesn't duplicate its rows.

## Open, honestly

- Claude's side of the scheduled cheap-check can be wired today via this
  session's available scheduled-task tooling. Codex's and Antigravity's
  scheduled checks need to be configured inside those applications,
  pointed at the same command — not yet done, and depends on whether each
  supports running an arbitrary shell command from its scheduling feature
  versus only re-firing a chat prompt.
- The shared-working-tree risk above is real and currently live (observed
  directly: Antigravity mid-edit, uncommitted, in this same checkout,
  2026-09-21). Worktree-per-agent is proposed, not adopted.
