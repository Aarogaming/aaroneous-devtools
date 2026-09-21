# Local/Cloud Orchestration Status

Evidence snapshot: 2026-09-21. Integrated branch:
`integration/local-cloud-orchestration` (devtools `ec696dc`, Aaroneous
`75de525`). This is a dated status report, not a release certification —
[`COORDINATION_QUEUE.md`](COORDINATION_QUEUE.md) (row C35) owns live claim
status, and [`architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`](../architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md)
is the detailed design record this document summarizes, not replaces.

## Bottom line

M40 through M56 are built, merged into one integration branch per repo, and
verified two ways: locally (`cargo xtask gate` on Aaroneous, full test/clippy/fmt
on devtools) and — as of this snapshot, for the first time this arc —
independently on real CI (`gh pr checks 3`: Foundation and Client Test, both
Ubuntu and Windows, all pass). M57 is designed and blueprinted, not yet built.
Several real capabilities have been proven against live external data (M54
against actual Bevy ECS source, M45 against the real Aaroneous checkout, M55
against the real crates.io API); several others are built and tested
synthetically but not yet exercised against a real live run. Neither integration
branch has been merged to `main` or independently reviewed by a person — both
remain open, unmerged pull requests.

## Implemented versus operational

| Area | Implemented evidence | Current limit |
| --- | --- | --- |
| External discovery | M41: `frontier-discover-serve`, real crates.io queries, proven live (found real Cranelift/Bevy-adjacent candidates) | Not scheduled to run automatically; must be invoked |
| Internal discovery | M47: capability-registry export + composition pairing, both sides built and tested | Never yet run against Aaroneous's real, full capability registry — only synthetic fixtures |
| Filtration (judgment gates) | `FRONTIER_FILTRATION_STANDARD_V1.md`'s G2/G3/Q1-Q4 — real, scored by hand this session against Cranelift | Genuinely requires a capable reviewer; no automatic trigger, by design |
| Filtration (mechanical gates) | M55: G1 (license) + G4 (ecosystem) — zero agent calls, real crates.io API verified; M54: G5 (size) — proven against real Bevy ECS | G2/G3 remain judgment-only; G4 is a heuristic signal, not a hard gate |
| Target decomposition | M54: module-boundary clustering, proven end-to-end against real Bevy ECS (153 files, 14 clusters passed, 7 correctly reported oversized) | Single-directory-level clustering only; Bevy's own core subsystems (entity/query/schedule/system/world) are still oversized at this granularity — recursive decomposition not built |
| Drafting + pre-proofing | M44: `dogfood::verify_draft`, opt-in, fully tested | Not enabled in the real deployed `worker.toml` — the three `pre_proof_*` fields are unset in the actual running config |
| Compliance auditing | M45: `cratify_audit`, proven end-to-end against the real Aaroneous checkout (real exit code, real "AUDIT GATE PASSED"); M53: selectable `ComplianceProfile` (`AaroneousStrict`/`DevtoolsOperatingModel`/`Custom`) | `DevtoolsOperatingModel`'s own evidence bar (a real run against real devtools code) was not met when M53 was built — proven only via shared-resolver construction, not a live run |
| Meta-tagging / comparison | M46: `perspective`/`goal`/`exploration_group`, exercised for real via M41's frontier tagging | No producer has yet generated a real `exploration_group` with multiple members for a human/agent to actually compare |
| Multi-perspective seeding | M48: `exploration_seed`, 3-perspective default, fully tested | Never run against a real goal — synthetic fixtures only |
| Cratification intake | M49: both sides, `CrateSpec` export + `code-draft` packet emission | The real Bevy ECS harvest this session produced was used for M54 (decomposition), not yet fed through M49's own draft-request path |
| Clean revert | M50: `revert_promotion`, proven the merge-vs-squash detection against a real fixture merge commit | Never exercised against a real promoted commit on this repo's actual history |
| Consensus readiness | M42: read-only report, correctly scoped to never auto-mark rows Done | Only ever sees "System A" (local-worker/cross-verify) candidates — System B (multithreader/intake) is structurally invisible to it, by design, documented |
| Ground-truth pre-verification | M56: designed, blueprinted | Not built |
| Accumulating metadata record | M57: designed, blueprinted | Not built |
| Cross-candidate retrieval/similarity | Named as the real gap in a memory/context audit (see below) | Not designed — correctly deferred, since it needs M57's record to exist first |
| Local worker (Qwen) | Fully provisioned: Ollama running, `qwen3.5:9b-q6` pulled, real `worker.toml` correctly configured | Not currently running as a persistent process — `local-worker-serve` has not been started |
| Devtools CI/PR | PR `Aarogaming/aaroneous-devtools#3`, all 4 real CI checks passing as of this snapshot | Not merged to `main`; no independent human or second-agent review yet |
| Aaroneous CI/PR | Branch `integration/local-cloud-orchestration` pushed, `cargo xtask gate` passes locally (11/11 stages) | No PR open — blocked twice by this session's own safety classifier (`Out-of-Place Publication`); manual link given, not yet opened by the owner |
| Outbound contribution (fix external OSS, contribute back) | Named future direction only | Not designed — explicitly deferred per owner's own internal-first sequencing decision |
| Transpilation / machine-native reverse engineering into `.si` models | Named future direction, connects to real `.si`/`SiForge` infrastructure | Not designed — explicitly research-grade, not a milestone |

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
| Long-term memory (persists across sessions) | `results/`/`archive/`, `COORDINATION_QUEUE.md`, `REVERT_LOG.md` (M50), `MULTITHREAD_PROGRAM.md`'s portfolio | multiple, already real |
| Forgetting/pruning (bounded working set) | `max_pending`-triggered archive rotation, dispatch pausing at capacity | `governance/LOCAL_WORKER_SERVICE.md` |
| Attention/prioritization | P0-P3 priority portfolio, capacity-based Ready/Claimed accounting | `MULTITHREAD_PROGRAM.md`, `scheduler.rs` |
| Retrieval (pull relevant past memory into a current decision) | Partial only — `scope_reserved` checks the queue, M46 groups siblings — no general "does this resemble something already decided" check | **The one real gap**, correctly sequenced behind M57 |

Conclusion: most functional areas already have a real answer, not by designing
toward the analogy but because sound systems engineering converges on the same
shapes for the same reasons (bounded input, persistent state, capacity-based
pruning). Retrieval is the one genuine, named gap, and it's already correctly
sequenced — it cannot be designed before M57's record exists to retrieve from.

## Path forward, in order

1. **Build M57** (accumulating per-candidate metadata) — the next concrete,
   unblocked, already-blueprinted step. Wraps M54 and M55 first, since both
   already produced real output this session to accumulate.
2. **Exercise what's built-but-unproven against real data**, the same way M54
   and M45 already were — M48 (a real seeding round), M49 (feed the real Bevy
   harvest through cratification-intake, not just decomposition), M50 (a real
   revert against real history), M53's `DevtoolsOperatingModel` profile (a real
   run against real devtools code, closing its own stated evidence gap).
3. **Get independent review on the two open PRs** — devtools #3 is CI-green and
   ready for a second reviewer; Aaroneous's branch needs its PR opened manually
   (owner action, blocked from this session).
4. **Retrieval over M57's record** — the named memory/context gap — once M57
   has real accumulated data to retrieve from.
5. **M56** (ground-truth pre-verification) and **M51** (the actual closed loop)
   remain correctly sequenced behind real usage data from the above, not
   speculative design.

## Relationship to other documents

- `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md` — the full design record,
  milestone-by-milestone, with dated decisions log; this document is its
  status rollup, not a replacement.
- `governance/DEVTOOLS_STATUS.md` — devtools' general status (dated
  2026-09-17, predates this arc); this document is scoped specifically to the
  M40-M57 orchestration work, not devtools as a whole.
- `governance/COORDINATION_QUEUE.md` (row C35) — the live claim record for this
  work.
