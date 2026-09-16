# Contributing to Aaroneous Devtools

Start with `FOUNDATION_BOUNDARY.md`, then claim a scoped row in
a `governance/COORDINATION_QUEUE.md` before editing shared work.

## Small, reviewable changes

- Use a focused branch: `feat/`, `fix/`, `docs/`, or `chore/`.
- Keep one purpose per commit. Include the relevant verification evidence in the queue entry.
- Rust changes must pass `cargo fmt`, `cargo test`, and strict Clippy for `foundation/`.
- Foundation commands require explicit input and output paths. Do not add ambient path discovery,
  product dependencies, or automatic Aaroneous source mutation.
- Treat migration content as historical input. Promote a tool only after documenting its owner,
  inputs, outputs, and maintenance cost.

## Definition of done

A change is done when its contract or documentation is updated, focused checks pass, the queue
records its commit and released paths, and the branch is ready to merge cleanly into `main`.
