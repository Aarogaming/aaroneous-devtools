---
notation: v1
purpose: A critic's review of the rolling-window pipeline proposal, a map of what the existing pipeline already covers and where it does not, and the boundaries to hold while devtools is rebased onto Aaroneous
authority: propose
status: proposal, advisory, grants nothing
relations:
  - relates-to: doc:12-compliance-and-graduation
  - relates-to: doc:14-operating-review-gaps-and-bottlenecks
  - relates-to: doc:15-red-team-review
  - relates-to: board:rebasing-and-refactoring-09-migration-slices-and-graduation
checked-by: partial: the coverage claims were checked by reading the modules and searching the code; nothing was run
---

# Rolling-window pipeline: critique, coverage and boundaries (proposal 16)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 16 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer other than the author records it.

## The request

Owner direction, 2026-10-11: review the rolling-window proposal as the critic; say what pipeline already supports it and what is missing; name the systems gaps and the module gaps;
and say where to draw the boundaries when devtools is rebased onto Aaroneous during the Codex operation.

## What was reviewed

`governance/proposals/ROLLING_WINDOW_PIPELINE.md` at commit `6b70039` on the local branch `proposal/rolling-window-pipeline` of another seat's worktree (not pushed when I read it; read through
git objects only). It describes three zones (0: pinned Aaroneous core; 1: a "rolling median buffer" of devtools, branches and worktrees; 2: outer signals) and four stages (perimeter intake, median
incubation, refinement and boundary alignment, an autonomous graduation gate). One factual note: the commit's author is recorded as the **owner's name and noreply email**, set explicitly on the
command line. A seat should commit as itself with a `Co-Authored-By` trailer; this is the impersonation problem the per-seat Apps exist to remove.

What it gets right: the direction of dependency, the idea that unfinished work may live in the middle but not cross into the core, the preference for compiled checks over model reasoning at the
perimeter, and the honest status line ("no automated promotion tooling has been implemented").

## Critique

| # | Sev. | Point |
|---|---|---|
| C1 | High | **Stage 4 contradicts the policy it sits under.** "Autonomous graduation" moves a component into the product repository, but the product repository stays owner-gated (`AUTOMATED_MERGE_POLICY.md`, `scripts/auto_merge.py`'s hard guard, the 2026-10-09 decision). What can be automated is everything **up to a complete, gate-verified candidate pull request**. The merge into Aaroneous is the one step with a different owner. The text should say so. |
| C2 | High | **Criterion 3 reverses the dependency direction.** "Cryptographic attestation from registered keys in `governance/identities.json`": that registry belongs to devtools. For the product to accept a graduation on devtools attestations it would have to trust a devtools artifact, which is the dependency the whole design forbids. The product-side evidence has to be something the product can check itself: its own gate on the PR, plus a candidate manifest it can read. Devtools attestations are devtools' internal evidence. |
| C3 | High | **"Blast radius isolated" is not true as written.** Zone 1 holds the write tokens, runs the CI that checks it, and includes the path (`agentic_registrar promote`) that pushes branches into a product checkout. The isolation is by convention, and the red-team review ([15](15-red-team-review.md)) found that the credentials of every seat sit under one OS user and that a pull request runs its own workflow definition. Isolation needs a stated mechanism or the claim should go. |
| C4 | High | **Nothing rolls.** The name promises a window and a median, but there is no window size, no work-in-progress limit, no ageing, no eviction. The failure the system has today is exactly an unbounded middle: 43 open PRs, eleven active rows against a capacity of three, 76% of leases expiring. The proposal describes where work goes and not what pushes it out. The scheduler's capacity is advisory (`schedule-review` recommends; nothing enforces). |
| C5 | Medium | **The `DEFER` gate is a string test.** "Zero unresolved `DEFER` annotations" is satisfied by deleting or rewording the tag, moving it to a sidecar, or writing `TODO`. The Codex plan already says removal proves nothing. The gate needs finding identities with dispositions (resolved with evidence, accepted with an owner, deferred with an expiry), not the absence of a word. There is also **no scanner**: the code has a `DEFERRED_FINDINGS.md` copy step in the maintenance pipeline and no `// DEFER` annotation reader. |
| C6 | Medium | **Zone 2 is untrusted and the proposal gives it no boundary.** Chat logs, issue text and upstream churn are attacker-controlled. "Regex and AST extraction into a typed card" does not say what happens to the raw payload (it is stored, retrieved and shown to models later), what size and rate limits apply, or how provenance is kept. There is no quarantine or sanitising step anywhere in the code (searched). |
| C7 | Medium | **Graduation criteria are necessary, not sufficient.** Passing the gate does not show behavioural parity, dependency admission (`alloc`, `ambient`, `abi`, `safety` scoring), licence, provenance, shadow comparison, a rollback path or consumer evidence. The product's own graduation rule requires proof in origin, a trait boundary, landing inert or in shadow, and **then** deleting the origin copy; the proposal retires the devtools wrapper on promotion. |
| C8 | Medium | **Zone 1 is one blob.** It merges scratch, frontier and the stable base. The distinction that matters (the thin stable base that must meet profile rules, versus experiments that need not) disappears, which is the monolith the Codex plan is trying to avoid. |
| C9 | Medium | **It is one-way.** Product-to-devtools movement (pin bumps, a product change that breaks a consumer, rollback after a bad graduation, Codex's "return") is absent. A pipeline that cannot go backwards has no recovery story. |
| C10 | Medium | **"No human bottleneck" names the wrong bottleneck.** The measured constraint is reviewer supply and evidence plumbing: independent accepts that do not count (a seat name read as an unknown seat), a gate box the builder cannot satisfy, reviewers who are the same model. Removing the owner from the loop has already been done; the pipeline is still slow. |
| C11 | Low | **A third vocabulary.** Zones 0/1/2, my rings (frontier, base, graduated) and the Codex categories (product-derived base, promotion candidates, devtools-specific composition, scratch) now describe the same thing three ways. Pick one. |
| C12 | Low | No cadence, deadlines or dating, and "pinned" has no pin: no stable product release exists and the Codex plan leaves selection open. |

## What the existing pipeline already covers

| Stage in the proposal | What exists now (searched or read on `origin/main`) | Covered? |
|---|---|---|
| 1 Perimeter intake | typed intake contract and validator (`validate-intake`, `generate-intake`; the latter is model-assisted), the job feeder (reads a product checkout read-only), topic-case export to intake, the steward's ingest of PR comments | Partly. No quarantine, no issue or chat ingestion, no size or rate limits |
| 2 Median incubation | immutable, digest-bound **virtual patch sets** kept outside Git until a reviewer selects one; local worker jobs; the contribution envelope that carries one identity from discovery to acceptance; `dispatch` and `promote` (isolated `local-agent/*` branches, high-risk allowlist, circuit breaker); worktrees and branches | Largely, for local-model drafts. Not for ordinary components, and no shadow comparison |
| 3 Refinement | `conformance.rs` (pure deterministic checks on drafts), `dogfood::verify_draft`, the product's `ast_auditor` (run by me on devtools crates), profile declarations (`hud_core`) | Partly. No profile-aware subset runner, no admission scoring |
| 4 Graduation gate | the registrar's `verify_gate_suite` runs the product's full `cargo xtask gate` on a checkout; the contribution envelope's independent-review interlocks; signed attestations and the merge gate for devtools PRs | Gate execution yes. **Graduation as an operation: no.** The word does not appear in the code |
| Autonomy | the automated merge pipeline for devtools, the circuit breaker, the steward's cards and leases | Yes for devtools; deliberately not for the product |

## Systems gaps

1. **A bounded window**: work-in-progress limits, ageing and eviction, enforced and not advisory.
2. **An input boundary** for Zone 2: quarantine, sanitising, size and rate limits, provenance, treated as hostile text.
3. **Finding identity and disposition**: a store where a deferred item has an id, an anchor, an owner, an expiry and a closing evidence record.
4. **Graduation as a first-class operation** with a candidate manifest, a product-side verification path, shadow comparison, a rollback step and a return path.
5. **The pin**: one recorded, checkable product baseline and a process to bump it.
6. **Reviewer supply and independence**: exact seat names, model-family and host classes, and evidence plumbing that works (lane C124 and the reviewer-rule lane).
7. **Credential and workflow boundaries** ([15](15-red-team-review.md)), without which "isolated" is a word.
8. **Flow measurement**: lead time, review latency, lease outcomes, merged-per-day, so the window can be tuned by evidence.

## Module gaps

- **Finding store** (id, anchor, disposition, expiry) and a `// DEFER` reader that feeds it.
- **Graduation module**: candidate manifest (crate, profile, dependencies and their admission scores, gate results, shadow results, rollback), a profile-aware gate subset runner built on the product's own tools.
- **Window manager**: a pure reducer for work-in-progress, ageing and eviction over the queue.
- **Intake quarantine and normaliser** for Zone 2.
- **Product pin module**: resolve, verify and record the immutable baseline; refuse path dependencies.
- **Shadow comparison runner** for two implementations behind one contract.
- From the duplication measured in [12](12-compliance-and-graduation.md): a **seats and identities** crate, one **model client**, one **queue model**, one **local server skeleton**, one **attestation module**.

## Boundaries to hold during the rebase

| # | Boundary | How it is enforced |
|---|---|---|
| B1 | **Direction.** Devtools consumes Aaroneous; Aaroneous never imports, reads or verifies devtools artifacts. | the dependency graph, and a product-side check that no devtools reference exists |
| B2 | **What may be linked.** Product libraries of `control`, `presentation` or `tooling` profile, by git revision; kernel crates only through narrow extracts. Dependencies point toward the same or a stricter profile. | the product's own dependency-direction check, run on devtools crates |
| B3 | **What may be written.** Devtools reads the product checkout read-only and writes it only through the isolated promote path to a candidate branch; the product repository changes only by its own pull request and gate, with its own owner. | the registrar's clean-tree and branch rules; the product repository's policy |
| B4 | **The gate machinery is frozen** during S0 to S2: `merge_gate`, `review_coverage`, `attest_sig`, `auto_merge`, the queue grammar and the golden vectors change only by fixes with vectors in the same pull request. Rebase the consumers, not the thing that checks them. | the golden-vector contract and its Rust port |
| B5 | **One pin.** A single recorded product baseline (revision, date, the tools used), bumped by pull request; no path dependencies; scratch probes stay in their own workspaces until adopted. | a pin file and a check |
| B6 | **Credentials.** No Zone 1 operator holds product write access; seats on one host count as one trust domain until credentials are separated ([15](15-red-team-review.md)). | the credential separation work; the reviewer-family rule |
| B7 | **Capacity.** At most three primary scopes at once; heavy verification and the GPU reserved separately. Entering the rebase is conditional on open rows being at or below capacity. | the scheduler, made enforcing |
| B8 | **Interchange proof.** A module is replaceable only when a second implementation passes the same contract tests; the old copy goes after a consumer proves the new one. | shared vectors, as the steward port already does |
| B9 | **Evidence direction.** Product-side acceptance rests on evidence the product can reproduce (its gate, its CI). Devtools attestations stay devtools' internal record. | the graduation manifest |
| B10 | **One vocabulary.** Zone 2 is input; Zone 1 is split into scratch, frontier and base; Zone 0 is Aaroneous; graduation moves a base component across B1/B3 and nothing else does. | this table |

## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| W1 | correct the proposal text (C1, C2, C3, C11) and push it as the seat that wrote it | the author seat |
| W2 | window manager reducer with enforcement of capacity (C4) | S0, 14 |
| W3 | finding store and the `// DEFER` reader (C5) | the Codex annotation slice |
| W4 | intake quarantine (C6) | 13 |
| W5 | graduation manifest and profile-aware gate subset (C7, B9) | 12 |
| W6 | the product pin and return path (B5, C9) | S1 |

## Decisions for this feature

Made by the author seat as an operator on the owner's standing direction (devtools decisions do not wait for the owner); each becomes real only when a reviewer other than the author records it.

1. **Adopt B1 to B10 as the working boundaries for the rebase.** Hold B4 (freeze the gate machinery) hardest.
2. **Rewrite Stage 4 as "automated up to a verified candidate pull request"**, with the product's own gate and owner for the merge. The word "autonomous" stays for devtools and does not apply to Aaroneous.
3. **One vocabulary:** Zone 2, then Zone 1 as scratch, frontier and base, then Zone 0. Retire "rolling median" unless a window is defined.
4. **Do not start W2 to W6 until the open rows are at or below capacity**, except the proposal text fix.

## Review focus

- Is C1/C2 right that attestations cannot be the product-side evidence, and what should the product check instead?
- Is B4 (freeze the gate) the right constraint, and does it block anything that cannot wait?
- Is a bounded, enforced window the right interpretation of "rolling", or is the intent something else?

## Not verified

- I did not run anything. The coverage table comes from reading module headers and searching the code on `origin/main` at `c78cf64` plus the merged identity chain; a module may do more than its header says.
- That the product repository has no existing check against references to devtools.
- That the proposal's local branch has not changed since commit `6b70039`.
