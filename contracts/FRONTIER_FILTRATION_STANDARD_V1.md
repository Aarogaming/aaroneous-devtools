# Frontier Repository Filtration & Refactoring Feasibility Standard (v1)

> **TIER 2 PROTOCOL SPECIFICATION**  
> **SCOPE**: Autonomous candidate repository filtration, cratification feasibility scoring, and forensic kernel extraction.  
> **APPLIES TO**: All external crate candidates, open-source repositories, and RFC-0005 ingestion pipelines.

---

## 1. Executive Rationale & The Cratification Principle

**Real-World Reality**: Almost zero third-party open-source crates comply with Aaroneous's ultra-strict architectural invariants on day one:
- They allocate `Vec`, `String`, and `Box` freely.
- They invoke ambient authority (`std::env::var`, `SystemTime::now()`, global singletons).
- They use `.unwrap()` / `.expect()` for error handling.
- Their memory geometry lacks explicit padding or `#[repr(C)]` / `Pod` / `Zeroable` derives.

**The Golden Rule of Ingestion**: A candidate crate must **never** be rejected simply because it is not yet compliant. Instead, the autonomous evaluation engine assesses **Refactoring Feasibility** — how easily and mechanically the local model and devtools can isolate the core algorithmic kernel ("the needle") and "Cratify" it into a compliant, zero-allocation Aaroneous component per [docs/FORENSICS_RFC0005.md](docs/FORENSICS_RFC0005.md) and [docs/CRATIFY_SPEC.md](docs/CRATIFY_SPEC.md).

---

## 2. Stage 1: Filtration Standard (Hard Infeasibility Gates)

Filtration gates do **not** enforce current compliance; they enforce **tractability of extraction and IP safety**. If a candidate violates any gate, refactoring is disqualified upfront.

| Gate ID | Invariant Gate | Pass Requirement | Rejection Condition |
| --- | --- | --- | --- |
| **G1** | `LicenseSovereignty` | Permissive OSI license (`MIT`, `Apache-2.0`, `BSD-2/3-Clause`, `ISC`, `Unlicense`). | Copyleft (`GPL`, `AGPL`, `LGPL`), source-available (`SSPL`, `BSL`), or proprietary terms. |
| **G2** | `KernelExtractability` | Core algorithm is logically separable from application wrappers, UI frameworks, or monolithic runtime loops. | Tightly coupled monolithic application framework where business logic cannot be isolated. |
| **G3** | `DeterministicTractability` | Algorithm operates deterministically or can be expressed as a pure state transition $S_{t+1} = f(S_t, I)$. | Irreducible dependency on hardware non-determinism, proprietary C runtimes, or unmanaged OS drivers. |
| **G4** | `EcosystemIndependence` | Can be compiled with standard stable Rust toolchain without C/C++ foreign build dependencies (CMake, make, MSVC scripts). | Foreign C/C++ build system requirements, vendor-locked proprietary prebuilts. |
| **G5** | `RefactoringSurfaceLimit` | Total estimated lines of code requiring extraction is bounded (< 5,000 LOC for a single component). | Sprawling multi-million LOC codebases where extraction cost exceeds clean-room implementation. |

---

## 3. Stage 2: Quadratic Review Criteria (The 4 Feasibility Quadrants)

Candidates passing Stage 1 are scored on four orthogonal quadrants from `0.0` to `10.0`:

### Quadrant I: Algorithmic Needle Value ($w_1 = 0.30$)
- **Core Value**: How sophisticated and valuable is the underlying data structure, mathematical model, or protocol logic?
- **Theoretical Soundness**: Is the algorithm state-of-the-art (e.g. lock-free SWMR ring buffer, SIMD-accelerated parser, verified SMT rule)?
- **Completeness**: Does it solve edge cases, race conditions, and boundary transitions?

### Quadrant II: Refactoring Feasibility & Cratification Cost ($w_2 = 0.30$)
- **Zero-Heap Replaceability**: Can dynamic heap allocations (`Vec`, `String`) be mechanically transformed into stack arrays (`[T; N]`) or caller-provided ring buffer slices?
- **Ambient Hoisting**: Can ambient calls (`std::env::*`, clock reads) be easily hoisted to constructor injection arguments (`Config`, `now: u64`)?
- **Panic Containment**: Can `.unwrap()` / `.expect()` calls be replaced with typed `Result<T, E>` without restructuring the call graph?
- **Layout Adaptability**: Can boundary types be adapted to `#[repr(C)]` with natural alignment and padding fields?
- **Rating Guide**:
  - `9.0 – 10.0`: **Trivial Port** (pure algorithmic logic, `#![no_std]` ready, pure functions).
  - `7.0 – 8.9`: **Moderate Cratification** (few allocations easily replaced with slices, simple constructor injection needed).
  - `4.0 – 6.9`: **Substantial Refactoring** (extensive allocation decoupling, error restructuring required).
  - `0.0 – 3.9`: **Intractable** (spaghettified global state, tangled async scheduler dependencies).

### Quadrant III: Test Rigor & Golden Fixture Portability ($w_3 = 0.20$)
- **Verification Harness**: Does the candidate have high test coverage, property-based tests, or fuzz benchmarks?
- **Fixture Reusability**: Can candidate tests be ported directly into `dev/emulator_harness` or negative contract suites as golden acceptance benchmarks?
- **Determinism**: Absence of timing-dependent or network-flaky assertions.

### Quadrant IV: Milestone Leverage & Custom Code Elimination ($w_4 = 0.20$)
- **Roadmap Alignment**: Direct utility for an active Aaroneous milestone (e.g. IPC ring buffers, SMT safety interlocks, DXGI capture, headless hypervisor execution).
- **Code Reduction Factor**: Estimated LOC of bespoke Aaroneous code eliminated or accelerated by adopting this design.

---

## 4. Quadratic Defect Penalty & Score Calculation

A candidate crate with a brilliant algorithm (Q1 = 10.0) but intractable refactoring difficulty (Q2 = 2.0) is a dangerous trap. Quadratic scoring heavily penalizes defect severity:

$$\text{Linear Score} = \sum_{i=1}^{4} w_i Q_i$$

$$\text{Defect Penalty} = \lambda \sum_{i=1}^{4} (10.0 - Q_i)^2, \quad \lambda = 0.05$$

$$\text{Composite Score} = \max\left(0.0, \; 10.0 \cdot \text{Linear Score} - \text{Defect Penalty}\right)$$

---

## 5. Ingestion Action Typestate (RFC-0005 Mapped)

| Composite Score | Ingestion Recommendation | Execution Workflow |
| --- | --- | --- |
| **80.0 – 100.0** | `DIRECT_ADOPT_WITH_WRAPPER` | Crate is already modular and near-compliant. Wrap in safe typed adapter crate with zero in-place mutation. |
| **60.0 – 79.9** | `CRATIFY_KERNEL_EXTRACTION` | Stage in `dev/legacy_staging/`. Run local agent extraction loop to isolate kernel needle, replace heap with ring buffers, and rebase into `crates/<target>/`. |
| **40.0 – 59.9** | `BORROW_SPEC_AND_SYNTHESIZE` | Do not import source. Use the candidate's test fixtures and algorithmic design as a reference for clean-room synthesis by local GPU agent. |
| **< 40.0** | `REFACTOR_COST_PROHIBITIVE` | Refactoring cost exceeds clean-room implementation. Disqualify. |
