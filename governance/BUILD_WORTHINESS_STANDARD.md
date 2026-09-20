# Build-Worthiness Standard

## Purpose

`contracts/FRONTIER_FILTRATION_STANDARD_V1.md` already turns "is this external
crate worth adopting" into an explicit, scored rubric instead of a vibe. This
document generalizes the same method to a question that standard doesn't cover:
**is this internally-generated idea worth building at all** — a candidate from
the capability composition explorer (`architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`
M47), a local-worker proposal, or any other source that isn't an external crate.
This is the "judgement, document-driven" rubric discussed in planning: the owner
delegates the *judgment call itself* to a fixed, written scoring method, the same
way `EVIDENCE_STANDARD.md` already delegates status-label judgment to fixed
evidence criteria.

This standard decides **roadmap admission** (should this enter
`governance/MULTITHREAD_PROGRAM.md`'s portfolio at all, and at what priority) — it
is upstream of and separate from `governance/CHANGE_VERIFICATION_TIERS.md` and
`governance/REVIEW_CONDITIONS.md`, which decide how much verification an
already-admitted change needs. An idea passes through this standard once, before
work starts; a change passes through the verification tiers every time it's
evaluated for merge.

## Stage 1: Admissibility gates (hard disqualifiers)

Mirrors `FRONTIER_FILTRATION_STANDARD_V1.md`'s Stage 1 — these do not score
quality, they disqualify outright:

| Gate ID | Invariant Gate | Pass Requirement | Rejection Condition |
| --- | --- | --- | --- |
| **B1** | `ScopeClarity` | The idea can be stated as a bounded change with identifiable file/module scope | Cannot be described without also redesigning something else first (route it through an RFC instead, per `WORKLIST.md` B4's own pattern) |
| **B2** | `GovernanceCompliance` | Does not require weakening a named `SECURITY.md` boundary, and does not require expanding local-worker authority beyond `LOCAL_AGENT_CONTROL_PLANE.md` | Any candidate that can only be realized by loosening an existing safety boundary is disqualified, not scored |
| **B3** | `Non-Duplication` | Not already `Done`, `In Progress`, or `Retired` for a documented reason in `MULTITHREAD_PROGRAM.md` | Re-proposing already-settled or already-retired work without new evidence |
| **B4** | `VerifiabilityFloor` | A plausible path exists to prove it works using the existing gate stack (`cargo xtask gate`, Foundation CI, or the equivalent for the repository it targets) | No conceivable automated verification path (e.g. depends entirely on subjective human taste with no test surface) |

## Stage 2: Quadratic scoring (candidates passing Stage 1)

Scored `0.0`–`10.0` per quadrant, same composite-score math as
`FRONTIER_FILTRATION_STANDARD_V1.md` so the two standards stay comparable and a
reviewer doesn't have to learn two different scoring languages:

### Quadrant I: Capability value ($w_1 = 0.30$)
How much new leverage does this add — a new capability, a closed gap, a real
efficiency gain — versus how marginal or redundant with existing capability it is.

### Quadrant II: Implementation feasibility ($w_2 = 0.30$)
Same spirit as `FRONTIER_FILTRATION_STANDARD_V1.md`'s Quadrant II, applied to a
from-scratch idea instead of a refactor: how cleanly does it fit the existing
architecture (Ring placement, dependency direction, `CONTRIBUTING.md` constraints)
without requiring invasive changes elsewhere first.
- `9.0–10.0`: Isolated, additive, no existing-code changes required.
- `7.0–8.9`: Touches one or two existing modules in a bounded way.
- `4.0–6.9`: Requires coordinated changes across several modules or both
  repositories.
- `0.0–3.9`: Requires redesigning something load-bearing first — not implementable
  as a single bounded change.

### Quadrant III: Verification rigor achievable ($w_3 = 0.20$)
How strong a proof of correctness the existing gate stack can actually produce for
this idea — a hot-path change with property tests scores higher than something
whose correctness is fundamentally a judgment call with no test surface.

### Quadrant IV: Roadmap leverage ($w_4 = 0.20$)
Direct utility for an active milestone or documented goal (see
`architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`'s milestones, or
`MASTER_ROADMAP.md`'s current horizon) versus a tangential nice-to-have.

## Score calculation (identical method to Frontier Filtration)

$$\text{Linear Score} = \sum_{i=1}^{4} w_i Q_i$$

$$\text{Defect Penalty} = \lambda \sum_{i=1}^{4} (10.0 - Q_i)^2, \quad \lambda = 0.05$$

$$\text{Composite Score} = \max\left(0.0, \; 10.0 \cdot \text{Linear Score} - \text{Defect Penalty}\right)$$

## Stage 3: Admission outcome

| Composite Score | Outcome | Action |
| --- | --- | --- |
| **80.0–100.0** | `ADMIT_TO_READY` | Enters `MULTITHREAD_PROGRAM.md`'s priority portfolio directly at `P1`/`P2` per normal intake rules. |
| **60.0–79.9** | `ADMIT_WITH_SCOPING` | Needs a scoping pass first (see `governance/ROADMAP_RECONCILIATION_PLAN.md`'s M24 split as the template for "too large/unclear to claim as one lane") before it enters the portfolio. |
| **40.0–59.9** | `ARCHIVE_AS_ROADMAPPED` | Recorded with `Dormant/Roadmapped` status per `EVIDENCE_STANDARD.md` — kept, not scheduled, revisited only if a later score or a changed dependency raises it. |
| **< 40.0** | `REJECT_LOG_REASON` | Not built. Logged with its score and the reason, in the same spirit as M46's archived rejected exploration-group siblings — so a later session doesn't blind-re-propose an already-scored-and-rejected idea. |

## Relationship to other governance documents

- `contracts/FRONTIER_FILTRATION_STANDARD_V1.md` — same scoring method, for
  external crate candidates instead of internal ideas. The two standards should
  stay numerically comparable; if one's weights or penalty constant change, revisit
  whether the other should too.
- `governance/EVIDENCE_STANDARD.md` — status labels used in Stage 3 outcomes
  (`Dormant/Roadmapped`, etc.) follow its evidence discipline.
- `governance/CHANGE_VERIFICATION_TIERS.md` / `governance/REVIEW_CONDITIONS.md` —
  apply *after* admission, to the actual change; this document only decides
  whether work starts at all.
- `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md` M47 — the capability
  composition explorer is the primary source of candidates this standard is meant
  to score; M46's exploration-group archival is the mechanism for Stage 3's
  `REJECT_LOG_REASON` outcome.
