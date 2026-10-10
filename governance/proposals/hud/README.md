---
notation: v1
purpose: Index, requirements and decision log for the HUD v2 and guild-hall proposal set
authority: propose
status: proposal, advisory, grants nothing
relations:
  - informs: doc:architecture/JOB_PIPELINE.md
  - informs: doc:proposals/SUPPORTED_OPERATIONS_SERVER.md
  - depends-on: doc:governance/UTILITY.md
  - overlaps: lane:C106
  - overlaps: lane:C112
  - overlaps: lane:C113
  - overlaps: lane:C118
checked-by: none yet (see each document's "Not verified")
---

# HUD v2 and the guild hall: proposal set (lane C119)

Contribution-ID: C119/claude-hud-v2-proposal-v1.
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposals for review**. No code, no policy change, no workflow change. Nothing here is accepted until a reviewer
other than the author records it. This directory was split into one document per feature so each can be reviewed, amended or
rejected on its own; the umbrella pull request carries this index and the lane row, and each feature has its own pull request.

Owner direction in chat, 2026-10-10 (summary): make the 127.0.0.1:8787 HUD better; local-first AI where software does the
work and a local model is the glue; inquiries become jobs; user-assisted blueprint and research packets; run operations from
the interface; every model pings it for cards, work and availability; fewer user interactions, more frequent work; token
optimization; configuration instead of hard-coded behaviour; default read-only; a standard native application; a 3D view;
the system as a guild hall with roles, skill-gated delegation and earned trust; local network first; local models as
advisory guild workers.

## The documents

| # | Proposal |
|---|---|
| 01 | [Settings layers and the permission engine](01-settings-and-permissions.md) |
| 02 | [Agent door and MCP adapter](02-agent-door-and-mcp.md) |
| 03 | [LAN roster, wire and domains](03-lan-roster-and-domains.md) |
| 04 | [Token optimization](04-token-optimization.md) |
| 05 | [Model profiles and prompt adaptation](05-model-profiles-and-prompt-adaptation.md) |
| 06 | [Brief compiler and packets](06-brief-compiler-and-packets.md) |
| 07 | [Guild workers, staff roles and inspection](07-guild-workers-staff-and-inspection.md) |
| 08 | [Roles, skill gating and earned trust](08-roles-skills-and-trust.md) |
| 09 | [Visualizer, exploration and digital twin](09-visualizer-exploration-and-twin.md) |
| 10 | [Native application, operations panel and reuse](10-native-app-operations-and-reuse.md) |

**Suggested review order:** 01, 02, 03 (foundations), then 06, 07, 08 (behaviour), then 04, 05 (economy), then 10, 09
(interface). Each document ends with a "Review focus" list, its own decisions and its own "Not verified" list.

## Requirements

- R1. Local-first: software answers whatever software can answer; a local model only glues, summarises and routes; cloud
  models are asked only when the first two cannot.
- R2. A user inquiry typed into the HUD becomes a job (a typed intake record), not a one-off prompt.
- R3. Blueprint and research packets are edited with the user's help in the HUD (propose, correct, approve), not generated
  and discarded.
- R4. Operations (tick, verify-log, schedule, review queue, update) are run from the HUD.
- R5. Every model and seat can ping the HUD for its card, available work and availability, and report back.
- R6. Fewer human interactions, higher frequency of work: the default is that the system proceeds and asks the user only
  at decisions that are theirs.
- R7. Token cost is measured and reduced, not assumed.
- R9. A human-to-machine translation layer: a survey and conversation with a light model fills a pre-built brief
  (who, what, why, when, how, constraints, acceptance), and the finished brief becomes one prompt for a larger model
  that works in the background pipeline. Owner direction, 2026-10-10.
- R10. A visualizer for analogy-based references, in a 3D application, with the HUD as the main centre of operations.
- R8. Behaviour comes from configuration, not code: every default (permissions, models, routing, limits, polling,
  caches) is a setting that can be changed later in user settings without a release. Owner direction, 2026-10-10.

## Layers

Owner framing, 2026-10-10: the HUD-HUB manages a **stable Aaroneous core**, with devtools as an **operational layer**
on top, and the **machine-dependent models** we host tied to their machine as local workers.

| Layer | What it is | Rule |
|---|---|---|
| Aaroneous core | the product repository and its crates; the pinned base | changes only through its own gate; devtools depends on it, never the reverse |
| Devtools (operational layer) | the hall: intake, queue, policy, steward and Guild, packets, settings | operates on repositories; holds no product authority |
| Hosted models | the local model on each machine, served by Ollama, vLLM or LM Studio | bound to its machine and its sponsor seat; low permission |

The user is the **guild manager**. The local model is the **staff** (secretary, organiser, bookkeeper, auditor, cleaner),
taking work as job cards at its own level (see [07](07-guild-workers-staff-and-inspection.md)).

## Design principles

1. **Software first, model second.** Every question goes through deterministic software (queue parsing, state collector,
   search over retained evidence, templates) before any model. The local model receives a compact, pre-digested context
   pack, never raw files. A cloud model receives only what the local model could not resolve, with the pack attached.
2. **The HUD is a thin operating layer over existing tools** (Guild CLI, steward scripts, packets, queue), not a second
   implementation. It calls them and shows their evidence; it does not replace their authority. This follows the
   toolbelt rule in the product `AGENTS.md`.
3. **Separation of church and state.** Devtools depends on Aaroneous, never the reverse. Nothing here adds a dependency
   from the product repo to the HUD.
4. **Same authority rules for models and people.** A model pinging the HUD gets cards and may report results; it does not
   gain merge or review authority. Reviews and merges stay with the steward gate and signed attestations.
5. **Silence on unchanged state.** Responses say "nothing new" in a few bytes (see [04](04-token-optimization.md)). Quiet is the cheapest answer.

## Overlap with existing lanes (reused, not duplicated)

- **C106** typed job intake: the intake format. HUD v2 consumes it and must not invent a second one.
- **C112** seat status/host availability: the availability record. HUD v2 serves it over HTTP (its Phase 3 mirror was
  pending an owner decision on the transport; this proposal answers it with "the HUD's agent door").
- **C113** Guild CLI: the operations behind the panel. HUD v2 calls it; it does not reimplement the scheduler.
- **C118** tray + update check also edits `maintenance_ui`: P1 will rebase on it; no file-level conflict is expected
  but sequencing is needed.
- **C102 / PR #135** supported operations server: HUD v2 is the local face of that proposal; whether they become one
  service or two is for the reviewers of #135 and this proposal.

## Phases across the set

| Phase | Document | Summary |
|---|---|---|
| P1a | 01 | settings layers and permission engine (lane C120, code in review) |
| P1, P1b, P2 | 02 | agent door, MCP adapter, event stream |
| G0b | 03 | LAN roster, register/heartbeat/lease, `network.*` settings |
| P3, P3b, G3 to G5 | 06 | intake, brief compiler, triage, supervisor meeting, taste checkpoints |
| W1 to W4 | 07 | worker queue, validators, traces and inspection, trials |
| G1, G2, G6 | 08 | roles and profiles, `can_take` gate, progression ledger |
| P4 | 04, 05 | token accounting and caches, model profiles and prompt adapters |
| P5, P7 | 10 | operations panel, native application shell |
| P7 (part), twin | 09 | 3D analogy view, exploration layer, preference log and twin |
| G7, P6 | 03, 08 | hosting for remote users, cloud dispatch; both deferred, each needing its own review |

Each phase is its own lane and review; this set claims no code.

## Decision log (recorded from chat, 2026-10-10)

This records chat instructions; it is not an attestation. "OPEN" items are listed again in the document they belong to.

| Decision | State | Document |
|---|---|---|
| Default permission mode is read-only; all behaviour is configuration | DECIDED | 01 |
| Native Rust application, not a webview; toolkit chosen by a spike | DECIDED | 10 |
| Glue model is a setting; the existing Ollama endpoint is its initial value | DECIDED | 05 |
| Local area network first; remote strangers deferred | DECIDED | 03 |
| Roster: `mimo` (Windows, OpenCode) and `ubuntu-worker-01` (Ubuntu, OpenCode) are seats; MiMo is the model behind OpenCode; local models are guild workers | DECIDED | 03, 07 |
| Local models do advisory work, not the pull-request stack | DECIDED | 07 |
| 3D is the analogy view inside a mostly 2D native app; the reading of "analogy reference" | DECIDED, CONFIRMED | 09 |
| Code lands in separate lanes, starting with C120 | DECIDED | 01 |
| First brief templates: research, blueprint, build | DEFAULT | 06 |
| Which machine hosts the hall and how machines are joined; which agents are local sessions; domain arrangement and merge rights in Aaroneous | OPEN | 03 |
| Inspector role holders, sampling rates, trace retention | OPEN | 07 |
| Taste boundary, levels, who promotes, stranger's program location, fine-tuning off, outside benchmarks as priors | OPEN | 08 |
| Prompts as reviewed files | OPEN | 05 |
| Twin and exploration scope questions | OPEN | 09 |

## Not verified (whole set)

- No code was written or run for these proposals, except the settings crate in lane C120 and the token-encoding probe in
  document 04. Numbers elsewhere are targets.
- Each document lists its own unverified items; read those before accepting it.
