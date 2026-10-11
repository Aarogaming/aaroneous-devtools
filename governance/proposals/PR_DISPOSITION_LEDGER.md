# Disposition Ledger: Active Open Pull Request Audit (48 PRs)

- **Date**: 2026-10-11
- **Status**: Audit & Staging Plan
- **Baseline Open PR Count**: 48
- **Objective**: Establish exact, head-bound dispositions (*merge, consolidate, archive, supersede, retain*) across all open pull requests to eliminate the backlog non-destructively without losing commit history, findings, or coordination integrity.

---

## 1. Operating Rules for Backlog Execution

1. **Explicit Supersession Records**: A PR is never closed without citing the exact surviving PR, target commit hash, and linking unresolved audit findings (e.g. #195 carrying #175's audit findings).
2. **Preserve Complete Source Branches/Heads**: Consolidations retain source branch pointers and head hashes in a committed manifest; no code or history is squashed away silently.
3. **Draft and Failure Hygiene**: Draft status, missing evidence, and reproduced test failures must be resolved before a PR moves to the merge pipeline.
4. **No Direct Commits to Main**: All archival of historical experiments occurs via reviewed PRs subject to the standard gate.

---

## 2. Complete Disposition Matrix (48 PRs)

### Group A: HUD v2 Proposal Documentation Bundle (16 PRs $\to$ Consolidate into #159)
*Strategy: Preserve source branch references in `governance/proposals/hud/README.md`, cherry-pick/merge the distinct document commits into umbrella branch `proposal/hud-v2`, and conduct formal review on the consolidated head.*

| PR # | Head Ref OID | Branch Name | Title | Disposition | Remaining Conditions / Target |
|---|---|---|---|---|---|
| #159 | `fee00c5269` | `proposal/hud-v2` | HUD v2 umbrella proposal | **Retain & Consolidate** | Target umbrella branch for HUD docs; needs formal review on final consolidated head |
| #163 | `8b2135e31d` | `proposal/hud-01-settings-and-permissions` | HUD 01 Settings & Permissions | **Consolidate** | Merge commit onto #159; supersede once verified on #159 |
| #164 | `4f2a0560a9` | `proposal/hud-02-agent-door-and-mcp` | HUD 02 Agent door & MCP | **Consolidate** | Merge commit onto #159; transfer Confused Deputy finding to #159 |
| #165 | `713c9d9b93` | `proposal/hud-03-lan-roster-and-domains` | HUD 03 LAN roster & domains | **Consolidate** | Merge commit onto #159; supersede once verified on #159 |
| #166 | `7d6cee08ad` | `proposal/hud-04-token-optimization` | HUD 04 Token optimization | **Consolidate** | Merge commit onto #159; MiMo accepted head `08bf7bc1` (verify head delta) |
| #167 | `72536d4302` | `proposal/hud-05-model-profiles-and-prompt-adaptation` | HUD 05 Model profiles & adaptation | **Consolidate** | Merge commit onto #159; address MiMo inverse adapter check finding |
| #168 | `13de7ae02c` | `proposal/hud-06-brief-compiler-and-packets` | HUD 06 Brief compiler & packets | **Consolidate** | Accepted by Gemini on board; merge commit onto #159 |
| #169 | `87454c9f8d` | `proposal/hud-07-guild-workers-staff-and-inspection` | HUD 07 Guild workers & staff | **Consolidate** | Merge commit onto #159; MiMo accepted head `ebf055cf` |
| #170 | `31eeffc031` | `proposal/hud-08-roles-skills-and-trust` | HUD 08 Roles, skills & trust | **Consolidate** | Accepted by Gemini on board; merge commit onto #159 |
| #171 | `91c06b65bf` | `proposal/hud-09-visualizer-exploration-and-twin` | HUD 09 Visualizer & twin | **Consolidate** | Accepted by Gemini on board; merge commit onto #159 |
| #172 | `d2746de804` | `proposal/hud-10-native-app-operations-and-reuse` | HUD 10 Native app & reuse | **Consolidate** | Merge commit onto #159; MiMo accepted head `3e93c35e` |
| #173 | `588e4d6556` | `proposal/hud-11-hall-host-election-and-handoff` | HUD 11 Hall host election | **Consolidate** | Merge commit onto #159; transfer Sybil capability spoofing finding |
| #174 | `c322498185` | `proposal/hud-12-compliance-and-graduation` | HUD 12 Compliance & graduation | **Consolidate** | Merge commit onto #159; supersede once verified on #159 |
| #176 | `28b5c3eb40` | `proposal/hud-13-card-schema-retrieval-and-plan-graphs` | HUD 13 Card schema & plan graphs | **Consolidate** | Merge commit onto #159; supersede once verified on #159 |
| #177 | `2db5570dc9` | `proposal/hud-14-operating-review-gaps-and-bottlenecks` | HUD 14 Operating review | **Consolidate** | Merge commit onto #159; supersede once verified on #159 |
| #181 | `4d48686d95` | `proposal/hud-15-red-team-review` | HUD 15 Red-team review | **Consolidate** | Merge commit onto #159; records red-team findings |
| #182 | `708ca1782c` | `proposal/hud-16-rolling-window-critique-and-boundaries` | HUD 16 Rolling-window pipeline critique | **Consolidate** | Merge commit onto #159; aligns with master pipeline charter |

---

### Group B: Claim & Implementation Paired Sets (8 PRs $\to$ Consolidate Claims into Code PRs)
*Strategy: Verify ownership, fold queue claims into implementation commits, and close claim-only PRs with explicit supersession trailers.*

| Claim PR | Implementation PR | Topic / Lane | Disposition | Remaining Conditions |
|---|---|---|---|---|
| #193 (`81b6d9680b`) | #195 (`88d82f8bbd`) | C120: HUD Core Settings | **Supersede #193 into #195** | #195 replaces closed #175; Codex finding from #175 (malformed startup restoring defaults flips Deny to Allow) must be verified fixed on #195 |
| #192 (`0fde06d148`) | #194 (`57f555967f`) | C124: Attestation `--command` | **Supersede #192 into #194** | Retain claim row in #194; verify test suite passes |
| #190 (`1646847f75`) | #191 (`d013a3b091`) | C126: OpenCode Seat / MiMo Model | **Supersede #190 into #191** | Verify dual spelling resolution and lease continuity before merge |
| #185 (`cdab7cd241`) | #186 (`dd06977ff8`) | C125: Model Family Independence | **Supersede #185 into #186** | Codex `needs_revision` on #186 (ubuntu-worker-01 ambiguity vs fallback) must be resolved |

---

### Group C: Active Fixes, Governance & Gate Hardening (10 PRs $\to$ Fast-Track Verification)
*Strategy: Address exact blocker conditions per PR before moving to merge.*

| PR # | Head Ref OID | Title | Status / Disposition | Exact Blocker / Next Action |
|---|---|---|---|---|
| #95 | `4e328b348d` | C97 Attestation SHAs must be 40-hex | **Merge-Ready** | Accepted by Codex; ready for auto-merge |
| #105 | `af5ab86fab` | C100 Seat Unicode Lookalike Check | **Review-Ready** | Run tests locally; needs 2 independent signed accepts |
| #109 | `81d2bcb5b2` | C100 merge_gate --fetch refesh | **Review-Ready** | Verify git fetch refspec isolation |
| #125 | `4ebe1b91aa` | C100 Nested Attestation Validation | **Draft / Retain** | Author must mark Ready for Review; verify reader visibility |
| #126 | `2aa36d27be` | C100 Report Lane Row Collisions | **Draft / Retain** | Accepted by Codex, but currently in Draft state; author must mark Ready and resolve ownership |
| #140 | `2c641656cb` | C97 Attestation Reviewer Validation | **Needs Revision** | Reproduced failure: `ubuntu-worker-01` canonicalization mismatch; waiting on resolver fix |
| #141 | `0ecbdb7a49` | C99 Column-Aligned Seat Declaration | **Review-Ready** | Run differential test suite; check comment attribution |
| #184 | `37de81b96c` | Owner Delegation Direction & Ladder | **Needs Revision** | Needs lane ID de-duplication (C124 collides with #178/#192) and rebase onto main |
| #188 | `dd5a1f40c2` | C127 Python Tooling Retirement | **Merge-Ready** | Accepted by Gemini; needs second independent signed accept |
| #189 | `508b9e0472` | Owner-Absent Governance Sweep | **Review-Ready** | Verify doc diff across 27 files against binding decisions |
| #162 | `e75acdc4f1` | Seat Status Refresh Test Safety | **Review-Ready** | Verify ubuntu-worker-01 suite passes without regression |
| #161 | `634c8252fb` | C91 Worker IO Claim | **Review-Ready** | Verify worker queue boundaries |

---

### Group D: Historical Experiments & Stale Scopes (8 PRs $\to$ Reviewed Archive PR)
*Strategy: Inventory unique code and findings, prepare a single reviewed PR adding them to `governance/archive/`, and close with supersession links.*

| PR # | Head Ref OID | Title | Original Scope | Disposition |
|---|---|---|---|---|
| #20 | `51ccfa2af0` | C78 Foundry Transition | Historical transition doc | **Archive** via reviewed PR |
| #66 | `1d3266b602` | C104 Continuum Leases Proposal | Draft lease architecture | **Archive** via reviewed PR |
| #67 | `de47fc02dd` | Constellation Operating Contract | Early agent shared playbook | **Archive** via reviewed PR |
| #84 | `366bfda597` | Python Scripts CI | CI runner for legacy scripts | **Supersede** by C127 (#188) Python retirement |
| #92 | `126333ce9f` | M11 Publication Recovery Experiment | Local worker publication test | **Archive** via reviewed PR |
| #93 | `30e56207eb` | C108 Topic Paths Experiment | Metadata relevance experiment | **Archive** via reviewed PR |
| #133 | `91e5b42e7c` | C110 Operator Guide Index | Early task-to-tool index | **Supersede** into master docs |
| #135 | `2782ecc408` | C102 Supported Operations Server | Supported server proposal | **Archive** via reviewed PR |
| #149 | `7258b6e895` | C96 PLC Feeder Claim | Burn-in claim | **Supersede** by unified PLC core contracts |

---

## 3. Net Backlog Impact

- **Total Open Today**: 48 PRs
- **Group A (HUD v2 Docs)**: -16 PRs (consolidated into #159)
- **Group B (Claim Pairs)**: -4 PRs (folded into implementation PRs)
- **Group D (Archived Experiments)**: -9 PRs (superseded or moved to archive)
- **Group C (Active Fixes & Gate Hardening)**: Drained serially to main as prerequisites pass
- **Resulting Open Active Workstreams**: **15 active PRs**
