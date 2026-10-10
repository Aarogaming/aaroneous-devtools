---
notation: v1
purpose: Proposal to grow the 127.0.0.1:8787 devtools HUD into the single operating surface that people and models both use
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
checked-by: none yet (see "Not verified")
---

# HUD v2: one local operating surface for people and models (proposal)

Contribution-ID: C119/claude-hud-v2-proposal-v1
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy. Nothing here is accepted until a
reviewer other than the author records it. Owner direction in chat, 2026-10-10: make the 127.0.0.1:8787 HUD better;
local-first AI assistance where software does the work and a local model is the glue; user inquiries become jobs;
user-assisted blueprint and research packets; run operations from the interface; every model pings the HUD for
cards, work and availability; fewer user interactions, more frequent work; use token-optimization methods.

## 1. What exists today (read from `origin/main`, 2026-10-10)

| Fact | Source |
|---|---|
| Server is `tiny_http` on 127.0.0.1:8787; every request must carry the exact `Host`, a per-session `X-Devtools-Token`, and a matching Origin; JSON bodies are capped at 1 KiB | `foundation/crates/maintenance_ui/src/main.rs` (`route`, lines ~312-330) |
| Routes: `GET /api/status`, `POST /api/result`, `/api/review`, `/api/packet`, `/api/mode`, `/api/shutdown`, `/api/check-update`; plus `/api/jobs`, `/api/tick`, `/api/verify`, `/api/cloud/send`, `/api/schedule` | `main.rs` lines ~351-442, ~888 |
| Packets: readiness, research, blueprint, refine-blueprint, build-job; Plan and Build modes | `packets.rs`, `lib.rs` |
| The local worker is polled every 120 s; it uses Ollama at 127.0.0.1:11434 with `qwen3.5:9b-q6` | `main.rs` line ~194, `lib.rs` line ~274 |
| UI is four embedded static files (about 220 lines of vanilla JS/HTML/CSS); the page is a webview | `src/app.js`, `client.mjs`, `index.html`, `style.css` |
| System tray icon and periodic git update check landed in lane C118 | `main.rs`, commit `c78cf64` |

Consequence: the HUD already has a hardened local surface and a packet pipeline, but it is **pull-only for humans**
(a person opens the page and clicks) and **closed to models** (they have no endpoint that tells them what to do or
lets them report in). The 120 s poll and the 1 KiB body cap make it unsuitable as a model-facing API as it stands.

## 2. Requirements (from the owner's message)

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

## 3. Design principles

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
5. **Silence on unchanged state.** Responses say "nothing new" in a few bytes (see 5.3). Quiet is the cheapest answer.

## 4. Proposed architecture

```
 people ──(browser/webview)──┐
                             ▼
 seats/models ──(bearer)──►  HUD server  127.0.0.1:8787  ──►  existing tools
 (cards, work, availability)  │   intake ▸ classify ▸ route     Guild CLI, steward, queue,
                              │   context packs + result cache   packets, local worker
                              ▼
                      local model (glue)  ──► cloud model (only when needed)
```

### 4.1 Two audiences, two doors on the same port

- **Human door** (today's): token in the page, Origin check, CSP. Keep unchanged.
- **Agent door** (new): `Authorization: Bearer <seat token>` on `/agent/v1/*`. Tokens are issued per seat by the HUD
  operator into `~/.steward/seats/<seat>/hud-token` (same folder convention as the seat Apps, never in the repo),
  stored hashed by the HUD, revocable, scoped to a seat name from `governance/identities.json`. The agent door never
  accepts the human session token and the human door never accepts a seat token. The Host-header check stays for both,
  which keeps the DNS-rebinding defence.

### 4.1a One core, several transports: MCP for models, a native app for people

Owner direction, 2026-10-10: use the HUD as an MCP (or LSP) server, and prefer a standard application to a webview.

**What exists (read from `origin/main`, 2026-10-10).**
- Devtools already has a loopback MCP endpoint: `foundation_cli/src/agentic_registrar.rs` serves `POST /mcp`
  (JSON-RPC 2.0, protocol revision 2024-11-05, service name `aaroneous-mcp-foundation`) with an `X-Devtools-Token`
  header, a 64 KiB body cap, and five tools: `verify_gate_suite`, `get_repository_status`, `register_vibe_task`,
  `dispatch_queue_tick`, `audit_ast_invariants`. It runs as a separate server from the HUD on 8787.
- The product repo has a `mcp_server` crate with `mcp_service` (auth, capability, http_api, transport) and a
  capability broker, built on tokio/tower. I have only listed its files, not read its design.
- I found **no LSP** anywhere in devtools. LSP is for editor language intelligence (diagnostics, completion for source
  files); it does not fit cards, work and availability. MCP does, and it is what the cloud and local agent tools already
  speak.

**Proposal.** The HUD core (settings, permissions, store, router, accounting) is a library with **no transport and no
UI**. Everything else is an adapter over the same operations and the same permission engine:

| Adapter | Audience | Notes |
|---|---|---|
| MCP server (stdio for tools launched by a seat; HTTP on loopback with a bearer token for long-running ones) | models and seats | Tools: `get_card`, `list_work`, `get_availability`, `set_availability`, `submit_inquiry`, and later `report_result`. Resources: the card, the work list, the events feed. Each tool is one row of the section 4.2 table |
| Plain loopback HTTP | scripts, the steward, health checks | Same operations; this is the section 4.2 API |
| Native desktop application | people | Calls the core **in-process**, so the human path needs no port, no session token and no Host/Origin defences |

Consequences: one permission engine and one audit log for every path; the existing registrar MCP tools become
operations registered in the core rather than a second server; and the 8787 webview page becomes an optional
adapter, not the product. Which MCP protocol revision and transport the seats' client tools actually speak, and
whether the existing endpoint is reachable from them today, is NOT VERIFIED; P1 must test it with a real client.

Boundary: devtools may depend on Aaroneous crates; Aaroneous never depends on devtools. Reusing the product
`mcp_server` crate would be allowed in that direction, but it brings tokio/tower into a tool that is synchronous
today, so it needs the async-justified case from `OPERATING_MODEL.md` and is not assumed here.

### 4.2 Agent API (read-first)

| Endpoint | Purpose |
|---|---|
| `GET /agent/v1/card` (MCP `get_card`) | The seat's current task card (what `steward_run.py` writes today), with `ETag`; `304` with no body when unchanged |
| `GET /agent/v1/work` | Open lanes/jobs this seat may take, filtered by availability and capability, with `ETag` |
| `PUT /agent/v1/availability` | Seat reports `{state, model, limits, until}`; this is the C112 seat-status record served over HTTP |
| `GET /agent/v1/availability` | Everyone's availability, so a seat can see who is free |
| `GET /agent/v1/events?since=N` | Long-poll (and later SSE) of an ordered event stream with sequence numbers; replaces fixed-interval polling |
| `POST /agent/v1/result` | A seat reports a result against a card/job; stored as retained evidence; does **not** complete the card (completion stays a PR with a receipt) |

Phase 1 ships the three GETs and the availability PUT only. Writes of results come after the owner decision in section 9.

### 4.2a Configuration and permissions (R8)

Nothing below is a constant in code except the **built-in defaults**, which live in one embedded `defaults.toml`
(readable, testable, and the only place a default is written down). Everything else is read from settings.

**Layers, lowest to highest precedence:** built-in `defaults.toml`, then user settings
(`~/.steward/hud/settings.toml`), then project settings (`<repo>/.devtools/hud.toml`, committed, shared), then local
overrides (`<repo>/.devtools/hud.local.toml`, uncommitted). A setting can be marked `locked = true` at a lower layer
(for example by the owner in user settings) so a higher layer cannot loosen it. The HUD shows, for every effective
value, which layer set it, and validates the files on load: a bad file keeps the previous good configuration and
reports the error rather than falling back to something permissive. Changes are picked up without restart.

**Permission model, patterned on the permission settings of the agent tools these seats already run under** (this is
the same shape, not shared code): a *mode*, plus explicit *allow*, *ask* and *deny* rule lists, where deny beats ask
beats allow.

| Mode | Meaning |
|---|---|
| `read-only` (**the default**) | reads are allowed with no prompt; anything that changes state is refused unless a rule below allows or asks for it |
| `ask` | reads allowed; each state-changing action waits for an explicit approval in the HUD |
| `auto` | pre-approved operations run without asking; everything else still asks. Not available until the owner enables it in user settings |

Rules name an actor (a seat from `governance/identities.json`, `human`, or `*`), an action and an optional scope, for
example `allow seat:* GET card`, `allow seat:* PUT availability`, `ask seat:* POST result`, `deny * merge`. Reads of
a seat's own card and the work list are allowed by default; availability is the one write a seat may do unasked, because
it is status, not state of record, and the owner can turn it off. Results, intake, operations that run commands, and
anything that touches git or GitHub start as `ask` or `deny`. Merge, push, review and attestation are `deny` and are
not configurable from the HUD at all: they stay with the steward gate and signed attestations (principle 4).

**What is configurable, with the proposed initial value:**

| Group | Settings (examples) | Initial default |
|---|---|---|
| permissions | mode, allow/ask/deny rules, per-seat overrides, locked keys | `read-only`, rules as above |
| agent door | enabled, token lifetime, per-seat rate limit, body cap | enabled, 1 day, 60/min, 64 KiB |
| routing | tier order (software, local, cloud), per-class tier ceiling, confidence thresholds | software then local; cloud off |
| models | named backends (endpoint, model, kind, context size, timeout); which backend serves which role | the Ollama endpoint the HUD already uses |
| budgets | token budget per seat per day, per job, per tier; what happens on exhaustion (queue, ask, refuse) | accounting on, no hard cap |
| cache | result-cache on/off, size, TTL, pack retention | on, bounded |
| events | long-poll timeout, sequence retention, heartbeat | conservative |
| operations | which operations appear, which need approval | list only; none run unasked |
| ui | theme, panels shown, notification level | quiet |

Because behaviour is data, the same HUD can be run stricter or looser per machine, and a seat that misbehaves can be
narrowed by one line in settings. The settings schema is a versioned contract with tests (every default parses; deny
overrides allow; a locked key cannot be overridden; an invalid file does not weaken the active configuration).

### 4.3 Inquiry to job to answer

1. A user types a question or request in the HUD (or a seat posts one). The HUD turns it into a **typed intake record**
   following `contracts/job-intake-v1.md` (lane C106), so one record format serves humans, seats and the CLI.
2. **Software classifier** (no model): keyword/structure rules and queue lookups decide the class: *answerable from
   retained evidence*, *operation to run*, *research*, *blueprint*, or *needs a person/cloud model*.
3. **Local model as glue**: for the classes that need language, it receives a context pack (see 5.2) and does one bounded
   job: summarise, draft the research packet, propose blueprint sections, or choose among the already-listed
   operations. Its output is a structured proposal, never a direct action.
4. The HUD shows the proposal; the user approves, edits or rejects. Approved operations run through the existing
   tools; results are retained. Anything beyond the local model's reach is queued as a job for a cloud seat with the
   pack attached.

### 4.3a Brief compiler: from a conversation to one background prompt (R9)

The pipeline is built so that **the light model never writes the final prompt**. It fills slots in a pre-built brief;
software assembles the prompt.

1. **Brief template** (configuration, R8): a typed schema chosen by the inquiry class, with named slots such as
   `who` (requester, audience, executing seat/tier), `what` (task, scope, out of scope), `why` (goal, decision it
   serves), `when` (deadline, urgency, schedule), `how` (method, constraints, tools allowed), `inputs` (evidence
   references by digest), `acceptance` (what done looks like, how it is checked), `budget` (tokens, time, tier ceiling)
   and `risk` (what must not happen). Each template marks which slots are required. This extends the existing packet
   types and the C106 intake record; it does not create a third format.
2. **Survey and conversation**: the light model asks one targeted question at a time, for the empty required slots
   only, offers choices drawn from the retained evidence and the queue where it can, and writes its answer into a slot
   as structured output validated against the schema.
3. **Slot states**: `empty`, `derived` (filled by software from the queue or state), `proposed` (filled by the model),
   `confirmed` (accepted or edited by the user), `unverified` (a claim with no evidence). Only `confirmed` and
   `derived` slots reach the compiled prompt unmarked; `proposed` ones are shown for approval, never silently used.
4. **Completeness gate**: a brief cannot be dispatched while a required slot is empty or merely proposed. The user is
   asked only for those, which is where the interaction saving comes from.
5. **Compile**: a deterministic renderer turns the confirmed slots plus a context pack into the final prompt. The same
   brief always compiles to the same prompt, so it is testable, cacheable and cheap to diff.
6. **Dispatch**: the compiled brief becomes a job for the background pipeline with its routing, budget and permission
   level taken from the slots and the settings. The whole conversation is retained as evidence, but **only the
   compiled prompt goes to the larger model**, not the transcript: that is the token saving, and it makes the large
   model's input small, structured and reviewable. Results return as a job result and are presented for review.

### 4.3b The 3D visualizer for analogy references (R10)

**My reading of the request, to be confirmed:** the architecture and proposals already lean on analogies (the spiral
staircase and stepped double helix, the PLC scan cycle, the biology-flavoured legacy names, patterns borrowed from
other runtimes). The visualizer is a map of those references: *concepts*, *documents*, *lanes*, *components* and the
*analogies* that link a source idea to a target one, with the mapping, how strong it is, and what evidence supports it.

- **Data model (a view over data we already keep, not a new store):** nodes are typed (concept, document, lane,
  component, analogy); edges are typed relations, reusing the relation vocabulary of the C101 notation proposal
  (`informs`, `depends-on`, `overlaps`) plus `analogous-to` with a mapping list. Every edge carries provenance and a
  status (`unverified` until a reviewer other than its author confirms it), so a picture can never present a
  model-suggested analogy as established.
- **Who makes the analogies:** software extracts the explicit relations already in front matter and the queue; the light
  model may propose `analogous-to` edges; a person confirms. Layout is computed by software with a fixed seed, so the
  same graph draws the same way and screenshots are comparable.
- **What 3D is for, honestly:** 3D earns its place where the metaphor is spatial (layers, helix, scan cycle, depth for
  abstraction level) and where seeing density and neighbourhoods helps. It does not help reading text, editing a brief or
  approving an action, and dense 3D graphs have occlusion and navigation costs. So 3D is **one view in the application,
  next to 2D panels** for the survey, brief, packets and operations, and the same graph must be readable as a flat list
  and a 2D map (also for accessibility).
- **Cost:** a GPU renderer competes with the local model for the same graphics card and memory. The renderer's frame
  rate, resolution and effects are settings (R8), it drops to the 2D view when the model is running or the machine is
  under load, and it can be off entirely. The core and MCP adapter never depend on it.
- **Toolkit:** a native Rust 3D renderer inside the same application as the 2D panels. A game-engine crate with an
  immediate-mode GUI layer is the likeliest fit, and a lighter 3D crate is the fallback; both are heavy dependencies with
  long build times (general knowledge, NOT VERIFIED), so they are chosen by a spike and scored under the devtools
  criteria in `OPERATING_MODEL.md` before adoption. A spike must show: the packet editor and survey usable, a 2,000-node
  graph at an acceptable frame rate while the local model is running, and keyboard navigation.

### 4.4 Blueprint and research packets with the user in the loop

Today the packets are generated and shown. Proposed: each packet is a **document with revisions** in the HUD: the model
drafts, the user edits fields in place or comments, the model revises only the affected sections (using the
existing `refine-blueprint`), and each accepted revision is retained with its source evidence digests. "Approve"
produces the existing build-job packet. This keeps the packet logic in `packets.rs` and adds an editing surface and
a revision history, nothing more.

### 4.5 Operations panel

A single page that lists the known operations with their current state and a run button each: Guild `tick`,
`verify-log`, `status`; schedule review; queue view with claim/expiry status; update check. Each runs the existing
command, streams its output, and retains the result. Destructive or externally visible actions (merge, push, post)
are **not** run from the HUD in the first phases; the HUD links to the PR instead.

## 5. Token optimization (ranked by expected saving, all measurable)

1. **Tiered routing (biggest).** Software, then local model, then cloud, as in 4.3. Expected effect: most routine
   status/queue/"what's next" questions never reach a cloud model.
2. **Context packs, content-addressed.** The collector builds a small, structured digest of the relevant state
   (open lanes, the seat's card, last N events) and names it by hash. A model is given the pack plus the question,
   not the files. Identical pack + identical template = identical prompt prefix, which also helps provider-side
   prompt caching.
3. **Conditional responses.** `ETag`/`304` on cards and work lists: an unchanged card costs a few bytes and zero model
   tokens. This also gives "stay quiet on unchanged healthy state" a mechanical form.
4. **Result cache** keyed by (template version, input digests). Re-asking the same question over unchanged evidence
   returns the stored answer.
5. **Delta cards.** After the first full card, a seat receives only what changed since its last `ETag`.
6. **Event stream with sequence numbers** instead of repeated polling; a seat sleeps until something relevant changes.
7. **Structured outputs and short templates** for the local model; stable prefixes; cap output length per class.
8. **Accounting.** Every model call records input/output token counts, the tier that answered, and cache hit/miss in the
   retained log. The HUD shows tokens saved by tier and cache hits. Without this, none of the above is verifiable.

Savings figures are not claimed in this document; they are to be measured by item 8.

## 6. Security and safety

- Binding stays on 127.0.0.1. Remote access (to reach the HUD from other machines or the seats on other hosts) is a
  separate owner decision (section 9) and, if chosen, goes through an authenticated tunnel rather than binding
  0.0.0.0.
- Larger request bodies (intake needs more than 1 KiB) are allowed only on the agent door and the intake route, with a
  new explicit cap (proposed 64 KiB) and a size/rate limit per seat.
- Seat tokens are separate from GitHub App credentials and from signing keys; leaking one never gives merge or review
  rights, only HUD access for that seat.
- Model output is untrusted data: it can propose, never execute. Any operation a model suggests is shown to the user
  (or matched against a pre-approved operation list) before it runs.
- All inbound agent text is treated as data when building context packs (no instruction following from retained results).
- No new third-party dependency without the admission scoring in the product `AGENTS.md`. Long-poll and SSE are
  achievable on the current `tiny_http`; a different server library would need admission scoring.

## 7. Phased plan

| Phase | Deliverable | Depends on | Exit evidence |
|---|---|---|---|
| P0 | This proposal reviewed and the section 9 decisions made | owner | recorded decisions |
| P1 | Settings layers and permission engine (`defaults.toml`, schema, precedence, locks, effective-value view); then the agent door (bearer tokens) with `GET card`, `GET work`, `GET/PUT availability`, `ETag/304`; token issue/revoke command; tests | C112 seat-status format, C106 for work shape | schema and precedence tests; a seat fetches its card over HTTP under `read-only`; a write is refused until a rule allows it; 304 on unchanged |
| P1b | MCP adapter over the same operations (stdio and loopback HTTP), tested with a real MCP client; the registrar's five tools registered as core operations | P1 | a seat calls `get_card` through MCP under `read-only` |
| P2 | Event stream (long-poll with sequence numbers); retire the fixed 120 s poll for seats | P1 | a seat wakes within a second of a change |
| P3 | Inquiry intake in the HUD to typed jobs (C106) with the software classifier and answer-from-evidence class | C106 | an inquiry answered from retained evidence with zero model calls |
| P3b | Brief compiler: brief templates as configuration, slot states, completeness gate, deterministic compile, dispatch as a background job; survey driven by the light model | P3, C106 | a brief filled by the survey compiles to the same prompt twice; a missing required slot blocks dispatch |
| P4 | Local-model glue with context packs, result cache and token accounting; user-assisted packet editing with revisions | P3 | accounting shows tier and cache; a packet edited and revised in the HUD |
| P5 | Operations panel over Guild/schedule/queue; results retained | C113 | each operation runs from the HUD with retained output |
| P7 | Native application shell with the 2D panels (survey, brief, packets, operations), then the 3D analogy view over the graph data model | P1, P3b, toolkit spike | spike criteria in 4.3b met, or the 2D view ships alone |
| P6 | Optional: cloud-seat job dispatch with attached packs; remote access | owner decision | per decision |

Each phase is a separate lane/PR; this document claims no code.

## 8. Overlap with existing lanes (not duplicated, reused)

- **C106** typed job intake: the intake format. HUD v2 consumes it and must not invent a second one.
- **C112** seat status/host availability: the availability record. HUD v2 serves it over HTTP (its Phase 3 mirror was
  pending an owner decision on the transport; this proposal answers it with "the HUD's agent door").
- **C113** Guild CLI: the operations behind the panel. HUD v2 calls it; it does not reimplement the scheduler.
- **C118** tray + update check also edits `maintenance_ui`: P1 will rebase on it; no file-level conflict is expected
  but sequencing is needed.
- **C102 / PR #135** supported operations server: HUD v2 is the local face of that proposal; whether they become one
  service or two is for the reviewers of #135 and this proposal.

## 9. Owner decisions (recorded from chat, 2026-10-10)

The owner confirmed the reading of "analogy reference" in 4.3b and said to go ahead with the recommended defaults.
Recorded here so reviewers see what is decided and what stays open. This records a chat instruction; it is not an
attestation, and a reviewer other than the author still has to accept this document.

1. **Agent write access.** DECIDED: default `read-only`, modelled on the permission modes of the agent tools in use;
   everything is configuration (4.2a). Default for the availability write: allowed, switchable off in settings.
2. **UI technology.** DECIDED: a standard native Rust application embedding the HUD core in-process (4.1a), not a
   webview. The webview page remains a fallback adapter until the native app matches it. The toolkit is chosen by a
   spike whose first test is the packet editor (4.4).
3. **Glue model.** DECIDED: the existing Ollama `qwen3.5:9b-q6` endpoint is the initial value of a `models` setting,
   not a constant. The product `AGENTS.md` names vLLM Qwen2.5-Coder-14B at :8000, which was not reachable when checked.
4. **Remote reach.** DECIDED: loopback only. Reaching the HUD from other machines is a separate later decision.
5. **Lane ownership.** DECIDED: this lane (C119) is the design; code lands in separate lanes, starting with C120 (the
   settings and permission engine). C106, C112 and C113 keep the formats and tools the HUD consumes.
6. **3D scope.** DECIDED: 3D is the analogy-visualizer view inside a native app whose working screens are 2D; it starts
   after the brief compiler (P3b).
7. **Analogy reference.** CONFIRMED: the reading in 4.3b.
8. **First brief templates.** DEFAULT, not separately answered: research, blueprint and build first (the existing packet
   types), review and incident later. Mandatory slots are set per template in configuration.

Still open: the exact slot lists per template, the toolkit spike result, and the MCP client compatibility test (P1b).

## 10. Reusing other Rust tools: how modular this is today, and where imports fit

Owner question, 2026-10-10: can features be imported from other Rust tools and applications, and how is it set up now?

**Current layout (read from `origin/main`).** `foundation/` is a workspace of three crates: `steward` (pure reducers:
cards, chain, coverage, gate, attribution; no I/O), `foundation_cli` (about twenty modules in one crate: Guild,
scheduler, local worker, intake, maintenance pipeline), and `maintenance_ui` (the HUD: one binary crate with a
`lib.rs` of 651 lines and a `main.rs` of 918). `steward` is a genuine component with a contract (golden vectors).
The other two are modular by file, not by crate: there is no trait boundary between "HUD routes", "model backend"
and "operations", so importing a feature today means editing a large file.

**Proposal: make the seams first, then import behind them.** Under P1 and P3, carve small crates with one trait
each, following the graduation rule (new component behind a workspace trait with contract tests, landed inert or in
shadow mode):

| Seam (trait) | Today | What could sit behind it |
|---|---|---|
| `ModelBackend` (complete, usage) | direct `reqwest` calls to Ollama in `lib.rs` | Ollama, vLLM/OpenAI-compatible, a cloud seat; token usage returned by the provider |
| `ContextStore` (pack by digest, result cache) | none | on-disk content-addressed files using `sha2`, already a dependency |
| `EventSource` (ordered events with sequence numbers) | 120 s poll | file-change triggers, queue/state diffs |
| `Operation` (name, dry description, run, retained output) | ad hoc routes | Guild commands, schedule review, update check |
| `Classifier` (software routing rules) | none | rule table first, a model only as fallback |

**Import candidates, and the honest status.** Dependency evaluation for devtools follows
`governance/OPERATING_MODEL.md` (need, licence, platform support, maintenance cost, async only when justified), and
`PATTERNS_FROM_AGENT_RUNTIMES.md` already recommends clean-rooming patterns over admitting heavy runtimes. I have
not evaluated any crate below against those criteria; the list says where to look, not what to adopt.

| Need | Lean option | Alternative to evaluate | Note |
|---|---|---|---|
| Model client | keep `reqwest` behind `ModelBackend` | a Rust LLM client library | mine its provider-trait pattern; do not take the framework |
| Token counting | provider-reported usage fields, plus a byte-length estimate | a tokenizer crate | a tokenizer is large and model-specific; only if estimates prove too coarse |
| Event triggers | polling the state files the steward already writes | a file-watching crate | small, but cross-platform behaviour (Windows) must be tested |
| Search over retained evidence | plain scan of retained records first | an embedded search index crate | add only when the scan is measured to be too slow |
| HTTP/SSE | stay on `tiny_http` | an async web framework | brings an async runtime; needs the "async justified" case |
| Coordination ideas | clean-room (Foremerge, ZeroClaw patterns, already mined) | none | already decided in the patterns document |

If any of this ever crosses into the Aaroneous product repo, `AGENTS.md` section 4 admission scoring applies, not the
devtools criteria. Aaroneous never depends on devtools or on external tooling.

## 11. Not verified

- No code was written or run for this proposal. All numbers in section 5 are targets, not measurements.
- That `tiny_http` can serve long-poll/SSE without blocking other requests (it handles requests on its own threads,
  but I have not tested it).
- That the Ollama model, the HUD, and the worker can sustain the added load of seat pings on the owner's machine.
- That the C106 intake contract, as written, fits the inquiry and seat-ping cases (I read it; I have not exercised it).
- That an agent running in a sandbox or cloud session can reach 127.0.0.1:8787 at all; cloud seats likely cannot, which
  is why P6 and decision 4 exist.
- The state of the C112 availability format at merge time; P1 must re-read it.
- That the settings layering and permission rules in 4.2a express every case the seats need; the permission shape is
  modelled on the agent tools' settings from memory of how they behave, not from their source.
- Crate names and properties in section 10 are from general knowledge, not checked against current releases; no candidate has been scored.
- That any Rust GUI option can do the packet editor (4.4) acceptably on the owner's machine; to be settled by the spike in decision 2.
- That a 3D renderer, the local model and the 2D panels can share the owner's GPU acceptably; to be measured in the toolkit spike.
- That a light local model fills brief slots reliably enough as structured output; to be measured against the slot schema before the survey is trusted.
