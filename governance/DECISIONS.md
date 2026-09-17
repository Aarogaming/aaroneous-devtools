# Decision Log

Short decisions prevent rediscovering old tradeoffs. Add a dated entry when a choice affects a
contract, dependency direction, security boundary, or long-term maintenance cost.

## 2026-09-16 — Rust-first foundation with explicit external inputs

Foundation logic is Rust. External support directories are allowed only as explicit CLI or manifest
inputs and are recorded in versioned reports. Aaroneous does not depend on the foundation workspace.

## 2026-09-16 — Practical companion, independent framework

Devtools becomes a user-facing local maintenance companion. It keeps Rust-first reusable logic but permits ordinary services, allocation, async work and a TypeScript UI. Aaroneous's stricter execution-plane rules apply to product changes, not all companion code. Existing Git, OS scheduling and maintained tooling are reused; deterministic software precedes model inference. Cloud escalation is disabled until its data and spending policy is configured.

The operating model owns organization; the multithread portfolio owns priorities; the coordination queue owns live claims; the application design owns delivery milestones. Old worklist briefs retain context without competing as current status. No prototype relocation or service-path change is authorized by a documentation cleanup alone.
