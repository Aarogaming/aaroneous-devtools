# Dependency review report — schema v1

Produced by `foundation dependency-review --checkout <path> --output <path>`
(`foundation/crates/foundation_cli/src/dependency_review.rs`). Read-only: the
command runs `cargo audit --json` and `cargo metadata` as external processes
against the given checkout and never writes to it. See
`governance/DOGFOOD_PROTOCOL.md` — this is the "External runner" level, and
the report itself is a "Read-only" contract-fixture artifact.

Satisfies WORKLIST.md S2: a lockfile-aware scanner's output, with advisory
paths, fixed versions, and a recorded mitigation recommendation per finding.
No dependency is updated by this command or by generating this report —
applying a fix is a separate, reviewed change in Aaroneous itself.

## Fields

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | integer | Always `1` for this schema. Bump on any breaking field change. |
| `checkout` | string | The `--checkout` path as given (display form, not canonicalized). |
| `revision` | string | `git rev-parse HEAD` in the checkout at generation time. |
| `tool` | string | Always `"cargo-audit"` for schema v1. |
| `tool_version` | string | Output of `cargo audit --version`, for reproducibility. |
| `generated_at_unix_ms` | integer | Wall-clock time the report was generated, milliseconds since Unix epoch. |
| `summary.total_findings` | integer | `findings.len()`. |
| `summary.external` | integer | Findings classified `external`. |
| `summary.local_workspace_name_collisions` | integer | Findings classified `local_workspace_name_collision`. |
| `findings[]` | array | See below. |

### `findings[]` entries

| Field | Type | Meaning |
| --- | --- | --- |
| `kind` | string | `"vulnerability"`, `"unmaintained"`, `"unsound"`, or another cargo-audit informational kind. |
| `advisory_id` | string | e.g. `"RUSTSEC-2026-0285"`. |
| `title` | string | The advisory's title, as published. |
| `url` | string \| null | Advisory URL if cargo-audit provided one. |
| `package` | string | Package name as it appears in the checkout's `Cargo.lock`. |
| `installed_version` | string | The version currently locked. |
| `patched_versions` | string[] | Version requirement(s) that resolve the advisory; empty if none is published yet. |
| `classification` | `"external"` \| `"local_workspace_name_collision"` | See below. |
| `recommendation` | string | Human-readable next step. Never applied automatically. |

## `classification`: why it exists

`cargo-audit` matches advisories by package **name and version** against the
entire `Cargo.lock`, including path-dependency workspace members. A local
crate whose name happens to collide with an unrelated, vulnerable crates.io
package gets flagged even though it isn't that code at all — this workspace
has exactly one such collision today: `crates/scratchpad` (an Aaroneous
path dependency) shares its name with a real, separately-vulnerable
`scratchpad` published to crates.io.

A finding is classified `local_workspace_name_collision` when **both**:
1. the matched package's `Cargo.lock` entry has no `source` (a `null`/absent
   source is Cargo's signal for a path dependency, never a registry crate),
   **and**
2. its name appears in `cargo metadata --no-deps`'s package list for the
   same checkout (i.e. it's a workspace member).

Everything else is `external` — a real dependency worth reviewing. Treat
`local_workspace_name_collision` findings as informational only; renaming
the local crate is the only real remedy if the collision becomes confusing,
never "upgrading" it (there is nothing to upgrade).

## Example

See `contracts/dependency-review/v1/reports/aaroneous-2026-09-16.json` for
a full example generated against Aaroneous commit `41b6408`.
