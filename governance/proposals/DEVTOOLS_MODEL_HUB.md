# Devtools as the model hub (lane C133)

Status: proposal plus the first Rust slice (`foundation/crates/steward/src/model_hub.rs`). Author: Claude seat. Builds on HUD
proposals 02 (agent door), 04 (token use), 05 (model profiles) and C129 (platform verification). Grants nothing.

## The idea, and the line it must not cross

Devtools becomes the one place that talks to models: it supervises the local model server on each machine, calls remote providers
with keys it alone holds, applies the permission engine and budgets, adapts prompts per model, and records what answered. One hub
means one place for routing, secrets, telemetry and audit.

One hub is also the most valuable target (every key, all the traffic). So the line:

1. **Merge, push, review and attest never go through the hub.** They are the permission floor; the seats keep their own credentials.
2. **A model never decides a verification.** C129 verification is deterministic suite execution on signed packets. Models may
   summarise a result; they never produce or alter one. If the hub or a provider is down, the platform nodes keep verifying.
3. **Untrusted text has no hands.** What a model reads is data; what it returns is data. Tool use by a model is gated by the
   permission engine like any other actor, default read-only.

## Reuse, not a second gateway

The product repository already has `llm_gateway` (provider abstraction, cache, rate limiter, model registry, MCP gateway) and
`local_inference` (explicit-backend llama.cpp-style engine). Devtools calls them through a **pinned capability manifest**, the
arrangement already chosen for the steward's graduation: Aaroneous never depends on devtools. This lane builds only what devtools
needs around them. I have not exercised either crate; their provider coverage is what their sources claim.

## Keys and `.env`

Keys may live in a per-machine secrets file, **read once at the program's entry point** into typed config and passed to constructors.
Nothing else reads the environment (AGENTS.md bans `std::env::var` outside bootstrap entrypoints, and the gateway crates comply).

- The file lives outside any repository, owner-only permissions, ignored by git; Windows uses the credential store.
- Config files hold **references** (`api_key_ref = "provider-openai"`), never values. `config_value_problem` refuses a secret-named
  field whose value is not a reference name, and refuses a value shaped like a credential anywhere.
- Keys never appear in logs, cards, receipts or a model's context. Seat credentials (signing keys, App tokens) never enter the hub.

## Does the model fit the machine

The Ubuntu machine is an RTX 4060 with 8 GB of VRAM and 16 GB of RAM (the seat-status file names the card; the sizes are the
owner's). The sizing guard turns "is this too much" into arithmetic the hub runs on every configured model.

Inputs are declared per model, measured on the host: weights, key/value bytes per token (2 x layers x KV heads x head dim x bytes),
context length, server overhead. Illustration with published architectures (estimates, **not measured here**):

| Model | Weights | KV at 8k context | Needs | On 8 GB less 600 MB reserved |
|---|---|---|---|---|
| Qwen2.5-Coder-14B AWQ (the AGENTS.md default) | ~9.4 GB | 1.5 GB | ~11.9 GB | does not fit; offload would spill ~4.3 GB into RAM, slow |
| a 7B-class Q4 GGUF | ~4.7 GB | 0.45 GB | ~5.6 GB | fits with ~2 GB spare |

So the 14B default in AGENTS.md does not fit this card; that default belongs to the product repository (owner-gated) and I have not
edited it. In devtools the model is a per-host setting and `choose` picks the largest candidate that runs fully on the GPU, never
an offloaded one. The OpenCode seat's operating evidence says a 9B Q4_K_M model is resident on that machine today; `fit` is the
check that decides whether that, or a larger context, is safe. Server choice matters on a small card: a server that reserves a fixed
share of VRAM up front is less forgiving than a llama.cpp-style server with partial offload (an expectation, not measured).

## Receipts: what actually answered

Every call the hub makes records a receipt: node, provider, the model asked for, **the model the server said answered**, digests of
the request and the response, and the tick. A mismatch (a silent fallback, a mislabelled alias) is flagged. This is the first
runtime-backed answer to the limit noted in review of C125: the model named in an attestation is self-declared. Once an attestation
can cite a receipt id, "the model that ran" for hub-made calls is observed, not claimed. Seats whose models run outside the hub (the
Claude, Codex and Gemini sessions reach theirs through their own applications) stay self-declared.

## Endpoints

`endpoint_problem` allows loopback always; any other host must be listed, and plain `http` off loopback must be listed separately
(an encrypted overlay or a trusted segment). It refuses embedded credentials, bad ports, whitespace and look-alike hosts such as
`127.0.0.1.evil.example`. This is the `network.require_encrypted` rule from proposal 03 as code.

## Phases

1. **This PR:** the pure core (sizing, endpoint and secret-reference validation, receipts) with tests, and this document.
2. A config file format and the entry-point loader; `foundation hub check` prints `fit` for every configured model on this host.
3. Supervise the local model server (start, health, restart, one slot per GPU).
4. The pinned capability manifest and calls to `llm_gateway`; prompt adaptation per model (proposal 05).
5. Remote providers, one at a time behind typed config and budgets.
6. Receipts into attestations; the verification nodes (C129) report model-server health in their heartbeat.

## Verified and not

Verified: `cargo test -p steward --lib model_hub` (15 tests, in both the debug and the release profile), `cargo clippy -p steward --all-targets -- -D warnings`,
`cargo fmt --all -- --check`; two review findings are fixed with regressions (a four-label hostname `127.attacker.example.com` was admitted as loopback because the check counted dots instead of parsing an address; and unchecked multiplication let `kv_bytes_per_token = 2^63` wrap to a 1 MB fit in a release build, so loopback now means a parsed 127.0.0.0/8 or `::1` address or the literal `localhost`, and the sizing uses saturating and 128-bit arithmetic); seven deliberate breakages (no VRAM reservation, offload as the default, any host allowed, secret
fields accepting values, `choose` accepting an offloaded model, the tight threshold removed, a substituted model not flagged) each
fail a test. **Not run:** nothing touches a machine, a network or a model; VRAM figures are published-architecture estimates, not
measurements of the Ubuntu host.
