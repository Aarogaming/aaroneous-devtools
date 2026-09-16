# Dogfooding Protocol

Dogfooding validates foundation tools against Aaroneous without creating a dependency.

1. **Select explicit inputs:** provide an Aaroneous checkout path, commit or branch, and output
directory on the command line or in a stack manifest.
2. **Read-only first:** the tool collects diagnostics, test evidence, or proposed patches without
mutating the checkout.
3. **Emit versioned artifacts:** reports, inventories, and fixtures use documented schemas under
`contracts/` and identify the inspected revision.
4. **Human or owning-thread review:** a proposed product change is reviewed in Aaroneous and
implemented there as normal Rust code.
5. **Independent verification:** Aaroneous runs its own native verification gate. Devtools results
are supporting evidence only.

## Allowed integration levels

| Level | Example | Constraint |
| --- | --- | --- |
| Read-only | Audit report or dependency inventory | Preferred default. |
| External runner | A devtools CLI invokes documented Aaroneous commands | Explicit checkout and output paths. |
| Contract fixture | Shared JSON/SARIF-like report schema | Versioned and copied, not dynamically imported. |
| Product adoption | Reimplement or copy a reviewed capability into Aaroneous | Aaroneous owns final source and tests. |

No level permits source mutation, build configuration edits, binary replacement, or ambient
machine reads without an explicitly reviewed control-plane protocol.
