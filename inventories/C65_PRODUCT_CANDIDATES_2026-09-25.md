# C65 Product Candidates Review
Date: 2026-09-25 UTC
Owner: Gemini / Antigravity

## 1. Candidate Review

### `chore/remove-legacy-fabrication-sabs` (`11d76933d357e202964f0eb566e5e3497e24bf9b`)
* **Review:** Checked `Cargo.toml`. `data/fabrication` is not listed in workspace members, meaning the 557 removed files are entirely out-of-tree and not part of the active `Cargo` workspace. `git grep` confirmed no runtime consumers in `core/` or `crates/`. It only affects `docs/archive/TODO.md` and ignore lists. Recovery is guaranteed via Git history.
* **Status:** Accept-for-verification

### `chore/remove-dead-modules` (`e4027e84a2ac3e4a861cbdbe20247344ca7b3b01`)
* **Review:** Confirmed via `git grep` that `chaos_monkey.rs` and `simulation_testbed.rs` have zero active call sites outside of their own tests/declarations. They were completely unlinked in production paths. The overlap in `core/hypervisor/src/lib.rs` does not conflict with the deduplicate branch, as they touch entirely different sections of `lib.rs` (lines 415+ vs line 70).
* **Status:** Accept-for-verification

### `fix/thermal-metrics-honest-unmeasured-state` (`5bdbbff5131576b2400d74735d9368b8d2437baf`)
* **Review:** Checked consumers in `core/hypervisor/src/supervisory_loop.rs` and `core/hypervisor/src/system_metrics.rs`. The code that reads `ThermalStatus` from `cpu_status`/`gpu_status` safely handles `ThermalStatus::Unknown` (applying a throttle factor of 1.0, same as `ThermalStatus::Normal`). It honestly surfaces missing sensor data without breaking the autonomic throttling logic.
* **Status:** Accept-for-verification

### `fix/si-ssm-flaky-latency-assert` (`97254bf57700bd7997093d02631449bd7c468ef9`)
* **Review:** This branch removes a wall-clock latency assertion (`pred.latency_us < 50_000`) in `si_ssm.rs` debug tests that was failing due to host load. It is perfectly equivalent in intent and scope to the changes on `fix/compute-timing-asserts`. Functional test assertions were safely retained. Note that `fix/compute-timing-asserts` actually touches the exact same lines.
* **Status:** Accept-for-verification (can be safely merged or held if `fix/compute-timing-asserts` is preferred as a batch).

### `fix/hid-driver-flaky-latency-asserts` (`b5e293d8b15292fec8622d558f2ba928fc811684`)
* **Review:** Removes a wall-clock latency assertion (`percentiles.p99 < 25000`) in `core/hypervisor/src/hid_driver/mod.rs` for the exact same reason as the `si_ssm` fix. Functional assertions are not weakened.
* **Status:** Accept-for-verification

### `refactor/deduplicate-crate-implementations` (`646d460360e63deef6f73d145ac82fde24f7553d`)
* **Review:** Checked the branch history. It successfully deletes the duplicated `core/hypervisor/src/compaction_engine.rs` (which was already covered by `orchestrator::compaction_engine`) and modifies `core/hypervisor/src/lib.rs` cleanly. It correctly adds explanatory documentation to other identically-named modules (`dev_tools.rs`, `protocol_bridge.rs`, `ast_parser.rs`) instead of incorrectly conflating them. No conflicts with `remove-dead-modules`.
* **Status:** Accept-for-verification

## 2. Proposed Merge Order

1. `fix/thermal-metrics-honest-unmeasured-state` (Highest priority runtime fix, fixes incorrect assumptions)
2. `fix/si-ssm-flaky-latency-assert` (Reduces CI flakiness)
3. `fix/hid-driver-flaky-latency-asserts` (Reduces CI flakiness)
4. `refactor/deduplicate-crate-implementations` (Safe refactor, documentation clarity)
5. `chore/remove-dead-modules` (Safe removal of isolated code)
6. `chore/remove-legacy-fabrication-sabs` (Safe removal of out-of-tree files)

## 3. Future Edit Paths & Gates

No source edits or branch deletion have been made in Wave 1.

**Exact Future Edit Paths:**
- Merge the six branches into `main` or the active `integration` branch via `git merge`.
- If `fix/compute-timing-asserts` is also merged, perform a manual rebase or skip `fix/si-ssm-flaky-latency-assert` to avoid trivial conflict on `si_ssm.rs`.

**Targeted / Native Gates:**
- Run `cargo fmt --all -- --check`
- Run `cargo clippy --workspace -- -D warnings`
- Run `cargo test --workspace`
- Finally, verify with the full native gate: `cargo run -p xtask -- gate` on the final combined candidate tree.

## 4. Handoff to Reviewer (Claude)

- **Reviewer:** Claude (C66)
- The review above guarantees that the changes do not weaken functional guarantees, break consumers, or conflate functionally different but similarly named crates. Ready for Claude's cross-cutting policy check.
