# Companion operating model

Devtools is Aaroneous's practical development companion: a user-facing maintenance program and local automation workspace. It is allowed to be more flexible than the framework. It helps contributors build Aaroneous; Aaroneous must never need devtools to build, test, release or run.

## Standards proportional to the job

| Concern | Devtools standard |
| --- | --- |
| Rust execution | Ordinary heap allocation, collections, async I/O, synchronization and service tasks are allowed. No framework-wide zero-allocation or PLC scan requirement applies. |
| Dependencies | Prefer maintained libraries and standard tools. Add a dependency for a concrete need, with its license, platform support and maintenance cost considered. |
| Languages | Rust owns durable maintenance logic. TypeScript/React may provide the user interface. Thin OS adapters and declarative provider formats are allowed; research stays isolated. |
| Configuration | The user selects repositories, providers and storage. A bootstrap/UI may explicitly discover candidates and ask the user to select them; core tools receive typed, explicit inputs. No silent home-directory or credential discovery. |
| Errors and safety | Useful errors, bounded retries, safe path handling and reviewed unsafe code. No generated code is trusted or declared verified just because a model returned it. |
| Verification | Foundation Rust changes: formatting, meaningful tests and strict Clippy. GUI changes: its build/type check plus relevant behavior checks. Documentation: links, encoding and diff checks. Product changes: Aaroneous's full native gates. |
| Experiments | Quick prototypes are welcome outside maintained paths. Label demonstration state and promotion criteria; do not run prototypes as production services. |

The lighter standard relaxes implementation constraints, not evidence, ownership, credentials or spending boundaries. Product rules apply whenever a task changes Aaroneous. A reusable block can later be ported into Aaroneous under its own contracts and gates; do not create a reverse dependency for dogfooding.

## Repository map

| Location | Owns | Lifecycle |
| --- | --- | --- |
| Root README, AGENTS, CONTRIBUTING, FOUNDATION_BOUNDARY | Entrypoints and binding boundary | Keep short; link to details |
| `foundation/` | Rust maintenance core and CLI | Maintained, tested implementation |
| `gui/` | User interface | Currently uncommitted in the D: checkout; reconcile before adoption |
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

Do not create empty scaffolding for every possible tool. Add a directory when a maintained concern needs it. Do not move the uncommitted D: registrar/GUI/scripts until their owner hands them over and all runtime paths are reconciled. Existing installed service paths stay stable during organization work.

## One home for each planning fact

- `MULTITHREAD_PROGRAM.md`: prioritized cross-repository portfolio and capacity.
- `COORDINATION_QUEUE.md`: live claims, exact files, dependencies, handoffs and review evidence. Handoffs meet [independent continuability](#independent-continuability).
- `architecture/MAINTENANCE_PROGRAM.md`: application delivery sequence and acceptance milestones.
- `WORKLIST.md`: historical task briefs and links into current planning; old labels are not current certification.
- `SCHEDULE.md`: intended recurring cadence; actual installed state comes from service records.
- `DECISIONS.md`: durable choices and reasons, not another task list.

Generated findings enter an inbox first. A reviewed finding gets a canonical portfolio ID and a bounded acceptance criterion. A Ready queue row is created only when dependencies, exact scope and reviewer capacity are available. Do not copy the full roadmap into README, agent prompts or new audit documents.

## Independent continuability

Every maintained contribution must be continuable by another authorized contributor without the original
contributor's availability. No person, model, session, provider or machine is an undocumented
dependency. This is the canonical standard; handoff and review requirements reference it and do not
restate or fork it.

A maintained contribution leaves, in canonical records only:

- **Contracts**: what it does, its inputs and outputs, and its configuration, all explicit.
- **Retained verification**: the commands, exit codes and log paths or hashes needed to reproduce the
  checks, as the review rubric already requires.
- **Recoverable state**: how to operate, repair, roll back or restart it, and where its operational state
  lives. State that exists only in a session, a chat or one machine does not count.
- **Replaceable boundaries**: a component sits behind a workspace-defined boundary so a provider, model or
  machine can be swapped without rediscovery.
- **An actionable handoff** whenever work stops short of done: the destination (PR, queue row or document),
  the blockers, the next permitted step, and the acceptance condition.

Apply this in proportion to the job. A one-line docs fix needs a clear commit and PR; a service, gate or
state-holding component needs the full set. Prefer discoverable authoritative state (the queue, the PR,
the existing registers) over new documents; never create a competing register.

**Cold-handoff check.** Acceptance includes: an independent contributor, using only canonical records and
normal authorized access, can (1) locate the current state, (2) reproduce the relevant verification, and
(3) take the next authorized step. A reviewer applies this at review time (rubric R8); a failure is a
finding, not a style note.

**Ownership transfers explicitly.** A handoff names the new owner in the queue row or PR. An absent owner
never grants extra authority, bypasses review, or permits overriding a claim; a claim held by an
unavailable owner changes hands only through the claim and lease rules in the coordination protocol.

**Measure outcomes, not documentation volume**: less rediscovery (questions answered from records rather
than from the original contributor), fewer owner-only operations, and shorter time from handoff to
verified next step. Take these from existing PR and queue timestamps and review packets; do not add a
metrics document.

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

Weekly, reconcile expired claims, stale plans, service failures and retained artifacts. Monthly, review dependencies and promote or retire one prototype. Every maintained tool needs documented inputs, output contract, failure behavior, owner and install/pause/uninstall procedure. Measure completed reviewed work, repeated failures, queue age and cost rather than generated-token volume.

Retention must preserve dispositions and retry/idempotency history. The present local worker suppresses unchanged archived inputs, so indiscriminate archive deletion can rerun old jobs. Introduce explicit compaction/retention support before automated cleanup; never recursively prune product checkouts.

## Adoption order

1. Reconcile the two devtools checkouts and unfinished prototypes through a named handoff.
2. Implement truthful read-only health/status and connect the existing GUI.
3. Add local review/disposition, bounded retries and durable scheduling.
4. Add isolated verified patches, then budgeted cloud escalation and installation polish.

Use the detailed milestones in the maintenance application design. This document establishes organization and operating standards; it does not certify future capabilities as implemented.
