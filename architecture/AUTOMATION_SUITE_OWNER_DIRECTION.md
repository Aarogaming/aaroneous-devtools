# Aaroneous Automation Suite: owner direction, 2026-10-30

Status: **owner-directed architecture record proposed for independent review**. Source: owner's discussion in this chat on 2026-10-30. This captures intended direction, not claims that the named systems or deployment targets are implemented, qualified, approved for release, or that existing rules have been amended.

## Vision

Aaroneous is the Rust component foundation and intended long-term home of the Aaroneous application and Automation Suite. It should support composing reusable, narrowly scoped capabilities into applications and operational systems. The aim is broad, calculated operation: flexibility is permitted within explicit authority, resource limits, evidence requirements and governance. A system name describes a coherent upper layer or preset, not a bypass of descriptive internal names, crate ownership, product profiles, verification or dependency rules.

An installable desktop/server application and constrained deployments are intended to share selected foundation capabilities. Target profiles may include Linux/Ubuntu and Windows first, with Android and supported PLC/microcontroller environments later. A constrained target selects only capabilities it can support; it is not expected to host every service, model or UI. AI-assisted analysis and deterministic control have separate authority and execution boundaries. Industrial deployment requires target-specific qualification and any applicable assurance, timing and regulatory evidence; none is implied here.

## Suite nomenclature and intended responsibilities

| Name | Owner-directed intent | Boundary to reconcile |
| --- | --- | --- |
| Aaroneous | Component foundation and intended home for the full suite. | No product build, test, release or runtime dependency on Devtools; individual capabilities graduate through qualified product changes. |
| Guild | Operations kernel member for coordination, routing, governance and resource/meta awareness. | Policy decisions, task state, trusted review and evidence retain explicit authoritative owners. |
| Steward | Operations kernel member stewarding intentions, job cards, assignment, handoffs, attempts and final disposition. | Reuse existing authoritative verification, wire and receipt records; proposed lifecycle is not yet adopted. |
| Merlin | Operations kernel member providing intelligence, model management, creation/scaling, inference, user queries and web research. | Model output is a proposal or observation, not a grant or independent verification. GGUF is named. Product `si_format` defines utilities for the existing `.si` container format; the desired future model-creation and scaling workflows and their qualification remain to be established. |
| Library | Raw documentation, research, knowledge, durable evidence and relationships. | Define storage, provenance, trust, retention and index ownership; evidence does not become true by being stored. |
| Chimera | Software operations, including development, analysis, execution, virtual branching and Git-based work. | Separate proposal, authorized effects, independent review and integration. |
| Marionette | Desktop use and domain-application interaction, including supported back-of-house interfaces where permissible. | The target and permitted operations determine the adapter; application rules and platform access are independent constraints. |
| Platform Bridge | Platform/OS adapters and potential support for outbound connectivity. | Existing primer assigns OS isolation; inspect transport, federation and other current components before assigning all routing, messaging or peer discovery to this crate. |
| Argus | Security division. | Specify trust, authentication, policy enforcement and security verification boundaries; do not grant authority from a label. |
| Cratify | Verification and component conformance. | Reuse actual existing audits, profiles and gates; distinguish verification evidence from approval. |
| Studio HUD | Official presentation component and host. | Platform-specific windowing and overlay support require separate qualification. |
| Ariel | Presentation and interaction persona. | Persona cannot grant permissions or become a substitute for accessible, inspectable controls. |
| Maelstrom | Gamified/spatial operating experience and customizable views hosted through Studio HUD. | A view over authoritative records, not a second authoritative ledger. |

## Operation, communication and recovery

- A user intention should express outcome, context, permitted and prohibited effects, resources, budgets, success criteria and decision authority; a bounded job carries a scoped portion of that intention.
- Local, LAN and remote operations are explicit selectable scopes. Machines can host individual operations centers and collaborate through authenticated, bounded adapters; selecting a capability, host or model does not grant authority.
- A result, observation, verification, independent review, integration and deployment are distinct facts. Retain provenance, exact subjects, attempts, failures and unknown outcomes; a merge is not proof that a coordination job is complete.
- Components expose roles, compatibility, resources and effects through versioned contracts; registered upper-layer systems compose lower-level capabilities rather than repackage or silently duplicate them.
- Different loops may operate at different cadences. Each needs a purpose, input/output, budget, supervision and a defined recovery/hold path. Lease expiration does not prove a worker has stopped; stale effects require fencing or reconciliation.
- Outbound messaging, machine bridging, API and MCP integration require explicitly authenticated transport and delivery semantics. Do not assume Platform Bridge owns all those concerns before source mapping and review.

## Research, presentation and model representation

Library and Merlin support research through source retention, indexed facts, hypotheses, experiments, evaluation and revisable conclusions. A spatial or constellation view can render nodes and typed relationships, with metadata, search, filters and alternate list/hierarchy views. NASA imagery is an aesthetic inspiration; scroll-as-hierarchy navigation requires usability testing. The view does not become the authority for underlying facts.

Translate human intent into validated, structured machine-readable records and return explanations to users. Graphs, retrieval and other representations are research inputs, not proof that a new model language or binary training is superior. Evaluate alternatives experimentally, including efficiency, interpretability and interoperability. Model/hardware selection should use explicit requirements and measured available resources; smaller models, deterministic algorithms or no model may be appropriate.

Digital twins may model software, devices or processes. Human-behavior testing can use explicitly chosen simulation parameters such as response intervals or fatigue; inferred psychological traits are not established facts about a person. Game automation is an independent experimental consumer and may be connected to qualified components later; a simulator or recorded fixture can test the intended interface without claiming permission to operate a particular third-party application.

## Hardware-aware operation and resource planning

The intended suite should adapt work to the user's selected hardware and priorities. Guild routes bounded work using measured available resources and explicit configuration; Merlin selects a suitable model or non-model capability; target-specific adapters report resources without silently discovering credentials or granting authority. Devices may have their own operation centers and cooperate over authorized local, LAN or remote links. Foreground responsiveness, resource budgets, consent and independent acceptance remain distinct from scheduling.

### Operating modes

| Mode | Intended behavior |
| --- | --- |
| Idle | Event-driven waiting and lightweight status; models may be unloaded. No assumption that a continuous inference loop is needed. |
| Observe | Bounded watchers or deterministic analysis; invoke heavier components when an event or task warrants them. |
| Assist | One selected small model or deterministic capability handles interactive requests within configured resource limits. |
| Work | Explicitly admitted specialists run tests, builds, indexing, screen processing or model inference; concurrency is budgeted. |
| Escalate | Route a bounded task to a larger local or remote model or another authorized machine when policy allows. |
| Constrain | Reduce concurrency, defer work or preserve a hold when resource limits, foreground priorities or safety conditions require it. |

A platform profile selects only compatible components. A desktop or server may run the operations suite; a PLC or microcontroller may run a small deterministic component and use a separate coordinator when appropriate. Neither AI connectivity nor every service is required on a constrained target. Industrial timing and assurance remain separate qualification requirements.

### Illustrative workstation planning ranges

These are rough **whole-machine starting points for one active user**, not measured Aaroneous requirements, guarantees of responsiveness, minimum specifications, or purchased-hardware recommendations. Actual use depends on the operating system, application, local versus hosted models, context and cache, screen rate, build parallelism and concurrent users.

| Example workload | Starting point to benchmark | Likely pressure |
| --- | --- | --- |
| Idle operation center | 2-4 CPU cores; 8-16 GB system RAM; no dedicated GPU needed | Background services, storage and network |
| Study assistant with hosted model | 4 CPU cores; 8-16 GB system RAM; no dedicated GPU needed | Documents, browser, network and remote latency |
| Study assistant with small local model | 6-8 CPU cores; 16-32 GB RAM; GPU helpful | Model and context memory |
| Coding assistant with one local model and modest builds | 8-12 CPU cores; 32 GB RAM; 8-12 GB GPU memory helpful | Inference, compilation, indexing and disk |
| Game-like observer and local decisions | 8-12 CPU cores; 32 GB RAM; GPU depends on game, screen analysis and model | Application rendering and AI competing for GPU memory |
| Desktop/browser software tester | 8-12 CPU cores; 16-32 GB RAM; more for parallel environments | Target apps, screenshots, test runners and vision |
| Several concurrent code/test workers | 12-16+ CPU cores; 64 GB+ RAM and fast SSD; GPU sized for actual model concurrency | Parallel builds, isolated environments, inference and I/O |

Examples of model package sizes illustrate why configuration matters: Ollama lists Qwen3 8B Q4 at about 5.2 GB and Qwen3.5 9B variants around 6.6-7.6 GB. Package size is not total runtime RAM or VRAM: execution overhead, context, cache, vision processing, offload and other GPU users add to it. Sources: [Qwen3 tags](https://ollama.com/library/qwen3/tags) and [Qwen3.5 library](https://ollama.com/library/qwen3.5). Verify the selected artifact and runtime on the actual host; these examples are not mandated models.

The Ubuntu operations host's RTX 4060 is an intended test resource. Do not assume its exact VRAM or available headroom without measuring it. Keep screen-observation frequency separate from model-decision frequency and measure each against the target application's responsiveness. Windows and Ubuntu can execute independent jobs and exchange bounded requests and evidence, subject to existing ownership and review; current worker recovery is not qualified for unrestricted unattended dispatch.

### Measurements and qualification

Before publishing resource requirements or declaring a profile supported, run reproducible idle, interactive and worker workloads on each target. Record machine and OS, software revision, selected components, model artifact and quantization, context and concurrency, CPU and system RAM, GPU/VRAM, disk and network use, latency and throughput, thermal state and power where measurable. Record test inputs, duration, results and limitations, plus responsiveness and restart/recovery behavior. Measure a machine's actual energy consumption rather than equating GPU nameplate power with whole-system average.

Monthly electricity consumption can be estimated as `average system watts × hours per day × days / 1,000` kWh; multiply by the user's actual tariff. Retention and cleanup policies for model artifacts, logs, recordings and build caches must preserve required evidence. These profiles are goals for future qualification, not a deployment, benchmark result or a claim that the current worker is reliable.

## Qualification sequence

1. Preserve existing ownership and independent review; reconcile this owner direction with the suite composition blueprint and current product constitution before adopting names or policy.
2. Qualify the existing product capability export, caller authority and verification contracts (C79/C80/C85) and independently resolve Guild operation decisions (C158).
3. Correct job-result acceptance, durable history, claims, effect fencing, unknown-outcome recovery and merge-to-queue reconciliation in their appropriately claimed lanes.
4. Demonstrate a narrow end-to-end operation: explicit intent; one job; one selected capability; retained evidence; independent evaluation; recorded final or unknown state; restart recovery.
5. Exercise the same contract with a Devtools consumer and a separate non-mutating game-like fixture. Then qualify selected components and platform adapters on both Windows and Ubuntu.
6. Separately qualify constrained-target components against real target budgets and gates; add features and deployment profiles only when supported by evidence.

## Canonical adoption handoff

The intended long-term home for the owner's decisions is `governance/DECISIONS.md` with the architecture and navigation cross-references reconciled under a non-overlapping claimed lane. At the time of drafting, C163 has `governance/DECISIONS.md` in Review, while C158/C170/C171 cover the main Guild plan. Consult the canonical local coordination queue identified by the agent guide; recheck it, remote main, open PRs, path ownership, capacity and current heads before editing. Contribution-ID: C177/codex-owner-suite-direction-v1. Do not create a duplicate Ready row. The C177 claim covers this new architecture record only. Reconcile authoritative documents in separately scoped lanes and retain independent review before treating new policy or interfaces as adopted.
