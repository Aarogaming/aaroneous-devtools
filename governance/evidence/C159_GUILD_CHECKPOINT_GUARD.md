# C159: Guild checkpoint recovery guard

Status: author verification complete; independent review required. Not deployed.
Contribution-ID: C159/codex-guild-checkpoint-guard-v1.
Date: 2026-10-21 UTC.
Base: `a262a04bd4b688c3ebd96e6b7774297b523f1646`.
Branch: `codex/guild-checkpoint-guard`.
Checkout: `D:/aaroneous-devtools/tmp/guild-checkpoint-guard`.

## Reproduced failure

The Rust Guild writer bypassed a retained checkpoint when its computed tip was
GENESIS and ignored malformed or incomplete checkpoint JSON. Six actual CLI
regressions demonstrated successful ticks and outbox writes despite deleted or
empty logs, malformed checkpoint JSON, missing digest, wrong count, or invalid
count type. Deleted/empty logs restarted at tick one. This was not merely an
inspection finding. A stale checkpoint after append already failed safely.

## Bounded change

[Guild implementation](../../foundation/crates/foundation_cli/src/guild.rs)
now shares checkpoint validation across tick, status, and verify-log, including
the missing/empty-log case. JSON checkpoints require a nonnegative integer
count and string tip; duplicate required fields and invalid shapes are rejected.
Count and tip must agree with the verified log. Appending checks the retained
checkpoint again. Errors propagate without silently replacing that evidence.

[CLI regressions](../../foundation/crates/foundation_cli/tests/guild_cli.rs)
exercise actual subprocesses using an offline committed snapshot and disposable
state. Rejection cases compare log, checkpoint, and outbox bytes before/after;
status and verify-log also reject the damaged state without changing it.

Compatibility is explicit:

- A fresh run with neither log nor checkpoint remains allowed.
- A legacy log without a checkpoint remains allowed, but is not externally anchored.
- A valid plain SHA-256 checkpoint remains supported and is upgraded on append.
- JSON lacking count, malformed JSON, and duplicate required fields now fail closed.
- `--expect-tip` is an additional check, not permission to bypass a damaged local checkpoint.
- Unknown JSON fields remain tolerated. No new persisted format is introduced.

## Author verification

Native Windows; `rustc 1.100.0-nightly (a36d05efa 2026-09-09)` and
`cargo 1.100.0-nightly (3c0b53475 2026-09-04)`. These results do not establish
stable-toolchain or native Ubuntu qualification. No live host/service was changed.
All commands below ran from the checkout above. Cargo used the explicitly
reserved target directory with two jobs, locked dependencies, and offline mode.
Logs reside under `D:/aaroneous-devtools/tmp/`; they are local retained evidence,
not repository artifacts. A remote reviewer should reproduce the commands.

Common Cargo options for the first four commands below:

```text
--manifest-path foundation/Cargo.toml --locked --offline --target-dir D:/aaroneous-devtools/tmp/c159-target -j 2
```

| Command (with common options) | Exit | Result | Retained log |
| --- | --- | --- | --- |
| `cargo test -p foundation_cli --test guild_cli checkpoint_` (original implementation, first eight added tests) | 101 | 2 passed, 6 failed: unsafe ticks returned success | `c159-before-confirmed.log` |
| `cargo test -p foundation_cli --test guild_cli` (initial repair) | 0 | 12 passed, 0 failed | `c159-guild-tests.log` |
| `cargo test --workspace --all-targets` (final tests) | 0 | 380 passed, 0 failed, 1 existing differential test ignored; includes 15 Guild tests | `c159-workspace-tests.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | No warnings | `c159-clippy.log` |

Additional checks: `cargo fmt --manifest-path foundation/Cargo.toml --all -- --check`
and `git diff --check` both exit 0. Recheck the queue evidence gate against the
base and final commit; its result is recorded in the coordination handoff.

SHA-256 of retained logs:

```text
511dc248467d1f89d845352ce5eca209014a4dce0f573118a6b26a0ea8e08fb0  c159-before-confirmed.log
5f68bb419bc08df514fecd2aa91fe71e9b1bc804b272c9ae4618c9045c4f4f61  c159-guild-tests.log
7c53fab72b5f4ee8c3e34403ade0116062058d8a49145aa60019a4ceabd16717  c159-workspace-tests.log
d766dcd16d839e2bab30a829978073912cbef5cbd636bfda5015d23ad0eab309  c159-clippy.log
```

The initial sandbox-only attempt (`c159-before.log`) failed fixture setup with
Windows access-denied errors, not the checkpoint defect. Native execution with
approved temporary-file access established the red/green results above. An
intermediate baseline (`c159-before-native.log`) also had an over-specific error
assertion for the already-safe stale-checkpoint case; the confirmed baseline
accepts either existing "sidecar" or new "checkpoint" terminology.

## Recovery and remaining risks

On a recovery hold, stop all writers for the affected stream and preserve exact
copies of the log, checkpoint, outbox, diagnostics, and their hashes before any
repair. Compare with the last trusted external anchor or backup. Do not delete
the checkpoint merely to make the next tick succeed. Resume only after a
separately authorized, reviewed recovery establishes the intended history.
This patch neither invents recovery data nor automatically rolls back a stream.

This is a partial durability prerequisite, not an industrial-readiness claim:

1. Log append and checkpoint update are still separate non-transactional writes;
   flush is not a power-loss guarantee. Fault injection and a durable commit/
   recovery protocol remain required.
2. Rechecking a checkpoint is not mutual exclusion. Direct tick and daemon
   writers still need shared locking/fencing and concurrency qualification.
3. Deleting or consistently replacing both local files cannot be detected from
   those files alone. Trusted external anchoring remains necessary.
4. Outbox publication is not an exactly-once delivery protocol.
5. This run supplies no native Linux test evidence, deployment acceptance, or
   independent approval. The existing adapter PR stack is unchanged.

Rollback the scoped source/test change only through review: it reopens the
demonstrated failure mode. Preserve operational state; reverting source is not
permission to reset or discard evidence. Next lanes should specify and test
writer exclusion and crash recovery before enabling autonomous operational use.
