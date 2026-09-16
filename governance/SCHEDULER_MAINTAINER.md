# Scheduler and Maintainer

`foundation schedule-review` is the deterministic control plane for the multithread program. It reads only an explicitly supplied queue and program, then emits a versioned report with available capacity, ready items, and the next eligible claims in queue order.

It does not start models, modify a queue, claim a task, inspect an undisclosed checkout, or make implementation decisions. A contributor remains responsible for task evidence, exact-path claims, verification, and review.

## Operating cycle

1. Run the review against absolute paths and save its JSON report outside the framework checkout.
2. An integration owner evaluates the report and selects at most its reported capacity of independent claims.
3. Owners add claims to `COORDINATION_QUEUE.md`; the next review reports those active lanes.
4. Completed work becomes `Review`, then `Done` only after the applicable verification gate.
5. The weekly maintenance review records new evidence in the worklist before adding any queue row.

```text
evidence -> worklist -> ready queue -> schedule review -> human claim -> review -> done
```

## Command

```text
cargo run --manifest-path foundation/Cargo.toml -p foundation_cli -- schedule-review --queue <absolute-queue> --program <absolute-program> --output <absolute-report>
```

The JSON format is defined in `contracts/schedule-review-v1.schema.json`. The command has no Aaroneous crate dependency and never writes an input file.

## Future adapters

Host adapters may invoke this command and present its report, but must remain thin. They may not bypass claims or automatically dispatch work. Any model-assisted task analysis must consume explicit, versioned evidence and emit a proposed queue change for human review.
