# Round Table Ping

A low-cost "something changed, come look" signal for every agent working
across Aaroneous and devtools — Claude, Codex, Antigravity/Gemini, and any
future participant, each running its own background/scheduled check inside
its own session. This file exists so that check can stay almost free: diff
this one small file instead of re-reading `COORDINATION_QUEUE.md`, the
orchestration plan, or the full worklist on every tick.

## The check (a shell command your scheduler runs — not an LLM call)

```
git fetch --quiet
git log -1 --format=%H origin/<shared-branch> -- governance/ROUND_TABLE_PING.md
```

Use whichever branch you're actually tracking for shared work — currently
`integration/local-cloud-orchestration` in devtools (see
[`COORDINATION_QUEUE.md`](COORDINATION_QUEUE.md) row C35), `main` once that
merges. Compare the returned hash against the hash you saw last time, kept
in your own **local, untracked** state (do not commit your last-seen marker
anywhere shared — that would itself look like a change to every other
agent's check, causing a re-trigger storm).

- Hash matches → stop. Zero tokens spent, no read, done.
- Hash differs → now spend real tokens: read this file, look at the
  `Paths` column of every row newer than the one you last acted on, and
  read/act on only those underlying files — not the whole repo.

This only works once a ping is actually pushed, not just committed locally
— `git fetch` is required, plain `git log` against local `HEAD` would miss
another agent's unpushed-to-you work.

## Severity

- `notice` — read next time you're idle or between tasks; nothing urgent.
- `urgent` — read before starting or continuing other work. This carries
  no enforcement of its own (unlike `circuit_breaker_override.toml`, it
  does not block `promote()` or anything else) — it's a signal, not a
  gate. Use `circuit_breaker_override.toml` instead when you actually need
  to stop a pipeline, not this file.

## Convention

- Worth a row even with no file diff behind it: taking on or dropping a
  standing role (e.g. "I'm hosting the local worker now"), or a direct
  relay to another agent ("need X from whoever's active"). These aren't
  file-change notices — say so plainly in the summary rather than forcing
  a `Paths` entry that doesn't really apply.
- Opening or closing a [brainstorm session](BRAINSTORM_PROTOCOL.md) is
  exactly one row each — the session's own file is where the actual
  multi-agent content lives, never this one.
- Add a row only when something is genuinely worth another agent's
  attention: a new milestone, a design decision touching shared files, a
  finding that changes what someone else is mid-building. Don't ping for
  routine commits — `COORDINATION_QUEUE.md`'s own history already covers
  those, and pinging on every commit defeats the point of a cheap signal.
- Newest row at the top.
- Never delete a row; a resolved or stale ping is still useful history —
  mark it resolved in a later row instead.
- Acknowledging a ping is local state (your own scratch memory, not a
  write here). Writing to this file should only happen when there's
  something genuinely new to announce, so the hash-diff signal stays
  meaningful instead of drifting into per-agent chatter.
- If you're about to touch files another row points at and its status is
  unclear, check `COORDINATION_QUEUE.md` for a live claim before assuming
  it's safe — this file tells you *that* something changed, the queue
  tells you *who owns it now*.

## Log

| UTC | From | Severity | Paths | Summary |
| --- | --- | --- | --- | --- |
| 2026-09-21T16:20:00Z | Claude Sonnet 5 | notice | `crates/compute/src/si_packer.rs`, `core/hypervisor/src/supervisory_loop.rs`, `crates/ipc_bus/src/observation_buffer.rs`, `crates/compute/src/si_solid_state.rs`, `crates/si_format/src/verify.rs`, `core/hypervisor/bin/hypervisor.rs`, `xtask/src/workspace_metadata.rs` (all Aaroneous) | Codex left 9 real review findings on Aaroneous PR #51, 3 independently re-verified against current source (all confirmed genuine). Logged as `COORDINATION_QUEUE.md` row C40, Ready/unclaimed for pickup once credits reset Wednesday — not fixing now. See that row for per-finding detail and recommended priority order. |
| 2026-09-21T16:05:00Z | Claude Sonnet 5 | notice | `governance/BRAINSTORM_PROTOCOL.md` | Added scheduled/live-window sessions — an explicit-time invite (`Starts`/`Window` fields), each agent self-schedules its own wake via its own durable scheduler rather than any new shared listening infrastructure, tighter polling permitted only inside the agreed window. Owner's idea, in response to wanting real engagement without an always-on service. Likely obsoletes the `round-table-serve` long-poll server floated the prior turn — not building that unless something else needs it. |
| 2026-09-21T15:45:00Z | Claude Sonnet 5 | notice | `governance/{BRAINSTORM_PROTOCOL,CROSS_AGENT_COORDINATION_PROTOCOL,ROUND_TABLE_PING}.md` | Added a bounded multi-agent brainstorm mechanism (one file per topic, explicit close condition, no reply obligation) after the owner asked how to get everyone actively engaging on one question without event-chain chaos. Also clarified that role announcements ("hosting the local worker now") and direct relays are legitimate ping-worthy even with no file diff behind them. |
| 2026-09-21T15:10:00Z | Claude Sonnet 5 | notice | `governance/ROUND_TABLE_INTRO_PROMPT.md`, `AGENTS.md` (devtools), `README.md` (devtools), `../Aaroneous/AGENTS.md`, `../Aaroneous/README.md` | Added a ready-to-paste onboarding prompt for a new agent session joining the round table, and linked it (plus `CROSS_AGENT_COORDINATION_PROTOCOL.md`) from both repos' `AGENTS.md`/`README.md`. Aaroneous's copies use prose references rather than hyperlinks into devtools, since devtools is a private repo and Aaroneous's README is public. |
| 2026-09-21T14:40:00Z | Antigravity (Gemini) | notice | `foundation/crates/foundation_cli/src/frontier_discovery.rs`, `foundation/crates/foundation_cli/src/discovery_digest.rs`, `governance/DISCOVERY_DIGEST.md` | Connected `RUST_DISCOVERY_SEED_CATALOG.md` into cyclic rolling-window frontier discovery engine with G1/G4 pre-filtering, hooked automated regeneration of `governance/DISCOVERY_DIGEST.md` on local worker ticks, and verified live evaluation with Qwen 9B. |
| 2026-09-21T00:05:00Z | Claude Sonnet 5 | notice | `governance/CROSS_AGENT_COORDINATION_PROTOCOL.md` | Formal umbrella doc for the whole cross-agent coordination system ("the round table" in conversation): names the five pieces, gives a new-agent onboarding checklist, and names the shared-working-tree collision risk as a known, unfixed gap. Linked from `COORDINATION_QUEUE.md`'s intro. |
| 2026-09-21T00:00:00Z | Claude Sonnet 5 | notice | `governance/RUST_DISCOVERY_SEED_CATALOG.md`, `governance/ROUND_TABLE_PING.md` | Added a ~100-project Rust discovery seed catalog for `frontier-discover-serve` (M41), and this ping protocol itself — first real entry, demonstrating the mechanism it describes rather than just documenting it. |
