# Foundation Boundary

`aaroneous-devtools` is Aaroneous's **foundry and functional organizer**: the private workspace where
Aaroneous-adjacent capabilities are grown, exercised, and proven before they are distilled into the
framework. It orchestrates work; Aaroneous provides the capability matrix and execution substrate it
orchestrates.

## Roles

| | Aaroneous | Devtools |
|---|---|---|
| Is | Capability matrix, execution substrate, compiled library of components | Organizer, experiment space, software foundry |
| Standard | Cratify v2 (`Aaroneous/docs/CRATIFY_SPEC.md`): universal floor + declared compliance profile | [Companion operating model](governance/OPERATING_MODEL.md); non-Cratify-compliant code is allowed |
| Owns | Types, traits, reducers, providers, verifiers, and anything describing *how* an agent, job, model, or verdict works | Configuration, job content, schedules, queue/portfolio parsing, reports, GUI, and anything describing *how this owner runs their operation* |
| Grows by | Graduation of proven patterns from devtools and external sources | Experiments, prototypes, and operational tooling |

## Dependency direction

- **Devtools -> Aaroneous: allowed.** Foundation crates may depend on Aaroneous crates (today:
  `ast_auditor`, `governance`, `llm_gateway_types`, `llm_gateway`) and should prefer an existing
  Aaroneous capability over re-implementing it.
- **Aaroneous -> devtools: forbidden.** Nothing in the Aaroneous workspace, CI, release, or runtime
  may depend on devtools. Devtools is never required to build, test, release, or run Aaroneous.
- **Source mutation:** devtools never writes Aaroneous source automatically. User-approved patches
  flow into an isolated Aaroneous branch and pass Aaroneous's native gates.

## Graduation into Aaroneous

A devtools capability that proves generally useful graduates under Cratify v2's graduation gate
(`CRATIFY_SPEC.md` section 6):

1. **Proven** in devtools with recorded results.
2. **Classified**: fills a gap (new Aaroneous crate), overlaps an existing component (upgrade in
   shadow mode, never a second implementation), or sits adjacent (new crate wired through the registry).
3. **Scored** against its target profile; its dependencies pass admission (`CRATIFY_SPEC.md` section 5).
4. **Rewritten behind an Aaroneous-defined trait** with contract tests, not copied in devtools' shape.
5. **Landed inert** (feature flag) or run in **shadow mode**.
6. **Origin copy deleted**: devtools switches to the Aaroneous crate in the same cycle. Two maintained
   copies of one capability is a defect.
7. **Ratcheted**: violations found while porting become `ast_auditor` rules.

Devtools keeps the parts that are operational rather than general: its configs, job manifests,
schedules, and its choice of backend where Aaroneous defines a trait (for example, devtools may keep a
`sled` store behind an Aaroneous storage trait even though `sled` is not admitted into Aaroneous).

The current graduation map is [architecture/AARONEOUS_FRAMEWORK_INTEGRATION.md](architecture/AARONEOUS_FRAMEWORK_INTEGRATION.md).

## What does not belong here

Devtools is not an archive for retired framework source. Aaroneous Git history is authoritative for
removed product artifacts. A transferred item must have a maintained foundation or operations use
case, a named owner, and documented inputs and outputs. No autonomous writer may overwrite shared
product work or treat generated output as verified.
