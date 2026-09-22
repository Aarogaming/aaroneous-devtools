# Cyclic Gain: devtools → Aaroneous Portability

**Status**: Documented, not yet started  
**Date**: 2026-09-21  
**Branch**: integration/aaroneous-framework-imports

## Concept

devtools and Aaroneous can create a cyclic gain loop:

1. devtools builds tooling against Aaroneous (framework → tooling)
2. devtools abstracts reusable patterns into shared crates
3. Aaroneous adopts those crates with stricter standards (tooling → framework)
4. devtools imports the upgraded crates (framework → tooling, cycle completes)

This avoids duplication while letting each repo enforce its own quality bar.

## Porting Candidates (Priority Order)

### 1. Feedback Injection Loop
**devtools source**: `local_worker.rs` - `[PRIOR REVIEWER FEEDBACK]` injection  
**Pattern**: When a reviewer rejects an outcome with reasoning, inject that reasoning into the next generation attempt's context. Creates a human-in-the-loop quality cycle.  
**Porting effort**: Medium  
**Value to Aaroneous**: Very High - any AI-assisted verification pipeline needs this  
**Target**: `feedback_injection` crate

### 2. Schema Validation Pipeline
**devtools source**: `local_worker.rs` - `draft_schema()` → `verify_proposal_quality()` → `ast_auditor` → `verify_proposal_citations()`  
**Pattern**: Multi-layer validation: structural schema → semantic checks → AST audit → ground-truth verification  
**Porting effort**: High  
**Value to Aaroneous**: Very High - directly applicable to verification gates  
**Target**: `proposal_validator` crate

### 3. Job Packet Format
**devtools source**: `worker-jobs/*.toml` - the `Job` struct and TOML format  
**Pattern**: Declarative bounded task manifest with versioned schema, explicit evidence paths, bounded instruction text  
**Porting effort**: Low  
**Value to Aaroneous**: High - enables human-readable task definitions  
**Target**: `task_manifest` crate

### 4. Probation Ledger
**devtools source**: `local_worker.rs` - `update_probation_ledger()`  
**Pattern**: Quality tracking with pass/fail rates, probation status machine (ON_PROBATION → PASSED/FAILED), rolling 100-entry audit log  
**Porting effort**: Medium  
**Value to Aaroneous**: High - formalized quality tracking for verification gates  
**Target**: `quality_ledger` crate

### 5. Outbox Review System
**devtools source**: `outbox.rs` - verdict recording, feedback bridge  
**Pattern**: Structured review verdicts (accept/reject/needs-changes/uncertain) with append-only JSONL journal  
**Porting effort**: High (filesystem coupling)  
**Value to Aaroneous**: Medium-High - formalized review tracking  
**Target**: `review_verdict_journal` crate

### 6. Adaptive Pacing
**devtools source**: `local_worker.rs` - `serve()` function  
**Pattern**: Tick-based delay that examines what each tick actually did, adjusts delay dynamically  
**Porting effort**: Medium-Low  
**Value to Aaroneous**: Medium - useful for maintenance tasks  
**Target**: `adaptive_loop` utility

## Shared Infrastructure Needed

Before porting, build these once in a shared crate:

| Component | Purpose | Used By |
|---|---|---|
| **Clock trait** | `fn now_secs() -> u64` | 5 of 6 components |
| **FileSystem trait** | Bounded read/write/metadata | All 6 components |
| **BoundedString** | `BoundedString<const N>` for zero-alloc | Hot paths |
| **Ring buffer** | Fixed-capacity replacement for `Vec::remove(0)` | Probation ledger |

## Items That Do NOT Port

- `delegate()` function - devtools-specific (Ollama HTTP client)
- `reservation.rs` RAII lock - too simple, rewrite in Aaroneous if needed
- `draft_schema()` JSON Schema - tied to Draft output format
- `prepare_prompt()` / `build_system_prompt()` - tied to LLM interaction model

## Cyclic Flow

```
devtools builds tooling → ports abstraction to shared crate →
Aaroneous adopts with stricter standards → devtools imports →
devtools builds more tooling → ports again → ...
```

## Next Steps

- [ ] Build shared infrastructure crate (Clock, FileSystem, BoundedString, RingBuffer)
- [ ] Port Feedback Injection Loop as first candidate
- [ ] Port Schema Validation Pipeline
- [ ] Port Job Packet Format
- [ ] Port Probation Ledger
- [ ] Port Outbox Review System
- [ ] Port Adaptive Pacing
