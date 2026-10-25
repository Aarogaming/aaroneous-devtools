# Rust-First Hybrid Stack Matrix

Rust owns reusable logic, parsing, orchestration, reports, and policy checks. Other formats remain
only where they express an external contract or platform boundary.

| Stack | Use | Boundary |
| --- | --- | --- |
| TypeScript / React | User-facing dashboard | Presentation and typed API client only; durable scheduling, budgets and maintenance policy stay in Rust. |
| Rust stable | Foundation CLI, validators, report generation, fixtures, adapters | Default implementation language. |
| Rust nightly, isolated | Miri or experimental analysis | Separate CI lane; no product requirement. |
| TOML / JSON | Stack manifests and versioned report schemas | Data only, schema-validated. |
| YAML | CI, Kubernetes, and external service declarations | Declarative external contract. |
| Terraform / Helm | Infrastructure definitions | Kept outside Rust only while the provider requires it. |
| PowerShell / shell | Minimal OS/service launcher | Must call a documented Rust command; no business logic. |
| Python / notebooks | One-off research only | `experiments/`, no foundation or product dependency. |

## Candidate stack manifests

- `stacks/local-rust.toml`: stable Rust, formatting, clippy, tests, native auditors.
- `stacks/windows-release.toml`: Windows compiler/toolchain and installer adapter inputs.
- `stacks/analysis.toml`: read-only audit commands and report outputs.
- `stacks/local-model.toml`: optional private model endpoint settings; never loaded by Aaroneous.

Every manifest declares inputs, commands, expected artifacts, failure behavior, and a responsible seat.
