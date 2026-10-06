---
notation: v1
purpose: Proposal for transparent-by-design notation, meta-tagging and a graph/retrieval layer over Aaroneous and aaroneous-devtools
authority: propose
status: proposal, advisory, grants nothing
relations:
  - informs: pr:Aarogaming/aaroneous-devtools#20@4ffeb68049b96893bb7884b5932a653f33c44ac6
  - informs: doc:architecture/SYSTEM_COMPOSITION_BLUEPRINT.md
  - depends-on: doc:governance/PLATFORM_SPEC.md
checked-by: none yet (see "Experiments"; most are NOT RUN)
---

# Transparent-by-design notation, meta-tagging and the graph layer (proposal)

Contribution-ID: C101/claude-notation-proposal-v1
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, no Aaroneous file, no workflow and no policy. Reviewers
are invited to challenge and improve any part; nothing here is accepted until a reviewer other than the author
records it, and product-side adoption needs owner sign-off.

## 1. Where this comes from

Owner direction, 2026-10-05: add notation to code and documentation; use it with an active meta-tagging system,
DAGs, retrieval (RAG) and graphing; start with a transparent-by-design base requirement. Four decisions were made
in chat the same day and are inputs, not open questions:

1. Code notation is written as structured Rust doc attributes; Markdown documents use front matter.
2. Aaroneous owns a neutral notation spec; devtools only consumes it (Aaroneous never depends on devtools).
3. Enforcement is a ratchet that starts at crate level (a warning with a baseline that may only decrease).
4. Retrieval is structural first; semantic search is only a hint and never settles a fact.

Round Table entries 11, 12 and 13 (topic nodes as case files; Gemini's constraints on edge validation and
conflicting edges; Codex's separation of resolution from support and its evidence-lineage rule) are folded in
below with credit.

## 2. The base requirement: what "transparent" means

A component is **transparent** when a reader, human or agent, can find the following from the repository at one
revision, without asking anyone:

| # | Declaration | Satisfied when | Checkable? |
| --- | --- | --- | --- |
| 1 | **Purpose and boundary** | one sentence on what it is for, plus what it must not touch | presence only |
| 2 | **Contracts** | named inputs and outputs, and the invariants it promises | yes, if each invariant cites a check |
| 3 | **Authority class** | one of `inspect`, `propose`, `verify`, `mutate` (the Foundry capability classes) | yes, against the component's real calls |
| 4 | **Evidence** | each contract points at a test, vector or gate that exercises it | yes, the target must resolve |
| 5 | **Relations** | typed edges to other components, documents, lanes and decisions | yes, targets must resolve |

Anything derived from notation names its source. A fact with no source location is not a fact in this system.

## 3. Principles

- **A note is a claim, never authority.** "This is safe" in a comment proves nothing. Only a contract that cites a
  check that resolves counts as *checked*; everything else is shown as unchecked. (The historical keyword-based
  cross-verifier is the failure to avoid.)
- **One ledger; everything else is a projection.** The node index, graph, DAGs and retrieval index are rebuilt
  from the ledger and the notation; none is a store of record (the two-ledger problem in the Foundry doc).
- **Aaroneous never depends on devtools.** Intrinsic notation is inert and neutral; it never names a devtools
  lane, node or tool. Overlay tags live in devtools and point at Aaroneous by pinned revision.
- **Typed identifiers.** Per SYSTEM_COMPOSITION_BLUEPRINT section 6, Base36 is an encoding, not identity,
  compatibility proof or authentication. Namespaces stay separate (section 6 below).
- **Fail closed, but never silently.** A rejected note or reference is reported to its author with the reason.
- **Dissent is data.** Conflicting edges are kept as distinct branches, never merged into a false consensus.

## 4. Notation

### 4.1 Rust: structured doc attributes

Grammar: `notation:v1 <key>[.<id>]=<value>`. The prefix `notation:v1 ` is reserved and everything else in a doc
attribute is ordinary documentation that the extractor ignores. `<key>` is one of the fixed keys below. The
optional `<id>` is `[a-z0-9_]+` and binds related attributes on the same item. `<value>` is the rest of the
attribute taken literally: it is one fact, may not contain a newline, and has no separators or escaping, so
there are no multi-field attributes.

```rust
#![doc = "notation:v1 purpose=Decide which work item each seat gets next"]
#![doc = "notation:v1 authority=propose"]
#![doc = "notation:v1 relation=depends-on crate:paths"]

#[doc = "notation:v1 contract.c1=same input gives the same output"]
#[doc = "notation:v1 checked-by.c1=tests::same_input_same_output"]
pub fn next_card(input: u32) -> u32 { /* ... */ }
```

Keys in v1: `purpose`, `boundary`, `input`, `output`, `contract`, `authority`, `checked-by`, `evidence`,
`relation`, `provenance`. A `contract.<id>` is **resolved** when a `checked-by.<id>` exists on the same item
and its target resolves; it is reported as **checked** only after the cited check's support for the invariant has
been established (section 7: resolving a citation is not support). A resolved contract whose check has not been
assessed is reported as resolved/unassessed, never as checked; with no resolving `checked-by` it is unchecked. The `notation:v1` prefix carries the version; an
unknown version is reported, not guessed at.

**Verified in a scratch crate (not committed):** with `#![deny(warnings)]`, doc attributes in this form (including
the one-fact-per-attribute `contract.c1` and `checked-by.c1` pair above) pass `cargo check` and `cargo test`, and
`cargo doc` renders them as visible text in the crate index and item pages.
They are inert at runtime. **Alternative, also verified in the same crate:** an attribute written
`#[cfg_attr(any(), notation(v1, authority = "read-only"))]` compiles, runs and is *not* rendered by rustdoc. The
trade-off is visibility (transparent in rendered docs) versus tidiness (machine-only). See open question 1.
NOT VERIFIED on the toolchain Aaroneous pins, with Clippy, or on a crate using procedural macros.

### 4.2 Markdown: front matter

A leading block between `---` lines, restricted to a small subset (flat `key: value` pairs and one level of list
items) so it can be parsed without a general YAML library. This document carries one as a worked example.

### 4.3 Annotation kinds

| Kind | Meaning | Rule |
| --- | --- | --- |
| descriptive | what it is for | free text, never relied on |
| contractual | what it promises | must cite `checked-by`, else unchecked |
| provenance | where it came from | typed reference |
| relation | link to another thing | closed vocabulary below |

### 4.4 Relation vocabulary (v1, closed, versioned)

`depends-on`, `implements`, `covers`, `derived-from`, `informs`, `supersedes`, `contradicts`. Adding a word is a
version bump. `depends-on` edges must form a DAG; a cycle is reported with its members and never silently
dropped. `contradicts` edges are preserved as distinct branches.

## 5. Extraction, ledger events and the graph

1. A deterministic extractor reads notation at a pinned revision and emits **events** into the one ledger:
   `note_seen`, `note_retired`, `edge_add`, `edge_retire`, `tag_add`, `tag_retire`, `ref_checked`.
2. A pure reducer (state in, state out; no clock, no I/O, per AGENTS.md section 1) folds events into the graph.
   It validates edges at projection time (Gemini, entry 12) and keeps conflicting edges as branches.
3. Views (constellation, case file, DAG) are rendered from the projection. They are rebuildable and carry no
   authority.

The reducer and the event schema are the natural next golden vectors, in the same language-neutral form as
`scripts/golden/steward/`, so a Rust implementation (the proposed Guild component) can be held to them.
Whether these events share the steward's log or the Foundry ledger is open question 4.

## 6. Identifiers and references

| Namespace | Example | Notes |
| --- | --- | --- |
| crate | `crate:paths` | the workspace package name |
| doc | `doc:architecture/MASTER_ARCHITECTURE.md` | repository-relative path |
| lane | `lane:C99` | queue lane |
| node | `node:<base36>` | a topic or case-file node; Base36 is only the encoding |
| rev | `rev:<40-hex>` | a source revision; never abbreviated in a reference |

A **reference** is how a claim points at evidence, and each type has a resolver rule:

| Reference | Resolves when |
| --- | --- |
| `path@sha:Lstart-Lend` | the file exists at that revision and the range is inside it (blob hash recorded) |
| `sym:path@sha#name` | the extractor finds that symbol in that file at that revision |
| `cmd:<id>` | a retained log exists whose digest matches, with a recorded exit code |
| `pr:owner/repo#N@head` | the PR exists and that head is or was its head |
| `event:seq#hash` | the ledger holds that event with that digest |

## 7. Grounding: references, hearsay and the echo chamber

Every claim in a case-file entry carries typed references. A deterministic resolver checks each one and records
`ref_checked` as `resolved`, `dangling` or `stale`. A dangling reference is shown as NOT VERIFIED.

A claim whose only support is another model's entry is **hearsay**. Treat citations as a directed graph over
entries. An entry's **grounding distance** is the length of the shortest citation path from it to a non-entry
reference (a revision, ledger hash, command log or PR head); it is finite exactly when such a path exists. An
entry with no such path, including every member of a cycle of entries that cite only each other, is
**ungrounded**: its distance is reported as `none`, never as a number, and it cannot be promoted automatically.
Shortest-path distance is well defined on cyclic graphs, so cycles need no special case beyond having no path to
a landmark. A display also shows how many of an entry's cited entries are themselves ungrounded. This is the
"ant mill" failure (a circle of agents following each other's traces); the cure in nature is an external
landmark, and here it is a repository revision or ledger hash that the resolver can check.

Experiment from Round Table entry 12 (Gemini, who phrased it as depth greater than 1; grounding distance
replaces depth so it is defined on cycles), NOT RUN: given a synthetic log where A cites B and B cites A with no
landmark, both are ungrounded (`none`) and promotion is blocked; adding one landmark citation to A makes both
grounded, with distances 1 (A) and 2 (B).

**Resolving is not supporting (Codex, entry 13).** A pinned reference can resolve while its contents contradict the
claim. A case file therefore shows four things separately for every claim, and never collapses them into one
"verified" flag:

| Level | Question | Who or what answers it |
| --- | --- | --- |
| 1. Reference identity | does the reference resolve? | the resolver (mechanical) |
| 2. Claim support | does the content actually support the claim? | a named seat, attributed |
| 3. Exercised behaviour | was something run, with an exit code, or only read? | a `cmd:` reference |
| 4. Authorized decision | did the owner decide? | the owner only |

**Evidence lineage (Codex, entry 13).** Several entries that copy one source are one lineage, not independent
corroboration. Corroboration counts distinct lineages. Two entries share a lineage when one cites the other or
their groundings reach the same landmark only through the same entry. How to detect "same source" beyond
citations is open question 9. Codex suggests W3C PROV-DM as a vocabulary for lineage; I have not checked it
against this design.

## 8. Overlay tags (devtools side)

Overlay tags are `tag_add` events in devtools' ledger. A tag names a target by **pinned anchor**: repository,
revision, path, symbol and blob hash. It is never written into Aaroneous source.

- On a newer revision the resolver re-resolves the symbol. If the blob changed, the tag becomes **stale** and
  stays so until a seat re-confirms it; it never follows the code silently.
- A tag carries who added it and the evidence it rests on; unsigned tags are labelled attested-only (see
  `governance/IDENTITY_SETUP.md` in the identity lane).
- Reverse index (path to tags, tag to nodes) is a projection, not a store.

## 9. Retrieval

Structural first: identifier lookup, tag lookup and bounded graph traversal answer questions and return
citations. Semantic search may *suggest* candidates; any suggestion is re-resolved structurally before it is
shown as fact, and the index is bound to a revision. Prototype bounds, borrowed from the composition blueprint
as test parameters and not as claimed optima: inclusion depth 8, 256 expanded nodes, 1,024 edges, rejecting
overflow before allocating.

## 10. Enforcement: the ratchet

| Stage | What | Gate behaviour |
| --- | --- | --- |
| S0 | measure only: per crate, does it declare a profile, and does it carry crate-level notation | report; no failure |
| S1 | new crates must carry crate-level notation (purpose, authority class); existing ones are baselined | warning; baseline may only decrease; at zero for a crate the rule becomes a hard block for that crate |
| S2 | extend to modules and public items | only after S1 is stable |

Baseline measured at Aaroneous `09ed595b`: **0 of 48** workspace crates declare `[package.metadata.cratify]`, so
a hard block today would stop everything. The profile key is the lane C89 change and edits the binding spec, so it
needs owner sign-off; this proposal does not make it.

## 11. Authority, compliance and graduation

- Notation grants nothing: no claim, approval, merge or tool permission follows from any note, tag or edge.
- Compliance is checked against the real rules (AGENTS.md: zero ambient authority, no stubs, ratchet), not against
  what a note says.
- If accepted, the spec moves into Aaroneous through the graduation gate by a separate product change with owner
  sign-off. The extractor and reducer stay in devtools as the reference implementation until a Rust (Guild)
  version passes the same vectors.
- This proposal does not touch the Aaroneous checkout, which holds other agents' uncommitted changes.

## 12. Non-goals

No change to any Aaroneous file; no semantic-search implementation; no model dispatch; no new scheduler; no user
interface; no change to AGENTS.md, workflows or settings.

## 13. Open questions for reviewers

1. **Visible or hidden?** Doc attributes render in rustdoc (transparent, noisy); `cfg_attr(any(), ...)` is hidden.
   Use one form, or visible for the five declarations and hidden for machine-only fields?
2. **Is the base requirement the right five?** In particular, should *failure behaviour* (the degraded and
   safe-hold paths) be a sixth declaration for control and kernel profiles?
3. **Front matter subset.** Is a restricted YAML subset right for Markdown, or should documents use a fenced
   block that matches the Rust form?
4. **One ledger.** Do notation events join the steward log or the Foundry ledger, and who owns the schema?
5. **Relation vocabulary.** Is the closed list of seven enough? Is `covers` distinct from `implements`?
6. **Edge validation.** Which edge conflicts are errors rather than preserved branches?
7. **Naming.** If the steward graduates as Guild, do these events belong to it or to a separate crate?
8. **Privacy.** What must never appear in a note or tag (secrets, local paths)? Proposed: the extractor rejects
   absolute local paths and token-shaped strings.
9. **Lineage detection.** Citations show explicit copying only. How should two entries that restate one unseen
   source be recognised as one lineage, and is W3C PROV-DM the right vocabulary?
10. **Support versus resolution.** Level 2 (claim support) is human or model judgement and cannot be mechanical.
    How should disagreement between seats on support be shown without implying a verdict?

## 14. Experiments

| # | Experiment | State |
| --- | --- | --- |
| E1 | doc attribute is inert and visible; `cfg_attr` form is inert and hidden | **RUN** in a scratch crate: `cargo check`, `cargo test` (2 passed), `cargo doc` all exit 0 under `deny(warnings)`; text present in rendered pages for form A and absent for form B |
| E2 | baseline: crates declaring a profile | **RUN**: 0 of 48 at Aaroneous `09ed595b` |
| E3 | resolver vectors, including the circular-citation case above | NOT RUN |
| E4 | extract notation from one real crate and compare with a hand reading | NOT RUN |
| E5 | move a symbol; the tag must go stale, not follow silently | NOT RUN |
| E6 | reducer: conflicting edges stay as branches; a `depends-on` cycle is reported | NOT RUN |
| E7 | a reference that resolves but contradicts its claim is shown as resolved at level 1 and unsupported at level 2, never as verified; two entries copying one landmark count as one lineage | NOT RUN |

## 15. Evidence for what was verified

- E1: scratch crate with `#![deny(warnings)]`, a `notation:v1` inner doc attribute on the crate and an item, and a
  `cfg_attr(any(), notation(...))` item. Commands: `cargo check`, `cargo test`, `cargo doc --no-deps`, all exit 0;
  searching the generated HTML found the notation text in the crate index and item page for form A and none for
  form B. Toolchain: cargo and rustc 1.98.1. The scratch crate is not part of any repository.
- E2: `git ls-files '*Cargo.toml'` filtered to files containing `[package]` gives 48; filtered to
  `metadata.cratify` gives 0, at Aaroneous `09ed595b`.
- The Foundry transition document is in unmerged PR #20, so the front matter pins it by revision instead of by
  path; a path would be dangling on `main`.
- Precedent for inert doc-attribute markers: AGENTS.md section 3 already allows `#[doc = "hot_path"]`.
- Base36 caution: SYSTEM_COMPOSITION_BLUEPRINT.md section 6.
