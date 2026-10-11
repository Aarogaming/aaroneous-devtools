---
notation: v1
purpose: A transport-free HUD core with an MCP adapter and a read-first agent API so models ping the hall for cards, work and availability
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:01-settings-and-permissions
  - overlaps: lane:C106
  - overlaps: lane:C112
  - overlaps: lane:C113
checked-by: none yet (see "Not verified")
---

# Agent door and MCP adapter (proposal 02)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 02 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## What exists today (read from `origin/main`, 2026-10-10)

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

## Two audiences, two doors on the same port

- **Human door** (today's): token in the page, Origin check, CSP. Keep unchanged.
- **Agent door** (new): `Authorization: Bearer <seat token>` on `/agent/v1/*`. Tokens are issued per seat by the HUD
  operator into `~/.steward/seats/<seat>/hud-token` (same folder convention as the seat Apps, never in the repo),
  stored hashed by the HUD, revocable, scoped to a seat name from `governance/identities.json`. The agent door never
  accepts the human session token and the human door never accepts a seat token. The Host-header check stays for both,
  which keeps the DNS-rebinding defence.
## One core, several transports: MCP for models, a native app for people

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
| MCP server (stdio for tools launched by a seat; HTTP on loopback with a bearer token for long-running ones) | models and seats | Tools: `get_card`, `list_work`, `get_availability`, `set_availability`, `submit_inquiry`, and later `report_result`. Resources: the card, the work list, the events feed. Each tool is one row of the agent API table below |
| Plain loopback HTTP | scripts, the steward, health checks | Same operations; this is the agent API below |
| Native desktop application | people | Calls the core **in-process**, so the human path needs no port, no session token and no Host/Origin defences |

Consequences: one permission engine and one audit log for every path; the existing registrar MCP tools become
operations registered in the core rather than a second server; and the 8787 webview page becomes an optional
adapter, not the product. Which MCP protocol revision and transport the seats' client tools actually speak, and
whether the existing endpoint is reachable from them today, is NOT VERIFIED; P1 must test it with a real client.

Boundary: devtools may depend on Aaroneous crates; Aaroneous never depends on devtools. Reusing the product
`mcp_server` crate would be allowed in that direction, but it brings tokio/tower into a tool that is synchronous
today, so it needs the async-justified case from `OPERATING_MODEL.md` and is not assumed here.
## Agent API (read-first)

| Endpoint | Purpose |
|---|---|
| `GET /agent/v1/card` (MCP `get_card`) | The seat's current task card (what `steward_run.py` writes today), with `ETag`; `304` with no body when unchanged |
| `GET /agent/v1/work` | Open lanes/jobs this seat may take, filtered by availability and capability, with `ETag` |
| `PUT /agent/v1/availability` | Seat reports `{state, model, limits, until}`; this is the C112 seat-status record served over HTTP |
| `GET /agent/v1/availability` | Everyone's availability, so a seat can see who is free |
| `GET /agent/v1/events?since=N` | Long-poll (and later SSE) of an ordered event stream with sequence numbers; replaces fixed-interval polling |
| `POST /agent/v1/result` | A seat reports a result against a card/job; stored as retained evidence; does **not** complete the card (completion stays a PR with a receipt) |

### Requirements added after review (Codex seat, 2026-10-11)

- **One typed, authenticated principal across every adapter.** The adapter authenticates the caller and attaches the principal (seat or
  human, token id, policy revision); a client-supplied seat or actor field **never** chooses identity.
- **Capability parity for the existing registrar tools.** The five registrar tools (`verify_gate_suite`, `get_repository_status`,
  `register_vibe_task`, `dispatch_queue_tick`, `audit_ast_invariants`) become registered actions in the core. The old handlers are removed or
  wrapped so a legacy tool cannot reach its effect except through the permission engine and the floor. A test lists every registrar tool name and
  proves each has an action entry and is unreachable without going through the core.
- **Concurrency.** The current HUD server takes one request at a time and routes it synchronously (`recv_timeout`, then `route`, in
  `foundation/crates/maintenance_ui/src/main.rs`), so a blocking long poll would stall availability writes, shutdown and other seats' card
  fetches. Required before any event transport: a bounded number of concurrent waiters, a per-seat connection cap, cancellation on client
  disconnect and on token revocation, and short requests handled independently of waiters.
- **Event cursors expire.** A cursor older than the retention window returns a resync instruction, not silence or a gap.
- **Authorization-aware cache validators.** An `ETag` or `304` is bound to the authenticated seat, the resource and the policy (authorization)
  revision; a cached `304` cannot survive a permission change or token revocation, and one seat's validator never returns another seat's card.
- **Falsifying experiment (credit: Codex seat, NOT RUN):** fill the configured waiter limit with silent clients, revoke one token, then fetch a
  different seat's card and submit its allowed availability update; require bounded latency, released waiter capacity, and no revoked or
  cross-seat cache hit, over both HTTP and MCP.

Phase 1 ships the three GETs and the availability PUT only. Writes of results come after the owner decision recorded in the [README](README.md).
## Security and safety

- Binding defaults to 127.0.0.1. LAN use (decided 2026-10-10, the LAN decision, see [03](03-lan-roster-and-domains.md)) is an explicit `network.bind`
  setting naming an interface, never `0.0.0.0` by default, with an encrypted path required for tokens
  ([the LAN proposal](03-lan-roster-and-domains.md)) and a configured list of accepted `Host` names.
- Larger request bodies (intake needs more than 1 KiB) are allowed only on the agent door and the intake route, with a
  new explicit cap (proposed 64 KiB) and a size/rate limit per seat.
- Seat tokens are separate from GitHub App credentials and from signing keys; leaking one never gives merge or review
  rights, only HUD access for that seat.
- Model output is untrusted data: it can propose, never execute. Any operation a model suggests is shown to the user
  (or matched against a pre-approved operation list) before it runs.
- All inbound agent text is treated as data when building context packs (no instruction following from retained results).
- No new third-party dependency without the admission scoring in the product `AGENTS.md`. Long-poll and SSE are
  achievable on the current `tiny_http`; a different server library would need admission scoring.
## Tool manifests are generated, not hand-written (Gemini seat, board entry 95)

The MCP tool definitions are generated at build time from the core's registered operations and their permission entries (a build step or macro), so a schema and
the code behind it cannot drift apart and a renamed field stops the build until the manifest agrees. This needs no model. **Falsifying experiment (NOT RUN):**
rename a field in a core operation and confirm the manifest build fails until it is updated.

## Phases for this feature

| Phase | Deliverable | Depends on | Exit evidence |
|---|---|---|---|
| P1 | agent door with bearer tokens: `GET card`, `GET work`, `GET/PUT availability`, `ETag/304`; token issue and revoke command | 01, C112 format, C106 shape | a seat fetches its card over HTTP under `read-only`; a write is refused until a rule allows it; 304 on unchanged |
| P1b | MCP adapter over the same operations (stdio and loopback HTTP) tested with a real MCP client; the registrar's five tools registered as core operations | P1 | a seat calls `get_card` through MCP under `read-only` |
| P2 | event stream (long-poll with sequence numbers); retire the fixed 120 s poll for seats | P1 | a seat wakes within a second of a change |

## Decisions for this feature

1. Agent write access: default read-only (see 01). Results by pull request only until the model-glue phase.
2. Remote reach is superseded by the LAN decision; see [03](03-lan-roster-and-domains.md).
3. Which MCP protocol revision and transport the seats' clients speak is decided by testing in P1b.

## Review focus

- Is one core with several adapters (MCP, loopback HTTP, in-process) the right split?
- Is the read-first agent API (card, work, availability, events, result) the right set, with `ETag`/304 and sequence-numbered events?
- Are the security properties (separate human and agent doors, hashed per-seat tokens, body caps, data-not-instructions) sufficient?
- Does folding the registrar's five MCP tools into core operations beat running two servers?

## Not verified

- Whether the server library can serve long-poll and SSE without blocking other requests. My earlier statement that it handles requests on its own threads was wrong for this application, whose loop is single-threaded (corrected after the Codex seat's review); a concurrency design and a load test are required.
- That the Ollama model, the HUD and the worker can sustain the added load of seat pings on the owner's machine.
- That an agent in a sandbox or cloud session can reach the hall at all; cloud seats probably cannot (see 03).
- The state of the C112 availability format at merge time; P1 must re-read it.
- That a pull-and-heartbeat design matches how each tool (OpenCode, Codex, Claude, Gemini) can actually call the hall; MCP support in each client must be tested in P1b.
- Whether the existing registrar MCP endpoint is reachable from the seats' client tools today.
