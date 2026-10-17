# Topic paths with relevance: isolated experiment

Contribution-ID: C108/codex-topic-path-experiment-v1

Status: experimental proposal, not a supported server, API, adopted schema or
release authority. The owner selected familiar path navigation with relevance
layered on it: existing topics determine depth, metadata describes topics, typed
assertions explain relationships. Scroll up enters a selected topic; scroll down
returns. Breadcrumbs and explicit controls provide another route.

## What this exercises

- Identity-based paths retain meaning when titles collide.
- Explicit parent/child references determine traversal; shared tags or relevance
  never imply containment, merge identities or authorize work.
- Related assertions retain direction, advisory status and stable assertion ID.
  Cross-path and unresolved targets remain references, not silent reparenting.
- Navigation reads a captured projection without modifying the records.
- Metadata fields are attributes rather than invented child topics.

The fixture is entirely synthetic. Its illustrative relation words are not the
C101 vocabulary and must not become a canonical contract by accidental reuse.
The fixture's revision identifies this experiment only; it is not a live server
revision. Node layout is vertically spaced with separate relation lanes; browser
overlap, keyboard/scroll behavior and accessibility acceptance remain NOT VERIFIED.

## Files and reproduction

- `topic-path-core.cjs`: pure, dependency-free projection/path adapter. No I/O,
  dispatch, mutation, inference or acceptance decisions.
- `topic-path-core.test.cjs`: synthetic negative and navigation checks.
- `topic-projection-fixture.json`: 13 illustrative topics, metadata and eight links.
- `explorer.html`: inline visualization fragment, not a standalone deployed page.
  It embeds the fixture and adapter, and uses conversation-host styles and optional
  presentation-state storage. It does not fetch data or write canonical records.

From this directory:

```text
node --test --test-isolation=none topic-path-core.test.cjs
```

The non-isolated runner avoids a sandbox child-process restriction observed in
the original environment (`spawn EPERM`); it does not relax test assertions.
No dependencies, services, model providers or paid calls are required.

## Evidence limits and review

Nine original checks exercised duplicate labels, valid traversal, return-path
preservation, separation of tags from containment, directional cross-path dissent,
dangling links, missing parents, cycles, duplicate identity and immutable input.
Additional publication checks validate the captured fixture and compare the
embedded adapter/fixture with these files. These are synthetic structural checks,
not independent acceptance, complete UI testing or durable event replay.

NOT RUN: live contribution migration, authentication, crash recovery, optimistic
durable writes, graph DB/API deployment, voting/scheduling and flat-versus-topic
ROI measurement. Baseline and time/token savings are NOT MEASURED.

Related design proposals: [C101 notation/relations PR #59](https://github.com/Aarogaming/aaroneous-devtools/pull/59)
and [C105 constellation lifecycle PR #67](https://github.com/Aarogaming/aaroneous-devtools/pull/67).
C102 [server intake PR #60](https://github.com/Aarogaming/aaroneous-devtools/pull/60)
does not grant this experiment a maintained implementation scope. This publication
preserves their reserved paths and does not adopt their proposals.

Next maintained slice needs independently reviewed schema/writer reconciliation,
an exact Ready claim and the existing canonical event/projection route. Consume
actual topic IDs, parents and tagged metadata; do not infer a hierarchy from
similarity or replace existing records with this fixture. Relevance should expose
its source, reason and uncertainty. Rust owns durable maintained policy.

Rollback removes this experiment directory through the normal reviewed process
and records retirement in the queue while preserving claim/review evidence.
Runtime history, product sources, installed services and schedules stay intact. Draft PR
publication makes the proposal reviewable; merge and official adoption remain
separate decisions by the applicable authority.
