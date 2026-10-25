# Automated Merge Policy

Owner-directed, 2026-10-04. This closes the "Raising the auto-merge bar"
open item in [`LOCAL_MODEL_AUTONOMY_POLICY.md`](LOCAL_MODEL_AUTONOMY_POLICY.md)
(`:98`/`:118`'s "not yet" framing, `:170`'s open item verbatim) and the
same blanket stance in [`COORDINATION_QUEUE.md`](COORDINATION_QUEUE.md)'s
header ("No self-certification or automatic merge authority") and
`LOCAL_MODEL_AUTONOMY_POLICY.md:98` ("Never auto-merge … for now"): until
now, every PR — however much independent evidence it carried — was parked
waiting on the owner to personally run the merge. The owner's direct instruction starting this
document: the goal is to build a real, mechanical, bounded process that
takes the owner out of that loop for the categories of change where it's
safe to, not to relax review standards. This document is that process, not
a request to relax them — it replaces "ask the owner every time" with
"meet a bar that's at least as strict, check it mechanically, and only
then merge," matching the Tier 0 principle this repo already runs on:
nothing bypasses deterministic verification, a model never grades its own
work, and independent review stays mandatory. What changes is *who* presses
merge once every one of those conditions has already been met in public,
retained evidence.

**Scope, owner decision 2026-10-07, reaffirmed 2026-10-09.** This policy's
automation applies only within `Aarogaming/aaroneous-devtools`.
`Aarogaming/aaroneous` (the product) is never eligible for automatic
merging under this document, regardless of what path or kind of change is
involved — it stays entirely owner-gated until a devtools-built capability
graduates into it through the component-graduation process in
`LOCAL_MODEL_AUTONOMY_POLICY.md`, at which point *that* merge is the
owner's checkpoint. Auditing Aaroneous (reading it, reporting drift,
proposing a fix PR) is in scope for the cadence in
[`RECURSIVE_AUDIT_CADENCE.md`](RECURSIVE_AUDIT_CADENCE.md); merging into it
automatically is not, ever, under this document. This is the one absolute
boundary this document does not relax, below or anywhere else in it.

**Amendment, owner decision 2026-10-11.** The "never/ever" above is a
**phase boundary, not a permanent one** (owner, direct: Aaroneous owner-gating
exists "for early development and at least until the devtools pipeline and
Aaroneous are capable of automating the process"; recorded in `DECISIONS.md`).
Enforcement is unchanged today — product merges remain owner-pressed — but the
boundary lifts by a recorded owner decision once the pipeline can automate
product merges end-to-end with this document's evidence bar intact; the
mechanism and its graduation order are defined in
[`proposals/OWNER_DELEGATION_LADDER.md`](proposals/OWNER_DELEGATION_LADDER.md).
The same decision removes owner participation from the devtools pipeline
entirely ("devtools ... should not require owner anywhere"): routine devtools
work is not to be routed through the owner.

## Why gate here instead of relaxing the evidence bar

The existing process (draft PR + CI + independent review packets, see
`CROSS_AGENT_COORDINATION_PROTOCOL.md` and `REVIEW_RUBRIC.md`) already
produces everything a merge decision needs: green CI, a diff, and one or
more independent accept verdicts with retained commands/exit-codes. The
only thing it doesn't produce is the merge itself. So this policy adds a
mechanical "is every required signal actually present and genuine"
checker, and automates pressing the button only once that checker passes —
it does not lower how much evidence is required, and it does not let
any party certify its own work (self-certification stays banned exactly as
`CROSS_AGENT_COORDINATION_PROTOCOL.md` already requires).

## No path-based owner gating (revised, owner decision 2026-10-09)

An earlier version of this document split changes into "Tier A" (anything
touching a `kernel`-profile crate, generated `unsafe` code, branch
protection, any CI/CD workflow file, credentials, dependency-admission
verdicts, any repository "constitution" file — `AGENTS.md`, `SECURITY.md`,
`CONTRIBUTING.md` — any settings/permissions file under `.github/`, or
this policy document itself — never auto-merged, always the owner) and
"Tier B" (everything else, auto-mergeable once the mechanical conditions
below passed). `scripts/merge_gate.py` enforced Tier A as a blocking
checklist box and a `decide()` condition (`TIER_A_PATHS`/`PROTECTED`,
`touches_protected()`, `tier_of()`, the former box 0, and condition f).

The owner's direct, repeated, forceful correction: *"I DO NOT want this
system dependent on me, and I DO not want to manually review ANYTHING
going on in devtools. I only need to review things for aaroneous.
devtools should be the automation system behind Aaroneous, and in turn
devtools needs to be automated."* And, when an owner-review requirement
(`CODEOWNERS` for `aaroneous-devtools`) was drafted anyway: *"the entire
time you were building those I kept telling you to stop including me in
the loop on PR's. Those are there FROM YOU"* — the Tier A split, like the
trust-building counter the 2026-10-07 entry below removed, was this
document's own author being cautious beyond what was asked for, not a bar
the owner set.

**Tier A is removed. There is no tiering.** Nothing within
`Aarogaming/aaroneous-devtools` is gated by path — not workflow files, not
`AGENTS.md`/`SECURITY.md`/`CONTRIBUTING.md`, not `governance/identities.json`,
not this policy document itself. The mechanical conditions below
(especially condition 3's ≥2-independent-accept-seat bar) are the only
gate, for every path, with no exception carved out for any file in this
repository. `scripts/merge_gate.py` no longer has a tier concept at all:
`TIER_A_PATHS`, `PROTECTED`, `touches_protected()`, `tier_of()`, the
former checklist box 0, and `decide()`'s former condition f are deleted,
not merely disabled.

This removal applies only within `aaroneous-devtools`, per the Scope
section above — `Aarogaming/aaroneous` stays entirely owner-gated
regardless of path, with no tier concept to begin with, since this
document never granted it automatic merging of anything.

## Mechanical conditions (all required, checked by tooling, not by a model's say-so)

For a given PR head commit, before anything merges it automatically:

1. **CI green on that exact head.** The real GitHub Actions run, not a
   local approximation — `check-and-test`, and for Aaroneous specifically
   also `security-audit.yml` and `semver-checks.yml` per the gates they
   cover (see Aaroneous `AGENTS.md` section 6).
2. **No merge conflict** against the current base branch tip.
2a. **Base is the repository's default branch.** A PR stacked on a
    non-default base (a sequencing branch like `codex/foundry-transition`)
    is never auto-merge-eligible, regardless of path or trust status —
    stacked-landing sequencing stays an explicit owner or author-seat
    decision, matching the steward-grant precedent in `#44`.
3. **≥2 independent accept verdicts** at that exact head, from reviewer
   seats that are not the implementer and not each other (the honour-system
   identity rule in `CROSS_AGENT_COORDINATION_PROTOCOL.md` still applies —
   this policy cannot mechanically verify seat identity beyond what that
   protocol already asks of every participant). A verdict scoped to only
   part of the diff ("delta-only") does not count unless paired with an
   earlier whole-diff accept that delta chains from, exactly as the
   existing review-packet convention already records.

   **Exception, owner decision 2026-10-11 (job-application exemption).**
   This condition does not apply to a PR whose sole changed path is
   `governance/identities.json` and whose only edits append a login or key
   to a seat that already exists in the file — the set of seat names and
   the `owner` list must be unchanged, and nothing present at the base may
   be missing at the head. A seat registering its own already-provisioned
   GitHub App login or SSH attestation key is submitting paperwork, not
   making a trust decision — the trust decision (creating the GitHub App,
   clicking Install) already happened outside the repository, by the owner.
   Admitting a brand-new seat, removing a seat, or removing/rotating an
   existing login or key is **not** covered by this exception and still
   needs the ordinary ≥2-accept bar: those are judgement calls about who is
   trusted, not paperwork. `scripts/merge_gate.py`'s
   `additive_identity_registration()` implements the check; the owner
   reasoning that motivated it: *"This was a bottleneck that must be
   addressed... The lack of available authority created a supply cutoff
   despite available demand and proper credentials... agent registration/app
   can be given a pass to be applied without review. They're registering
   keys. Like putting in a job application."* This closes the "known
   residual consideration" `DECISIONS.md`'s 2026-10-09 entry flagged and
   deliberately left open rather than acting on unilaterally.
4. **Zero open red-circle / blocking-labelled review threads** from any
   bot or human reviewer at that head. Optional/nit findings do not
   block. GitHub itself only exposes resolved/unresolved on a review
   thread, not a severity label, so the checker treats every *unresolved*
   thread as blocking by default (stricter than the stated rule, not
   looser) unless the thread's own content is legibly marked
   optional/nit per the labelling convention `CROSS_AGENT_COORDINATION_PROTOCOL.md`
   and `REVIEW_RUBRIC.md` already use.
5. **Evidence retained, not asserted.** Every accept verdict cites the
   exact command and exit code it ran, per the standing evidence rule —
   a verdict without that is not counted toward condition 3.
6. **Reversibility.** The change must be revertible with a plain
   `git revert` (no manual data migration, no irreversible external side
   effect like a published release or a credential rotation). If it
   isn't, it is not eligible for automatic merge, regardless of what else
   is true about it — this condition has no path-based carve-out.
7. **Circuit breaker clear.** `circuit_breaker_override.toml` has
   `tripped = false`. A trip blocks every automatic merge until a human
   clears it, exactly as it already blocks local-model promotion.

Any PR failing condition 1, 2, 2a, 3, 4, 6, or 7 is not eligible for
automatic merge at that head, full stop — it waits until the failing
condition is actually met (CI goes green, a second independent seat
accepts, the breaker clears, and so on), by whichever identity next makes
that happen; nothing here routes the wait to the owner specifically.
Condition 5 is a quality bar on condition 3, not a separate gate.

## Literal checklist the checker must output per PR (not a prose summary)

Every run of the mechanical checker against a PR head produces this exact
checklist, not a narrative — a check that can't be rendered as one of
`[x]`/`[ ]`/`[blocked: reason]` per line isn't a real check:

```
[ ] 1. CI green on head <sha> (run id / workflow / conclusion)
[ ] 2. No merge conflict vs. <base-branch> tip <sha>
[ ] 2a. Base branch is the repository default branch
[ ] 3. >=2 independent accept verdicts at <sha>, seats: <a>, <b>, ... (or: exempt — additive-only identities.json registration)
[ ] 4. Zero open blocking/red-circle threads (count checked: N)
[ ] 5. Each counted verdict cites command + exit code (verdict ids: ...) (or: exempt, no verdict required)
[ ] 6. Revertible via plain `git revert` (no migration/external side effect)
[ ] 7. circuit_breaker_override.toml: tripped = false
```

Any unchecked box stops the loop at "would merge, blocked on box N" — it
never rounds up to "basically ready" or merges with a box left `[ ]`. A
box the checker cannot evaluate (API error, ambiguous reviewer identity,
an unparseable CI conclusion) is rendered `[blocked: reason]`, which fails
the gate exactly like `[ ]` does; the checker never guesses a box to green
just to finish the run.

## Failure protocol — what the checker does on missing or ambiguous data

A prompt (or a checker) that's silent on what to do when something is
missing invites exactly the corner-cutting this policy exists to prevent,
so each gap gets an explicit instruction instead of being left to
judgment:

- **CI status missing or pending** → treat as fail-closed (box 1 `[ ]`),
  re-poll on the next scheduled pass. Never treat "no status yet" as
  "presumably fine."
- **Reviewer seat identity ambiguous** (two accept comments that might be
  the same underlying agent under different names) → do not count both
  toward condition 3; count one, flag the ambiguity in the checklist
  output, and let a human resolve the seat-identity question once rather
  than have every future run re-guess it.
- **An accept verdict is missing its command/exit-code evidence** → it
  does not count toward condition 3 at all (not "counts with a caveat").
- **`circuit_breaker_override.toml` itself fails to parse or fetch** →
  fail closed exactly as `promote` already does for the same file
  (`LOCAL_MODEL_AUTONOMY_POLICY.md`'s "~~Revert-triggered circuit breaker
  trips.~~ Closed (C27)" entry) — never proceed as if untripped.
In every one of these cases the checker does the stated fallback itself —
it does not pause and ask a model to decide, because the entire point of
this mechanism is a deterministic gate a model's judgment can't soften.

## Self-check pass before the checklist is trusted

Before the checklist above is treated as final for a given run, the
checker (or the agent invoking it) re-verifies against the three failure
modes real review history on this repo has actually produced: (1) a
verdict that's confirmatory language without a cited command/exit-code —
recheck condition 5 specifically; (2) a delta-only accept presented as if
it were whole-diff — recheck that condition 3's chain-from rule actually
holds for every counted verdict; (3) a reviewer seat that is the PR's own
implementer under a different label — recheck condition 3's independence
requirement. This is a second deterministic pass over the same evidence,
not a model re-reading its own conclusion and feeling more confident about
it — per `LOCAL_MODEL_AUTONOMY_POLICY.md`'s core point, a context grading
its own work is a weak signal regardless of how the re-check is framed, so
this pass must be re-derivable by a different run or a different party
from the same retained evidence, not merely re-asserted by the same one.

## No trust-building waiting period (revised, owner decision 2026-10-07)

An earlier draft of this document proposed a consecutive-clean-merge
counter (5 manual merges per category, zero reverts, before automation
turns on for it) before Tier B auto-merge went live anywhere. The owner's
direct correction: that counter was this document's author being
cautious beyond what was asked for, not a bar the owner set. Tier B
auto-merge is live immediately, for every category Tier B already covers,
with no waiting period and no per-category counter to clear.

The replacement trust mechanism is not a gate in front of merging at all —
it's [`RECURSIVE_AUDIT_CADENCE.md`](RECURSIVE_AUDIT_CADENCE.md): periodic
state/roadmap/documentation/code/compliance review, applied recursively
at every scale from a single script up through the whole repository, on a
standing schedule that doesn't depend on merge volume. It catches drift
*after* merging rather than withholding merges until a count is reached.
Quality isn't quantifiable as a threshold to cross; it's upkeep that never
stops.

## Mechanism (what actually presses the button)

Built: `scripts/merge_gate.py` (the checker — evaluates conditions 1-7
including 2a and 2b against the live repo, producing the literal
checklist above, no path-based tier anywhere in it per the section
above) and `scripts/auto_merge.py` plus `.github/workflows/auto-merge.yml`
(the pipeline — calls `merge_gate.evaluate_all()` and runs `gh pr merge`
on whatever it marks eligible; hard-refuses any repository but
`aaroneous-devtools`). What this looked like as a plan, before it
existed:

- A small checker that takes a PR number and evaluates conditions 1-7
  (including 2a) against the GitHub API, producing a pass/fail with the
  specific condition(s) that failed. (`scripts/merge_gate.py`.)
- A kill switch: the same `circuit_breaker_override.toml` mechanism
  already used for local-model promotion, extended to also block this
  checker's merge action when tripped. One file, one meaning, reused
  rather than inventing a second breaker to keep in sync.
- Something that calls this checker once a PR reaches green+accepted
  state. On pass, for any PR within `aaroneous-devtools` (see Scope,
  above), it merges — no new credential, no new always-on writer, per
  the standing rule against adding either casually. On fail, it reports
  which box failed and stops. (`auto_merge.py`'s hard repo-guard plus the
  workflow's scheduled and event-triggered runs.)
- Every automatic merge logs a line in `DECISIONS.md`: PR, head sha,
  which conditions were checked, and the two-plus accept verdicts it
  relied on — so an automatic merge has *more* retained evidence than a
  manual one typically has today, not less.

## What this does not change

- Self-certification is still banned. An implementer's own "looks good to
  me" never counts toward condition 3, automated or not.
- `Aarogaming/aaroneous` stays entirely owner-gated, with no path or
  category that becomes automatic under this document today — as amended on
  2026-10-11, this is an early-development phase boundary (lifted only by an
  explicit owner decision once the graduation criteria in
  `proposals/OWNER_DELEGATION_LADDER.md` are satisfied), not a permanent
  architectural impossibility.
- A human can still merge anything by hand at any time; this policy only
  adds a path that doesn't require them to.
- The circuit breaker, once tripped by any party for any reason, blocks
  both local-model promotion and automatic merging until a human clears
  it — this policy does not add a way around that.

## Open items

- Condition 3's ≥2-accept bar is a tightening versus today's typical
  single-accept practice (e.g. `PLATFORM_SPEC.md`'s merged convention).
  Intentional, and recorded here as an owner decision rather than left
  implicit, but worth revisiting if it turns out to be the pacing
  bottleneck now that automatic merging is live for every path.
- Who owns building the checker tool itself and on what timeline —
  tracked as a queue item, not committed to in this document. A
  report-only implementation already exists in `scripts/merge_gate.py`
  (`#51`, `--checklist` mode) and renders this policy's boxes; whether it
  becomes *the* checker or a second one gets built is still open.
- `RECURSIVE_AUDIT_CADENCE.md`'s counters (merged-PR count and elapsed
  time per scope) need their own golden vectors before a Guild/Rust port
  can reproduce them — tracked there, not here.
