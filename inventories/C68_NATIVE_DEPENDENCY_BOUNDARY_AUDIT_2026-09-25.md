# C68 Aaroneous native dependency boundary audit

Date: 2026-09-25 UTC  
Owner: Codex  
Product revision: `906c01ee7cacaf9252b2e75d290476c4e45bf523` (`main`)

## Result

Aaroneous does not currently have a pure-Rust default dependency base. The
locked Windows workspace graph resolves 647 distinct package/version pairs and
includes bundled C, C++, assembly, build-time libclang, runtime dynamic-library
loading, and operating-system/GPU FFI. Several boundaries are legitimate only
as isolated platform adapters, but they are not consistently isolated or
feature-gated today.

The most urgent defect is not merely that native code exists. It is that native
implementation provenance is invisible to the architecture contract:

- `hypervisor` directly and unconditionally selects RocksDB, bundled SQLite,
  mimalloc, Oniguruma through tokenizers, zstd, networking crypto, and GPU APIs;
- RocksDB is both unused through its unconditional public wrapper and described
  elsewhere as feature-gated by an undeclared `rocksdb-dna` feature;
- default Rustls features select AWS-LC, so ordinary `reqwest` use compiles a
  bundled C/assembly crypto implementation;
- `ring` is also present, producing two native crypto implementation families;
- Candle 0.11 unconditionally selects tokenizers 0.22 with Oniguruma on
  non-WASM targets, while `hypervisor` also selects tokenizers 0.23 with
  Oniguruma directly;
- runtime library loading remains in the SDK and RFC-0006 development host,
  while `capabilities` declares an apparently unused `libloading` dependency.

Native code is not automatically a defect for an OS, GPU, cryptographic, or
hardware adapter. An undeclared or default-wide native boundary is a defect.
The compliant shape is a pure-Rust default core plus explicit, narrow,
target/feature-gated adapter crates whose unsafe and native provenance is
reviewed independently.

The local-model workflow has the same provenance problem at a process boundary.
Aaroneous and Foundation currently call Ollama over loopback. That keeps
Ollama's implementation outside the Rust address space, but it does not make
the runtime part of a Rust-assured base: model parsing, inference, GPU drivers,
process lifecycle and the served API remain an external trust dependency.
Aaroneous already contains a better candidate path: `llm_gateway` optionally
uses `llama-gguf 0.14` with `default-features = false` and only `cpu`. The cached
package contains no C/C++/assembly/CUDA source files, and its build script does
no external compilation for the CPU-only feature. That candidate still needs
correctness, bounds and performance verification before it replaces Ollama.

## Evidence and method

The inspected checkout's `Cargo.toml` files and `Cargo.lock` have no diff from
product `main`, so dependency resolution is pinned to the named revision even
though the shared checkout is on another branch. No product source was edited
and no compiler/build/test command was run.

Read-only commands used:

- `cargo metadata --format-version 1 --no-deps --offline`
- `cargo metadata --format-version 1 --locked`
- `cargo tree --workspace --locked --target x86_64-pc-windows-msvc --edges normal,build`
- reverse `cargo tree -i` queries for each native family
- searches of workspace manifests/call sites and cached package manifests/build scripts
- Git diff/history checks against product `main`

The first full metadata read required downloading the two already-locked,
previously absent source packages `rsqlite-vfs 0.1.1` and
`sqlite-wasm-rs 0.5.5`. This changed only the user's Cargo source cache. It did
not change `Cargo.lock`, a repository file, a target directory, or product
state. Subsequent reads ran offline.

Resolved facts:

- 33 workspace packages.
- 900 packages in Cargo's target-inclusive metadata universe.
- 647 distinct package/version pairs in the default Windows normal/build graph.
- 19 resolved packages declare Cargo `links`. `links` alone is not proof of
  native implementation: `defmt`, `prettyplease`, `rayon-core`, and
  `wasm-bindgen-shared` use it without bundled native sources in this graph.
- The default Windows graph includes the native families listed below.

## Default Windows native implementation families

| Boundary | Reachability | Evidence | Disposition |
| --- | --- | --- | --- |
| RocksDB and compression stack | `hypervisor -> rocksdb -> librocksdb-sys -> {bzip2-sys, libz-sys, lz4-sys, zstd-sys}` plus `bindgen -> clang-sys` | `rocksdb = "0.21"` is unconditional. `EventLogStore` is declared/re-exported but has no consumer. The separate registry backend uses an undeclared `rocksdb-dna` cfg. `librocksdb-sys` contains 1,252 native source/header files in the cached package. | **Accidental/dead, P0 removal.** Remove the unused wrapper/export and unconditional dependency. Do not replace it until a storage contract proves a durable backend is required. |
| mimalloc | `hypervisor -> mimalloc -> libmimalloc-sys` | `hypervisor/src/lib.rs` installs `mimalloc::MiMalloc` as the global allocator. The sys package contains 78 native files. | **Live but noncompliant default-wide boundary.** Remove or move behind an explicitly approved allocator adapter/feature with measurement and failure-policy evidence. |
| bundled SQLite | `hypervisor -> rusqlite -> libsqlite3-sys` | `rusqlite` enables `bundled`; live consumers include profile registry, persistence manager and cluster DB. The sys package compiles bundled SQLite C. | **Live storage boundary.** Isolate behind a storage component trait and non-core feature. A replacement is a separate compatibility/migration decision, not a blind crate swap. |
| Oniguruma regex | `hypervisor -> tokenizers 0.23 -> onig -> onig_sys`, and many component crates `-> candle-core 0.11 -> tokenizers 0.22 -> onig -> onig_sys` | Both paths explicitly enable `onig`; `onig_sys` contains 95 native files. Candle's non-WASM manifest selects tokenizers with `onig` even where Aaroneous code only uses tensors. | **Live transitive contamination.** Split tensor and tokenizer capabilities, remove the duplicate direct tokenizer where possible, and evaluate a pure-Rust tokenizer/regex path before accepting Candle in core profiles. |
| ESAXX | Both tokenizers versions resolve `esaxx-rs` | `esaxx-rs` is present, but its `cpp` feature is not enabled in the resolved graph; its build script is a no-op without that feature. | **Present but not currently native-active.** Add a policy regression check so `esaxx_fast`/`cpp` cannot become enabled silently. |
| zstd | `hypervisor -> zstd -> zstd-safe -> zstd-sys`; also duplicated below RocksDB | Live sovereign-package encode/decode call sites. `zstd-sys` contains 101 native files. | **Live packaging boundary.** Isolate or replace only with archive compatibility, corruption, size and throughput evidence. RocksDB removal eliminates the duplicate path but not direct zstd. |
| AWS-LC | Workspace HTTP/NATS paths `-> rustls 0.23 default -> aws-lc-rs -> aws-lc-sys` | Rustls default features explicitly select `aws_lc_rs`, `prebuilt-nasm`, and post-quantum preference. `aws-lc-sys` carries a large C/assembly source tree. | **Live security boundary selected implicitly.** Choose and document one approved crypto provider; disable provider defaults in every root. Treat replacement as cryptographic-policy work, not ordinary dependency cleanup. |
| Ring | Direct `ast_auditor -> ring`; also selected by networking packages | Live ECDSA certification/signature call sites. The package contains C/assembly sources and a native build. | **Live security boundary; duplicates AWS-LC family.** Reconcile certification and transport crypto under one explicit provider policy or separately justified adapters. |
| GPU/window/system APIs | Direct `wgpu`/window dependencies reach `ash`, EGL/WGL bindings, `libloading`, Windows APIs and installed GPU drivers | Live Hypervisor compute and Studio HUD rendering call sites. These are mostly Rust wrappers over required external system APIs rather than bundled implementations. | **Required platform boundary, misplaced in part.** Confine to `platform_bridge`/presentation or a dedicated GPU adapter; core reducers depend on traits and supplied handles, not loaders/drivers. |
| Dynamic plugin loading | `sdk -> libloading 0.9`; RFC-0006 development host; transitive loaders in graphics/clang | SDK code calls unsafe `Library::new` and exposes Rust trait objects through an `extern "C"` ABI. The development POC deliberately exercises an FFI ABI. `capabilities` declares `libloading` without a production call site. | **High-risk explicit boundary.** Remove the unused `capabilities` dependency. Keep the POC quarantined. Re-review or retire the SDK loader: a Rust trait object is not a stable C ABI, and loading code requires authentication, signature, lifecycle and panic-containment evidence. |

## Target-only and optional boundaries

These are not active in the default Windows graph but still require declared
policy because `--all-features` or other targets can activate them:

- `governance` optionally enables `z3` / `z3-sys`.
- Hypervisor's `gpu-metrics` feature enables NVML bindings.
- Linux UI targets reach Wayland/X11 system libraries; macOS reaches Objective-C
  and framework bindings; Android reaches native activity glue.
- WASM SQLite support resolves `sqlite-wasm-rs` for its applicable target.
- RFC-0006 deliberately tests a native plugin ABI and must remain development
  evidence, not an implicit production loader.

These adapters are compatible with a Rust component framework only when they
are isolated by target/feature, excluded from pure-core profiles, and backed by
an allowlist naming owner, purpose, implementation language, unsafe surface,
update policy and verification gate.

## Local inference runtime boundary

### Current state

- Aaroneous `llm_gateway::providers::local` sends Ollama-specific
  `/api/generate` and `/api/embeddings` requests to an injected endpoint.
- Foundation's `local-proposal`, intake generator, worker and registrar also
  speak Ollama-specific loopback HTTP and name Ollama in their error contracts.
- Operational policy pins Qwen through Ollama. The model weights may be local,
  but the inference implementation is not governed by the Rust workspace.
- Aaroneous's `GGUFProvider` already executes `llama_gguf::engine::Engine`
  in-process when `llama-gguf` is enabled. Its comments incorrectly describe
  this as “direct integration with llama.cpp”; the selected crate is a separate
  Rust implementation.
- The current `llm_gateway` crate is not yet a clean proof of a pure-Rust
  inference component. Its ordinary dependency graph also reaches AWS-LC via
  `reqwest` and Oniguruma via `transpiler -> compute -> candle-core`, even when
  the CPU-only `llama-gguf` engine itself does not.

### Required replacement shape

1. Extract a minimal local-inference component containing typed generation and
   embedding contracts, GGUF parsing/inference through CPU-only
   `llama-gguf`, explicit model path/config inputs, bounded context/output and
   deterministic test fixtures. It must not depend on HTTP, Candle,
   HuggingFace tokenizers, RocksDB, SQLite, dynamic loading or GPU APIs.
2. Make Aaroneous `GGUFProvider` use that component and correct all llama.cpp
   claims. Keep HTTP/Ollama as an optional compatibility adapter during measured
   migration, never as the compliant default.
3. Add an independent Foundation adapter using the same upstream Rust engine
   and the existing proposal schemas. Do not make Aaroneous depend on devtools
   or devtools source. The thin adapters remain repository-local; inference
   behavior is checked against shared versioned fixtures and exact model hashes.
4. Run parity trials on the actual Qwen GGUF used by C67: schema conformance,
   stop conditions, context limits, deterministic temperature-zero output,
   malformed-model rejection, bounded memory, cancellation and crash recovery.
5. Switch the local worker only after the Rust backend passes those gates.
   Retain Ollama as an explicitly noncompliant fallback until retired; do not
   silently route to it when the Rust backend fails.
6. Keep acceleration separate. CUDA, DX12, Vulkan, Metal or vendor runtimes are
   native adapter profiles and cannot be used to certify the CPU pure-Rust
   baseline.

This avoids replacing one opaque dependency with another. “Written in Rust” is
necessary for this baseline but not sufficient: GGUF bounds checking, tensor
shape validation, numerical parity, resource limits and adversarial model-file
handling still require direct evidence.

## Architectural findings

1. **Default profile is not a compliance profile.** `default = []` on
   Hypervisor does not imply a pure or portable build because non-optional
   dependencies still activate all native families above.
2. **Feature lint suppression hid drift.** Hypervisor permits unexpected cfgs,
   while source refers to undeclared `rocksdb-dna`. That converts a manifest
   contract error into dormant code plus an unconditional dependency.
3. **Root crates own too many implementations.** Hypervisor directly chooses
   allocator, databases, compression, tokenization, networking, crypto and GPU
   implementations. Constructor injection cannot repair a dependency graph
   that hard-wires those choices at compile time.
4. **Rust wrapper does not establish Rust safety.** A crate published on
   crates.io may compile or load C/C++/assembly or expose system FFI. Policy
   must inspect resolved provenance and enabled features, not package names.
5. **Pure core does not mean no platform functionality.** It means foreign code
   terminates in narrow adapters. Typed messages, handles and results cross the
   boundary; native implementation types do not.

## Required enforcement

Add a checked-in native-boundary policy and make it a gate before dependency
compilation:

1. Generate the resolved graph for each supported target and compliance
   profile from locked metadata.
2. Fail on any new package with Cargo `links`, active C/C++/assembly build,
   bindgen/libclang, `libloading`, or system-library link unless an allowlist
   entry matches exact package/version, target, features, owning adapter and
   justification.
3. Fail when core execution packages directly depend on an allowlisted native
   package. Only named adapter crates may do so.
4. Fail on undeclared cfg feature names; remove the broad
   `unexpected_cfgs = "allow"` suppression after known drift is corrected.
5. Maintain at least two graphs: `pure-core` (no native implementation or
   runtime loader) and `platform` (approved target adapters). Optional hardware,
   SMT, GPU, UI and plugin profiles get separate graphs.
6. Record bundled-versus-system mode and native source hashes in the dependency
   report/SBOM. A `-sys` crate version alone is insufficient provenance.
7. Require migration/compatibility evidence before replacing durable storage,
   archive codecs, tokenizers or cryptographic providers.

## Bounded remediation order

1. **P0 dead-boundary removal:** remove unconditional RocksDB and its unused
   `EventLogStore`; correct or remove the dormant registry feature/documentation.
   Remove unused `capabilities -> libloading` after a focused call-site check.
2. **P0 enforcement:** add the resolved native-boundary allowlist/gate before
   further dependency upgrades. Seed the baseline from this report; do not
   silently grandfather packages without a disposition.
3. **P1 core isolation:** move allocator, SQLite, compression, tokenizer/Candle,
   crypto/network and GPU choices out of Hypervisor's default dependency set
   into explicit adapter crates/features. Establish a native-free `pure-core`
   build that exercises reducers and contracts.
4. **P1 security decisions:** review the SDK loader ABI and select explicit
   cryptographic provider policy. Build and verify the CPU-only Rust GGUF
   inference component, then migrate Aaroneous and Foundation off their default
   Ollama dependency. These require independent security review.
5. **P2 migrations:** evaluate pure-Rust storage, regex/tokenizer and compression
   implementations against actual schemas, artifacts and performance. No
   candidate is accepted merely for being implemented in Rust.
6. **P2 cross-target closure:** audit Linux/macOS/Android/WASM and optional
   Z3/NVML graphs with the same gate before declaring the workspace compliant.

## Acceptance boundary

This report establishes dependency reachability and a policy/remediation plan.
It does not certify any native package safe, select a replacement, alter product
policy, or authorize a source change. Each remediation needs an exact queue
claim, compatibility evidence, focused tests, the native Aaroneous gate on the
actual candidate revision, and independent review.
