# Change Verification Tiers

## Purpose

This document supports the goal of developer-emulation-grade automation described
in `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`: getting as much of the
research/discovery/drafting/verification loop automated as possible, with every
increase in automated scope matched by an increase — not a decrease — in testing,
safety, and verification rigor. It defines how much evidence a change needs before
it is *considered mergeable*, scaled to the change's blast radius. It does not
grant any agent standing authority to act on that evidence alone; what to do with a
verified change remains a live decision made in the session doing the work, under
whatever confirmation rules that session already operates under.

This is the same instinct behind `governance/EVIDENCE_STANDARD.md` (no status label
without concrete evidence) applied to change risk instead of roadmap status: more
automation is safe exactly to the extent the verification bar under it is real and
enforced, not vibes.

## Tiers, by blast radius of the change

### Tier 0 — Governance, planning, and documentation

Plan documents, roadmap/portfolio entries, non-code governance text.

- **Verification required:** referenced file paths resolve; no fabricated status
  claims (`EVIDENCE_STANDARD.md` applies to any status label used); consistent with
  neighboring governance docs.

### Tier 1 — Isolated, low-risk product or devtools code

Code that does not touch `unsafe`, FFI boundaries, credential/token handling,
sandbox/containment boundaries, or the security surfaces named in `SECURITY.md`
(workspace path containment, the micro-VM, constant-time auth, named pipe client
restriction, HID failsafe, desktop sandbox, integrity monitoring). Examples:
isolated new components, test fixtures, internal refactors with no security-surface
contact.

- **Verification required:** a clean `cargo xtask gate` run, a clean `ast_auditor`
  pass, `typos` clean, and (once M44 lands) `cross_verifier`/`consensus_engine`
  acceptance — all run against the actual change, not inferred. A logged record of
  what was checked and what passed, so the evidence trail is inspectable after the
  fact, matching `EVIDENCE_STANDARD.md`'s existing discipline.

### Tier 2 — Security-relevant or memory-safety-relevant code

Anything touching `unsafe`, FFI/ABI boundaries, the sandboxed micro-VM, HID
interception, path containment/traversal checks, authentication/credential
handling, named pipe access control, or any surface listed in `SECURITY.md`'s
Security Architecture section.

- **Verification required:** everything Tier 1 requires, plus a second,
  independently-run review pass specifically tasked with finding reasons the
  change is wrong rather than confirming it looks right, and an explicit check that
  the change does not weaken any named `SECURITY.md` boundary (or, if it
  intentionally changes one, that the change updates `SECURITY.md` in the same
  change). Two passes over the same reasoning are not a substitute for independent
  expert review; treat this tier's bar as the best available check given current
  reviewer constraints, not as equivalent to one.

## Ambiguity defaults to the higher tier

If it is unclear whether a change belongs in Tier 1 or Tier 2, treat it as Tier 2
and gather the stronger evidence. Getting more verification than a change strictly
needed is cheap; getting less than a security-relevant change needed is not.

## Relationship to other governance documents

- `governance/LOCAL_AGENT_CONTROL_PLANE.md` — the local worker's constraints
  (never claims work, never writes, never decides verification passed; output is
  always a proposal) are unaffected by this document and remain in force
  regardless of tier.
- `governance/EVIDENCE_STANDARD.md` — its evidence discipline for status labels
  applies inside every tier above.
- `governance/COORDINATION_QUEUE.md` — unresolved verification disagreements on a
  claim stay `Blocked` there rather than proceeding on an unresolved tier question.
- `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md` — M44's local pre-proofing and
  M45's cratification path are the mechanisms that actually produce the Tier 1/2
  evidence this document requires.
