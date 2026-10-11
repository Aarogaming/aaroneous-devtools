# Retiring the Python tooling: lane C127

Status: proposal and claim (2026-10-11). Author: Claude seat. Measured at `origin/main` 9993d6d plus the open PRs named below.

## The problem in one paragraph

This repository is the development layer for a Rust framework, and its merge authority is written in Python. `scripts/` holds 16 programs (about 5,200 lines) and 16 test files (about 5,000 lines); the Rust `steward` crate (about 3,700 lines) is a second copy of the pure logic, kept honest by golden vectors; and `foundation_cli` holds a third, partial copy of the collector, the identity checks and the signature checks (`guild_collector.rs`, 1,355 lines). Three implementations of one gate is two too many, and every rule added to the Python side must be written again. The owner has already decided the direction (the steward graduates to Rust as Guild); this lane turns that into an ordered, checkable plan and makes one rule binding today.

## The rule that applies from now

1. **No new Python logic.** A Python change is allowed only to keep the oracle and the vectors in step with a Rust change, and the PR says so.
2. **Rust first.** New gate, steward, identity or attestation logic lands in `foundation/crates/steward` (pure functions) or `foundation_cli` (adapters), with a vector family in `scripts/golden/steward/` and a `MIN_CASES` floor. Python gets the minimum the vector generator needs.
3. **One implementation per capability.** Before adding to `foundation_cli`, check whether the `steward` crate already has it; the duplicates listed below are consolidated into `steward`, not extended.
4. **Mutation proof.** A port counts as done when breaking the Rust makes the golden test or the differential fail. C125 did this for `independence`: with the stem rule made unconditional, 261 differential cases and golden case 4 failed.

## Inventory (what exists today)

| Python program | Lines | What it is | Rust twin today | Gap |
|---|---|---|---|---|
| `review_coverage.py` | 746 | seat names, coverage, independence, snapshot report | `steward::{seats,coverage,independence}` | attestation parsing (`parse_attestations`, `validate`, `reviews_to_records`) exists only in Python and in `guild_collector.rs` |
| `merge_gate.py` | 421 | checklist, decision, CI/threads/breaker parsing, `gh` calls | `steward::{checklist,gate}` | the `gh`/`git` adapter (`gather_facts`, `--fetch`, `main`) |
| `attest_sig.py` | 219 | registry validation, `allowed_signers`, `ssh-keygen` sign/verify | `guild_collector.rs` (separate copy) | consolidate into `steward`; two copies of `validate_identities` exist, and the Rust one does not check the `profile` and `model_families` fields C125 adds, so they have started to drift |
| `collect_state.py` | 455 | live state from `gh` and `git` | `guild_collector.rs::collect_live_state` | parity with Python never measured |
| `next_task.py` | 479 | card generator | `steward::cards` | none known (golden scenarios) |
| `steward_run.py` | 527 | one tick plus event log | `steward::{engagement,chain,receipts}`, `foundation guild tick` | tick wiring parity never measured |
| `steward_loop.py` | 126 | interval loop | `foundation guild daemon` | not compared |
| `auto_merge.py` | 80 | the one place that presses merge | none | needs the adapter above first |
| `make_attestation.py` | 144 | builds the signed record | none | pure part (`build_record`) is small |
| `check_coordination_evidence.py` | 396 | queue-row evidence check (CI) | none | pure text rules; port with vectors |
| `check_seat_status.py` | 182 | seat declaration check | none | pure |
| `seat_app_setup.py` | 391 | GitHub App manifest flow, token minting | none | needs a local HTTP callback and JWT signing; the hardest piece |
| `worktree_inventory.py`, `profile_manifest_report.py` | 173 | read-only reports | none | small |
| `golden_vectors.py`, `differential_cases.py` | 900 | the oracle's vector and fuzz generators | n/a | the last thing to go (phase 4) |

Two workflows call Python: `auto-merge.yml` (the merge gate; it installs Python 3.12 and runs `scripts/auto_merge.py`) and `coordination-evidence.yml`. `foundation-ci.yml` and `gui-ci.yml` are already Rust.

## Phases, each with its exit evidence

**Phase 0, this PR.** The rule above and this inventory. Exit: merged through the gate.

**Phase 1: every pure function has a Rust twin.** Port, in this order: attestation parsing and `validate`; `build_record`; identities validation and `allowed_signers` (one copy, in `steward`, replacing the one in `guild_collector.rs`); `check_coordination_evidence` and `check_seat_status` rules. Each gets a vector family, a `MIN_CASES` floor and a differential family. Exit: `scripts/golden/steward` has a Rust-checked family for every pure Python function, and a script that lists Python functions without a vector reports none.

**Phase 2: measure the adapters against each other.** Run `foundation guild tick` and the Python steward on the same recorded snapshots (`next_task.py` and `review_coverage.py` already accept `--snapshot`; `steward_run.py` does not and would need it) and diff the cards and log. Do the same for the gate: Rust `checklist` fed by the Rust collector against `merge_gate.py --json` on the live open PRs. Exit: a retained comparison over at least 50 real PR states with every difference explained or fixed. This is the step that has never been done; until it is, the Rust path is untested against reality.

**Phase 3: shadow, then switch.** Add a non-blocking workflow step that runs the Rust gate beside the Python one on every PR and posts only a mismatch. Exit: 30 days or 100 PRs (whichever is later) with zero unexplained mismatches. Then switch `auto-merge.yml` to the Rust binary in its own PR, with the Python path kept one release as the rollback.

**Phase 4: flip the oracle, then delete.** The Rust crate writes the golden vectors (`cargo run -p steward --bin vectors -- --write`); the Python tests are retired; then `scripts/*.py` and the two generators are deleted. `seat_app_setup.py` is last and may stay as a documented one-time setup tool if a Rust port is not worth its cost; that exception is decided on its own evidence. Exit: `git ls-files '*.py'` is empty, or lists only the named exceptions.

## Safety constraints during the migration

- The merge gate is the authority. Nothing in phases 1 and 2 changes what merges. Phase 3 changes it only after the shadow period, in a PR that the existing gate itself must pass.
- A Rust gate that disagrees with Python blocks the switch; it is never resolved by loosening either side.
- The workflow definitions are the one file a PR could use to change its own judge; the switch PR therefore needs the same two-family accepts as any other change (box 3, C125), and the Python path stays until the Rust one has run clean.
- No step adds a dependency without the admission record the repository already requires.

## Not decided here

Whether `seat_app_setup.py` is ported or kept as a named exception; whether the Rust adapters live in `steward` or stay in `foundation_cli`. Both are phase 2 findings.

## Evidence for the numbers

Counts from `git ls-files` and `wc -l` on `origin/main` plus the C125 branch: 57 tracked `.py` files (13.7k lines, tests included) against 49 `.rs` files (24.6k lines); the product repository `Aarogaming/aaroneous` tracks no `.py` files. Python test suite: `python -m unittest discover -p "test_*.py"` in `scripts/`, 485 tests at C125 head ad45004.
