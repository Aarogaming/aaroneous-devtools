# Foundry gap register

This register converts verified boundary and governance gaps into maintained
work. It is the perception layer: design and implementation follow only after
the evidence, affected contract and queue disposition are visible. It is not a
second queue; `COORDINATION_QUEUE.md` remains the sole claim authority.

## Entry rules

1. Cite current source, an observed command result, or a committed review.
2. State the affected boundary and the risk of leaving it unchanged.
3. Link one corrective queue lane, a blocked owner decision, or an explicit
   rejection. A finding without a disposition is incomplete.
4. Update the entry when its lane is accepted, rejected, superseded, or finds
   that the premise was false. Do not silently delete historical evidence.

## G-2026-01 — broad API lacks a pinned capability-and-gate contract

- Boundary: Aaroneous product capability discovery to devtools Foundry.
- Evidence: `xtask/src/workspace_metadata.rs:217-251` discovers its root from
  `std::env::current_dir()` and emits metadata JSON, but has no supplied Git
  revision/fingerprint, gate selection contract, command-result schema, or
  declared artifact set.
- Risk: a controller can label a checkout without proving which revision and
  native verification contract it actually operated against.
- Disposition: **C79 Ready** — define and implement an explicit product
  capability-manifest export; C82 consumes it in devtools.

## G-2026-02 — MCP dispatch is not a safe broad external authority

- Boundary: Aaroneous MCP capability service.
- Evidence: `crates/mcp_server/src/mcp_service/service.rs:124-148` documents a
  self-issued dispatch token and mints `"*"` with `FullPrivilege`; its comment
  explicitly says it is not per-caller authorization. `tools/call` uses that
  token at lines 214-218.
- Risk: exposing this surface as the Foundry's general API would grant each
  caller the service's full authority and cannot distinguish a reviewer from
  an implementer.
- Disposition: **C80 Ready** — add caller-bound, least-privilege capability
  authorization with an explicit read-only/propose/verify/mutate boundary.

## G-2026-03 — model and path inputs still have ambient fallback paths

- Boundary: product LLM and path adapters used by any external controller.
- Evidence: `crates/llm_gateway/src/lib.rs:102-115` defaults local endpoint,
  model and GGUF path; `crates/paths/src/lib.rs:114-134` falls back to
  `current_exe` and OS data directories despite `WorkspacePathsConfig` having
  explicit fields.
- Risk: identical Foundry requests can select different local resources on two
  hosts and produce untraceable model or path provenance.
- Disposition: **C81 Ready** — provide strict explicit-input constructors and
  reject implicit fallback in externally driven execution.

## G-2026-04 — no adapter binds product facts to the development ledger

- Boundary: Aaroneous API output to devtools control transformer.
- Evidence: C78 component review established that product owns gates and
  runtime contracts while devtools owns leases, virtual patches, PR readiness,
  reviewer separation and evidence. No versioned contract currently connects
  those two state domains.
- Risk: devtools can recreate a product registry or infer gate semantics from
  prose rather than consuming a revision-bound product record.
- Disposition: **C82 Ready** — specify the Foundry adapter-manifest contract
  and its read-only status surface, consuming C79's export.

## G-2026-05 — gap tracking is documented but not mechanically checked

- Boundary: governance freshness.
- Evidence: the companion operating model previously required reviewed
  findings to enter an inbox, but did not require a verified architectural or
  service gap to retain a corrective disposition.
- Risk: reviews discover defects that remain chat-only and drift away from the
  queue before design or implementation begins.
- Disposition: **C83 Ready** — add a deterministic register-to-queue policy
  checker and fixtures; no agent may self-certify that a finding is closed.
