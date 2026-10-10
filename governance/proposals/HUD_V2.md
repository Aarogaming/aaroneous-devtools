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

### 4.2 Agent API (read-first)

| Endpoint | Purpose |
|---|---|
| `GET /agent/v1/card` | The seat's current task card (what `steward_run.py` writes today), with `ETag`; `304` with no body when unchanged |
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
| P2 | Event stream (long-poll with sequence numbers); retire the fixed 120 s poll for seats | P1 | a seat wakes within a second of a change |
| P3 | Inquiry intake in the HUD to typed jobs (C106) with the software classifier and answer-from-evidence class | C106 | an inquiry answered from retained evidence with zero model calls |
| P4 | Local-model glue with context packs, result cache and token accounting; user-assisted packet editing with revisions | P3 | accounting shows tier and cache; a packet edited and revised in the HUD |
| P5 | Operations panel over Guild/schedule/queue; results retained | C113 | each operation runs from the HUD with retained output |
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

## 9. Decisions needed from the owner (recommended defaults first)

1. **Agent write access.** DECIDED in chat 2026-10-10: default is read-only, modelled on the permission modes of the
   agent tools in use, and everything is configuration (section 4.2a). Remaining detail for review: whether the
   availability PUT is allowed by default (proposed yes) or also starts as `ask`.
2. **UI technology.** Recommended: keep vanilla JS in the webview and add SSE/long-poll; revisit a native UI after P5.
3. **Glue model.** Recommended: keep the Ollama `qwen3.5:9b-q6` endpoint the HUD already uses as the initial value of a
   `models` setting, not a constant;
   the product `AGENTS.md` names vLLM Qwen2.5-Coder-14B at :8000, which was not reachable when checked.
4. **Remote reach** (seats on other machines): recommended: a tailnet/SSH tunnel to the loopback port, decided
   separately; the loopback-only default stays until then.
5. **Lane ownership.** Recommended: this proposal's lane (C119) owns the agent door and the HUD operating layer;
   C106, C112, C113 keep the formats and tools it consumes.

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
