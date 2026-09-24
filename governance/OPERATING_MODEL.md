# Companion operating model

Devtools is Aaroneous's foundry and functional organizer: a user-facing maintenance program, local automation workspace, and experiment space. It uses Aaroneous as its capability matrix and execution substrate, pulls the strings, and is allowed to be more flexible than the framework, including code that is not Cratify-compliant. It helps contributors build Aaroneous; Aaroneous must never need devtools to build, test, release or run. See [FOUNDATION_BOUNDARY.md](../FOUNDATION_BOUNDARY.md) for the binding roles and dependency direction.

## Standards proportional to the job

| Concern | Devtools standard |
| --- | --- |
| Rust execution | Ordinary heap allocation, collections, async I/O, synchronization and service tasks are allowed. No framework-wide zero-allocation or PLC scan requirement applies. |
| Dependencies | Prefer an existing Aaroneous crate over re-implementing a capability. Otherwise prefer maintained libraries and standard tools, added for a concrete need with license, platform support and maintenance cost considered. Cratify dependency admission does not apply here, but code intended to graduate should avoid dependencies Aaroneous would reject, or keep them behind a trait boundary. |
| Languages | Rust owns durable maintenance logic. TypeScript/React may provide the user interface. Thin OS adapters and declarative provider formats are allowed; research stays isolated. |
| Configuration | The user selects repositories, providers and storage. A bootstrap/UI may explicitly discover candidates and ask the user to select them; core tools receive typed, explicit inputs. No silent home-directory or credential discovery. |
| Errors and safety | Useful errors, bounded retries, safe path handling and reviewed unsafe code. No generated code is trusted or declared verified just because a model returned it. |
| Verification | Foundation Rust changes: formatting, meaningful tests and strict Clippy. GUI changes: its build/type check plus relevant behavior checks. Documentation: links, encoding and diff checks. Product changes: Aaroneous's full native gates. |
| Experiments | Quick prototypes are welcome outside maintained paths. Label demonstration state and promotion criteria; do not run prototypes as production services. |

The lighter standard relaxes implementation constraints, not evidence, ownership, credentials or spending boundaries. Product rules apply whenever a task changes Aaroneous.

## Foundry and graduation

Devtools is where capabilities are grown and proven; Aaroneous is where proven patterns end up. A block that proves generally useful graduates into Aaroneous under Cratify v2's graduation gate (`Aaroneous/docs/CRATIFY_SPEC.md` section 6): rewritten behind an Aaroneous-defined trait, landed inert or in shadow mode, and then **deleted from devtools**, which consumes the Aaroneous crate from then on. Devtools keeps only the operational parts: configuration, job content, schedules, and backend choices behind Aaroneous traits. The active map and sequence are in [architecture/AARONEOUS_FRAMEWORK_INTEGRATION.md](../architecture/AARONEOUS_FRAMEWORK_INTEGRATION.md).

When building something new here, ask whether it describes *how an agent, job, model or verdict works* (a graduation candidate: keep its core free of ambient reads and panics so graduation is cheap) or *how this owner runs their operation* (stays here). Aaroneous never depends on devtools.

## Repository map

| Location | Owns | Lifecycle |
| --- | --- | --- |
| Root README, AGENTS, CONTRIBUTING, FOUNDATION_BOUNDARY | Entrypoints and binding boundary | Keep short; link to details |
| `foundation/` | Rust maintenance core and CLI | Maintained, tested implementation |
| `gui/` | User interface | Tracked presentation layer (`gui/`); build/type-checked via Dashboard CI, read-only integration |
| `contracts/` | Versioned reports, schemas and fixtures | Shared interface authority |
| `architecture/` | System boundaries and delivery designs | Target behavior clearly distinguished from implemented behavior |
| `governance/` | Contributor process, portfolio, ownership and service procedures | One authoritative document for each decision |
| `stacks/` | Tool/provider profiles and ecosystem catalogue | Declarative, validated configuration |
| `worker-jobs/` | Approved bounded local support manifests | Reviewed inputs, not autonomous primary claims |
| `platform/` | Thin installation/startup adapters | Target home; create when an adapter is adopted |
| `experiments/` | Isolated prototypes and evaluations | Target home; no maintained runtime imports |
| `inventories/` | Curated classification and migration evidence | Versioned decisions, not transient logs |
| `migrations/` | Preserved transferred material | Historical input; not active services |
| `artifacts/`, `reports/` | Runtime state, generated reports, drafts and logs | Ignored scratch; bounded retention, no secrets in Git |

Do not create empty scaffolding for every possible tool. Add a directory when a maintained concern needs it. Tracked registrar (`agentic_registrar.rs`) and GUI source are hardened under C15 and C19; write/promotion actions remain gated on explicit owner sign-off. Existing installed service paths stay stable during organization work.

## One home for each planning fact

- `MULTITHREAD_PROGRAM.md`: prioritized cross-repository portfolio and capacity.
- `COORDINATION_QUEUE.md`: live claims, exact files, dependencies, handoffs and review evidence.
- `architecture/MAINTENANCE_PROGRAM.md`: application delivery sequence and acceptance milestones.
- `WORKLIST.md`: historical task briefs and links into current planning; old labels are not current certification.
- `SCHEDULE.md`: intended recurring cadence; actual installed state comes from service records.
- `DECISIONS.md`: durable choices and reasons, not another task list.

Generated findings enter an inbox first. A reviewed finding gets a canonical portfolio ID and a bounded acceptance criterion. A Ready queue row is created only when dependencies, exact scope and reviewer capacity are available. Do not copy the full roadmap into README, agent prompts or new audit documents.

## Local automation beyond Git

| Job | Existing starting point | Next useful increment |
| --- | --- | --- |
| Repository onboarding and tool readiness | Stack/input validation and component inventory | Guided environment diagnosis and actionable missing-tool results |
| Verification runs | Dogfood reports and product-owned native gates | Isolated run orchestration, revision-tagged history and readable failures |
| Finding intake and work routing | Intake validator, schedule review and native candidate inbox | Deduplicated evidence, dependencies and user approval UI |
| Cheap support work | Bounded local worker and approved manifests | Review/disposition operations, retry policy and accurate liveness |
| Scheduling and resource sharing | Native tasks, quiet windows and shared reservation | One observable scheduling policy with restart/cancellation tests |
| Review and integration | Ownership queue and human review | Isolated patches, retained diff/gate evidence and authorized publication |
| Hard or urgent work | Application delivery design | Disabled-by-default paid adapters with explicit data and budget controls |

Prefer deterministic software for inventories, checks, parsing, deduplication, scheduling and bookkeeping. Use models for bounded interpretation and drafting. Reuse Git for branches, worktrees, diffs and history; do not implement competing version control. Local services do not automatically reproduce cloud CI or web research.

## Daily operation and maintenance

Start by checking repository state, running-service health and pending review debt. Review existing results before adding more jobs. Schedule only useful evidence-backed work; idle is a valid healthy state. Pause on missing tools, unavailable local models or spending limits rather than spinning indefinitely.

Weekly, reconcile expired claims, stale plans, service failures and retained artifacts. Monthly, review dependencies, promote or retire one prototype, and review graduation candidates against the map in `architecture/AARONEOUS_FRAMEWORK_INTEGRATION.md`. Every maintained tool needs documented inputs, output contract, failure behavior, owner and install/pause/uninstall procedure. Measure completed reviewed work, repeated failures, queue age and cost rather than generated-token volume.

Retention must preserve dispositions and retry/idempotency history. The present local worker suppresses unchanged archived inputs, so indiscriminate archive deletion can rerun old jobs. Introduce explicit compaction/retention support before automated cleanup; never recursively prune product checkouts.

## Adoption order

1. Reconcile the two devtools checkouts and unfinished prototypes through a named handoff.
2. Implement truthful read-only health/status and connect the existing GUI.
3. Add local review/disposition, bounded retries and durable scheduling.
4. Add isolated verified patches, then budgeted cloud escalation and installation polish.

Use the detailed milestones in the maintenance application design. This document establishes organization and operating standards; it does not certify future capabilities as implemented.
