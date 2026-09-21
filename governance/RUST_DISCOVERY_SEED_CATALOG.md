# Rust Discovery Seed Catalog

Purpose: a growing source list of real, notable Rust projects to seed
`frontier-discover-serve` (M41) and manual discovery passes, so the local
worker has somewhere concrete to look first instead of starting cold. This
is a **seed list, not an approval list** — every entry still has to clear
[`FRONTIER_FILTRATION_STANDARD_V1.md`](../contracts/FRONTIER_FILTRATION_STANDARD_V1.md)'s
G1-G5 gates (license via M55, size/decomposability via M54, ecosystem
independence) and Q1-Q4 scoring before anything is adopted, wrapped, or
learned from directly in code. Listing a project here says "worth looking
at," never "compliant" or "cleared" — see FRONTIER_FILTRATION_STANDARD_V1.md
§5.1 on wrapper-vs-compliance.

Each row is a study target: what about its design is relevant to a specific
Aaroneous or devtools concern, not a recommendation to depend on it. License
must be re-verified live at discovery time (crates.io's search endpoint does
not return license data — see M55's second-call pattern) since this table
can go stale.

This list is meant to expand. Append rows; do not renumber categories.

## How to use this as a seed set

`frontier_discovery.rs` currently takes a manually configured query set.
Until that's wired to read this file directly, the "Project" column values
below are usable as-is as query seeds. Each category maps to a real internal
concern so a triage pass can prioritize by what we're actively building
(e.g. prioritize "Agent / local-inference orchestration" while M51's closed
loop is being assembled).

---

## Component / reactive architecture

Relevant to: `hypervisor/supervisory_loop.rs`, `capability_broker.rs`,
`consensus_engine.rs`'s `Record` state transitions — anywhere we have a
pure state-transition boundary that could be made more explicit.

| Project | Category | What to study it for |
| --- | --- | --- |
| `iced` | GUI, Elm architecture | Strict `Message → Update → View` purity; how it keeps state transitions decoupled and testable without lifetime hell |
| `leptos` | Reactive web framework | Fine-grained reactive signals/effects without a virtual DOM — relevant only if the M36 dashboard work grows a live-updating surface |
| `dioxus` | Cross-platform reactive UI | Component tree diffing model; VirtualDom architecture as an alternative to Leptos's fine-grained signals |
| `egui` | Immediate-mode GUI | Immediate-mode vs retained-mode tradeoffs; useful if any tooling UI favors simplicity over reactivity |
| `slint` | Declarative native UI | Separate declarative markup compiled to Rust — a different component-boundary philosophy worth contrasting against Iced's |
| `xilem` | Experimental reactive UI (Linebender) | Xilem's view-tree + reactive core is a newer, more Rust-idiomatic take on the Elm pattern than Iced's |
| `freya` | Iced-adjacent UI on Dioxus core | Shows how a second UI layer can be built on top of another's reactive core — relevant to any internal "framework on a framework" question |

## ECS / data-oriented systems

Relevant to: `capability_broker.rs` + `capability_composition.rs`'s
registry model, and `scheduler.rs`'s job-batching — both are
registry/composition-based already but lack a real query/schedule graph.

| Project | Category | What to study it for |
| --- | --- | --- |
| `bevy` (`bevy_ecs`) | ECS / game engine | "Everything is data, systems are decoupled logic over queries" — the strongest direct lens for `scheduler.rs`'s non-conflicting-lane derivation (M43) |
| `hecs` | Minimal standalone ECS | A much smaller, dependency-light ECS core — useful if Bevy's full ECS is overkill and only the query-over-data pattern is wanted |
| `legion` | Archetype-based ECS | Alternative archetype storage strategy to Bevy's, useful for comparing storage-layout tradeoffs |
| `specs` | Older archetype ECS (predecessor lineage) | Historical design decisions and why the ecosystem moved away from some of them — useful negative-space learning |
| `fyrox` | Full Rust game engine (uses its own ECS-adjacent scene graph) | A complete engine's component/scene lifecycle management at a larger scale than a bare ECS crate |
| `macroquad` | Minimal game framework | Contrast: deliberately *not* ECS — useful as a counterexample for when a registry/composition model is unnecessary overhead |

## Agent / local-inference orchestration

Relevant to: `LOCAL_AGENT_CONTROL_PLANE.md`, `local_worker.rs`,
`consensus_engine.rs`'s cross-verify pattern — the closest direct analogs
to what devtools already is.

| Project | Category | What to study it for |
| --- | --- | --- |
| `adk-rust` (zavora-ai) | Agent dev kit, local model runners | Closest direct comparison to `LOCAL_AGENT_CONTROL_PLANE.md` — its `Runner`/session-context pattern and tool-call boundary design is real independent evidence for or against our own |
| `rig` (`rig-core`) | LLM app framework | Ergonomics for defining vector stores/document loaders/contextual agents as plain structs — relevant to M57/M59's evidence-bounded metadata records |
| `mistral.rs` | In-process local model runner | Alternative to our Ollama-subprocess approach — an in-process runner would remove the HTTP hop `local_worker.rs` currently takes |
| `candle` (huggingface) | Rust-native ML tensor framework | What a from-scratch Rust inference stack looks like, relevant if Ollama's process boundary ever becomes a bottleneck |
| `burn` | Rust-native deep learning framework | Alternative to Candle; compare training/inference API ergonomics |
| `llm-chain` | LLM prompt/chain orchestration | Chain-of-prompts patterns — relevant if `worker-jobs` packet sequencing ever needs multi-step prompt chaining instead of one-shot packets |
| `swiftide` | Agentic pipeline / RAG framework | Pipeline-stage composition for ingest→index→retrieve — comparable in shape to our discover→filter→decompose→draft→audit pipeline |
| `kalosm` | Local-first Rust AI toolkit | Local-first design philosophy (no cloud dependency by default) — directly aligned with our cloud-minimization goal |

## Async runtime / concurrency

Relevant to: `local-worker-serve`'s adaptive backoff loop, `scheduler.rs`,
anywhere devtools needs bounded concurrent job dispatch without starvation.

| Project | Category | What to study it for |
| --- | --- | --- |
| `tokio` | Async runtime | Already a likely dependency; study its cooperative scheduling and backpressure primitives for the worker loop's adaptive backoff |
| `async-std` | Alternative async runtime | Contrast API philosophy against Tokio; mostly historical/comparative value now |
| `smol` | Minimal async runtime | Small, composable executor — useful if a lighter-weight runtime is ever wanted for a bounded subprocess |
| `rayon` | Data-parallelism | Work-stealing parallel iterators — relevant to any CPU-bound batch audit pass (e.g. M54 clustering a large source tree) |
| `crossbeam` | Lock-free concurrency primitives | Channel/queue primitives underlying a lot of the ecosystem; relevant if `local_worker.rs`'s job queue ever needs to go lock-free |
| `flume` | MPMC channel | Simpler alternative to `crossbeam-channel` for the worker-jobs producer/consumer boundary |
| `parking_lot` | Faster sync primitives | Drop-in mutex/rwlock replacements — worth a look if any hot lock shows up in profiling |

## CLI / developer tooling

Relevant to: `foundation_cli`'s own CLI surface and `xtask` conventions
already in use across both repos.

| Project | Category | What to study it for |
| --- | --- | --- |
| `clap` | CLI argument parsing | Likely already a dependency; study derive-macro ergonomics for any new `foundation` subcommand |
| `cargo-xtask` (pattern, not a crate) | Build automation convention | The convention our own `cargo xtask gate` already follows — worth confirming we're not diverging from idiomatic xtask structure |
| `cargo-nextest` | Test runner | Faster, better-isolated test execution — relevant given devtools already runs single-threaded test suites for isolation reasons |
| `mdbook` | Documentation generator | If governance docs ever need a browsable rendered form instead of raw markdown |
| `ripgrep` | Fast recursive search | Reference implementation for fast, correct file-tree traversal — relevant to M54's target decomposition scanning large trees |
| `bat` | `cat` replacement with syntax highlighting | Minor, but a clean example of small-scope CLI tool design done well |
| `starship` | Cross-shell prompt | Plugin/module architecture for composable, independently-toggleable features — comparable in shape to capability composition |
| `zellij` | Terminal multiplexer | Session/pane state persistence model — relevant if any long-running devtools process needs a resumable session concept |
| `watchexec` | File-watch + rerun | Relevant if `local-worker-serve` should react to worker-jobs directory changes instead of polling on a fixed interval |
| `just` | Command runner | Alternative/simpler to `cargo xtask` for trivial repeated commands |

## Networking / IPC / RPC

Relevant to: `hypervisor`'s capability broker IPC boundary and any
cross-process contract between Aaroneous and devtools.

| Project | Category | What to study it for |
| --- | --- | --- |
| `tonic` | gRPC | Schema-first RPC with codegen — relevant if the file-contract between repos (worker-jobs TOML) ever needs a typed RPC alternative |
| `quinn` | QUIC | Modern transport with built-in multiplexing/encryption — relevant if IPC ever needs to cross a real network boundary, not just local files |
| `iroh` | P2P networking (QUIC-based) | Already referenced elsewhere in the roadmap (M20's blocked iroh upgrade) — direct relevance, not just inspirational |
| `tarpc` | RPC framework | Lightweight typed RPC over async channels — simpler alternative to gRPC for pure-Rust-to-Rust IPC |
| `capnp-rpc` | Cap'n Proto RPC | Object-capability RPC model — directly relevant conceptually to `capability_broker.rs`'s naming and intent |
| `interprocess` | Cross-platform IPC primitives | Named pipes/Unix sockets abstraction — relevant to any future non-file-based local IPC between Aaroneous and devtools |
| `zbus` | D-Bus implementation | Relevant only if Linux desktop integration is ever in scope; otherwise low priority |

## Serialization / schema

Relevant to: `worker-jobs/*.toml` packet schema, `candidate_metadata.rs`'s
JSON store, and any cross-repo file contract.

| Project | Category | What to study it for |
| --- | --- | --- |
| `serde` | Serialization framework | Already load-bearing everywhere; study its `#[serde(default)]`/versioning patterns for schema evolution (already used for M46's added fields) |
| `rkyv` | Zero-copy deserialization | Relevant if `candidate_metadata.rs`'s JSON store ever needs to scale past comfortable JSON-parse cost |
| `prost` | Protobuf codegen | Pairs with `tonic`; relevant only if a typed-schema RPC path gets built |
| `flatbuffers` (Rust bindings) | Zero-copy schema serialization | Alternative to `rkyv` with cross-language support, relevant if a non-Rust consumer ever needs the metadata store |
| `postcard` | Compact binary serde format | Small-footprint binary format, relevant for any embedded-adjacent or size-constrained artifact |
| `bincode` | Binary serde format | Simpler binary alternative; useful baseline comparison against `postcard`/`rkyv` |

## State / persistence / embedded databases

Relevant to: `results/`/`archive/` rotation, `candidate_metadata.rs`,
`COORDINATION_QUEUE.md` — all currently flat files, which is a deliberate
choice worth periodically re-justifying against real alternatives.

| Project | Category | What to study it for |
| --- | --- | --- |
| `sled` | Embedded KV store | Would replace flat-file `results/`/`archive/` rotation if file-count ever becomes a real problem — currently unproven need |
| `redb` | Embedded KV store (newer, ACID) | Alternative to `sled` with a more conservative design; worth a comparative read before ever migrating off flat files |
| `heed` (LMDB bindings) | Embedded KV store | Battle-tested LMDB wrapper; relevant if durability guarantees stronger than flat-file writes are ever needed |
| `fjall` | LSM-tree embedded storage | Newer entrant, worth watching rather than adopting yet |
| `surrealdb` | Multi-model embeddable database | Relevant only if candidate metadata ever needs real query/graph capability beyond M59's Jaccard similarity |
| `sqlx` | Async SQL toolkit | Relevant only if a real relational store ever replaces the markdown-table-as-database pattern in `COORDINATION_QUEUE.md`/`MULTITHREAD_PROGRAM.md` |
| `diesel` | Compile-time-checked ORM | Alternative to `sqlx`; comparative reference only |

## Compiler-adjacent / incremental computation

Relevant to: `ast_auditor`, `cratify`, and anywhere devtools parses or
transforms real Rust source.

| Project | Category | What to study it for |
| --- | --- | --- |
| `cranelift` | Code generator backend | Already a live discovery candidate this session (found via `frontier-discover`); relevant to any future JIT/codegen need |
| `salsa` | Incremental computation framework (used by rust-analyzer) | Directly relevant to M57's "accumulating metadata record" — salsa's memoized/invalidation-on-change model is a more principled version of the 24h-freshness caching we hand-rolled |
| `rust-analyzer` | Full Rust IDE backend | The most complete real-world example of incremental Rust source analysis at scale — directly relevant to `ast_auditor`'s design |
| `tree-sitter` (+ `tree-sitter-rust`) | Incremental parsing | Relevant if `ast_auditor` ever needs incremental re-parse instead of full re-parse per audit pass |
| `syn` / `quote` / `proc-macro2` | Rust syntax tree manipulation | Already likely underlying `ast_auditor`/`cratify`; study for any gaps in current AST handling |
| `rustc` itself (compiler source) | Reference implementation | The canonical example of "explain why, don't just reject" diagnostics — direct inspiration for the rejection-quality section of `DISCOVERY_REVIEW_LOOP.md` |
| `cargo` itself | Dependency resolution, workspace handling | Reference for any future workspace-aware decomposition beyond M54's directory clustering |

## Testing / formal verification

Relevant to: `dogfood::verify_draft` (M44), pre-proofing, and the
"tested through reality, not just in theory" goal driving this whole arc.

| Project | Category | What to study it for |
| --- | --- | --- |
| `proptest` | Property-based testing | Directly relevant to hardening any pre-proofing harness against inputs beyond hand-picked fixtures |
| `kani` | Model checker for Rust | Formal verification of unsafe/invariant-heavy code — relevant to the one known unsafe reader in `hypervisor.rs`'s concurrence.shm |
| `loom` | Concurrency permutation testing | Directly relevant to any lock-free/concurrent code touched via `crossbeam`/`parking_lot` adoption |
| `miri` | Undefined-behavior interpreter | Relevant to auditing any `unsafe` block introduced during cratification of external code |
| `insta` | Snapshot testing | Relevant to `candidate_metadata.rs`'s JSON output — snapshot tests would catch unintended schema drift better than hand-written assertions |
| `arbitrary` | Structured fuzzing input generation | Pairs with `cargo-fuzz`; relevant to fuzzing `ast_auditor`'s parser boundary |
| `cargo-fuzz` | Fuzzing harness | Real chaos/adversarial-input testing — directly addresses the "tested through reality, not just in theory" goal for any parser-facing code |

## Security / supply chain

Relevant to: G1 (license) and G4 (ecosystem independence) gates in
`FRONTIER_FILTRATION_STANDARD_V1.md`, and the dependency remediation plan.

| Project | Category | What to study it for |
| --- | --- | --- |
| `cargo-audit` | RustSec advisory scanning | Could feed G1/G4 gate automation directly — checks dependency tree against known vulnerabilities |
| `cargo-deny` | License/ban/advisory policy enforcement | Directly relevant to automating G1 (license) beyond the current crates.io API call — cargo-deny already encodes SPDX-aware license policy |
| `cargo-vet` | Supply-chain audit trust propagation | Relevant if devtools ever needs to track "who audited this dependency" across the growing candidate set |
| `cargo-geiger` | Unsafe-code detection | Relevant signal for G4/Q-scoring — unsafe density is a reasonable proxy for integration risk |
| `rustsec/advisory-db` | Vulnerability database (data, not a tool) | The actual data source `cargo-audit` reads; worth knowing directly in case a custom lighter-weight check is ever built |

## Observability

Relevant to: understanding the local worker's real behavior over a 12hr+
run, beyond the current plain-text log lines.

| Project | Category | What to study it for |
| --- | --- | --- |
| `tracing` | Structured, async-aware logging | Would replace `local-worker-serve`'s plain println-style log lines with structured, filterable spans — directly useful for diagnosing the exit-code-127 crash with more context than a flat log |
| `tracing-subscriber` | `tracing` output formatting/filtering | Pairs with `tracing`; relevant to making worker logs queryable instead of just tailed |
| `metrics` | Metrics facade | Relevant to actually measuring "how long can we keep the local agent busy" quantitatively instead of eyeballing log ticks |
| `opentelemetry-rust` | Distributed tracing/metrics export | Overkill for a single local process today; relevant only if devtools ever runs distributed across machines |
| `tokio-console` | Async runtime debugger | Relevant only once/if the worker loop moves onto a real async runtime instead of the current blocking-subprocess model |

## WASM / sandboxing

Relevant to: any future need to run untrusted or externally-sourced code
(e.g. a drafted port from a discovered crate) in a bounded sandbox before
it touches a real build.

| Project | Category | What to study it for |
| --- | --- | --- |
| `wasmtime` | WASM runtime | The most mature Rust WASM sandbox — directly relevant if pre-proofing ever needs to execute untrusted drafted code in isolation rather than just `cargo check` it |
| `wasmer` | Alternative WASM runtime | Comparative reference to Wasmtime; different plugin/embedding ergonomics |
| `extism` | Plugin system built on WASM | Directly relevant if devtools ever wants a real sandboxed plugin boundary instead of subprocess-based isolation |
| `wasm-bindgen` | Rust/WASM/JS interop | Low relevance unless a browser-facing dashboard is ever built |
| `wit-bindgen` | WASM component model bindings | Relevant to the already-referenced WIT packaging mentioned in M6's roadmap history |

## Distributed systems / consensus

Relevant to: `consensus_engine.rs`'s cross-verify (author≠auditor)
invariant — currently a simpler two-party check, worth knowing what "real"
distributed consensus looks like even if we never need its full weight.

| Project | Category | What to study it for |
| --- | --- | --- |
| `openraft` | Raft consensus implementation | Reference for what real multi-party consensus looks like, in case cross-verify ever needs more than two parties |
| `tikv` | Distributed transactional KV store | A full production Rust distributed system — relevant as a scale reference, not a near-term dependency |
| `hydroflow` | Dataflow/distributed programming | Relevant if the discover→filter→decompose→draft→audit pipeline is ever reframed explicitly as a dataflow graph |

## Parsing / language tooling

Relevant to: `ast_auditor`, `job_converter.rs`'s TOML parsing, and any
future DSL for worker-jobs packets.

| Project | Category | What to study it for |
| --- | --- | --- |
| `nom` | Parser combinators | Reference for hand-rolled parsers if TOML ever stops being sufficient for worker-jobs packets |
| `pest` | PEG parser generator | Grammar-file-driven alternative to `nom`; useful if a worker-jobs DSL ever needs a formal grammar |
| `logos` | Lexer generator | Fast tokenization, pairs with `pest`/hand-rolled parsers |
| `chumsky` | Parser combinators with error recovery | Notably strong error-message ergonomics — directly relevant to the rejection-quality/rustc-style-diagnostics goal |

## Misc systems utilities

| Project | Category | What to study it for |
| --- | --- | --- |
| `polars` | DataFrame library | Relevant if candidate-metadata analysis (M57/M59) ever outgrows ad hoc JSON scanning and needs real tabular querying |
| `sccache` | Compilation caching | Relevant to speeding up the pre-proofing harness's repeated `cargo check` calls across many drafted candidates |
| `typst` | Rust-native typesetting | Low relevance; included as an example of a large, well-structured pure-Rust application worth a structural skim |
| `mise` (formerly rtx) | Dev-environment/tool version management | Relevant only if cross-machine devtools setup reproducibility ever becomes a real pain point |

---

## Status

~100 real, currently-maintained Rust projects across 15 categories, each
tied to a specific existing file or milestone rather than generic praise.
This is a living document — add rows as new candidates surface through
`frontier-discover-serve` or manual research; do not delete entries that
turn out to be low-priority, just leave them for later re-scoring instead
of re-researching from scratch.

Next step, not yet done: wire `frontier_discovery.rs`'s query set to read
this file's "Project" column directly instead of the current manually
configured list, so `frontier-discover-serve` consumes it automatically.
