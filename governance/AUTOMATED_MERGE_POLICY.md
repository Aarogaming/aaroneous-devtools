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

## Eligibility tiers

**Tier A — never auto-merged, always the owner.** Anything touching a
`kernel`-profile crate (per Aaroneous `AGENTS.md` section 1), anything
containing generated `unsafe` code, anything changing branch protection,
**any CI/CD workflow file, unconditionally** (added, modified, or
removed — there is no carve-out for a new workflow file, below or
anywhere else in this document), credentials, dependency-admission
verdicts, or this policy document itself. Matches the existing high-risk
allowlist pattern in `LOCAL_MODEL_AUTONOMY_POLICY.md`'s Tier 2. No
trust-building period changes this tier — it is a permanent floor, not a
starting point. (A new workflow file is exactly the case Aaroneous
`AGENTS.md` section 6 flags post-#89: adding one without its matching
local gate and drift test silently reopens the CI/local-gate coverage gap
that section exists to close, so it stays owner-only rather than earning
a carve-out.)

**Tier B — auto-mergeable once the checks below pass.** Everything else:
documentation, governance process docs, `control`/`presentation`/`tooling`
profile crates, test-only files, Dependabot version bumps already covered
by `cargo-semver-checks`. This is the category this policy actually
activates automation for.

## Mechanical conditions (all required, checked by tooling, not by a model's say-so)

For a given PR head commit, before anything merges it automatically:

1. **CI green on that exact head.** The real GitHub Actions run, not a
   local approximation — `check-and-test`, and for Aaroneous specifically
   also `security-audit.yml` and `semver-checks.yml` per the gates they
   cover (see Aaroneous `AGENTS.md` section 6).
2. **No merge conflict** against the current base branch tip.
2a. **Base is the repository's default branch.** A PR stacked on a
    non-default base (a sequencing branch like `codex/foundry-transition`)
    is never auto-merge-eligible, regardless of tier or trust status —
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
4. **Zero open red-circle / blocking-labelled review threads** from any
   bot or human reviewer at that head. Optional/nit findings do not block.
5. **Evidence retained, not asserted.** Every accept verdict cites the
   exact command and exit code it ran, per the standing evidence rule —
   a verdict without that is not counted toward condition 3.
6. **Reversibility.** The change must be revertible with a plain
   `git revert` (no manual data migration, no irreversible external side
   effect like a published release or a credential rotation). If it isn't,
   it's Tier A regardless of what else is true about it.
7. **Circuit breaker clear.** `circuit_breaker_override.toml` has
   `tripped = false`. A trip blocks every automatic merge across every
   tier until a human clears it, exactly as it already blocks local-model
   promotion.

Any PR failing condition 1, 2, 2a, 3, 4, 6, or 7 is not eligible for
automatic merge at that head, full stop — it waits for the owner like
every PR does today. Condition 5 is a quality bar on condition 3, not a
separate gate.

## Literal checklist the checker must output per PR (not a prose summary)

Every run of the mechanical checker against a PR head produces this exact
checklist, not a narrative — a check that can't be rendered as one of
`[x]`/`[ ]`/`[blocked: reason]` per line isn't a real check:

```
[ ] 1. CI green on head <sha> (run id / workflow / conclusion)
[ ] 2. No merge conflict vs. <base-branch> tip <sha>
[ ] 2a. Base branch is the repository default branch
[ ] 3. >=2 independent accept verdicts at <sha>, seats: <a>, <b>, ...
[ ] 4. Zero open blocking/red-circle threads (count checked: N)
[ ] 5. Each counted verdict cites command + exit code (verdict ids: ...)
[ ] 6. Revertible via plain `git revert` (no migration/external side effect)
[ ] 7. circuit_breaker_override.toml: tripped = false
[ ] 8. Category <pattern> has cleared trust-building threshold (X/5)
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
- **Trust-building counter for a category is missing or ambiguous**
  (e.g. the category pattern matches files from two different declared
  categories) → treat as not-yet-cleared (box 8 `[ ]`), report the
  ambiguity, and let a human either split the pattern or declare which
  category governs, rather than averaging or picking the looser one.

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

## Trust-building period before this applies to any real category

Per `LOCAL_MODEL_AUTONOMY_POLICY.md`'s own deferred criterion: before
Tier B auto-merge is *live* for a given file-pattern category (e.g.
`governance/*.md`, or `**/tests/*.rs`), that category needs a logged
track record — a specific number of consecutive PRs in that category that
met all seven conditions above and were merged by the owner manually with
zero reverts afterward. Proposed starting threshold: **5 consecutive
clean manual merges in that category, zero reverts, logged in
`DECISIONS.md`**, before automatic merging is turned on for it. This
number is deliberately conservative and owner-adjustable; it exists so the
first automatic merge in any category is never the first time that
category's evidence pattern has been checked.

Until a category clears that bar, the mechanism below still *runs* (so its
checks get exercised and proven against real PRs) but only *reports*
"would merge" instead of merging — logged visibly so the gap between
"ready" and "merged" is itself evidence toward the threshold, not silence.

## Mechanism (what actually presses the button)

This is infrastructure to build, not yet built as of this document:

- A small checker (xtask subcommand in Aaroneous, or a script here) that
  takes a PR number and evaluates conditions 1-7 against the GitHub API,
  producing a pass/fail with the specific condition(s) that failed.
- A kill switch: the same `circuit_breaker_override.toml` mechanism
  already used for local-model promotion, extended to also block this
  checker's merge action when tripped. One file, one meaning, reused
  rather than inventing a second breaker to keep in sync.
- The cloud-agent PR-babysitting loop (already running today) calls this
  checker once a PR it's driving reaches green+accepted state. On pass
  *and* the category has cleared its trust-building threshold, it merges
  via the same GitHub access it already uses to push fixes — no new
  credential, no new always-on writer, per the standing rule against
  adding either casually. On pass but category not yet trusted, it posts
  "would auto-merge once trust period X/5 clears" and stops, same as it
  does today.
- Every automatic merge logs a line in `DECISIONS.md`: PR, head sha,
  which conditions were checked, and the two-plus accept verdicts it
  relied on — so an automatic merge has *more* retained evidence than a
  manual one typically has today, not less.

## What this does not change

- Self-certification is still banned. An implementer's own "looks good to
  me" never counts toward condition 3, automated or not.
- Tier A changes are never automatic, with no trust-building path to
  make them automatic later — that's a permanent floor, revisit only by
  explicit owner decision recorded here.
- A human can still merge anything by hand at any time; this policy only
  adds a path that doesn't require them to.
- The circuit breaker, once tripped by any party for any reason, blocks
  both local-model promotion and automatic merging until a human clears
  it — this policy does not add a way around that.

## Open items

- Exact category list for the first trust-building cohorts (proposed
  starting point: `governance/*.md` docs here, and `tests/*.rs`-only
  diffs in Aaroneous, mirroring the "narrow, proven-safe category"
  example — new test-only files — in `LOCAL_MODEL_AUTONOMY_POLICY.md`'s
  "Why no auto-merge yet" section, not Tier 1, which is the unrelated
  local-worker-dispatch tier).
- Whether the 5-clean-merge threshold should vary by category risk, or
  stay uniform for simplicity until there's evidence either way.
- Condition 3's ≥2-accept bar is stricter than today's typical
  single-accept practice; since each of the 5 trust-building merges
  needs 2 accepts, a category will clear its threshold slower than
  recent merge history alone would suggest. Worth it for the bar this
  closes, but the pacing is real and owner-adjustable if it's too slow.
- Who owns building the checker tool itself and on what timeline —
  tracked as a queue item, not committed to in this document.
