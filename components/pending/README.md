# Pending Components (Graduation Track)

Components in this tier are candidates for graduation into Aaroneous core crates (`aaroneous/crates/capabilities`, `aaroneous/crates/compute`, etc.).

## Graduation Lifecycle

1. **Incubation**: Developed in devtools under explicit input constraints.
2. **Shadow Evaluation**: Run alongside live operations or against golden vectors to evaluate stability, determinism, and token efficiency.
3. **Audit**: Scanned with `ast_auditor` for forbidden patterns (ambient authority, panics, unwrap violations, unbounded allocations).
4. **Graduation PR**: Transferred to an isolated product branch in `aaroneous` with full native verification gates green.
