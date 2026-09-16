# Local Agent Control Plane

The scheduler is deterministic. A local model may analyze bounded evidence and return a proposal,
but it cannot claim work, edit either repository, invoke a shell, alter a schedule, or decide that
verification passed. The integration owner makes each claim after reviewing the proposal.

## Layers

| Layer | Responsibility | Authority |
| --- | --- | --- |
| Scheduler | Read queue/program, calculate capacity, recommend eligible claims | Read-only |
| Evidence builder | Select explicit reports, source excerpts, and task contract | Read-only |
| Prompt compiler | Produce a bounded, versioned task prompt and policy preamble | Write only a prompt artifact |
| Local worker | Ask an explicit Ollama endpoint for a structured proposal | Write only an output artifact |
| Evaluator | Validate JSON shape, declared scope, and required evidence references | Read-only |
| Integration owner | Claim work, choose worker, approve a patch, run verification | Human authority |

## Qwen worker contract

The default worker is `qwen3.5:9b-q6` at an explicitly supplied local Ollama endpoint. Each call
uses temperature `0.0`, at least 4096 predicted tokens, a strict no-reasoning preamble, and a
single bounded responsibility: inventory analysis, test fixture proposal, report classification,
or a narrowly described code draft in a scratch artifact.

Input consists only of declared artifact paths copied into a versioned evidence bundle. Output is a
proposal record with task ID, input digests, suggested scope, risks, test plan, and optional patch
text. The proposal is not a queue claim or approval.

## Native proposal command

`foundation local-proposal --endpoint <http-url> --model qwen3.5:9b-q6 --evidence <absolute-file>
--output <absolute-file>` sends only the declared evidence file to the supplied local endpoint.
It fixes temperature at `0.0`, rejects a token budget below 4096, limits evidence to 64 KiB, and
requires the model response to parse as the versioned proposal record in
`contracts/local-proposal-v1.schema.json`. It never receives a checkout path and cannot mutate a
queue or apply a patch.

## Rust stack choices

- **Direct Ollama adapter first:** a small Rust HTTP boundary exposes a stable local-worker
  contract without making framework code depend on any AI crate.
- **Rig second:** use Rig's Ollama provider behind that boundary when prompt composition, tool
  schemas, or provider interchangeability becomes useful. The adapter keeps Rig out of product
  code and allows a replacement without rewriting coordination policy.
- **Candle experiment lane:** use a separate experimental foundation crate only for explicitly
  provisioned, Candle-compatible model artifacts. It is not the runtime for the installed GGUF
  Qwen model and does not belong in the scheduler's critical path.
- **Embeddings and retrieval:** defer until evidence bundles are too large for deterministic
  selection. If introduced, index only approved private artifacts with a reproducible manifest.

## Required gates

1. The endpoint, model, evidence bundle, output path, and token budget are constructor/CLI inputs.
2. Prompts cannot request command execution, repository writes, credentials, or task claims.
3. Model output must parse against a proposal contract; malformed output is retained as failed
   evidence, never retried into a shared branch.
4. All generated patches stay in scratch space until an owner reviews and applies them.
5. Native Aaroneous verification remains the only evidence that product work is correct.

## Delivery order

1. Add prompt and proposal contracts with fixtures.
2. Add a Rust local-Ollama proposal command using explicit inputs and bounded output.
3. Add deterministic proposal validation and audit trail reporting.
4. Add an optional Rig adapter behind the same contract.
5. Evaluate Candle separately against a named experiment and measurement plan.
