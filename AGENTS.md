# Agent Guide

This is the operational entrypoint for a contributor joining Aaroneous work. Aaroneous is the product repository; `aaroneous-devtools` is the private coordination and foundation repository. Read this guide before changing either repository.

## Repository standards

Read [the companion operating model](governance/OPERATING_MODEL.md). Devtools permits ordinary heap allocation, async I/O and synchronization; Aaroneous hot-path restrictions do not apply to companion implementation. Rust owns durable maintenance policy, TypeScript may own presentation, and experiments stay isolated. Changes to Aaroneous still obey its own constitution. Local and paid models share the same evidence and authority boundaries; paid escalation does not bypass ownership or verification.

## Before selecting work

1. Read `FOUNDATION_BOUNDARY.md`, `governance/AARONEOUS_COMPONENT_FRAMEWORK_PRIMER.md`, `governance/MAINTENANCE_SYSTEM.md`, and `governance/MULTITHREAD_PROGRAM.md`.
2. Read `governance/COORDINATION_QUEUE.md`. Do not edit a path owned by an active claim.
3. Review the task's cited evidence, current branch, and repository-local `AGENTS.md`.
4. Run the read-only scheduler review if capacity or eligible work is unclear.

## Claiming a lane

Claim only a `Ready` item. Record the owner, branch, UTC start time, exact files, intended evidence, and verification gate in the queue before editing. One primary agent owns a file scope at a time. Claims expire after 24 hours without an update; a successor records a handoff before taking over.

### Seat-status self-refresh exception (owner-authorized 2026-10-09, C112)

After an ordinary exact-path claim has created `governance/seat-status/<seat-id>.md`, that seat may refresh **only its own existing file** without another queue claim. A refresh must run `python3 scripts/check_seat_status.py governance/seat-status` and cite its exit in the commit or PR. It may not create, rename, delete, or edit another seat's status file; edit `governance/seat-status/README.md`; change the validator or its tests; claim work; grant authority; or include any unrelated path in the refresh. Initial creation, protocol/schema changes, and removal remain ordinary claimed work.

Choose an independent task. Do not open overlapping inventory, terminology, interface, or package work merely because a second model is available. Keep the default to three primary lanes and one review lane.

## Working rules

- Aaroneous product code follows its own `AGENTS.md` and native verification gates.
- Devtools must never become a product build, CI, release, runtime, or crate dependency.
- Use explicit checkout, endpoint, output, and support-directory inputs. Do not infer machine paths or read ambient configuration.
- Keep generated artifacts, experiments, model outputs, and incomplete patches out of shared product paths.
- Make small commits with clear evidence; do not rewrite or discard another contributor's work.

## Local Qwen support

Qwen is a support worker, not a primary owner. It may analyze a declared evidence bundle, draft a test fixture, classify a report, or propose a narrow scratch patch. It cannot claim queue work, edit a shared branch, execute commands, or declare verification successful.

Follow `governance/LOCAL_AGENT_CONTROL_PLANE.md`: use `qwen3.5:9b-q6`, temperature `0.0`, at least 4096 predicted tokens, a strict concise-output preamble, explicit inputs, and a versioned proposal artifact. A seat independent of the submitter accepts every output before adoption.

## Completion and handoff

Run the applicable verification gate, record the command/result and commit in the queue, then move the claim to `Review`. A reviewer checks scope, evidence, and the required gate before marking it `Done`. If blocked, state the exact blocker, preserve the evidence, release the paths, and leave a succinct handoff for the next claimant.

## Starter prompt

```text
You are joining the Aaroneous multithread program. Begin by reading aaroneous-devtools/AGENTS.md, the coordination queue, and the multithread program. Do not edit any repository until you have selected an unowned Ready item, recorded an exact-path claim, and read Aaroneous AGENTS.md. Work only within the claim scope. Use Qwen only as a bounded, independently reviewed proposal worker under LOCAL_AGENT_CONTROL_PLANE.md. Record evidence, verification, commit, and a handoff in the queue before declaring completion.
```

## Feeding the local support inbox

The native polling worker is described in `governance/LOCAL_WORKER_SERVICE.md`. After checking queue ownership and claiming the exact devtools manifest path, an integration owner may submit a narrowly approved analysis, test-fixture, or code-draft job in `worker-jobs/`. Use existing sources and bounded acceptance criteria; do not fabricate busywork. The worker automatically discovers new manifests and changed evidence. A support job is not a primary queue claim and its output is not approval. Review pending results before submitting more; archive reviewed records with their history intact. Cooperating model work must honor the shared reservation and quiet windows rather than starting independent Ollama loops.
