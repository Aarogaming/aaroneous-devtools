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
means healthy idle with the evidence that was checked.

## Byte format of the log (decision needed before Guild)

Each line is `json.dumps(event, sort_keys=True, ensure_ascii=False)` with Python's **default separators**
(a space after `,` and `:`), and the chain hash is SHA-256 over the line's UTF-8 bytes without the newline.
That is the current format, so a Rust implementation has to match it exactly (`chain.out.json`). It is not
how most canonical-JSON schemes are written (RFC 8785 is compact). Moving to a compact canonical form would
be a breaking, versioned contract change; deciding between the two is an open question for Guild.

## Not covered yet

These parts of the steward have no vectors and are therefore not yet shown to be replicable:
`collect_state` (the GitHub read adapter), `review_coverage.evaluate` (the head-bound coverage rules),
`merge_gate` (the checklist), the outbox text layout, and the lease/stall logic inside `run_tick`. Coverage
rules are the next most important vectors, because merge safety depends on them.
