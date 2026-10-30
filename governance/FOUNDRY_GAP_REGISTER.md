# Foundry gap register

## G-2026-09 — resource fit is not job-specific provider qualification

- Boundary: Steward job-to-intelligence selection.
- Evidence: `foundation/crates/steward/src/model_hub.rs`, `choose`, at
  `513a246e76aac42b9d49fd522bc14e7c67d5ffc7` ranks GPU-fitting models by weight;
  it does not assess task evidence, data eligibility or human/tool candidates.
- Risk: a resource-fit recommendation can be mistaken for competence or permission.
- Disposition: **C178 Review** — add a pure report-only capability broker with
  explicit policy inputs and adversarial tests. Live dispatch and adaptive
  translation remain outside this increment; independent review is required.

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
- Disposition: **C82 Blocked on C79** — specify the Foundry adapter-manifest
  contract and its read-only status surface only after C79's export is
  accepted.

## G-2026-05 — gap tracking is documented but not mechanically checked

- Boundary: governance freshness.
- Evidence: `governance/OPERATING_MODEL.md:45-48` requires reviewed findings
  to enter an inbox and receive a portfolio ID/acceptance criterion, but did
  not require a verified architectural or service gap to retain a corrective
  disposition.
- Risk: reviews discover defects that remain chat-only and drift away from the
  queue before design or implementation begins.
- Disposition: **C83 Review** — add a deterministic register-to-queue policy
  checker and fixtures; no agent may self-certify that a finding is closed.

## G-2026-06 — verification is comprehensive but lacks a cost-layered contract

- Boundary: Aaroneous native verification and Foundry gate selection.
- Evidence: `xtask/src/gate.rs:5-192` provides a comprehensive, CI-parity
  sequence, including encoding, formatting, workspace Clippy, native audit,
  compilation, tests, AST audit, ratchets and feature checks. It runs
  whole-workspace Clippy before several cheaper metadata/ratchet/structural
  checks and exposes no revision-bound, machine-readable preflight tier
  contract for an external controller.
- Risk: a Foundry either pays full compilation cost before cheap decisive
  failures are known, or invents its own gate meaning and risks treating a
  preflight pass as product verification.
- Disposition: **C85 Ready** creates the product contract; **C86 Blocked on
  C79 and C85** consumes it without duplicating product checks.

## G-2026-07 — Cratify classifies crates but does not yet provide a complete lifecycle sheet

- Boundary: Aaroneous crate declaration, verification and retirement lifecycle.
- Evidence: `AGENTS.md:67-85` and `docs/CRATIFY_SPEC.md` require a compliance
  profile, dependency direction, graduation and a ratchet. The current
  `xtask/src/workspace_metadata.rs` output only contains name, version, ring,
  capability metadata and direct dependencies; it has no revision-bound
  lifecycle state, required gate/result set, compatibility/retirement record,
  or per-crate generated build sheet.
- Risk: a crate can be classified without a mechanically inspectable record of
  what it is allowed to do, what must verify it, or how it advances, retires or
  is superseded.
- Disposition: **C87 Blocked on C79, C85 and C88 (C89 accepted in PR #111)** —
  C89 landed in product PR #111 moving Cratify profile assignments into crate
  manifests; C87 derives a build sheet from accepted product facts once C79,
  C85 and C88 land.

## G-2026-08 — Base36 notation is a stated design intent, not an implemented contract

- Boundary: portable component/version/lifecycle identifiers.
- Evidence: product-wide source search on 2026-09-29 found no `base36`,
  `base_36`, `base-36` or radix-36 encoding contract in tracked Rust, TOML,
  JSON or documentation sources. Current workspace metadata exports a free
  string version and Cratify defines profiles, but neither defines an encoding
  alphabet, canonical form, ordering or schema revision.
- Risk: different tools can produce visually similar but incomparable IDs,
  making lifecycle records and cross-host artifacts ambiguous.
- Disposition: **C88 Ready** — define and test the product-owned Base36
  contract before C87 consumes it.
