# Multithread Program

This program expands the available roadmap without expanding active work beyond review capacity.
It coordinates any number of contributors through one queue, while normally allowing three primary
implementation lanes at once.

## Capacity model

| Capacity | Use |
| --- | --- |
| 3 primary lanes | Default: one focused product change per primary thread. |
| 1 support lane | Qwen performs a bounded, owner-reviewed analysis or fixture task in scratch space. |
| 1 review lane | A primary owner reviews a completed change before another broad lane opens. |
| Expansion | Add a primary lane only when file scopes, dependencies, and reviewer capacity are all independent. |

A large roadmap is useful; a large number of simultaneous edits is not. A claim moves through
`Ready → Claimed → Review → Done` or `Blocked`. Claims contain exact paths, dependencies, and
evidence, not just an issue title.

## Priority portfolio

| Priority | ID | Work | Dependency | Preferred capability |
| --- | --- | --- | --- | --- |
| P0 | M1 | Converge and verify the active integration branch | CI result | Codex integration |
| P0 | M2 | Port the text-encoding checker into `ast_auditor` | C1 | Rust CLI / tests |
| P0 | M3 | Turn dependency-review evidence into one bounded remediation plan | C3 done | Supply-chain review |
| P0 | M4 | Decide removal versus repair for dead `studio_hud` plugin API code | C2 handoff | Rust API review |
| P1 | M5 | Validate and prune the five stale configuration manifests | C4 inventory | Read-only inventory then focused deletion |
| P1 | M6 | Verify WIT and fabrication packaging/runtime usage before archival | C4 inventory | Build/package analysis |
| P1 | M7 | Deprecate terminology aliases with migration tests | C4 inventory | Compatibility-focused Rust refactor |
| P1 | M8 | Establish the bounded reducer reference path | A2 | Deterministic systems testing |
| P1 | M9 | Build a read-only dogfood harness in devtools | F1–F3 done | Foundation Rust |
| P2 | M10 | Enforce execution-host dependency policy | M8 | Architecture and Cargo policy |
| P2 | M11 | Narrow hypervisor public surface | M10 | API refactor |
| P2 | M12 | Define plugin lifecycle RFC | M4 | Interface design |
| P2 | M13 | Publish measured performance assurance | M8 | Benchmarking |
| P3 | M14 | Add thin platform adapters around stable foundation commands | M9 | Windows/Linux integration |
| P3 | M15 | Evaluate Candle, Rig, Burn, and ONNX Runtime in isolated foundation crates | M9 | Experimental Rust stacks |

## Intake rules

A new lane needs a source, priority, exact scope, dependency, expected evidence, and rollback or
retirement decision. The worklist may contain future work; the coordination queue contains only
work ready to own. No task enters the queue because it merely sounds useful.

## Review gates

- Product code: native Aaroneous verification plus focused tests.
- Foundation code: Foundation CI, contract fixture update, and explicit-input validation.
- Config/interface retirement: runtime, packaging, and external-consumer evidence.
- Public terminology change: compatibility migration and deprecation window.

## Weekly portfolio review

Reprioritize only from evidence: CI failures, completed dependency reports, reproduced security
findings, or finished inventory work. Keep at least one lane available for a small corrective task.
