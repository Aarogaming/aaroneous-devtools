# Native-purity P1 remediation: a phased, ordered plan

Date: 2026-09-27 UTC
Owner: Claude, at the owner's direction ("scope it into a concrete phased
plan first" — see `AUDIT_2026-09-26_DOCUMENTATION_AND_PROCESS.md`, item 9)
Builds directly on: Codex's
`inventories/C68_NATIVE_DEPENDENCY_BOUNDARY_AUDIT_2026-09-25.md`. This
document adds no new findings — it takes that report's P1 section ("core
isolation: move allocator, SQLite, compression, tokenizer/Candle,
crypto/network, and GPU choices out of Hypervisor's default dependency set")
and orders it into independently-shippable, gate-verified slices with an
effort/risk estimate each, so the owner can greenlight one at a time instead
of facing it as one undifferentiated multi-week block.

**This document authorizes no code change.** Each slice below needs its own
queue claim, its own PR, its own gate run, and (per the branch protection
enabled 2026-09-27) its own CI-green merge. Nothing here should be read as
"in progress" until a coordination-queue row says so with evidence.

## What's already done (context, not part of this plan)

- P0 dead-boundary removal: RocksDB and `librocksdb-sys` fully removed from
  `Cargo.lock` (Product `242e134a`, verified — zero matches in current lock).
  Unused `capabilities -> libloading` removed.
- P0 enforcement: `cargo xtask check-native` (Gate 3.5) exists, is wired into
  `cargo xtask gate`, reads a 27-entry allowlist (`native-policy.toml`), and
  passes today.
- A first, minimal `crates/local_inference` scaffold already exists (131
  lines): wraps `llama-gguf` with explicit `cpu`/`cuda`/`vulkan`/`metal`
  feature gates and a `default = []` (no silent CPU fallback) — this is
  exactly the shape Codex's report asked for in its "Required replacement
  shape" step 4, just not yet the full core (no typed generation/embedding
  contracts, no `GGUFProvider` migration, no parity trials against Ollama).

## Ordering principle

Ordered by ascending blast radius: crates with the fewest, most mechanical
call sites first; crates where getting it wrong risks silent behavior
change (numerical results, TLS handshake failures, model output drift)
last. Each slice's "Acceptance" criteria must all hold before it's
considered done — none of them are "make it compile," all of them include a
`cargo xtask gate` pass with the new allowlist entry in place.

---

### Slice 1 — Allocator (mimalloc): make the default explicit, not remove it

**Current state**: `core/hypervisor/src/lib.rs` installs `mimalloc::MiMalloc`
as the global allocator unconditionally.

**Effort: S. Risk: Low.** A global allocator is exactly one
`#[global_allocator]` static in exactly one place. This is not about
deciding mimalloc is wrong — it's about making the choice a reviewable,
named policy instead of an unconditional dependency baked silently into
`hypervisor`'s manifest.

**Concrete step**: Add a `mimalloc` Cargo feature to `core/hypervisor`
(default-on, to make this a zero-behavior-change slice), move the
`#[global_allocator]` declaration behind `#[cfg(feature = "mimalloc")]`,
and fall back to the system allocator when the feature is off. Document the
policy (why mimalloc, what the fallback is for — `pure-core`/portable
builds) in `docs/CRATIFY_SPEC.md` next to the dependency-admission rules.

**Acceptance**: `cargo build --no-default-features -p hypervisor` (or the
relevant profile) no longer pulls in `mimalloc`/`libmimalloc-sys`; default
build behavior (mimalloc on) is bit-for-bit unchanged; `native-policy.toml`
gets an explicit, justified `mimalloc` entry instead of it being an
undeclared default; full gate green.

---

### Slice 2 — Crypto provider consolidation (AWS-LC vs Ring)

**Current state**: two native crypto implementation families are both
present — `rustls` defaults select `aws-lc-rs`/`aws-lc-sys` (bundled
C/assembly), while `ast_auditor` and other networking paths separately pull
in `ring` (also C/assembly). Nothing has chosen one on purpose.

**Effort: M. Risk: Medium** — this is a security-relevant decision
(which TLS/crypto backend the whole workspace stands behind), not just a
mechanical refactor, and gets it wrong quietly (a TLS handshake failure
under an unexpected cipher suite, or a certification path that stops
working) rather than loudly.

**Concrete step**: Pick one provider (recommendation: `ring`, since it's
already a direct dependency for certification/signing and has a longer
track record in this ecosystem; `aws-lc-rs` is the alternative if
post-quantum cipher support matters more than that). Disable `rustls`'s
default crypto-provider feature in every root `Cargo.toml` that enables
`reqwest`/`rustls`, and set the chosen provider explicitly. Audit every
TLS-terminating call site (HTTP clients, NATS, any QUIC/Iroh transport) for
the connections that actually matter, not just "it compiles."

**Acceptance**: `cargo tree -i aws-lc-sys` and `cargo tree -i ring` show
exactly one native crypto family reachable from the default graph (or both,
each behind a named, justified adapter, if a real reason to keep both
surfaces); real TLS round-trip tests against at least the HTTP and NATS
paths pass under the chosen provider; full gate green.

---

### Slice 3 — SQLite behind a storage-component trait

**Current state**: `rusqlite` (bundled SQLite C) is a live, direct
dependency of `hypervisor`, used by the profile registry, persistence
manager, and cluster DB.

**Effort: M. Risk: Low-Medium.** Mechanical refactor (define a trait,
route the three call sites through it), no behavior change if the default
implementation stays SQLite-backed. Risk is scope creep — do not use this
slice to also evaluate a pure-Rust storage replacement (that's explicitly
Codex's separate P2 item: "No candidate is accepted merely for being
implemented in Rust").

**Concrete step**: Define a minimal `StorageBackend`-style trait covering
what the profile registry / persistence manager / cluster DB actually call
today (get/put/scan, whatever the real surface is — read the three call
sites before designing the trait, don't guess it up front). Move the
`rusqlite` dependency and its `bundled` feature into a small adapter crate
or module implementing that trait; `hypervisor`'s library depends on the
trait, not `rusqlite` directly.

**Acceptance**: `hypervisor`'s library source has zero direct `rusqlite`
references outside the new adapter; behavior and on-disk format unchanged
(existing persistence tests pass unmodified); `native-policy.toml` entry
moves to name the adapter crate as `owning_adapter` instead of `hypervisor`
directly; full gate green.

---

### Slice 4 — Tokenizer / Candle split (Oniguruma)

**Current state**: `hypervisor` directly selects `tokenizers 0.23` with
`onig`, while `candle-core 0.11` (pulled in by several component crates)
separately selects `tokenizers 0.22` with `onig` — two duplicate native
regex-engine paths, and Candle is present even for crates that only use
tensors, not tokenization.

**Effort: L. Risk: Medium-High.** Touches ML/tensor code paths directly;
getting it wrong risks silent numerical or tokenization drift, not just a
compile error. Needs real before/after output comparison on actual model
fixtures, not just "it builds."

**Concrete step**: First split tensor and tokenizer *capabilities* apart
in the dependent crates so a crate that only needs tensors doesn't
transitively pull in a tokenizer at all (this alone likely removes several
`onig` reachability paths for free). Then evaluate whether `hypervisor`'s
own direct `tokenizers` dependency can be removed in favor of whatever
`llm_gateway`/`local_inference` already need, before evaluating a pure-Rust
tokenizer/regex replacement (a separate P2 decision — don't conflate the
two in this slice).

**Acceptance**: at most one native regex-engine (`onig_sys`) reachability
path in the default graph, not two; tokenization output byte-identical on
a fixed corpus before/after; full gate green.

---

### Slice 5 — GPU / windowing isolation

**Current state**: `wgpu`/window dependencies reach `ash`, EGL/WGL
bindings, `libloading`, and installed GPU drivers directly from
`hypervisor`'s compute paths and `studio_hud`'s presentation layer, mixed
with core reducer logic in places rather than confined to an adapter.

**Effort: L. Risk: Medium.** Mostly a structural/architectural refactor
(core reducers take traits and supplied handles, not loaders/drivers
directly) rather than a behavior change, but GPU code is failure-prone
across driver/OS combinations and hard to fully test in CI (Codex's own
report flags this as "mostly Rust wrappers over required external system
APIs rather than bundled implementations" — lower native-purity urgency
than the crypto/tokenizer items, more of an architecture-boundary cleanup).

**Concrete step**: Audit `crates/compute` and `core/hypervisor` for direct
GPU/window API usage outside `platform_bridge`/`studio_hud`/a dedicated GPU
adapter crate; migrate core reducers to depend on a trait + injected handle
per the Constructor Injection rule already in `AGENTS.md` section 2, rather
than instantiating GPU resources themselves.

**Acceptance**: `core/hypervisor`'s library has no direct `wgpu`/`ash`
dependency outside an explicit adapter; existing rendering/compute paths
behave identically; full gate green including the GPU-feature compile-only
gates (10-11).

---

### Slice 6 — Rust-owned local inference core (largest, already started)

**Current state**: `crates/local_inference` exists as a minimal
explicit-backend-selection scaffold (see "What's already done" above).
Aaroneous and Foundation both still default to calling Ollama over
loopback HTTP for the actual production model-serving path; `GGUFProvider`
executes `llama_gguf::engine::Engine` in-process but its comments
incorrectly describe this as "direct integration with llama.cpp" (it's a
separate Rust implementation) — a small, free documentation fix worth
doing as part of this slice regardless of how much of the rest lands.

**Effort: XL. Risk: Medium-High**, but bounded by Codex's own required
gates (parity trials before any production switch) rather than open-ended.
This is the biggest single item in the whole P1 list and should probably
be its own multi-slice project, not a single PR — listed last and largest
on purpose.

**Concrete steps** (from Codex's C68 report's "Required replacement shape,"
unchanged, reproduced here for sequencing only):
1. Extract the minimal Rust-owned local-inference core (typed
   generation/embedding contracts, GGUF parsing, model/scheduler/KV-cache
   ownership, no HTTP/Candle/HuggingFace/RocksDB/SQLite/dynamic-loading/
   concrete-GPU-API dependency).
2. Migrate Aaroneous's `GGUFProvider` onto it; fix the llama.cpp comment
   regardless of sequencing.
3. Add the parallel Foundation adapter without creating an
   Aaroneous-to-devtools dependency.
4. Define the inference-backend trait; ship the CPU reference adapter
   (exists in embryo today) plus explicit GPU adapters (DX12/Vulkan/CUDA/
   Metal).
5. Run the CPU-vs-GPU parity trials Codex specified (schema conformance,
   tensor shapes, numerical tolerance, stop conditions, context limits,
   deterministic temperature-zero output, malformed-model rejection,
   bounded memory, cancellation, device-loss/crash recovery) against the
   actual Qwen GGUF used in production.
6. Only then switch the local worker's default, with an explicit,
   non-silent failure mode (no silent fallback to CPU or Ollama) and
   Ollama retained as an explicitly noncompliant fallback until retired.

**Acceptance**: matches Codex's own report verbatim — this slice doesn't
get a lighter bar than the original audit set for it.

---

## Recommended starting order

**1 (allocator) -> 3 (SQLite) -> 2 (crypto) -> 4 (tokenizer) -> 5 (GPU) -> 6
(inference core)**, i.e. do the cheapest, lowest-risk structural slices
first to build confidence and allowlist-entry precedent, save the
security-sensitive crypto decision for when there's a clear head to review
it properly, and treat the inference core as the standalone project it
actually is rather than a P1 checkbox. Reordering is entirely reasonable if
priorities differ — this is a sequencing proposal, not a dependency
requirement between slices (1-5 are independent of each other; 6 is
independent of all of them).

## What this plan deliberately does not do

It does not start any of the six slices. It does not pick a crypto provider
on the owner's behalf (Slice 2's recommendation is a starting point for
discussion, not a decision). It does not estimate calendar time, because
none of these slices have been attempted yet and a guess dressed as an
estimate is exactly the kind of unearned confidence this project has had
enough of this week.
