---
notation: v1
purpose: A standard native Rust application embedding the core, an operations panel over Guild, and reuse of other Rust tools behind workspace traits
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:01-settings-and-permissions
  - depends-on: doc:02-agent-door-and-mcp
  - overlaps: lane:C113
  - overlaps: lane:C118
checked-by: none yet (see "Not verified")
---

# Native application, operations panel and reuse (proposal 10)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 10 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## User interface technology

Owner direction, 2026-10-10: a standard application, not a webview. Proposed: a native Rust desktop application that embeds the core in-process (see 02), built after the core and the MCP adapter exist. The webview page remains a fallback adapter until the native app matches it. The product already uses `egui`/`eframe` in `studio_hud`, so that stack is the default candidate; the toolkit is chosen by a short spike whose first test is the packet editor (see 06), because long-text editing and accessibility are the weak points of most Rust GUI toolkits (general knowledge, not verified). The spike must show: the survey and packet editor usable, keyboard navigation, and responsiveness while the local model is running.

## Operations panel

A single page that lists the known operations with their current state and a run button each: Guild `tick`,
`verify-log`, `status`; schedule review; queue view with claim/expiry status; update check. Each runs the existing
command, streams its output, and retains the result. Destructive or externally visible actions (merge, push, post)
are **not** run from the HUD in the first phases; the HUD links to the PR instead.
## Reusing other Rust tools: how modular this is today, and where imports fit

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
## Wider spike criteria (Gemini seat, board entry 99)

Testing only the packet editor is too narrow. A toolkit passes the spike only if it can also: host a live event stream (server-sent events or WebSocket) for real-time
updates; render a scrollable log viewer for worker traces (1,000 lines is the test size); and handle system-tray integration on Windows without the tray errors the
current HUD already hit (HRESULT 2147500037 was reported in `main.rs`; I have not reproduced it). A toolkit that cannot do all of these fails the spike whatever the editor
result. **Falsifying experiment (NOT RUN):** a minimal app in the candidate toolkit that does the stream, the 1,000-line viewer and the tray icon together.

## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| P5 | operations panel over Guild, schedule review and the queue; results retained | C113, 02 |
| P7 (part) | native application shell with the 2D panels (survey, brief, packets, operations) | 01, 02, 06, toolkit spike |

## Decisions for this feature

1. UI technology: a standard native Rust application embedding the core in-process. DECIDED; toolkit chosen by a spike.

## Review focus

- Is the native-app-embeds-the-core design right, with the webview kept only as a fallback?
- Is the spike's pass condition (the packet editor first) the right test of a toolkit?
- Are the seams (model backend, context store, event source, operation, classifier) the right traits to carve first?
- Is the import policy (clean-room patterns over heavy runtimes; admission scoring for anything entering the product) respected?

## Not verified

- Crate names and properties are from general knowledge, not checked against current releases; no candidate has been scored.
- That any Rust GUI option can do the packet editor acceptably on the owner's machine; to be settled by the spike.
