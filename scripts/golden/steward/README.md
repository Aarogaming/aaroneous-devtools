# Steward golden vectors (lane C99)

These files are the behavioural contract a Rust re-implementation of the steward (the proposed Guild
component) must reproduce. They are plain JSON, so the contract does not depend on Python.

## Files

| Files | Meaning |
| --- | --- |
| `<scenario>.in.json` | `{"tick": N, "state": {...}}`, the input to the dispatcher (`next_task.build_cards`) |
| `<scenario>.out.json` | the exact list of cards expected for that input |
| `chain.in.json` | events (`type`, `data`, `now`) appended in order to an empty log |
| `chain.out.json` | the exact log lines and the SHA-256 of the last line (the tip) |
| `attribution.*` | `[{body, strict}]` and the seat `comment_seat` returns for each (`""` when none) |
| `heads.*` | `[{body, head}]` and whether `cites_head` is true: an 8 to 40 character prefix of the head, never a different full head |
| `receipts.*` | `[{name, snapshot, repair_owners, implementers_by_pr}]` and `done_receipts`: `{card key: completing seat}` |
| `rejected.*` | `[{name, pr, row}]` and `rejected_verdicts`: the repair entries, at most two per seat, current head only |
| `stalls.*` | `[{name, events, agents, stall_after, window}]` and `{stalled, attribution_weak}` |
| `pushes.*` | `[{name, previous, prs}]` and `{heads, credited}` (`steward_run.pushes`: a moved or new head credits the PR's seat) |
| `seats.*` | `[string]` and `[seat, ...]` sorted: `review_coverage.seat_names` (a spelling names a seat by prefix; one string may name several; an unknown string keeps its first token) |
| `coverage.*` | `[{name, head, merge_base, records, ancestry}]` and `review_coverage.evaluate`: `{covered, reasons, accepts_at_head}` |
| `audit.in.json`, `audit.out.json` | a healthy state and, for ticks 0-13, the tasks issued: the periodic audit on every 6th tick only |
| `gate_tier.*` | `[{name, paths}]` and `{tier, protected, plan_level, category}`: owner-only paths (a prefix match, so a lookalike also counts), tier B only for a recognised safe family, everything else unclassified |
| `gate_ci.*` | `[{name, rollup}]` and `ci_state`: missing, pending, failure and success; skipped is not success |
| `gate_breaker.*` | `[{name, text}]` and `breaker_state`: a real TOML boolean, otherwise `null` (blocked) |
| `gate_threads.*` | `[{name, data}]` and `open_threads`: a count, or `null` when a page is truncated, pagination info is missing or the response is malformed |
| `gate_checklist.*` | `{green, cases: [{name, override}]}` and `{lines, would_merge, first_blocker}` for the nine-box checklist; a box that cannot be evaluated is blocked and fails the gate |
| `gate_decide.*` | `[{name, row, paths, base, trial, owner}]` and `{decide, reconciled_with_all_green_checklist}` |

## How an implementation uses them

1. For each `<scenario>.in.json`, run the dispatcher on `state` at `tick` and compare the result with
   `<scenario>.out.json` **as JSON values** (key order is irrelevant; list order and every value are not).
2. For `chain.*`, append the events to an empty log and compare the lines **byte for byte**.

Python regenerates the files with `python scripts/golden_vectors.py --write` and checks them with `--check`.
Regenerating is a deliberate act: a changed vector is a contract change and needs review like any other
change. `scripts/test_golden_vectors.py` reads only these files, never the scenario definitions, so it checks
the code against the contract and not the generator against itself.

## What the scenarios pin

Review assignment excludes every implementer; unresolved threads go only to the author; an uncovered draft gets
a review card and a covered draft gets the flip-ready card; a PR stacked on a non-default base gets no merge
brief; the framing (lens) advances with each hand-out and a held lease keeps its lens; a seat that let a lease
expire is not handed the same item again while another seat can take it; "head moved" is reported against the
seat's last card; seats that finish work are served first; everything leased means standby; nothing actionable
means healthy idle with the evidence that was checked; a verdict the coverage tool rejected goes back to its
author as a repair card carrying the reason (never to an implementer, and not once the PR is covered).
Unresolved threads that nobody on the PR's side has answered are a fix card for the author only; threads that all have an
answer are a `resolve_threads` card for a seat that is not an implementer (the author cannot complete it).
A standing rejecting verdict (`needs_revision`, `changes_requested`) at the current head is an `address_review` card for the
author only, and no other review of that head is requested while it stands.

## Byte format of the log (decision needed before Guild)

Each line is `json.dumps(event, sort_keys=True, ensure_ascii=False)` with Python's **default separators**
(a space after `,` and `:`), and the chain hash is SHA-256 over the line's UTF-8 bytes without the newline.
That is the current format, so a Rust implementation has to match it exactly (`chain.out.json`). It is not
how most canonical-JSON schemes are written (RFC 8785 is compact). Moving to a compact canonical form would
be a breaking, versioned contract change; deciding between the two is an open question for Guild.

## The Rust port

`foundation/crates/steward` is the Rust port (lane C99). `cargo test -p steward` runs it against every vector family
here (`tests/golden.rs`) and fails when a family exists that is neither ported nor listed in its `PENDING` list, when a
ported family disagrees with its `*.out.json`, or when a family listed as pending no longer exists. Ported so far:
`seats`, `heads`. Port a family by moving it from `PENDING` to `check`.

## Not covered yet

These parts of the steward have no vectors and are therefore not yet shown to be replicable: the GitHub read
adapter itself (`collect_state.collect`, `review_coverage.live_snapshot`, `merge_gate.gather_facts`), the
outbox text layout, the signed-attestation path, the commit-trailer and Qwen/Mimo independence additions (not on
this branch), and lease replay inside `run_tick` (expiry, rotation by history are covered through the card
scenarios, but `lease_table` itself is not). The rule vectors above pin the decisions merge safety depends on:
what counts as a completion, who may complete what, which head a comment cites, when a seat is stalled, and
when a head is covered. Two mutation probes of 16 one-line changes each (the steward rules, then the merge-gate checklist) are caught 16 of 16 by
these vectors; the original 30 card vectors caught 1 and 0.
