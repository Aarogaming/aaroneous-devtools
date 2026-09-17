# Contributing to Aaroneous Devtools

Start with `FOUNDATION_BOUNDARY.md`, then claim a scoped row in
`governance/COORDINATION_QUEUE.md` before editing shared work.

Read [the companion operating model](governance/OPERATING_MODEL.md) for the lighter devtools standards. Heap allocation and ordinary service I/O are welcome here; framework constraints apply to product changes.

## Small, reviewable changes

- Use a focused branch; Codex-created branches use `codex/`. Preserve existing contributor branch conventions.
- Keep one purpose per commit. Include the relevant verification evidence in the queue entry.
- UI changes require its build/type checks and meaningful behavior verification. Documentation changes require valid links and clean encoding/diffs. No test needs to mirror a prose edit.
- Rust changes must pass `cargo fmt`, `cargo test`, and strict Clippy for `foundation/`.
- Foundation commands require explicit input and output paths. Do not add ambient path discovery,
  product dependencies, or automatic Aaroneous source mutation.
- Treat migration content as historical input. Promote a tool only after documenting its owner,
  inputs, outputs, and maintenance cost.

## Definition of done

An implementation moves to Review when its contract/documentation is updated, applicable checks pass, and the queue records evidence, commit and released paths. An independent reviewer marks it Done after checking scope and verification. Do not self-certify generated artifacts.
