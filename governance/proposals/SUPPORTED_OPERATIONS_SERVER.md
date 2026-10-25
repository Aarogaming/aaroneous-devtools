# Supported operations server and home operations host
Status: DESIGN PROPOSAL, NOT IMPLEMENTED, NOT INDEPENDENTLY ACCEPTED.
Devtools design lane: C102; proposal delivered for review in PR135. Implementation and deployment require separate scoped claims.
Intended repository destination: governance/proposals/SUPPORTED_OPERATIONS_SERVER.md.
Owner direction, 2026-10-05: expand the local gathering into a supported Windows/Linux devtools server; use an Ubuntu backup PC as a persistent operations center. Earlier design-first, claim and independent-review requirements remain.

## 1. Existing evidence and overlap
Historical source snapshot (2026-10-05) at devtools main c42a87aba43fbff9036b65460473597805b7d0f0; source locations and operational observations below are dated evidence, not current installation or availability claims:
- maintenance_ui already builds the Rust devtools binary with explicit worker-config, runtime and foundation inputs, a loopback HTTP server, a runtime lease, a two-minute research poll and local-worker-tick child invocation. Source read only; server was NOT RUN for this proposal.
- maintenance_ui owns packet authoring, status, result review and archive operations. Extend this capability through a coordinated source lane; do not port the temporary Python board into a second maintained server.
- foundation_cli schedule-review reads queue/program and emits a capacity report. It is a read-only planning tool, not the continuous execution scheduler.
- .github/workflows/foundation-ci.yml defines Ubuntu and Windows formatting, tests and Clippy. Read only; no new cross-platform service or remote-agent test executed here.
- Existing local runtime board and native worker receipts are previous prototype evidence, not official devtools support or Ubuntu deployment evidence.

The existing steward stages remain the one capability home:
| Capability | Existing home | Proposed connection |
| --- | --- | --- |
| Acquire repository/PR state | collect_state (#49 onward) | Explicit read adapter emits revision-bound input |
| Assess independent review coverage | review_coverage (#51 onward) | Read-only stage; gathering agreement cannot substitute |
| Assign bounded work | next_task (#49/#56/#58) | Lease/outbox adapter; no second dispatcher |
| Retain events and advance ticks | steward_run (#52/#56/#57) | Existing writer/lease logic gains an explicit stage interface |
| Decide merge eligibility | merge_gate (#51/#55) | Separate report gate; no automatic approval from a discussion |

These PRs were OPEN when inspected. #59 head340ed3585999faabe0e17831bc07f6ac395a5923 already covers notation, typed edges and case-file projections. Extend its schema/view responsibility rather than duplicating it. Its ledger choice remains open. #57 golden vectors are a compatibility starting point, not proof of a Rust implementation. C91/C93/C96 worker lanes and open implementation reservations must be reconciled before source changes.

Native schedule-review exited0 and reported8 Claimed/Review lanes for capacity3, with no recommended claims. Its snapshot does not include open claim-PR reservations; source work must check those separately. This is why the new intake is Proposed, not an active implementation claim.

Compatibility evidence: board entry14 (MiMo label) cites an older checkout39b7d4b with only one tracked script. At the inspected main c42a87a, review_coverage.py and related tests are present, while collect_state.py, next_task.py, steward_run.py and merge_gate.py are absent from scripts/. Their proposed changes remain on open PRs. Do not treat an unmerged stage as an installed client route. This is source presence, not runtime behavior.
maintenance_ui/src/lib.rs:284-285 rejects unknown NewJob fields; approve_job:299-381 uses a reservation, queue path checks, bounded evidence, exclusive staging and hard-link publication. These existing boundaries must be retained. Typed gathering/client additions require an explicitly versioned contract and compatibility/migration checks; silently adding fields is not backwards compatible. Source read only, NOT RUN for this proposal. main.rs:190-227 keeps research verified:false and surfaces a retained disposition without claiming product verification.
A short PKT-WIRE revision-derived ID is existing deduplication behavior; it is not by itself full-revision binding, lease fencing or proof of immutable complete evidence. Proposed stale-lease/late-entry vectors are NOT RUN and need a claimed test scope.

## 2. Deployment topology
Ubuntu is the proposed persistent operations host. Windows can run the same server for development or become a worker/operator client. At one time, only one selected operations host owns canonical assignment and event commits.
- Operations host: shared history, topic/case projections, current state snapshots, outboxes, leases, artifact index and truthful status.
- Operator adapters: existing Codex, Claude, Gemini and OpenCode seats pull a bounded card and submit attributed output using their supported interfaces.
- Model workers: explicitly configured local inference endpoints, such as the Ubuntu GPU or the existing Windows worker. Their inference reservations remain enforced at the actual executor.
- Owner client: browser case files, pending decisions, findings, progress and pause/drain controls.
- Source: Git pins and PRs; each executing seat has its own isolated checkout. Runtime state does not live in a shared mutable Git working directory.

The owner reports16GB DDR5, i5-14400F, Asus RTX4060 with likely 8GB VRAM and shared Wi-Fi. Inventory, exact VRAM, disk, Ubuntu release, GPU drivers, SSH/Tailscale access and model fit are NOT VERIFIED. Initial pilot should run one bounded inference at a time and serialize heavy builds; measure peak RAM/VRAM, context, latency and failure behavior before increasing concurrency. CPU/state service planning is an engineering estimate, not a completed capacity benchmark.
Historical initial discovery did not establish an accessible Ubuntu host; later dated notes below supersede that observation. This is not a current availability check. Deployment still requires an explicitly selected endpoint and fresh authenticated evidence; private discovery inventories remain outside the shared proposal.

## 2.1 Client-owned memory and local operating loops
Each client is an operator node with its own bounded working context, durable memory and local controller. This is a proposed capability, NOT IMPLEMENTED by the current discussion board.
- Short-term memory holds the active task, captured inputs, tool receipts, unresolved questions and bounded recent conversation. Checkpoint it for restart; use explicit capacity and expiry rather than silently dropping obligations.
- Long-term memory retains reviewed findings, decisions, learned constraints, task lineage and links to immutable evidence. Store provenance, subject revision, disposition, ownership, retention and supersession. A summary or retrieved memory is not new evidence or authority.
- Keep private client memory separate from shared project records. Sharing is an explicit adapter action within data grants; do not publish raw private transcripts, secrets or another seat's memory by default.
- Keep durable stores on the owning client with a declared root and storage/version contract. Shared event records reference intentionally published artifacts; they do not require a second master copy of every private memory.
- A local controller runs a declared scan: acquire inputs -> reduce bounded state -> emit authorized outputs/receipts. It uses an injected clock, executor and resource budget, plus fixed named routes, cancellation, pause/drain, retry and backpressure controls. Compose existing worker/scheduler capabilities; do not invent a parallel scheduling engine per model.
- Local scheduling can advance previously authorized client-local work during a network outage. Actions affecting shared repositories, assignments or external publications require a valid scoped lease and current preconditions. Offline work without those preconditions stays a proposal; reconnect cannot retroactively authorize it.
- Peer exchange can carry bounded artifact references, findings and capability advertisements. Canonical assignment/event commits still go through the designated writer with deduplication and lease fencing; a peer relay is not an alternate authority.
- Profiles, task lineage and grants follow an assignment between nodes. Memory transfer cannot expand grants, remove reviewer exclusions or turn self-evaluation into independent acceptance.
- Retrieval checks freshness, attribution and access; conflicting memories remain visible with evidence links. Corrupt memory or unavailable storage degrades to retained input/blocked status, never a fabricated recollection. Compaction preserves original references and retained decisions.
- Scale by registering supported clients, stages, storage and transport adapters behind the same contracts. Demonstrate bounded queues and recovery before increasing peers or concurrency.

Required client verification, NOT RUN: memory isolation, crash recovery, stale/contradictory retrieval, compaction with evidence lineage, disconnected local work, shared-action refusal without a lease, duplicate peer delivery, reconnected stale-generation fencing, and drain/restart with active work.

## 3. One stage contract
Each registered stage receives a typed StageInput and returns StageOutput or a typed failure. This describes the common interface; no trait implementation is supplied here.
| Input | Purpose |
| --- | --- |
| stage ID and contract version | Select an existing registered capability |
| immutable snapshot revision and ledger tip | Establish exactly what was read |
| typed stage payload / captured adapter observations | Separate acquired facts from model interpretation |
| injected tick, deadline and cancellation input | No ambient clock or hidden task launch in a reducer |
| assignment ID, attempt, lease generation and holder | Fence stale or reissued work |
| immutable compliance-profile reference and scoped grants | Preserve obligations on reassignment |
| evidence IDs/digests and configured bounds | Keep source identity, capacity and coverage explicit |

StageOutput carries proposed typed events, derived artifacts, adapter action requests, next-state preconditions and a scoped outcome. Stages never receive a globally mutable state object or unrestricted command executor. Acquisition/output adapters perform authorized I/O; reducers only propose transitions.

## 4. Stack declaration and switching
A versioned stack manifest names an ordered list of registered stage IDs, contract versions, input/output bindings, policies and profile references. It contains typed options, not arbitrary prompt strings or shell commands.
- Validate required contracts, dependency edges, bounds and grants before activation.
- A serial stack can pass a committed artifact reference into the next stage.
- Independent stages may read one immutable snapshot concurrently. They return proposals to the same commit writer.
- Switching stacks creates a new manifest revision, drains active actions and retains the old revision. Activation binds to the evaluated manifest digest.
- Unsafe or incompatible switches fail closed with a useful reason. Rollback restores a compatible program/config pin without overwriting newer events or results.

Proposed default gathering route: collect_state -> bounded gathering intake -> case projection / synthesis proposal -> next_task. review_coverage and merge_gate remain distinct checks. Maintenance interpretation connects through the existing worker boundary, not as another assignment engine.

## 5. Shared state and commit authority
Choose ONE canonical event writer by reconciling the steward log with the Foundry ledger. Do not establish a second canonical writer while that decision is unresolved.
- Events are append-only and versioned; snapshots, topic indexes, graphs and outboxes are rebuildable projections.
- Many seats may read. Only the designated writer may append a validated commit transaction.
- Commit checks actor/grant scope, schema, expected revision, lease generation/deadline and artifact digest. A hash chain alone does not authenticate a seat or prove a statement.
- Persist the event before acknowledging acceptance of transport. Duplicate delivery keys return the original receipt.
- External actions require separate idempotency keys/receipts; do not claim universal exactly-once execution.
- Detect incomplete writes/corrupt chains during recovery. Hold dispatch until recovery completes; never silently discard dissent or prior output.

Git tracks source/configuration changes and reviewed PRs. A durable event/artifact store tracks runtime facts. Existing filesystem/JSONL formats can begin the pilot; select a transactional storage adapter after examining concurrency/recovery requirements. No database dependency is admitted by this proposal.

## 6. Topic cases and decision matrix
Use #59's case/notation projection and the existing round-table record: question, scope, evidence, attributed entries, options, positions, dissent, decision and applied revision.
A selected topic opens that record plus its unresolved findings and bounded next action. Typed links may cross topics without copying source contributions.
Show four independent statuses: reference resolved; claim supported/contradicted/unassessed; behavior exercised with coverage; decision authorized. Resolving a file/hash is not semantic verification.
Keep per-seat reasoning and disagreement. No average score, centrality ranking, natural analogy or model consensus authorizes code or a merge.
Nature-informed mechanisms stay hypotheses with failure tests. Retain roots/provenance, branches/alternatives, feedback/correction and iterative cycles as design inspirations without assuming their software adaptation is proven.

## 7. Input/output adapters and direct program operations
Adapters use explicit endpoints, checkout pins, artifact roots, credentials supplied at bootstrap, and allowlisted operations.
- Input: repository and PR snapshots; event replay; deferred findings/audit receipts; bounded contributions.
- Output: per-seat outboxes; returned artifacts/receipts; owner decision requests; separately authorized publication.
- Tool route: a fixed named recipe invokes an existing Aaroneous/devtools CLI/API and records exact command, exit and output digest.
- Agent route: supported CLI/API/client adapters preserve task context, history lineage, restrictions and attribution. A UI chat is not assumed remotely automatable.
- Provider route: existing worker/gateway contracts keep Qwen and Rig experiments modular. A cloud adapter needs explicit data/budget/credential configuration; a subscription seat is not an unrestricted API credential.
- Repository-changing recipes require current claims, an isolated checkout, native gates and independent review; the initial server is advisory/read-only.

Both WORK assignments and FINDINGS handoffs are proposed here, with separate typed payloads and authority checks. Parallel reads with serialized commits are proposed. These answer the owner's earlier questions as recommendations, not as accepted owner decisions.

Use one registered seat ID with explicit display-name aliases, adapter identity and authenticated credentials. A signed message proves possession of a key only under the configured trust registry; it does not independently establish the human/model label, capability, grants or review independence. Normalize display labels such as "MiMo (OpenCode)" through that registry, never through guessed name matches.

## 8. Rotation, leases, profiles and capacity
Reuse steward rotation/framing and golden vectors. Add a monotonic lease generation and full subject revision to the commit contract; a short PR head or holder label alone is insufficient fencing.
A changed model/seat preserves assignment lineage and compliance profile; it does not reset permissions, retry budget or reviewer exclusions. Reassignment may reduce scoped grants, never expand them implicitly.
Select seats by demonstrated capability, availability, task fit and cost constraints. Rotate perspectives without calling them independent reviewers. Track review debt, actual accepted work, oldest actionable item and repeated blockers.
Profile references are pinned to the governing standard and travel with the stage/assignment. Aaroneous's floor/profile rules remain binding at the product boundary. Devtools services retain the companion operating model; a profile label cannot certify conformance.
One shared inference reservation per physical executor plus central leased dispatch prevents competing model loops. Healthy unchanged inventory is idle, not failed work.

## 9. Failures, deferrals, archives and housekeeping
| Outcome | Retained record | Next action |
| --- | --- | --- |
| Model result returned | Pending-review artifact + execution receipt | Independent evidence review |
| Model/adapter execution failed | Failed attempt, error, exact receipt and retry count | Bounded retry only for declared transient errors; otherwise named blocker |
| Evidence stale/unavailable | Deferred/blocked finding with original references | Refresh/resolve authority before dispatch |
| Proposal contradicted | Rejected disposition + useful portions + original output | Bounded repair handoff or terminal close; no silent rerun |
| Reviewed useful output | Scoped disposition and evidence | Route the next authorized task; archive history separately |
| No new eligible evidence | Healthy idle status | Wait for changed inputs or existing scheduled acquisition |
| Retry budget exhausted | Dead-letter/blocked work item + actionable diagnosis | Owner/maintainer intervention |
| Retention pressure | Housekeeping brief with inventory and roots | Pause affected dispatch; cleanup only through explicit approved retention tool |

Archived means retained history, not successful processing. Mechanical process success is distinct from task acceptance.
Cache eviction remains disabled until explicit roots/rules/tool approval. Protect reviewed history, deduplication keys and receipts during compaction. Backups must include event history, artifact digests, snapshot/lease state and recovery metadata; perform a restore drill before claiming recoverability.

## 10. Crash, silence and network partition
A stage crash records the failed attempt and leaves durable inputs intact. A seat that does not answer loses its lease under the existing policy; the next eligible seat gets a fresh generation and retained context.
A late reply is stored as late evidence if authorized but cannot commit under an expired generation. Preserve dissent even when its action is rejected.
Shared Wi-Fi may fail. Clients stop obtaining new leases during disconnection, retain bounded local receipts and resend with idempotency keys. The central writer fences old generations. No automatic second master starts on Windows.
Read-only cached cases may remain viewable with stale labels. After reconnect, refresh state and source revision before making current-coverage claims.

## 11. Windows/Linux service hosting and private access
One maintained Rust server has a foreground bootstrap for development and thin platform hosting adapters:
- Ubuntu: systemd service packaging with explicit executable/config/data paths, readiness, stop/drain deadline and bounded restart policy.
- Windows: optional Service Control Manager adapter with explicit configuration and the same stop/drain semantics.
- No Windows Task Scheduler. OS hosting controls process lifecycle; the reviewed application scheduler owns work routing/cadence.
- Installation, accounts, firewall/network grants and boot registration are deployment changes. This design does not perform them or silently modify host settings.
- Start local-only. Proposed remote owner/worker access uses Tailscale Serve over the existing loopback server, with explicitly reviewed tailnet grants and application seat authentication.
- Do not advertise a subnet, expose a public Funnel or open an unauthenticated LAN endpoint for this pilot.
- Existing exact Host/Origin checks and token flow need a deliberate proxy-aware adapter before Tailscale Serve is usable. Blindly forwarding the current board would not establish a supported remote route.
- Tailnet device/user identity does not authenticate each Claude/Codex/Gemini/MiMo seat on one device. Retain per-seat application identity and independent-review checks.
- A headless service cannot require interactive browser launch/login during unattended operation. Owner onboarding supplies credentials explicitly; renew/expiry failures become actionable blockers.

Official references:
- Ubuntu systemd service lifecycle: https://manpages.ubuntu.com/manpages/noble/man5/systemd.service.5.html
- Windows SCM: https://learn.microsoft.com/en-us/windows/win32/services/service-control-manager
- Tailscale Serve private local-service proxy and identity limitations: https://tailscale.com/docs/features/tailscale-serve
- Tailscale SSH can serve Linux and accept clients on other Tailscale platforms: https://tailscale.com/docs/features/tailscale-ssh
These references establish hosting/network options, not this application's implementation or support.

## 12. Delivery, support and graduation gates
1. C102's design is now delivered in PR135. Resolve its independent review and retained findings before claiming any implementation. Reconcile existing platform-verification work: [PR197](https://github.com/Aarogaming/aaroneous-devtools/pull/197) supplies the pure verification core; [PR203](https://github.com/Aarogaming/aaroneous-devtools/pull/203), [PR204](https://github.com/Aarogaming/aaroneous-devtools/pull/204) and [PR206](https://github.com/Aarogaming/aaroneous-devtools/pull/206) are separate local-verification/integration/job-service proposals. Their presence or acceptance is not evidence of production deployment or of a browser-hosted discussion board.
2. Claim the existing maintenance_ui and relevant contract paths for a bounded topic/case read-view increment. Keep the temporary board intact until verified import preserves every original ID/digest/attribution and dissent.
3. Connect ONE existing seat end to end: outbox -> leased pull -> native tool receipt -> result -> independent disposition -> next bounded task. No invented jobs to keep a GPU occupied.
4. Reconcile the common stage interface with steward/native scheduler work. Freeze cross-language vectors before a Rust rewrite. Never run duplicate effectful dispatchers in shadow.
5. Add foreground server lifecycle/recovery verification on both OSes, then service-host packaging and deployment drills. Do not claim official 24/7 support from compilation.
6. Pilot Ubuntu with explicit host inventory/access, measured inference bounds and outage/restore tests; then add additional seats/providers.

Required verification, ALL NOT RUN for this design: topic/reference contradictions, expired/late leases, duplicate delivery, disk-full/interrupted log writes, unauthenticated/incorrect-seat inputs, stalled HTTP clients, stop/drain during inference, restart/rollback compatibility, Wi-Fi outage/reconnect, review backpressure, archive/dead-letter retention, and backup restore. Foundation formatting/tests/strict Clippy and Windows/Ubuntu CI are required for code increments.
Dependency admission evaluates maintenance/platform/license/compliance distance at the actual boundary. No new Rust/UI/database/service crate is admitted by this design.
Aaroneous builds/tests/releases/runs without devtools. Devtools consumes pinned product tools. Any reusable code entering Aaroneous follows classification, workspace trait/contract tests, proven origin behavior, inert/shadow landing, full product gates and deletion of its origin copy. No automatic release, compliance-profile loosening or acceptance is granted here.

## 13. Decisions and remaining inputs
Recommendations awaiting review: client-owned short/long-term memory and bounded local controllers; both WORK and FINDINGS; concurrent immutable readers with one fenced writer; extend maintenance_ui; Ubuntu persistent host; Tailscale private access optional.
Unresolved: canonical ledger/schema owner; stage contract acceptance; independent reviewer capacity; exact Ubuntu release/disk/VRAM/access; which supported adapter each external seat exposes; version/retention/backup standards and host deployment authorization.
Claim intake does not reserve implementation paths or override active lanes. No Aaroneous checkout, service, credentials, network settings or other agent checkout was changed to produce this proposal.

## Host-access follow-through — 2026-10-05
Historical access notes reported a responding private Linux peer followed by a refused SSH port. These were bounded transport observations, not authentication, inventory or service readiness. Host identifiers, endpoint addresses and raw access receipts belong in the operator's explicitly selected private evidence store and are intentionally omitted from this public proposal.

## Ubuntu SSH account follow-through — 2026-10-05
Historical follow-through reported an interactive identity check and timeout before the requested read-only inventory completed. It did not establish deployment readiness or authorize unattended access. Refer to private operator receipts for exact transport/account details; do not publish them as part of the design.
Operational finding: interactive Tailscale SSH checks may prevent unattended operations. Keep owner inspection access separate from any future explicitly authorized service identity/access policy; do not disable checks or widen permissions automatically.

## Ubuntu inspection completed — 2026-10-05 America/New_York
Historical inspection was subsequently reported successful under the designated operator account; private host-readiness receipts contain the exact commands and observations. The reported OS, GPU/driver and tool findings were a point-in-time 2026-10-05 snapshot, not a present health certificate or a claim that the proposed operations server was installed. Any provisioning or restart requires its own current scoped authorization and evidence.

## Model/operator network and optional Linear — 2026-10-05
Status: recommendation and rollout sequence, NOT IMPLEMENTED or accepted.

### Existing Windows truth
Status route: ordinary maintenance loop PID111468, pending0/archive17, stage blocked-calibration after two reviewed attempts; whole_job_accepted=false. The latest collected cycle returned0 in an existing quiet window. Process liveness is not successful task progression.
Ollama GET /api/tags responds with qwen3.5:9b-q6; inspected worker.toml points to localhost:11434. GET /api/ps returned no loaded models. Preserve this provider/config and its reservations/windows. No model unload/load, keep-alive setting or worker repair was performed.
The product local-agent-delegate skill declares a vLLM14B default, but it is not evidence that this maintenance worker uses vLLM. Owner's requested Ubuntu7B and Windows9B roles are explicit selections; do not silently substitute the skill's14B model or launch another Windows loop.

### Two local model roles
Assume "coder7" means Qwen2.5-Coder-7B-Instruct; confirm the checkpoint before provisioning. Pilot official Q4_K_M GGUF through a pinned CUDA-capable llama-server executable on Ubuntu, one request slot and an initial4K context. Official weights list about4.68GB; runtime/cache/display overhead is additional. This is a measured-fit candidate, not proof it fits the actual card.
Ubuntu GPU mismatch/reboot finding remains the first dependency. VRAM and nvidia-smi health must be established before a GPU-hosted pilot. Native server starts on loopback with an explicit model path, bounded outputs/cancellation and endpoint authentication. Model/executable revision and digests are pinned. No installation, downloads, launch, performance or behavior tests ran for this plan.
Use a reviewed Tailscale private service adapter to expose only the intended endpoint to approved operators. Per-seat application grants remain separate from tailnet machine identity. Existing library/profile graduation rules remain; hosting a third-party executable in devtools is not admitting it to Aaroneous.
Windows Qwen: maintenance/evidence classification. Ubuntu coder: bounded code/fixture drafts. Frontier seats: broader synthesis, challenges and independent evidence review. Roles can rotate with preserved grants and author/reviewer exclusions.

### Network consistency
Tailscale transports authenticated private device traffic; Git retains source/config/PR revisions. They do not themselves dispatch jobs, enforce lease fencing, preserve per-seat private memory or independently review results.
Reuse the steward/native operations capabilities for a single assignment writer with immutable task receipts and bounded per-seat outboxes. Pin each assignment to task ID, full repository revision, evidence digests, schema/stack/profile versions, lease generation, scoped grants and attempt lineage.
Clients retain private short/long-term memory and checkpoints. Only explicit published findings enter shared state. Worktrees remain isolated; no shared live checkout, automatic unrestricted transcript copying or merging divergent private memories.
Discussion route: one pinned question/evidence packet -> individually retained initial perspectives -> exchanged challenges -> synthesis/options/dissent -> independent disposition -> next authorized task. Shared evidence ground is consistent; opinions need not converge. Models repeating the same source are not independent corroboration.
Git/source acknowledgement and runtime event acknowledgement are separate. Network partitions retain bounded local receipts; shared effects stop when their preconditions/leases cannot be established. Reconnect uses deduplication and current-generation checks.
Each frontier seat needs a real supported client/CLI/API/outbox adapter and authenticated identity. Existing UI chats and board labels do not establish unattended model-to-model delivery or API access. Provider credits/data policies remain explicit.

### Linear admission as an optional adapter
Recommendation: extend existing steward/guild execution, use Linear for owner-visible requests, comments, triage and progress if desired. Do not replace the canonical claim/acceptance ledger or add a second dispatcher.
Current official pricing: Free $0, unlimited members,2teams,250issues and agent platform; paid coding sessions/Loops have AI-credit conditions. Agents are not billable Linear seats, but their providers can charge separately. Free project management is not unlimited free inference.
Map Linear issue IDs to canonical task IDs. Assignment/status changes are requests, not direct execution or approval. Preserve independent review and actual evidence receipts; publication to Linear needs explicit authorization.
Linear webhook consumers require a public HTTPS URL, so a private Tailscale Serve URL alone is not reachable by Linear's cloud webhook service. Initial private integration should use explicit authenticated outbound GraphQL polling with pagination/backoff/cursors and reconciliation. Full Agent Session integrations depend on webhook delivery and require a separately reviewed public ingress/relay; do not claim ordinary polling implements that API.
Linear agent APIs currently describe themselves as Developer Preview. Keep this external adapter replaceable and versioned. If disconnected or omitted, local task/event operations continue.
Linear integration exists in the plugin directory; suggested only, NOT installed/connected. No Linear workspace inspected, issue posted, OAuth grant, credentials or account settings changed.

### Ordered bounded delivery
1. Owner-controlled Ubuntu restart and read-only GPU recheck.
2. Approve/provision one pinned Ubuntu7B server and exercise bounded native provider/tool contracts, truthful receipts and resource fit.
3. Resolve Windows blocked-calibration handoff through the existing component/lane; extra compute or a new work board does not fix that defect.
4. Connect one existing operator outbox end to end with lease/result/review/next-action handling, then add other perspectives.
5. Add an optional Linear projection/intake adapter without changing canonical authority.
No new claim, broker, scheduler, inference job or deployment is established by this addendum. Formal design/source work still follows C102 Ready/claim and independent review.

Official references:
- https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct-GGUF
- https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
- https://tailscale.com/docs/features/tailscale-serve
- https://linear.app/pricing
- https://linear.app/docs/agents-in-linear
- https://linear.app/developers/webhooks
- https://linear.app/developers/graphql
- https://linear.app/developers/agents

## Owner direction: local first — 2026-10-05

This additive direction supersedes prioritizing the optional Linear adapter. Status: DESIGN / READ-ONLY RESEARCH; no tracker deployed, dependency admitted, model provisioned or new dispatcher built.

- Keep operational records, evidence and shared task state on owner-controlled hosts. Clients retain independent private short/long-term memory and bounded local execution, with explicit published findings crossing into shared state.
- Reuse the existing steward/native stage capabilities and canonical claim/review authority. A tracker presents cases and submits requests; it cannot grant execution, review acceptance or merge authority. Aaroneous never depends on devtools.
- Local models handle bounded supported work. Cloud peers remain optional adapters with explicit scope and data publication. A cloud outage must not remove local planning, evidence access or authorized local work.
- Ubuntu is the proposed shared operations host; Windows retains its existing maintenance provider. This is an intended topology, not established availability or a claim that Qwen is continuously resident.
- Tailscale provides private transport and Git source history. Clients must still fence stale leases, deduplicate receipts and reconcile explicit shared effects after disconnection. One server is not distributed offline sync.

### Replaceable Rust tracker candidate

Evaluate Lific first: its documentation describes a Rust binary, SQLite, web UI, REST/MCP and project roles. The inspected LICENSE is Apache-2.0; Cargo.toml declares edition 2024 and Rust 1.88. Its single-server storage does not provide cross-server synchronization. Auth-optional mode and unbound operator keys grant elevated access; initialization can install a service and connection setup can edit client configuration. These are documented behaviors, NOT independently exercised findings.

Keep any eventual evaluation in an isolated project with explicit config, bound identities, a narrow adapter and independently reviewed receipts. Preserve native task IDs and evidence provenance. Test restoration, access boundaries, duplicate/stale requests, rejected writes and disconnected recovery before considering adoption. Review dependency admission vectors before integrating source. No automatic init/connect, live claim import, service registration or repository edits are authorized by this research.

Other candidates: beads_rust is a local CLI alternative, but its LICENSE includes an OpenAI/Anthropic rider; do not present it as plain MIT or a cleared dependency. kanban-tool documents an unauthenticated web interface, so it is not selected as a shared operations server.

Next: finish independent review of this existing C102 design and reconcile the verification/job-service lanes before claiming a bounded adapter trial. Revalidate dated host/model assumptions through private operational evidence; no new deployment or service claim is made here.

References (read-only, branch views; immutable source revision NOT VERIFIED):
- https://github.com/VoidNullable/lific
- https://github.com/VoidNullable/lific/blob/master/Cargo.toml
- https://github.com/VoidNullable/lific/blob/master/LICENSE
- https://github.com/VoidNullable/lific/blob/master/SECURITY.md
- https://github.com/Dicklesworthstone/beads_rust/blob/main/LICENSE
- https://github.com/AZQN0/kanban-tool

Public commit/release pin lookup failed: gh api repos/VoidNullable/lific/commits/master --jq '{sha: .sha, commit_date: .commit.committer.date}' and gh api repos/VoidNullable/lific/releases/latest --jq '{tag: .tag_name, published_at: .published_at, prerelease: .prerelease}'. Both reported proxy connection refusal; combined shell exit 1. Web source reads succeeded. No build, test, benchmark or deployment command run.

## Replaceability contract and admission rubric — 2026-10-05

Owner requirement: swap components later without rebuilding the whole system. Status: PROPOSED; contract conformance, replacement and rollback have NOT RUN.

### Boundary rules
- Map each capability to its existing workspace-owned contract before extending it. Keep third-party SDK types, database tables, model prompts and credentials inside its adapter. Stage policy consumes typed inputs and returns typed outcomes; bootstrap injects dependencies.
- Interchangeable services use a versioned process protocol with explicit capabilities, request IDs, schema versions, evidence references, lease generations and typed errors. Bind adapter identity/endpoint/config at bootstrap through the reviewed stack manifest. This permits replacing a compatible service or adapter without relinking the whole platform.
- An in-process Rust trait alone does not promise binary replacement. Rust plugin ABI compatibility is not assumed. Incompatible contracts or statically linked adapters can require component rebuilds; negotiate versions or provide a separately reviewed migration.
- Retain the established canonical task/event authority. Adapter-specific issue IDs are mappings. Vendor exports or status fields cannot become the sole record of evidence, authority or independent acceptance.

| Capability boundary | Contract must preserve | Replacement evidence required |
|---|---|---|
| Stage execution | Typed input/output, profile, events and explicit limits | Existing stage vectors plus refusal/degraded outcomes |
| Model/tool provider | Bounded requests/results, tool grants, cancellation, actual receipts | Same declared provider cases, schema failures and interrupted execution |
| Tracker/client interface | Native task ID mapping, scoped requests and independent disposition | Round trip mapping, rejected writes, duplicates and stale updates |
| Client memory/storage | Private/shared separation, provenance, retention and explicit export schema | Export/import equivalence, redaction and recovery from interrupted migration |
| Event persistence | Ordered durable acknowledgements, evidence lineage and writer fencing | Restart/replay, duplicate delivery and stale-generation rejection |

### Common rubric
Use Pass / Fail / NOT RUN for each requirement, with exact input revision, command, exit code and artifact. Evaluate authority/profile preservation; contract compatibility; evidence truthfulness; bounded resource use; failure/recovery; data portability; local/offline operation; and replacement cost. Required security, authority, compatibility or durability failures block activation regardless of aggregate scores. Performance tradeoffs must be measured against declared budgets, not inferred from marketing.

Before dependency admission, retain the existing alloc/ambient/abi/safety distance scores (0-4 each) and Admit / Admit with conditions / Extract pattern / Reject verdict. A devtools service trial is not product dependency admission. Apply Aaroneous profile conditions and graduation when crossing the product boundary.

### Swap and rollback procedure
Prepare candidate with the same pinned contract cases; run shadow/read-only comparison; obtain independent review. Drain the affected route, checkpoint acknowledged state, fence the old writer and increment lease generation. Use an explicit export/import migration when needed, preserving original artifacts and recording mapping/receipts. Switch only the affected adapter binding; resume after reconciliation. Do not run two authoritative writers.

Retain the previous binary/config and a restoration checkpoint. Rollback must account for schema changes and any effects committed after the switch: do not erase accepted event history, replay already committed effects or claim an older binary can read a newer format without evidence. Failed swaps leave the route in documented degraded/safe-hold with a bounded repair packet.

These are design requirements, not implementation or conformance claims. Source changes still follow the existing intake/claim/review sequence.

## Inter-operative continuity — owner direction, 2026-10-05

Meaning: an operation remains coherent and recoverable while responsibility moves between stages, models, clients, adapters or hosts. Reuse the existing task/stage/outbox contracts and extend their missing fields through the coordinated design; this is not a second handoff engine. Status: PROPOSED / NOT VERIFIED.

### Continuity contract
Every handoff references a versioned operation record containing:
- Stable operation/task ID, parent/attempt lineage, intent, scoped responsibility and explicit completion criteria.
- Pinned source/evidence revisions and digests, recorded tool receipts, current findings, unresolved questions, dissent and deferred items with their dispositions.
- Governing contract/stack/profile versions, authenticated seat identity, scoped grants, review exclusions, lease generation and remaining retry/resource limits.
- Last durable checkpoint, acknowledged effects, outstanding effect IDs, next bounded action, and declared recovery/degraded route.

A summary is a view of this record, not its sole source of truth. Missing context or unsupported versions produce an explicit continuity gap; the receiving operator must not invent evidence or silently widen authority. Client-private memory stays private unless explicitly published; the shared record must contain enough authorized context for another operator to continue without access to that memory.

### Handoff and recovery
The sender records a handoff against its last durable checkpoint. The canonical writer validates authority, compatibility and lease preconditions before committing reassignment; the receiver acknowledges the accepted operation version and checkpoint before producing shared effects. Fence the previous lease. Distinguish delivery, durable receipt, execution and independent acceptance acknowledgements.

Retain in-flight effects by stable IDs and reconcile their outcomes before retrying. Deduplication and transactional updates handle supported internal effects; external effects may remain outcome-unknown after a crash and require reconciliation. Do not promise exactly-once execution across every adapter.

When a peer is unavailable, preserve the checkpoint and evidence, then use the declared bounded retry/reassignment or degraded/safe-hold route. Continue only work whose authority and inputs remain valid. Deferred findings preserve their dependency/revisit condition and owner/action route; archived artifacts retain their actual verdict rather than becoming successful by relocation.

### Continuity rubric and proposed cases
Evaluate preservation of intent, evidence provenance, acknowledged progress, restrictions, profile, reviewer independence and pending dispositions. Record Pass / Fail / NOT RUN separately for delivery, recovery and semantic sufficiency. A receiver restating a summary is not proof that the evidence supports it.

Proposed cases: model substitution; client restart; host/network interruption; stage crash with an outstanding effect; stale lease/result; unsupported contract version; adapter migration; and resuming a deferred task after its dependency changes. Each case must show what carried forward, what was withheld, what was reconciled and why the next action remained authorized. These cases have NOT RUN.

This contract preserves continued progress through supported transitions; it does not claim uninterrupted availability or transfer a prior operator's acceptance authority. Independent review still applies after reassignment.
