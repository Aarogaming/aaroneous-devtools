# Legacy Aarogaming repositories: transfer review

Date: 2026-10-30
Status: proposed evidence and development plan, pending independent review; not an implementation claim.

## Scope and evidence

Read repository descriptions across the accessible Aarogaming account and inspected selected source and contract files in the legacy suite. This is a targeted architecture survey, not an exhaustive audit of every file, test, dependency, license, or branch. Source statements describe old intentions or implementations; they do not establish current Rust behavior. The current devtools comparison below refers to the checked-out `codex/operating-spine-execution-plan` worktree and its existing contracts; it is not a live integration test.

The legacy suite sources were pinned to these commits where inspected:

| Repository | Revision | Survey |
| --- | --- | --- |
| AaroneousAutomationSuite | `4b52088a021abdbcb8029e3d3ad3a8cc3748a608` | README and described kernel/workflow/resource concepts |
| Merlin | `191b9371e20cbc246a140a2b5624aa5ebbefb227` | README: model routing and research manager |
| Workbench | `7df1b0ca5ec3275b18e597e4a25127df00a5dfc9` | README: asset and tool organization |
| Guild | `b3749df871403488532a950c51476805dabd77ab` | README and selected lifecycle, health and subsystem interfaces |
| Library | `9ee5442d05441525ff2aac70dd4be88c82f28836` | architecture, ownership, dependency and interop contracts |
| Maelstrom | `bb6e3608ec0e85cc4d07bb1cda9810a19d505fd0` | README and architecture |
| AndroidApp | `b68dbf351efa19b0d8508d109760714e8adf84a0` | README and operating protocol |
| MyFortress | `3a9b922f8ccc86b560cd38d049bdf30962c0b290` | README and gateway role |
| AutoWizard101 | `5b5c2400809dbcb9a146254036ffb86fce148094` | README, advertised operating modes and guards |
| Wizard101_DanceBot | `ca0ec613182a803b66dd9ce0f4a14ca631d9ab27` | README, provenance and perception use |
| circuit-simulator | `ac24c4f534b914a14481a31b0b16b580d4e02328` | README, origin and licensing notice |

Also screened the README of opencode, chatterbox, and Aarogaming.github.io. The archived Alexa SDK README and the suite's `docs/START_HERE.md` were unavailable through the attempted connector paths. These auxiliary projects have not received a deep architectural or code audit. Upstream metadata and licensing need verification before reuse. User ownership of a repository does not by itself establish ownership of all included third-party content.

## Recommended transfers, in priority order

### 1. Governed records and runtime projections — Library

The [control/data-plane ownership decision](https://github.com/Aarogaming/Library/blob/9ee5442d05441525ff2aac70dd4be88c82f28836/docs/source-of-truth/CONTROL_PLANE_DATA_PLANE_DB_OWNERSHIP.md) separates governed component/contract records from operational databases. Runtime systems can generate evidence and proposals; reproducible read models derive from governed artifacts. The [dependency tracking contract](https://github.com/Aarogaming/Library/blob/9ee5442d05441525ff2aac70dd4be88c82f28836/docs/source-of-truth/DEPENDENCY_TRACKING_CONTRACT.md) describes component-to-dependency and reverse-impact views and a single authoritative writer.

Adapt the rule, not the historic database layout: identify who may write each record, distinguish observations from accepted decisions, preserve references and historical decisions, and rebuild derived indexes without elevating runtime data into policy. Use existing Aaroneous/Devtools governance, not an automatic grant of authority to a newly named Library component.

Current overlap: `contracts/constellation-lifecycle-v1.md` describes an authorized writer, provenance, idempotency and projections. Its presence is a proposed contract, not evidence of a deployed Library. A first bounded slice could use fixtures to rebuild an evidence-linked dependency-impact view from versioned inputs; independently verify reproducibility, dangling references, unauthorized edits, and changed input revisions.

### 2. Handoff contract and real delivery semantics — Library, AndroidApp, MyFortress

Library's [Agent Interop V1](https://github.com/Aarogaming/Library/blob/9ee5442d05441525ff2aac70dd4be88c82f28836/protocols/AGENT_INTEROP_V1.md) specifies operation/request identities, intent, constraints, outputs, timestamps, acknowledgements, and status. AndroidApp's [operation protocol](https://github.com/Aarogaming/AndroidApp/blob/b68dbf351efa19b0d8508d109760714e8adf84a0/docs/REPO_OPERATION_PROTOCOL.md) emphasizes configured endpoints, mobile-client boundaries, protocol fixtures and redacted exports. MyFortress describes an auxiliary gateway and a client adapter.

Map these to the existing Platform Bridge/job transport and Steward rather than make another router. Add authenticity, authorization, bounded payloads, expiration, replay/idempotency protection, recovery and action fencing as explicit requirements; receipt ACK is not successful execution. Historical 30-second heartbeats and two retries are examples to qualify, not universal defaults. Avoid promoting endpoint paths, ambient configuration and keys from old deployments.

Current overlap: `contracts/guild-operation-v1.md` references the existing job-service spool, authenticated claim/result transport, acknowledgment and idempotent re-upload, and explicitly records missing failover and operation-lifecycle behavior. Verify actual implementation at the selected baseline before claiming a network feature works. Pilot with a local simulated pair and then configured Windows/Ubuntu peers; test duplicate messages, dropped ACK, restart, stale/expired claim, cancellation, and unauthorized peer.

### 3. Observable component lifecycle — Guild

The old [manager health protocol](https://github.com/Aarogaming/Guild/blob/b3749df871403488532a950c51476805dabd77ab/core/manager_health_protocol.py) defines service lifecycle states, distinct health status, status details and lifecycle events. Its [subsystem interfaces](https://github.com/Aarogaming/Guild/blob/b3749df871403488532a950c51476805dabd77ab/core/subsystem_interfaces.py) separate decisions, context, execution and safety concepts.

Adopt a small typed Rust-facing lifecycle and health vocabulary with dependency health and timestamps; keep service health separate from job completion and review. Treat a health check as an observation, not authority to perform an effect. Do not port the old mixin's boolean lock as a concurrency mechanism. Test illegal transitions, degraded dependencies, restart and stale status. Reconcile with existing lifecycle proposals before defining new types.

### 4. Explainable model routing and research provenance — Merlin

The [Merlin README](https://github.com/Aarogaming/Merlin/blob/191b9371e20cbc246a140a2b5624aa5ebbefb227/README.md) describes opt-in model routing, selected model and fallback metadata, and research sessions that connect claims to sources and confidence. These are descriptions, not independent verification of those features.

Extend the existing model-routing work with an auditable selection reason, availability/resource constraints and failure reason, with sensitive input redaction; do not assume prompt length or model confidence proves quality. Research artifacts should retain source, time, identity, claim, uncertainty, conflicts and revision. Evaluation against a fixed task set must separate quality, latency and cost and should not grant authority to the chosen model.

Current overlap: inspect `foundation/crates/steward/src/model_hub.rs` and Aaroneous' existing `llm_gateway` at the selected revision before deciding where an extension belongs. The explicit product/companion dependency boundary still applies.

### 5. Resource and application adapters — Workbench, MyFortress, Maelstrom, AndroidApp

Workbench's [README](https://github.com/Aarogaming/Workbench/blob/7df1b0ca5ec3275b18e597e4a25127df00a5dfc9/README.md) groups asset ingestion, indexing, search and tools. MyFortress presents a gateway, and [Maelstrom's architecture](https://github.com/Aarogaming/Maelstrom/blob/bb6e3608ec0e85cc4d07bb1cda9810a19d505fd0/ARCHITECTURE.md) makes module and protocol identity explicit. AndroidApp places orchestration on the host and focuses the client on configured remote interaction.

Use explicit capability descriptors (identity, version, inputs, outputs, permissions, platform and health) and a permission-scoped adapter boundary. Resource catalogs should describe provenance and available capabilities; indices are rebuildable views. The user's current direction assigns Maelstrom to the gamified presentation of Studio HUD, Ariel to interaction persona, and Marionette to permitted desktop/application interaction: old names and broad legacy responsibilities do not override that direction. Embedded or mobile targets must use only capabilities supported by their environment; no assumption that full desktop or model services fit them.

### 6. Simulation-first interaction — AutoWizard101 and DanceBot

[AutoWizard101's README](https://github.com/Aarogaming/AutoWizard101/blob/5b5c2400809dbcb9a146254036ffb86fce148094/README.md) describes a simulation-only default, policy gates, snapshots, and a separate command bridge; these assertions were not validated against code or runtime. DanceBot illustrates image classification and explicitly credits an upstream project.

The transferable test is a controlled application with recorded frames, action receipts, single input ownership, clear cancellation and post-action verification. Exercise off-target focus, stale frame, lost control, denied permission, timeout and emergency stop. A snapshot file or polling bridge alone is not a secure or exactly-once command channel. Keep the game application independent for now; platform terms and user permission govern any live target. Preserve attribution and verify rights before using any upstream assets or code.

### 7. External ecosystems and simulations — opencode, chatterbox, circuit-simulator, Alexa SDK

These appear to contain or reference third-party projects, not evidence that their internals belong in the Aaroneous kernel. Potentially useful future adapters: coding-agent interface, optional speech service and deterministic simulation/testing. Assess upstream origin, exact license, weights/data terms, target hardware and isolation before any adoption. Circuit-simulator's README explicitly points to original author restrictions; commercial compatibility is not established. The Alexa SDK was not inspected beyond the failed README fetch. The personal site README contained no substantive architecture.

## Proposed order of implementation work

1. Select a claimed lane and pin the real product and consumer baselines; inventory current contracts and disagreements with this review.
2. Design one bounded operation record linking intent, a single attempt, receipts and evidence to a terminal disposition. Preserve the existing review and ownership process. Demonstrate duplicate/stale receipt rejection and recovery without assuming safe failover.
3. Add a derived Library view over those records and a dependency-impact fixture. Show deterministic rebuild, provenance and refusal of unauthorized promotion.
4. Qualify the configured local/LAN transport using test peers and failure injection; enforce identity, policy, bounded messages and fencing before unattended work.
5. Add health and model routing receipts within existing facilities. Evaluate a controlled, permissioned desktop test fixture and then an independent game prototype as a later consumer.

Every step needs a separately claimed scope, exact-head verification, independent review and a recorded disposition. These are candidates, not new queue claims or evidence that prior PR blockers are resolved.

## Deliberate exclusions and verification limits

- No legacy code, models, assets, secrets or dependencies were imported; no services or automation were launched.
- No individual README's "production ready", security, test or performance claim was independently proven.
- Never infer that plugin discovery grants authority, or that retrying an effect is safe. Keep native Rust isolation and explicit resource limits.
- This proposal requires independent review and does not itself modify canonical governance or authorize implementation.
