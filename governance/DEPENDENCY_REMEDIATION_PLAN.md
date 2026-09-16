# Dependency remediation plan

Turns C3's dependency-review evidence into the one bounded plan M3 in
`MULTITHREAD_PROGRAM.md` asks for. Every line here is a decision already
made and enacted (or explicitly deferred with a reason) — this file
documents the plan; `.cargo/audit.toml` and `deny.toml` in Aaroneous are
where it's actually enforced. Do not duplicate the reasoning in a third
place; update those two files and this table together.

Source of truth for "what's currently flagged": re-run
`foundation dependency-review --checkout <path> --output <path>`
(`foundation/crates/foundation_cli/src/dependency_review.rs`) — it hits the
live RUSTSEC advisory-DB fetch, so counts drift over time as advisories are
published, amended, or their affected-version ranges are refined. This plan
was last reconciled against a run at Aaroneous commit `5af683c` on
2026-09-16 (see `contracts/dependency-review/v1/reports/aaroneous-5af683c.json`
for that run's raw output) and against `cargo-audit`/`cargo-deny`'s actual
CI results on Aaroneous PR #18 (both green as of commits `ab5bc74`,
`5af683c`).

## Status legend

- **Fixed** — a patched version is in `Cargo.lock` now.
- **Deferred, documented** — a real fix exists but isn't a safe minimal
  change; ignored in `.cargo/audit.toml`/`deny.toml` with a dated reason,
  re-evaluate when its blocker clears.
- **No patch available** — upstream hasn't published a fix; nothing to
  apply. Ignored with a reason; re-evaluate if a patch or a maintained
  alternative appears.
- **False positive** — not a real finding; documented so it isn't
  "fixed" by mistake later.

## Plan

| Advisory | Package | Status | Action taken / next step |
| --- | --- | --- | --- |
| RUSTSEC-2026-0285 | rustls 0.23.44 | **Fixed** | `cargo update -p rustls --precise 0.23.45` — patch release, in-range, no code changes needed. Aaroneous commit `ab5bc74`. |
| RUSTSEC-2026-0119, RUSTSEC-2026-0120 | hickory-proto / hickory-net 0.26.0-beta.4 | **Deferred, documented** | Pinned to exactly that beta by `iroh` 0.98.2's transitive `hickory-resolver = "=0.26.0-beta.4"` dependency; `cargo update -p hickory-net --precise 0.26.1` fails to resolve (verified — see commit `ab5bc74`'s message for the exact resolver error). Next step: evaluate an `iroh` version bump on its own (P2P federation code path in `core/hypervisor`), not as a CI-fix patch. Not currently reproduced by a live `cargo audit`/`cargo deny` run against the current advisory DB (drift — see note above); ignore entries kept in `.cargo/audit.toml`/`deny.toml` as defense-in-depth in case the DB's affected-range assessment reverts. |
| RUSTSEC-2026-0235 | rkyv 0.7.46 | **Deferred, documented** | 0.7 → 0.8 is a breaking major-version bump (derive/Archive API changed substantially); this workspace derives `Archive`/`Deserialize`/`Serialize` extensively, most heavily in `crates/ipc_bus`. `Cargo.lock` currently carries *both* rkyv 0.7.46 and 0.8.18 (something already depends on 0.8 directly) — next step is auditing exactly which crates still pull 0.7.46 and whether they can move to the already-present 0.8.18 without a wider migration, rather than assuming a full rewrite is required. Not currently reproduced by a live scan (DB drift, as above); ignore entries kept. |
| RUSTSEC-2021-0030, RUSTSEC-2025-0049 | scratchpad (crates.io) | **False positive** | These are RUSTSEC advisories against an unrelated crates.io package named `scratchpad`; this workspace's `crates/scratchpad` is a path dependency that only shares the name. Confirmed via `cargo metadata`'s workspace-member list crossed against `Cargo.lock`'s `source` field (a path dependency has no registry source) — this is exactly what `dependency_review.rs`'s `classify()` function checks, and it's covered by 3 unit tests. Not currently reproduced by a live scan; ignore entries kept as defense-in-depth. The only real remedy if this collision becomes actually confusing in practice (not just to tooling) is renaming the local crate — not planned unless it recurs. |
| RUSTSEC-2023-0089 | atomic-polyfill 1.0.3 | **No patch available** | `patched: []`. Unmaintained (author archived the repo), not a known vulnerability. A maintained alternative (`portable-atomic`) exists per the advisory but swapping it is a real dependency-graph change (who pulls this in transitively — not yet traced); not undertaken speculatively. Re-evaluate if this becomes reachable in a security-sensitive path. |
| RUSTSEC-2025-0141 | bincode 1.3.3 | **No patch available** | `patched: []`. Unmaintained, not a known vulnerability. |
| RUSTSEC-2026-0196, RUSTSEC-2026-0197 | cgmath 0.18.0 | **No patch available** | `patched: []` for both. Unmaintained, and the unsound `Matrix{2,3,4}::swap_columns` method is grepped as unused anywhere in this workspace (`rg swap_columns` — zero hits in `.rs` files outside `target/`), so the unsoundness isn't reachable even though the dependency is present. |
| RUSTSEC-2024-0436 | paste 1.0.15 | **No patch available** | `patched: []`. Unmaintained, not a known vulnerability. |
| RUSTSEC-2026-0253 | lru 0.16.4 | **Deferred, documented** | Patched in `>=0.18.2`; attempted `cargo update -p lru --precise 0.18.2` — fails to resolve with the same root cause as the hickory advisories: `iroh-relay` 0.98.0 (a transitive dependency of `iroh` 0.98.2) declares `lru = "^0.16.3"`, a caret range that excludes 0.18.x. All three of RUSTSEC-2026-0119/0120/0253 share one blocker and one fix: an `iroh` version bump. |

## Open next steps (ranked)

1. **`iroh` upgrade evaluation** — the single highest-leverage item: unblocks all three of RUSTSEC-2026-0119/0120/0253 (hickory-net, hickory-proto, lru) at once, confirmed by two independent `cargo update --precise` attempts hitting the exact same `iroh`/`iroh-relay` transitive pin. Needs its own scoped review of the P2P federation code path (`core/hypervisor`'s iroh usage) before attempting — a real version bump, not a CI-fix patch.
2. **`rkyv` 0.7→0.8 consolidation** — trace which crates still need 0.7.46 now that 0.8.18 is already present in the lockfile; may be smaller than a full migration if only a few call sites need updating.
3. **`atomic-polyfill` → `portable-atomic`** — only worth doing once atomic-polyfill's actual dependents are traced; not blocking anything today.

## Review gate

Per `MAINTENANCE_SYSTEM.md`'s operating rules: any change enacting one of
the "next steps" above is a product change and must pass Aaroneous's native
verification gates (`cargo xtask gate` / the CI workflows in
`.github/workflows/`), not just a devtools-side check. Update this table's
status and the corresponding `.cargo/audit.toml`/`deny.toml` entries in the
same change.
