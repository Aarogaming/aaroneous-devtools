# Parallel reconciliation dispatch

Prepared 2026-09-25 UTC under the owner's request to coordinate Codex, Gemini, Claude and local Qwen. This is a bounded integration/review plan, not approval to implement the proposed composition architecture or to merge unverified work.

## Shared control point

The live ledger for this round is `D:\aaroneous-devtools\governance\COORDINATION_QUEUE.md`, on `codex/coordination-reconciliation-20260925`. The primary product checkout remains on `main`. Implementation branches, untracked files and the sync stash remain preserved.

Read the [current queue](COORDINATION_QUEUE.md) before starting. Do not use an old worktree's copy as the live queue. At dispatch this coordination branch is local and unpushed: agents on this host can read the exact path; remote agents need an explicitly published handoff before starting. No external Gemini/Claude session has been launched or notified by this document.

Use three primary lanes, one bounded Qwen support slot, and sequential independent reviews drawn from the other primary owners. A preferred assignee is not a running agent. Only the assignee's acknowledgement and visible claim activate a lane. The coordinator serializes queue edits: send a claim request with lane, exact output, branch/worktree, UTC time and expected gate; wait for its recorded claim before writing. Do not concurrently patch the ledger or switch the shared checkout.

## Evidence snapshot

| Repository / artifact | Observed revision | Meaning |
| --- | --- | --- |
| Devtools main | `e2d9ff62b947122404ee27b83ccf7a2a5c4a7b46` | Matched fetched origin/main at sync; older queue |
| Devtools integration | `d89b0e1226b783b7f594c6cd16b813f701364162` | 151 commits ahead of main; not accepted as a batch |
| Preserved sync stash | `0bf9b6ff7cd16bbe1c776ed10e872d1895fab736` | Latest queue at `:governance/COORDINATION_QUEUE.md`; composition documents in third parent |
| Aaroneous main | `906c01ee` | Matched fetched origin/main at sync |
| Aaroneous integration/main-merge | `9f280634b4a1395db8b8eb0455d83c919adf61b9` | 34 ahead, zero behind main; unverified final integration |
| Aaroneous refactor/hive-mind-host-split-wip | `cd9f795254dda80309fc3b7661d122cfd6d23801` | 34 ahead, zero behind main; different content, not interchangeable with integration/main-merge |

Local branch inventory before creating this coordination branch: product 20 non-main branches already ancestral to main, 31 non-backup branches not ancestral; devtools 31 non-backup branches not ancestral. These counts exclude backup refs and are not merge counts. Patch-equivalent or nested histories require diff review. Remote-only branches were not included. Do not delete branches/worktrees from these counts alone.

Devtools' non-backup pending tips reduce by ancestry to `integration/aaroneous-framework-imports`, `feat/m63-backlog-throttle-priority-feedback-bridge`, and `wip/pre-session-local-worker-changes`. The latter two have unique commits, but uniqueness does not prove their changes are absent from integration. The preserved old main has no commits absent from integration.

## Queue audit and historical disposition

Both source ledgers are preserved as readable transcripts: [main](archive/COORDINATION_QUEUE_MAIN_2026-09-25.md) and [integration plus saved edits](archive/COORDINATION_QUEUE_INTEGRATION_2026-09-25.md). Exact originals remain in the Git objects above. Historical Ready/Claimed labels do not reserve files in this round.

| Finding | Correction now | Remaining work |
| --- | --- | --- |
| Main's C23/C24 say Ready; saved integration ledger records completion | Remove from current dispatch; preserve both reports | C64 compares completion evidence with actual branch content; do not reimplement |
| Main C25/C26/C27 identify daemon retirement/autonomy/breaker; integration uses C36/C37/C38 for those, and reuses C25-C27 for other tasks | Qualify old IDs by ledger source and title; new IDs start at C63 | Preserve aliases in any later migration; never correlate these by number alone |
| Saved ledger has 26 control characters and lacks intact table rows for C22, C35, C42-C52 | Preserve visible escaped transcript and immutable original; exclude damaged historical text from active parsing | C64 reconstructs IDs from Git history and cited commits, not guessed ordering |
| C55/C56/C60 claims are older than 24 hours | Release stale reservations by this dated handoff; carry evidence into C64 | Inspect dirty worktrees before touching any former path; expiry does not discard work |
| C61 broad source claim is still within 24 hours but the owner put construction on hold; branch progressed separately | Explicit coordinator handoff: release Codex's old source reservation, retain work at cd9f7952; review under C66 | Fresh gates and exact defect scopes before resuming implementation |
| C57-C59 still mix uncommitted/local-only claims with later committed evidence | Retain acceptance debt, withdraw stale live-state claims | C64 checks devtools evidence; C66 checks product policy/security evidence |
| C62 documents live only in the preserved stash | Preserve proposal status and exact recovery locator | C66 reads/reviews directly from Git; no stash pop or shared-tree overwrite |
| Old C10-C13/C15/C16/C19 were already blocked on acceptance | Carry forward review debt, not active file reservations | C64 identifies what is accepted, superseded, or still needs an independent reviewer |
| Legacy C40/C41 concern earlier defects/conflicts | Do not assume still broken or now fixed | C65 checks relevant narrow fixes; C66 checks integrated security/compatibility; link each finding to an exact revision |
| M40-M80 are absent from main's older portfolio and some existing labels are stale | Portfolio gets a current-dispatch override linking this reconciliation | C64 produces a source-qualified status map; no automatic Done claims |

No historical implementation is newly certified Done by this bookkeeping pass. Unresolved legacy items not specifically dispatched remain on hold, including expansion/graduation M75-M80 and new composition/Core runtime construction.

## Three independent first-wave lanes

First wave is read-only against both repositories' implementation branches. Each owner writes only its assigned report, on its own local branch/worktree. Reports may be read by other owners after a handoff. Do not create or consume a half-written shared report. Proposed branch names below are suggestions, not existing checkouts.

### C64 - Codex: devtools and queue evidence reconciliation

- Preferred branch: `codex/devtools-reconciliation-20260925` from the current coordination commit once available. Exact output: `inventories/C64_DEVTOOLS_RECONCILIATION_2026-09-25.md` in its own devtools worktree.
- Read d89b0e1, the two other pending tips, both ledger archives, relevant commit diffs, the preserved stash and explicitly selected runtime evidence. Do not pop the stash.
- Compare M63 and worker WIP changes by content against integration; recommend merge, selective recovery, already represented, or hold with evidence per branch.
- Reconcile all historical queue items into source-qualified IDs, actual branch/commit, implemented evidence, acceptance debt and successor. Recover the malformed C22/C35/C42-C52 records from history; if uncertain, say so.
- Separate generated digests, scratch scripts, job manifests and composition documents; do not commit all untracked material as a batch.
- Establish current local-worker result/review/backpressure facts before enabling C67. The 2026-09-23 record of 693 unreviewed results is historical, not today's count. Do not restart services or record blanket verdicts.
- Evidence gate: ancestry plus patch/content comparison, full path/commit citations, no unexplained dropped task, UTF-8/LF/links/diff. This first pass requires no builds or model calls. Reviewer: Gemini after its report is handed off; owner settles disputed closure.

### C65 - Gemini: six narrow product candidates

- Preferred devtools report branch: `codex/gemini-product-review-20260925`. Exact output: `inventories/C65_PRODUCT_CANDIDATES_2026-09-25.md` in its own devtools worktree. Product branches are read-only in wave one.
- Review these pinned candidates against product main and one another:

| Candidate | Tip | Principal change surface |
| --- | --- | --- |
| chore/remove-legacy-fabrication-sabs | `11d76933d357e202964f0eb566e5e3497e24bf9b` | `data/fabrication/` removals and `docs/archive/TODO.md` |
| chore/remove-dead-modules | `e4027e84a2ac3e4a861cbdbe20247344ca7b3b01` | hypervisor chaos_monkey, simulation_testbed, lib.rs |
| fix/thermal-metrics-honest-unmeasured-state | `5bdbbff5131576b2400d74735d9368b8d2437baf` | hypervisor system_metrics.rs and supervisory_loop.rs |
| fix/si-ssm-flaky-latency-assert | `97254bf57700bd7997093d02631449bd7c468ef9` | compute si_ssm.rs |
| fix/hid-driver-flaky-latency-asserts | `b5e293d8b15292fec8622d558f2ba928fc811684` | hypervisor hid_driver/mod.rs |
| refactor/deduplicate-crate-implementations | `646d460360e63deef6f73d145ac82fde24f7553d` | hypervisor compaction_engine/lib.rs and explanatory comments in adaptation_engine/capabilities/omni/platform_bridge |

- Correct a prior report's claim that fabrication removal touched no Rust/TOML: the actual diff removes 557 files, including `.rs` and `.toml`, with 268,067 deletions. Verify workspace, packaging, consumers and recovery before recommending this removal; it is not merely a documentation edit.
- Check overlap in hypervisor lib.rs, thermal Unknown behavior in actual consumers, meaningful functional assertions retained by timing fixes, and equivalence with fix/compute-timing-asserts. Do not resolve flaky tests by silently weakening intended functional guarantees.
- Report accept-for-verification, needs-change, or hold per candidate, the proposed merge order, exact future edit paths and targeted/native gates. No branch deletion, source edit, product merge or push in wave one.
- Evidence gate: pinned diffs, call-site/consumer checks, explicit risks and test plan, UTF-8/LF/links/diff. Reviewer: Claude, after C66 exposes cross-cutting blockers.

### C66 - Claude: security, policy and architecture review

- Preferred devtools report branch: `codex/claude-integration-review-20260925`. Exact output: `inventories/C66_SECURITY_ARCHITECTURE_REVIEW_2026-09-25.md` in its own devtools worktree. No product source reservation.
- Inspect integration/main-merge 9f280634 and host split cd9f7952 independently. Trace restoration of hotload/plugin_api to security-removal commits and reachable paths; distinguish build restoration from safe runtime behavior. Do not certify security merely from a green compile.
- Review Cratify v2 policy and actual enforcement, naming/ring/profile conflicts, xtask orchestration versus reusable policy ownership, and the proposed Core/composition boundaries. Main and integration have different constitutions: cite both; do not silently treat unpublished policy as landed law.
- Read `architecture/SYSTEM_COMPOSITION_BLUEPRINT.md` and `architecture/SYSTEM_COMPOSITION_REVIEW.md` from `0bf9b6ff7cd16bbe1c776ed10e872d1895fab736^3:<path>`. Compare the 14 proposed safeguards to actual interfaces. Proposal gaps are not automatically confirmed code defects.
- Deliver a blocking/nonblocking decision matrix, exact missing evidence, and bounded follow-ups. Review C65's eventual report independently; do not apply its fixes while doing this review.
- Evidence gate: source/call-site and commit references, authority/resource/identity separation, native-gate requirements identified, UTF-8/LF/links/diff. Reviewer: Codex for Claude-authored findings; unresolved policy/security decisions go to owner.

## C67 - Qwen support, gated by a cloud owner

Qwen is not a fourth primary lane and cannot claim the queue. Sponsor: Codex. Status starts Blocked until C64 checks pending outputs, selects one genuinely useful task and names the exact evidence bundle, output, endpoint/config and shared reservation.

First useful task: classify a small owner-selected set of existing unreviewed audit proposals against supplied source excerpts, returning supported, contradicted or insufficient-evidence with citations. Prefer deterministic Git tooling for branch ancestry; do not spend Qwen calls counting branches. No need to regenerate an existing adequate proposal.

Before dispatch, add the exact manifest/evidence/output paths to C67, review existing relevant results, and use the documented worker mechanism/reservation. `qwen3.5:9b-q6`, temperature 0.0, at least 4096 predicted tokens, concise no-reasoning preamble, versioned proposal output. No commands, queue writes, repo edits, verdict authority or self-certification. Cloud sponsor reviews every result. Do not start a separate Ollama loop, unattended generator, paid API call or service merely to fill the lane. Existing reservation and quiet-window policies apply to the explicitly selected configuration; conflicting service documentation is not permission to bypass either.

## Verification and integration sequence

1. Dispatch the three read-only reports concurrently after their claims are visible. No expensive builds are needed for that pass.
2. Cross-review findings and publish exact source-file claims for only the accepted fixes. Create isolated product/devtools worktrees from recorded commits; do not use or overwrite another agent's existing worktree.
3. Coordinator grants one heavy Cargo verification slot at a time on this host. Record holder, checkout, commit, output directory and command before starting; release it with actual result. Use separate explicit target directories and bounded build jobs. Never kill unrelated builds. Queue notes are cooperative, not a technical lock.
4. Verify Gemini's narrow product candidates first in the accepted order, then the combined candidate tree. Run focused regressions and the full product-native gate (`cargo run -p xtask -- gate`) for the actual candidate revision. A historical targeted pass is not fresh full verification. Any gate failure is recorded, not waived by the author.
5. Verify the accepted devtools candidate with its Foundation formatting, tests and strict Clippy, plus relevant UI/contract checks if touched. Runtime deployment and source acceptance are separate decisions.
6. Integrate only after independent review, exact revision/gate evidence and owner-authorized merge decision. Recheck target main immediately before integration; changed targets require reevaluation. No automatic merge/push authority is granted by this plan.
7. Revisit the larger host split, security-sensitive restored crates, composition policy amendments and graduation program only after blockers are resolved. Preserve the construction hold on new upper-layer features.
8. Reconcile queue and portfolio again after every accepted batch. Branch cleanup requires ancestry/content checks plus dirty-worktree and recovery checks; it is not part of this dispatch.

## Handoff record required from every owner

Record lane ID; owner; UTC start/update; exact checkout/branch/base/head; files changed; evidence and full command results (including failures); independent reviewer; decisions still needed; and next owner. Distinguish implemented, verified, reviewed, merged and deployed. Move to Review, not Done, when only the author's work is finished.

## Ready-to-send session prompts

### Gemini / Antigravity

Read `D:\aaroneous-devtools\AGENTS.md`, the current `governance/COORDINATION_QUEUE.md`, and this dispatch document at its absolute path. Take C65 only after Codex records your claim and exact devtools report worktree. Review the six pinned Aaroneous candidates read-only and write only the C65 report. Reconcile scope/consumer risks, meaningful tests and merge order; do not build, edit product source, merge, push, clean up or restart services in wave one. Send findings and exact next-step scope back to Codex; Claude is your independent reviewer.

### Claude

Read `D:\aaroneous-devtools\AGENTS.md`, the current `governance/COORDINATION_QUEUE.md`, and this dispatch document at its absolute path. Take C66 only after Codex records your claim and exact devtools report worktree. Review the pinned integration/host-split branches, restored plugin-loading surface, policy conflicts and stashed composition blueprint read-only. Write only the C66 report, distinguish confirmed defects from proposed safeguards, then independently review Gemini's C65 handoff. Do not edit product source, merge, push, start builds or restart Qwen. Send evidence and blockers to Codex.

### Codex continuation

Claim C64 in the canonical queue before writing its report. Reconcile the three devtools tips and all historical queue dispositions, retaining source-qualified IDs and all uncertain evidence. Check current review/backpressure facts before preparing any Qwen task. Coordinate acknowledgements, serialize queue writes and heavy builds, and obtain independent review; do not infer authority to merge, deploy or discard work from the owner's request to coordinate.
