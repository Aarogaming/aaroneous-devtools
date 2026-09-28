> **Recovery note (2026-09-27, Claude):** recovered alongside [SYSTEM_COMPOSITION_BLUEPRINT.md](SYSTEM_COMPOSITION_BLUEPRINT.md) from the same stash reference; see that file's recovery note for provenance. Verbatim, no content changed.

# System Composition Blueprint: Design Critique

Reviewed 2026-09-25 UTC against [blueprint revision 1](SYSTEM_COMPOSITION_BLUEPRINT.md), the foundation boundary, Cratify v2, and the current architecture references. This is the same author's second-pass review. No independent reviewer, runtime experiment, or product gate is represented by this document.

## Verdict

The proposal is coherent enough for policy and contract design. It is not ready for runtime implementation. Its strongest property is the separation of composition, resource ownership, and authority. Its largest unresolved dependency is how the execution host issues and enforces scoped authority through today's public APIs.

## Findings and dispositions

| ID / priority | Finding | Revision-1 disposition | Remaining acceptance evidence |
| --- | --- | --- | --- |
| R1 / blocking | Existing generic naming rules do not yet authorize all named systems. | Sections 1, 7, 11 label registration as a proposed amendment. | Reviewed product policy with exact replacement wording |
| R2 / blocking | Ring language implies privilege and can conflict with profile classifications. | Sections 3 and 11 separate structural rings from authority and require both existing constraints until reconciled. | Explicit compatibility/precedence matrix approved in product docs |
| R3 / blocking | Passing a handle alone does not prove bounded authority or tenant isolation. | SC-06 requires restricted handles plus host checks; sections 5 and 10 require negative/revocation tests. | Concrete issuer, enforcement point, grant lifetime, cancellation and revocation contract |
| R4 / high | A pinned commit does not prove the baseline works; the current product has changed since earlier checks. | Sections 1 and 8 require a fresh verified immutable baseline; observed HEAD is contextual only. | Recorded native gates and clean checkout before consuming the pin |
| R5 / high | Calling Core foundational could recreate a mandatory monolith or confuse Rust's `core`. | Section 2 distinguishes name, preset, directory and contract crate; section 7 defers packaging. | Minimal scope and name registration from inventory |
| R6 / high | Shared compilation was being mistaken for shared runtime resources. | Section 6 separates package/artifact reuse from resource-instance reuse. | Instance count, memory, startup and isolation measurements with two consumers |
| R7 / high | Base36 lineage could be mistaken for compatibility or authenticity. | Section 6 separates identity classes and rejects lineage as proof. | Per-ID encoding contracts and artifact/contract verification design |
| R8 / high | Nested presets could multiply budgets or union permissions. | Sections 4–5 impose bounded expansion, aggregate accounting and grant intersection. | Diamond-graph accounting and nested-escalation fixtures |
| R9 / high | A façade is not a sandbox for hostile native code. | Section 4 states the limitation and excludes dynamic loading from the first implementation. | Isolation design before any untrusted execution scope |
| R10 / medium | One new crate per historical name would add empty structure and duplicate capabilities. | Sections 2 and 7 allow data-only presets and require evidence before packaging. | Contract inventory and one concrete first scenario |
| R11 / high | A live path dependency defeats independent base development. | Section 8 requires an explicit immutable checkout and deliberate pin updates. | Reproducible consumer build and compatibility report |
| R12 / high | Private blueprint could become a second product constitution. | Sections 1 and 11 require publication to product and replacement with a planning pointer after adoption. | One canonical product policy location with links from devtools |
| R13 / high | Rollback is not necessarily safe after state migration or external effects. | Section 10 separates code rollback, data compatibility and compensating behavior. | Recovery test before stateful activation |
| R14 / medium | "Single implementation" cannot be proved by a simple lint. | SC-02 explicitly requires ownership review as well as structural checks. | Consumer/implementation map and reviewer judgment |

## Adversarial walkthrough

Consider Core and Cratify both referring to the same storage service, while a nested preset asks for write access. Resolution must identify the same declared resource binding and preserve one lifecycle owner. Core's presence cannot grant writes. The nested request is denied unless both the host policy and delegating grant allow it. The shared service keeps separate client scopes and quotas; one consumer shutting down cannot terminate the other's service. Revocation must block new operations, and the host must define treatment of in-flight work. A stale approved plan cannot activate against a changed artifact or provider.

The proposal accounts for these cases as requirements. It does not yet establish that current Aaroneous APIs can enforce them. That gap is why the contract inventory and authority design precede the pilot.

## Alternatives considered

- Put all compositions in the product immediately: simpler checkout arrangement, but breaks the agreed construction hold and couples evolving policy to the unfinished base. Defer until graduation.
- Fork or wrap-copy the base APIs: initially convenient, but creates duplicate implementations and drift. Reject; submit missing-contract proposals.
- Give Core superior authority: convenient discovery point, but produces implicit privilege and compulsory coupling. Reject; roles and grants stay separate.
- Build a general workflow language first: flexible, but introduces execution semantics and a much larger validation surface. Defer; version 1 remains data-only with fixed supported operations.
- Split every concept into a crate: maximizes apparent modularity while multiplying package boundaries. Reject as a default; split only for a real responsibility, profile, or ownership boundary.

## Follow-up review procedure

An independent reviewer should trace every SC rule to a failure case, challenge the authority and resource accounting model, inspect the proposed product-policy amendments, and confirm that the pilot uses only existing public APIs. Record accepted, revised, or rejected decisions by rule ID. Do not mark the blueprint Done based on this self-review.

The next deliverable is a bounded policy-amendment and contract-inventory packet, not a running system: exact naming/ring changes; one candidate scenario; existing callable interfaces; missing contracts; proposed ownership/grant issuer; accepted test budgets. Only after those decisions should the resolver implementation receive a Ready claim.

## Correction-program follow-up

The owner-directed addendum in blueprint sections 12–13 expands applicability to both repositories and defines the six-phase correction program. It preserves the original review verdict: proposed safeguards must not be presented as confirmed code defects before inspection.

| Review findings | Primary correction phases |
| --- | --- |
| R1, R2, R12: naming, rule consistency, authoritative documentation | Phase 1 |
| R4, R5, R10, R14: base evidence, Core scope, packaging, implementation ownership | Phases 2–3 |
| R3, R7, R8: authority, identity and nested accounting | Phases 2 and 4 |
| R6, R9, R11: resource sharing, isolation and independent development | Phases 4–5 |
| R13: stateful rollback and external effects | Phase 6, designed before stateful activation |

Second-pass check of the addendum: the program now distinguishes shared principles from profile-specific constraints, maps its six phases onto the existing delivery stages, and requires an evidence-backed correction record before implementation. Remaining naming, ring/profile and host-authority decisions are unchanged; this update records the remediation process and does not resolve those decisions or constitute independent approval.
