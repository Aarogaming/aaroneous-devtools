# Topic case v1

C109 adds a maintained Rust command-line slice for topic-bound brainstorming:
contributions -> qualified experiment proposal -> existing intake/review -> outcome receipt.
The [synthetic example](../fixtures/topic-case-v1.json) is usable without a server or a graphical interface.

## Authority and integration

A case is a versioned proposal/evidence artifact, like a virtual patch set. It is not a second coordination queue,
installed event journal, live board migration, permission grant, or authenticated review record. Supply every input
and output explicitly. The command never dispatches work, posts to GitHub, edits the queue, reads home configuration,
or starts a worker. Existing scratchpads can remain simple; a producer can assemble this artifact when a discussion
is ready for a bounded experiment.

`export-intake` uses the existing `intake::Proposal` v1 validator, including its exact-path rules and 64 KiB input limit
after provenance augmentation. It preserves the
proposed scope, dependencies and verification; its decision includes the topic, experiment, input digest, declared
work item and review destination. Submit that export through the existing owner-reviewed intake and queue process.
The declared work item is a reference, not a newly granted claim. A later receipt records the actual work/review
reference and pinned revision, costs, measurement, unresolved issues and lesson back under the same topic.

Actor names, owners, review verdicts and evidence references are supplied assertions. `review` always reports
`requires_owner_review: true` and `references_verified: false`; threshold achievement is an arithmetic observation,
not acceptance. Independent review must check the actual referenced PR/head, evidence and authority. This increment
does not select between the steward and Foundry live writers, adopt the C101 notation as an executable schema, or
change either reserved server proposal. A live writer adapter remains a separate implementation increment.

## Commands

Build the Foundation CLI from `foundation/`. Replace all paths below with absolute paths selected for your checkout
and artifacts. Output parents must already exist, and outputs must be new filenames.

```text
foundation topic-case review --case ABSOLUTE_CASE_JSON
foundation topic-case revise --case ABSOLUTE_CASE_JSON --event ABSOLUTE_EVENT_JSON --output ABSOLUTE_NEW_REVISION_JSON
foundation topic-case verify-parent --case ABSOLUTE_NEW_REVISION_JSON --parent ABSOLUTE_CASE_JSON
foundation topic-case export-intake --case ABSOLUTE_CASE_JSON --experiment experiment-1 --output ABSOLUTE_INTAKE_JSON
foundation validate-intake --proposal ABSOLUTE_INTAKE_JSON --output ABSOLUTE_VALIDATION_REPORT_JSON
```

Create an initial case by adapting the fixture and replacing synthetic assertions with your own pinned sources.
`review` prints a JSON projection without writing. To add a contribution, topic, relationship, experiment or receipt,
put one event entry in a file and use `revise`. A relationship retirement also uses `revise`. No special front end is needed.

## Representation and invariants

The root has `schema_version: 1`, `artifact_kind: "foundation.topic-case"`, optional `parent_digest`, and ordered
`events`. Every entry has a unique stable `id` and nested `event` object. Unknown fields, unknown event types and
unsupported versions are errors. Reads are bounded to 4 MiB and cases to 4096 events. IDs contain ASCII letters,
digits, `_` or `-`, up to 128 bytes. Nonempty text must contain no control characters.

Each event is one of these types; the fixture shows the full spelling of every experiment field:

| Type | Payload and meaning |
| --- | --- |
| `topic` | `topic`: stable `id`, optional `parent`, `title`, `question`, responsible proposal `owner`, `tags`, `evidence`. Parent must already exist. IDs cannot be replaced, so the hierarchy remains acyclic. |
| `contribution` | `contribution`: unique `id`, existing `topic`, `actor`, `kind` (`idea`, `alternative`, `dissent`, `evidence`), `text`, `lineage`, `evidence`. Attributions and alternatives remain separate; common lineage is not counted as independent agreement. |
| `relationship` | `relationship`: unique `id`, existing `from`/`to`, `kind` (`depends_on`, `related`, `contradicts`), `reason`, `evidence`. Dependency cycles, self-links and duplicate directed endpoint/kind tuples are rejected. Related/dissent cycles are allowed. Tags and relations never manufacture children or alter topic identity. |
| `relationship_retired` | `retirement`: existing `relationship` ID, `actor`, `reason`, `evidence`. Retires an active assertion once without deleting it. Graph checks use active links; reports retain historical links, retirements and an explicit active ID list. A corrected assertion uses a new ID. |
| `experiment` | `experiment`: unique `id`, existing `topic`, cited `contributions` from that topic, `owner`, intended outcome, metric/unit/method, typed baseline, directional threshold, cost/time budget, stop condition, declared existing work item/review destination, retry lineage and existing intake proposal. Intake IDs must also be unique. |
| `outcome` | `outcome`: existing `experiment`, matching `work_item`, reported `reviewer`/`verdict`, pinned `review`, actual cost/time, optional measured value with measurement evidence, `unresolved` issues and `lesson`. One receipt per attempt; retries retain the original receipt. |

Every evidence reference is `{ "reference": "...", "revision": "..." }`; both fields are required nonempty text,
and evidence lists must be nonempty where required. The revision is an opaque producer assertion, not verified by
the tool. Contribution lists, tags and unresolved issue lists reject duplicates.

An experiment baseline is either `{ "status": "measured", "value": NUMBER, "evidence": [PINNED_REFERENCE] }` or
`{ "status": "not_measured", "reason": "..." }`. The baseline and result share the experiment's metric/unit/method.
Threshold direction is `at_least` or `at_most`. All numbers must be finite; cost and time are nonnegative, and the
time budget is positive. Costs have `{ "value": NUMBER, "unit": "..." }`; receipt cost units must match the budget.
Report fields show the threshold observation, measured result-minus-baseline delta when both exist, and budget/time
exceedance. Missing baseline or result stays JSON `null`, never invented zero. No monetary ROI ratio is asserted
without a measured economic benefit; the outcome evidence supports subsequent ROI interpretation.

Receipts use `accepted`, `rejected`, `needs_changes` or `inconclusive` as reported verdicts. A retry requires a prior
receipt from the same topic with a non-accepted verdict, a new experiment/intake ID, and a nonempty `revision_delta`.
This enforces declared changed work; it cannot judge whether a textual delta is meaningful. Closed attempts cannot
be exported again. Actual measurement never substitutes for independent review, and unresolved issues are retained
even when a producer reports acceptance. Retry lineage may branch into independently bounded alternatives; the report
exposes all direct successors as `retry_children` so reviewers can inspect overlapping scope and cumulative attempt
costs. This artifact grants no concurrent execution or additional capacity.

## Revision and publication behavior

`revise` validates the complete input, appends exactly one validated event and sets `parent_digest` to the SHA-256 of
the exact input bytes. It retains every earlier event. `verify-parent` checks that digest, validates both files and
checks the exact one-event extension without rewritten history. `review` alone does not verify external ancestry;
retain parent files and run `verify-parent` when checking a handoff. Hashes establish byte identity, not authorship.

Both writing commands serialize before publication, write/sync an exclusive staging file in the output directory,
then publish with an exclusive hard link. Existing destinations, including aliases of inputs, are never overwritten;
two competing publishers can produce only one final output. Filesystems without hard-link support fail explicitly.
The staging file is cleaned on normal/error return; process termination may leave a `.topic-case-*.tmp` orphan.
The final pathname exposes a complete file, though power-loss directory durability is not claimed. There is no
automatic stale-lock reclamation, overwrite recovery or background writer.

## Verification and retirement

Run `cargo fmt --all -- --check`, `cargo test --workspace --all-targets --locked`, and
`cargo clippy --workspace --all-targets --locked -- -D warnings` from `foundation/`. Integration tests exercise the
actual binary, intake round trip, history, invalid inputs, graph rules, retries and no-overwrite publication.
Independent review of the pinned PR head remains necessary before adoption. Retirement reverts this increment;
external case files and their retained history remain readable artifacts rather than runtime dependencies.
