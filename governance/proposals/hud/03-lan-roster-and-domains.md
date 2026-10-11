---
notation: v1
purpose: Run the hall on the local network with a roster of seats, guild workers and instances, and one hall per repository and machine
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:01-settings-and-permissions
  - depends-on: doc:02-agent-door-and-mcp
  - overlaps: lane:C112
checked-by: none yet (see "Not verified")
---

# LAN roster, wire and domains (proposal 03)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 03 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## Local network first: the roster, the hall and the wire

Owner direction in chat, 2026-10-10: local area network first; the Ubuntu worker works alongside the Windows-hosted
worker; each machine also runs an OpenCode session, giving four workers on the LAN; then Codex, Claude and Gemini;
and the owner supplies inspiration and guidance. Treat it as a very advanced adventurers' or merchants' guild.
This **replaces** the earlier "loopback only" decision (see the decision log in the [README](README.md)) for the guild's own
machines; remote strangers stay deferred (G7).

### Roster: seats, workers and instances

Corrected 2026-10-10 from the owner's statement: the OpenCode session on Windows is the `mimo` seat; the OpenCode
session on Ubuntu is the `ubuntu-worker-01` seat ("Ubuntu-1"). Both run OpenCode, and each operates a local model on its
own machine. Confirmed by the owner, 2026-10-10: **MiMo is the model behind OpenCode on both machines, and the seat name `mimo` names
the model identity; OpenCode is the application used to reach it.** The Ubuntu seat is the same model on the second machine
under a host-specific seat name. The application and the model are recorded separately on each instance.

| Member | Kind | Machine | Notes |
|---|---|---|---|
| `mimo` | seat (operator), OpenCode with a cloud model | Windows | sponsor of the Windows guild worker |
| Windows guild worker | local model (Ollama, vLLM or LM Studio) | Windows | advisory classes only (see [07](07-guild-workers-staff-and-inspection.md)); GPU shared with the UI and the 3D view |
| `ubuntu-worker-01` | seat (operator), OpenCode with a cloud model | `aaroneoushome` (RTX 4060 per its seat-status file) | sponsor of the Ubuntu guild worker |
| Ubuntu guild worker | local model | Ubuntu | advisory classes only (see [07](07-guild-workers-staff-and-inspection.md)) |
| `claude`, `codex`, `gemini` | seats (operators), frontier cloud-backed agents | wherever each session runs | which are local sessions and which are cloud sessions decides whether they can reach the LAN directly |
| Guildmaster | the owner | Windows | gives inspiration and guidance; see 7.4 |

That is five seats, two guild workers and the owner. **Operator (a registered identity with a trust ledger) and instance
(one running process: host, runtime, model, capacity, heartbeat) remain different things:** trust and levels attach to
the seat and its skills; availability, capacity and leases attach to the instance. No new seat is needed for the
Windows machine; its local model is a worker under `mimo`. The worker instances are registered with the hall under their
sponsor, not in `identities.json`, because they do not sign attestations or reviews.

### One hall, workers that pull

- **One hall** (the HUD core with its MCP/HTTP adapter) runs on one machine; I recommend the Windows machine, where the
  owner works. Every worker is a client that **registers, heartbeats and pulls** work. The hall never needs to reach
  into a worker, so only the hall opens a port and workers behind other networks still work. This is the same
  "models ping the hall" shape already in the HUD proposals.
- **Register and heartbeat:** an instance announces itself (operator, host, runtime, model, skills it claims, resource
  slots) and refreshes on a short interval. Missed heartbeats mark it `offline`. This supersedes the hand-edited
  `seat-status` files for live use; the files remain as the retained, reviewed record (lane C112).
- **Leases, not trust in clocks:** a job is handed out under a lease measured by the hall's own clock; a dead worker's
  lease expires and the job returns to the board. Two machines will not have perfectly synchronised clocks, so
  nothing depends on comparing their timestamps.
- **Artefacts move through git, not through the hall.** Workers push branches; the hall passes references (branch,
  commit, digests). This is how the steward already works and keeps the hall small.
- **Resource slots per host,** as settings: for example one GPU slot, one heavy-build slot, a VRAM budget. The
  seat-status file already speaks of "the heavy Cargo slot" and an idle GPU; the hall schedules against those so two
  workers do not fight over one graphics card, and the 3D view yields to the model (see [09](09-visualizer-exploration-and-twin.md)).

### The wire: what changes when it leaves loopback

- The bind address is a setting (`network.bind`). Default stays `127.0.0.1`. A LAN address must be named explicitly;
  `0.0.0.0` is never the default. The accepted `Host` names are a configured list, replacing the single exact
  `127.0.0.1:8787` match.
- Bearer tokens on a LAN cross the network, so they need an encrypted path. Options, in my order of preference:
  (a) an encrypted overlay the machines already share (the Ubuntu worker's address `100.109.8.12` is in the range
  commonly used by a Tailscale-style overlay; **I have not verified that one is in use**); (b) TLS with a pinned
  certificate between the hall and each worker; (c) an SSH tunnel per worker. A setting (`network.require_encrypted`,
  default true) makes the hall refuse tokens over plain HTTP unless the interface is declared trusted.
- Per-instance tokens, hashed at the hall, revocable, scoped by the permission engine (default read-only, [01](01-settings-and-permissions.md)). A worker that is only registered and pulling can read its card and work list; reporting results and running
  operations remain `ask` or `deny` until the rules say otherwise.
- All the earlier limits stay: body caps, per-instance rate limits, untrusted-data handling of everything a worker sends.
- Cloud sessions that run off the LAN cannot reach it. For them the existing GitHub route (cards, receipts,
  queue) remains the channel, and the hall reads it. That is the same arrangement as today, not a regression.

### The guild vocabulary mapped onto real parts

| Guild idea | Real part |
|---|---|
| Quest board | the job queue: the C106 intake records plus the coordination queue |
| Rank and skill badges | per-skill levels (see [08](08-roles-skills-and-trust.md)), derived from receipts and attestations |
| Charter | the settings layers and the permission floor (merge, push, review and attest are never delegated) |
| Party | a job staffed by several operators with roles (draft, implement, review); the author is never the reviewer |
| Reputation ledger | the derived trust ledger (see [08](08-roles-skills-and-trust.md)); recomputable from retained evidence |
| Guildmaster's guidance | owner directives and taste decisions: entered through the same intake with attribution, they raise priority or set direction, are logged as preferences, and are never a bypass of a gate |
| Merchant side | a resource economy in compute terms, not money: GPU minutes on a host, local model time (cheap), cloud tokens (costly). Jobs carry a quote; workers advertise capacity; budgets (settings) cap spending; the hall prefers the cheapest cleared operator that meets the deadline |

Everything in the table is configuration over parts that exist or are proposed; none of it is new authority.

### Domains: one hall per repository and machine, with crossplay

Owner idea, 2026-10-10: run several devtools sessions with different clocks, one pointed at each repository. The Ubuntu
machine works on the Aaroneous repository, with devtools as the source of operations and Aaroneous as the pristine
base; devtools keeps advancing; because devtools is built on Aaroneous, both improve by dogfooding.

**Domain** = one hall + one repository + one machine, with its own settings layers, queue, permission floor and
**clock** (tick interval, lease length, heartbeat; all settings). A fast clock suits the interactive Windows hall; a slow
batch clock suits the Ubuntu build machine.

**Proposed first arrangement:** hall A on Windows over the devtools repository; hall B on Ubuntu over the Aaroneous
repository. The two halls do not share mutable state. Each repository keeps its own coordination queue, and lanes are
claimed only by a claim PR in the repository that owns the lane (the existing rule), so there is no cross-hall
lock to get wrong.

**Crossplay** means lending an instance: a worker registered with its home hall serves a job from the other hall under a
*visiting contract*. The home hall keeps the identity and the trust ledger; the visited hall issues leases scoped to its
repository; credentials are never shared; the result returns as git refs and receipts. Trust earned while visiting counts
in the home ledger only for the skills it actually exercised there, and only after the home hall verifies the receipts.
Skills are domain-specific: kernel-style Rust in Aaroneous is not the same skill as documents in devtools.

**The dogfood loop and the dependency rule.**
- Devtools pins an Aaroneous revision as its base and moves the pin by reviewed pull request.
- Aaroneous stays pristine because it changes only through its own gate and its graduation rule (code arriving from
  outside is proven in its origin, placed behind a workspace trait, landed inert, and the origin copy deleted).
- Hall B proposes changes to Aaroneous as pull requests under the product's rules; devtools depends on Aaroneous and
  never the reverse, and Aaroneous never depends on devtools.
- Whether hall B's operators may merge in the Aaroneous repository is the owner's decision; the default here is that
  they open pull requests and do not merge.
## Terminology

The guild-hall language is good for the interface (reception, supervisor, quest-board style status). Code, crates and
settings use plain names: intake, triage, planner, dispatcher, operator, profile, level. This follows the product's
naming rule and the existing "Guild" name for the steward service.
## Missed heartbeats are not failures (Gemini seat, board entry 96)

A single missed heartbeat from a Wi-Fi dropout or a cable swap must not revoke a lease or start an election. A configurable grace window (proposed: three
missed beats) with exponential-backoff probes comes first; only after it does a node become `unreachable`, and even then leadership changes only through the
fencing procedure in [11](11-hall-host-election-and-handoff.md). **Falsifying experiment (NOT RUN):** simulate a five-second partition between the two nodes and
confirm that no election or handoff starts.

## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| G0b | operator versus instance, register/heartbeat/lease, resource slots, `network.*` settings, encrypted path chosen | 01, 02, C112 |
| G7 | hosting for remote users, after a separate security review | owner decision |

## Decisions for this feature

1. First version is the owner's machines on the LAN, with guest profiles; remote strangers are deferred. DECIDED (supersedes loopback only).
2. Roster RESOLVED by the owner: `mimo` is the Windows OpenCode seat and `ubuntu-worker-01` the Ubuntu OpenCode seat; MiMo is the model identity behind OpenCode, which is the application; the local models are guild workers under them.
3. OPEN: which machine hosts the hall (recommended: Windows) and how the machines are joined (existing overlay network, certificate-pinned TLS, or SSH tunnels).
4. OPEN: which of Codex, Claude and Gemini run as local sessions on these machines and which as cloud sessions.
5. OPEN: first arrangement hall A (Windows) over devtools and hall B (Ubuntu) over Aaroneous, and whether hall B's operators merge in the Aaroneous repository (default: pull requests only).

## Review focus

- Is the operator-versus-instance split right, and is the roster in the table correct?
- Is pull-and-heartbeat with hall-clock leases a sound way to handle two machines?
- Are the wire requirements (named bind address, encrypted path, accepted Host list, per-instance hashed tokens) enough for a LAN?
- Does the domain and crossplay arrangement keep the dependency direction (devtools on Aaroneous, never the reverse) and the product's gate intact?

## Not verified

- The roster is the owner's statement of 2026-10-10; whether the seat names in `identities.json` match the running OpenCode sessions was not checked against the machines.
- That the machines share an encrypted overlay network; the `100.x` address is only suggestive.
- How the OpenCode sessions authenticate to GitHub and report today.
- That cloud sessions off the LAN can use the GitHub route as today; they cannot reach the hall.
