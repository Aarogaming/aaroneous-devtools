# Foundation Architecture

The foundation repository develops tools *around* Aaroneous. It must never become a hidden member
of the Aaroneous workspace or a prerequisite for the framework to build, test, release, or run.

## Layers

| Layer | Purpose | Implementation rule |
| --- | --- | --- |
| `foundation/` | Reusable maintenance, analysis, and release binaries | Rust workspace, explicit CLI inputs, versioned output. |
| `stacks/` | Declarative stack manifests | TOML/JSON only; no executable policy hidden in configuration. |
| `contracts/` | Stable artifact and report schemas | Versioned, documented, and consumable without a devtools checkout. |
| `platform/` | OS-specific launch/install adapters | Thin PowerShell, shell, or service adapters around a Rust command. |
| `experiments/` | Private prototypes and model evaluation | Isolated; never imported by foundation or Aaroneous production paths. |
| `migrations/` | Historical transfer evidence | Read-only input, not active tooling. |

## Dependency direction

```text
Aaroneous checkout ──produces──> versioned evidence/artifacts ──> foundation tools
Foundation tools ──produce──> reports, fixtures, proposals ──> human review
Approved product change ──is implemented and verified──> Aaroneous checkout
```

Aaroneous never imports a devtools crate, script, model profile, path, or environment setting.
Foundation tools receive repository paths, revisions, and configuration explicitly from their CLI
or manifest.

## First Rust workspace

The first implementation should be a small `foundation/` Cargo workspace with a single CLI and no
Aaroneous crate dependency. It reads a supplied checkout path, runs declared commands, and writes
versioned reports to a caller-provided directory. Additional tools become separate crates only
when they have an independent contract and test surface.
