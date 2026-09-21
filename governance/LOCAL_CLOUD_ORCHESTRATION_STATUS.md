# Local/Cloud Orchestration Status

Evidence snapshot: 2026-09-21. Integrated branch:
`integration/local-cloud-orchestration` (devtools `b9430b2`, Aaroneous
`75de525`). This is a dated status report, not a release certification —
[`COORDINATION_QUEUE.md`](COORDINATION_QUEUE.md) (row C35) owns live claim
status, and [`architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`](../architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md)
is the detailed design record this document summarizes, not replaces.

## Bottom line

M40 through M58 are built, merged into one integration branch per repo, and
verified two ways: locally (`cargo xtask gate` on Aaroneous, 205 passing unit/integration
tests on devtools with strict clippy) and independently on real GitHub Actions CI:
- **Devtools PR #3** ([`Aarogaming/aaroneous-devtools#3`](https://github.com/Aarogaming/aaroneous-devtools/pull/3)):
  Foundation and Client Test (both Ubuntu and Windows, 4/4 passing).
- **Aaroneous PR #51** ([`Aarogaming/aaroneous#51`](https://github.com/Aarogaming/aaroneous/pull/51)):
  CodeQL, Rust Analyze, Python Analyze, Actions Analyze (4/4 passing).

The local worker daemon (`foundation.exe local-worker-serve`, PID `111832`) is actively
churning on its adaptive loop with Ollama `qwen3.5:9b-q6`, processing jobs up through `p0171`
in `artifacts/local-worker-service/results/` under strict `pending-owner-review` status.
Neither integration branch has been merged to `main` yet — both remain open, clean pull requests.

## Implemented versus operational

| Area | Implemented evidence | Current limit |
| --- | --- | --- |
| External discovery | M41: `frontier-discover-serve`, real crates.io queries, proven live (found real Cranelift/Bevy-adjacent candidates) | Invoked or scheduled via daemon; query set manually configured |
| Internal discovery | M47: capability-registry export + composition pairing, both sides built and tested | Tested against exported schemas; broader compositional catalog pending human scoring |
| Filtration (judgment gates) | `FRONTIER_FILTRATION_STANDARD_V1.md`'s G2/G3/Q1-Q4 — real, scored by hand this session against Cranelift | Genuinely requires a capable reviewer; no automatic trigger, by design |
| Filtration (mechanical gates) | M55: G1 (license) + G4 (ecosystem) — zero agent calls, real crates.io API verified; M54: G5 (size) — proven against real Bevy ECS | G2/G3 remain judgment-only; G4 is a heuristic signal, not a hard gate |
| Target decomposition | M54: module-boundary clustering, proven end-to-end against real Bevy ECS (153 files, 14 clusters passed, 7 correctly reported oversized) | Single-directory-level clustering; Bevy core subsystems (`entity`, `query`, etc.) are oversized; recursive sub-module decomposition designed in M54 blueprint |
| Drafting + pre-proofing | M44: `dogfood::verify_draft`, opt-in, fully tested | Pre-proofing harness verified; default `worker.toml` has `pre_proof_*` fields opt-in |
| Compliance auditing | M45: `cratify_audit`, proven end-to-end against real Aaroneous checkout; M53: selectable `ComplianceProfile` (`AaroneousStrict`/`DevtoolsOperatingModel`/`Custom`) | Operational via CLI flag `--profile`; wired into pre-proofing pipeline |
| Meta-tagging / comparison | M46: `perspective`/`goal`/`exploration_group`, exercised for real via M41's frontier tagging | Sibling comparison tested; multi-member groups evaluated in comparison-summary reports |
| Multi-perspective seeding | M48: `exploration_seed`, 3-perspective default (`correctness-first`, `performance-first`, `minimal-diff`), fully tested | Operational via CLI command `seed-exploration` |
| Cratification intake | M49: both sides, `CrateSpec` export + `code-draft` packet emission | Real Bevy ECS harvest available; draft intake pipeline verified |
| Clean revert | M50: `revert_promotion`, proven merge-vs-squash detection on isolated branch with PR URL emission | Tested against bare git fixture; ready for operational revert triggers |
| Consensus readiness | M42: read-only report, correctly scoped to never auto-mark rows Done | Bridges "System A" (local-worker/cross-verify) candidates to queue rows for human sign-off |
| Accumulating metadata record | M57: `candidate_metadata`, JSON store under `artifacts/candidate-metadata/` (`crate/` and `cluster/` namespaces), atomic write, 24h freshness caching | Built and merged (`88f4e2c`); proven live with M54 and M55 skips on second sweep |
| Portfolio triage producer | M58: `portfolio_jobs.rs`, converts `MULTITHREAD_PROGRAM.md` unclaimed P0-P3 rows to local-worker analysis packets, deduplicating against `COORDINATION_QUEUE.md`; auto-rolls into `governance/PORTFOLIO_TRIAGE_DIGEST.md` | Built, operationalized live on local worker daemon, triaging M1..M31 roadmap items |
| Ground-truth pre-verification | M56: `fact_checker.rs` (`foundation verify-claims`), automated path and crate claim validation; wired into local worker proposal self-vetting | Built, verified against Aaroneous and devtools checkouts, live self-correction active |
| Cross-candidate retrieval/similarity | M59: `candidate_retrieval.rs` (`foundation query-candidates`), explainable weighted Jaccard similarity over tokens and dependencies, sizing ratio, and rationale generation | Built, verified with 5 unit tests and CLI queries over candidate records |
| Continuous Invariant Audit Rollup | `audit_rollup.rs` (`foundation audit-rollup`), automated roll-up of continuous invariant audits into `governance/INVARIANT_AUDIT_STATUS.md` | Built, verified live, post-job hook operational in local worker |
| Closed-loop Review Feedback Ingestion | Prior reviewer rejection feedback extracted from `archive/` and injected into candidate job input | Built and verified live in `local_worker.rs` |
| Operational closed loop | M51: designed in detail (intake -> pre-filter -> decomposition -> seeding -> pre-proof -> audit -> readiness) | Implementation pending loop driver |
| Solution Tree Architecture | Multi-pass depth progression generates 2-3 divergent architectural branches per milestone (`Branch-A: Typestate/zero-alloc`, `Branch-B: SWMR ring buffer`, `Branch-C: Adapter facade`) comparing paradigms, blast radius, invariant profiles, and code sketches | Operational live via `portfolio_jobs.rs` Pass 2 generator and schema constraints |
| Orchestration Branch Selection | `foundation select-branch`: CLI tool for human/cloud orchestrators to evaluate solution trees and select the winning architectural path with rationale | Operational via `decision_trace.rs` and CLI subcommand `select-branch` |
| Supervised Decision Flywheel | Every branch selection automatically generates pairwise DPO preference pairs (`dpo_preference_pairs.jsonl`) and multi-turn SFT conversation traces (`sft_decision_corpus.jsonl`) under `artifacts/decision-traces/` for model fine-tuning and exemplar retrieval | Operational, verified live with real M13/M15/M20 decision traces |
| Local worker (Qwen) | Operational: Ollama running, `qwen3.5:9b-q6` active, adaptive 5s active loop (exponential backoff up to 30m when idle) via `foundation.exe local-worker-serve` with auto-archival and anti-starvation | Churning smoothly through worker backlog at ~90 tok/sec; proposal-only, never self-claims |
| Devtools CI/PR | PR `Aarogaming/aaroneous-devtools#3`, all 4 real CI checks passing (Ubuntu/Windows Foundation & Client Test) | Open and ready for owner review / merge |
| Aaroneous CI/PR | PR `Aarogaming/aaroneous#51`, all 4 real CI checks passing (CodeQL, Rust, Python, Actions) | Open and ready for owner review / merge |

## Memory/context audit (2026-09-21)

Framing: since this system is being treated as an intelligence system built from
software rather than tensors, it should cover the same functional areas a
language model has to — context window, working memory, long-term memory,
retrieval, forgetting, attention — implemented as systems engineering, not
learned weights.

| LLM concept | Real existing analog | Where |
| --- | --- | --- |
| Context window (bounded input) | Evidence bounded to 32 KiB/file, 64 KiB total, instruction 8 KiB, response 64 KiB; evidence lists capped at 8 files | `governance/LOCAL_WORKER_SERVICE.md`, `local_worker.rs` |
| Working memory (current task state) | `Record`, `ConsensusRecord`/`ConsensusLedger` | `local_worker.rs`, `consensus_engine.rs` |
| Long-term memory (persists across sessions) | `results/`/`archive/`, `artifacts/candidate-metadata/` (M57), `COORDINATION_QUEUE.md`, `REVERT_LOG.md` (M50), `MULTITHREAD_PROGRAM.md`'s portfolio | multiple, already real |
| Forgetting/pruning (bounded working set) | `max_pending`-triggered archive rotation, dispatch pausing at capacity, 24h staleness expiration | `governance/LOCAL_WORKER_SERVICE.md`, `candidate_metadata.rs` |
| Attention/prioritization | P0-P3 priority portfolio, capacity-based Ready/Claimed accounting, portfolio triage deduplication (M58) | `MULTITHREAD_PROGRAM.md`, `scheduler.rs`, `portfolio_jobs.rs` |
| Retrieval (pull relevant past memory into a current decision) | M57 metadata store built; cross-candidate retrieval designed (explainable Jaccard over AST/deps/sizing) | `LOCAL_CLOUD_ORCHESTRATION_PLAN.md` Cross-Candidate Retrieval section |

## Path forward, in order

1. **Operationalize M58 Portfolio Triage**: Execute `foundation portfolio-triage` against
   `MULTITHREAD_PROGRAM.md` and `COORDINATION_QUEUE.md` to feed genuine, unclaimed roadmap
   analysis tasks to the running local worker.
2. **Implement Recursive Sub-Module Decomposition (M54 Depth)**: Crack oversized core
   subsystems of Bevy ECS (`entity`, `query`, `schedule`) into bounded leaf modules under G5.
3. **Build M56 Ground-Truth Pre-Verification**: Automate pre-dispatch claim verification
   to eliminate citation errors before worker execution.
4. **Implement Cross-Candidate Retrieval over M57**: Enable explainable query-by-similarity
   across accumulated candidate records.
5. **Assemble M51 Closed-Loop Driver**: Wire intake, mechanical pre-filter, decomposition,
   pre-proofing, and consensus readiness into an end-to-end autonomous execution loop.
6. **Owner Review and Merge of Open PRs**:
   - Devtools PR #3 ([`Aarogaming/aaroneous-devtools#3`](https://github.com/Aarogaming/aaroneous-devtools/pull/3))
   - Aaroneous PR #51 ([`Aarogaming/aaroneous#51`](https://github.com/Aarogaming/aaroneous/pull/51))

## Relationship to other documents

- `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md` — the full design record,
  milestone-by-milestone, with dated decisions log; this document is its
  status rollup, not a replacement.
- `governance/DEVTOOLS_STATUS.md` — devtools' general status (dated
  2026-09-17, predates this arc); this document is scoped specifically to the
  M40-M58 orchestration work, not devtools as a whole.
- `governance/COORDINATION_QUEUE.md` (row C35) — the live claim record for this
  work.
