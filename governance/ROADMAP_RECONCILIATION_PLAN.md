# Roadmap reconciliation plan

Breaks M24 ("Consolidate root TODO and frontier roadmap into an evidence-labeled
product roadmap") into bounded subtasks — M24 itself was too large to claim as one
lane: it covers three separate documents making dozens of unverified completion
claims, and a full verification pass is a multi-session audit, not a single bounded
change. This file records the concrete evidence that motivated the split and gives
each subtask (M28–M33 in `MULTITHREAD_PROGRAM.md`) its exact scope. M24 itself is
retired — do not re-claim it; claim one of M28–M33 instead.

## Evidence

### 1. Every roadmap doc claims a version far ahead of the real one

`Cargo.toml`'s `[workspace.package]` sets `version = "0.3.2"` — the actual, current
product version. All three roadmap documents ignore this:

- `MASTER_ROADMAP.md` — Phase 38 claimed "In Progress" at target version `v1.7.0`,
  with Phases 1–37 (versions `v0.4.0` through `v1.6.0`) all marked **Complete**.
- `docs/roadmap.md` — header states `**Version:** v1.7.0+`.
- `TODO.md` — same Phase 38 / `v1.7.0` framing, same Phases 1–37 "Complete" claims.

By the roadmap's own version-to-phase mapping, a `0.3.2` product hasn't even reached
Phase 1 (`v0.4.0`) yet. Every crate in the workspace is versioned `0.1.0`–`0.3.2`
(checked via `grep -h "^version" Cargo.toml core/hypervisor/Cargo.toml
crates/*/Cargo.toml xtask/Cargo.toml`) — none are anywhere near `1.x`.

### 2. At least one specific "Complete" claim is directly disproven by this session's own work

`MASTER_ROADMAP.md:108` and `TODO.md:89` both list Phase 5 (`v0.8.0`, "Safe dynamic
modification") as delivering a "`libloading` dynamic loader" / "Hot-reload ABI
plugins" — described as **Complete** in `MASTER_ROADMAP.md`.

Queue item C2 (this session, commit `ffb02ea`) reproduced two real, independent
unsoundness findings in exactly that mechanism (`crates/studio_hud/src/plugin_api.rs`)
and removed it rather than patch it — a fake signature check, and a fat pointer
carrying a non-`repr(C)` type across a DLL boundary (undefined behavior). M4 decided
the sound replacement is real design work, not a bounded fix, and RFC-0006
(`docs/rfcs/RFC-0006-...`, queue item M12) is that design — still a **proposed**
document, no implementation yet. "Complete" is not an accurate label for this phase
by any reasonable definition; the actual state is "attempted, found unsound, removed,
redesign proposed but unimplemented."

If one spot-checked claim is this wrong, the other ~37 "Complete" phases cannot be
trusted without checking each one individually — which is the actual size of the
work M24 named.

### 3. `MASTER_ROADMAP.md` has a text-corruption bug independent of its content accuracy

Systematic character-dropping and stray newline insertion, likely from some prior
lossy edit or generation pass — a mechanical problem, separate from whether the
underlying claims are true:

| Line(s) | Corrupted text | Likely intended |
| --- | --- | --- |
| 26, 53 | `ytemuck::Pod` | `bytemuck::Pod` |
| 37 | `ing_buffer.rs` | `ring_buffer.rs` |
| 67 | `(ig.rs)` | `(rig.rs)` |
| 67, 117 | `kyv` | `rkyv` |
| 68 | `watches \tokio::task::JoinHandles` (a literal tab character before `okio`) | `watches tokio::task::JoinHandles` |
| 93–94 | `via` then a stray newline then `ats_client.rs` | probably `via nats_client.rs` or `stats_client.rs` — ambiguous, needs the original intent traced or the line rewritten from scratch |
| 103–104 | `activate llama-gguf, ` then a stray newline then `vml-wrapper` | `nvml-wrapper` |
| 136, 140, 144 | `` `ash `` (single backtick) | ` ```bash ` (a corrupted triple-backtick code fence) |

### 4. The three documents disagree with each other, and `TODO.md` disagrees with itself

`TODO.md` references its own completed-phases archive at two different paths in two
different places:

- Line 101: `dev/docs/blueprints/COMPLETED_PHASES_ARCHIVE.md` — **does not exist**.
- Line 138: `docs/archive/stale_sprawl/blueprints/COMPLETED_PHASES_ARCHIVE.md` — **exists**.

`MASTER_ROADMAP.md`'s own appendix links to `ARCHITECTURE_AUDIT.md`, which also does
not exist at the repo root (checked directly).

`MASTER_ROADMAP.md`, `docs/roadmap.md`, and `TODO.md` additionally duplicate the same
content three times with independent drift: the same "5 Pillars" table, the same
"7-Horizon Frontier Framework" table, and overlapping phase tables, each phrased and
formatted slightly differently, with no single one marked canonical.

## Subtask breakdown

| Status | ID | Work | Dependency | Preferred capability |
| --- | --- | --- | --- | --- |
| Done | M28 | Established evidence standard in [`governance/EVIDENCE_STANDARD.md`](file:///d:/aaroneous-devtools/governance/EVIDENCE_STANDARD.md): defines strict evidence criteria for status labels (`Complete`, `In Progress`, `Attempted/Redesigned`, `Dormant`) and reconciles phase tags (`v0.4.0`–`v1.7.0`) as architectural milestone horizons against the authoritative root `0.3.2` workspace version. | none | Release governance / documentation architecture |
| Done | M29 | Repaired all 11 mechanical text corruptions in `MASTER_ROADMAP.md` (`bytemuck::Pod`, `ring_buffer.rs`, `rig.rs`, `rkyv`, `nats_client.rs`, `nvml-wrapper`, `aaroneous`, `tokio JoinHandles` tab space, bash triple-backtick fences) using original git log intent. | none | Mechanical text fix |
| Done | M30 | Verified Phases 1–8 against codebase; relabeled Phase 5 in `MASTER_ROADMAP.md` from Complete to Redesigned (historical fat-pointer FFI loader removed for unsoundness in SEC-01/SEC-03; in-process loader active; RFC-0006 command-buffer ABI proposed). Verified Phases 1–4, 6–8 per `governance/EVIDENCE_STANDARD.md`. | M28 | Rust systems review / evidence verification |
| Done | M31 | Verified Phases 9–37 against codebase per `EVIDENCE_STANDARD.md`. Mapped phase tags (`v1.2.0`–`v1.6.0`) to architectural milestone horizons (H1–H7) against root workspace version `0.3.2`. Updated status labels and crate mapping notes in `MASTER_ROADMAP.md`. | M28 | Rust systems review / evidence verification |
| Done | M32 | Verified active Phase 38 / Horizon 6 work claims (`CapabilityBroker`, `SystemHealthGovernor` backpressure/pacing, `Disruptor` ring buffer event bus, `EngineStatePublisher`). Updated `docs/roadmap.md` Section 3.1 & 3.2 with verified completion evidence for M20, M26, M27, and Phase 38 active scope per `governance/EVIDENCE_STANDARD.md`. | M28 | Rust systems review / evidence verification |
| Done | M33 | Resolved doc-vs-doc redundancy by establishing `docs/roadmap.md` as canonical evidence-labeled roadmap, `TODO.md` as thin redirect & terminology index, and `MASTER_ROADMAP.md` as 7-Horizon framework spec with top-level banner pointer to `docs/roadmap.md`. Fixed dead `ARCHITECTURE_AUDIT.md` link to `docs/architecture/MASTER_ARCHITECTURE.md` in `MASTER_ROADMAP.md`. | M30, M31, M32 | Documentation architecture |

## Review gate

Same as `DEPENDENCY_REMEDIATION_PLAN.md`'s: any of M29–M33 that touches Aaroneous
product docs is a product change and should pass the relevant native verification
(at minimum `cargo run -p ast_auditor -- audit` isn't applicable to markdown, but
`typos` CI and a check that every referenced file path actually resolves are cheap
and directly relevant here — worth running before marking any of these Done).
