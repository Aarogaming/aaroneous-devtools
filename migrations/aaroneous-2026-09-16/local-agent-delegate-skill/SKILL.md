---
name: local-agent-delegate
description: >-
  Delegate code generation, method synthesis, refactoring, and test fixture tasks to a local GPU-hosted model (Ollama / qwen3.5:9b-q6) for token optimization, zero latency, and private offloading. Includes prompt staging, reasoning runaway suppression, and invariant verification loops.
---

# Local Agent Delegation (`local-agent-delegate`)

## Overview

This skill defines the operational protocol for delegating computation, code generation, refactoring, and test synthesis tasks to locally hosted models running on the user's workstation (primarily Ollama at `http://localhost:11434`, using `qwen3.5:9b-q6` or `qwen2.5-coder:14b`).

This pattern drastically reduces cloud token consumption, leverages local GPU compute, and ensures tight iteration loops for high-volume systems code generation.

---

## Architecture & Subsystems

```
┌─────────────────────────────────────────────────────────────┐
│                      Antigravity Agent                      │
│            (Decomposes task, drafts specifications)         │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│              scripts/local_agent_delegate.ps1               │
│  • -PromptFile (avoids shell quote escaping corruption)     │
│  • -SystemPrompt (suppresses reasoning/thinking runaway)    │
│  • -OutputFile (streams directly to source file on disk)    │
│  • -NumPredict 4096 (allocates generous generation headroom)│
└──────────────────────────────┬──────────────────────────────┘
                               │ HTTP POST /api/chat
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 Local Ollama Instance (GPU)                 │
│                 Model: qwen3.5:9b-q6 (or Qwen 2.5 Coder)    │
└──────────────────────────────┬──────────────────────────────┘
                               │ Writes output to disk
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                  Invariant Audit & Compile                  │
│  • cargo check & cargo test                                 │
│  • cargo run -p ast_auditor -- review                       │
│  • AGENTS.md Conformance Gates (Zero Heap, #[repr(C)])      │
└─────────────────────────────────────────────────────────────┘
```

---

## Operating Protocol

### 1. Verification & Model Health
Always verify the local server is reachable and inspect loaded models before dispatching large jobs:
```powershell
# Check Ollama API health and list active models
Invoke-RestMethod -Uri "http://localhost:11434/api/tags" | Select-Object -ExpandProperty models | Select-Object name, size
```

Primary models configured on this workstation:
- **`qwen3.5:9b-q6`**: 9B Q6_K default reasoning model (sharp, high precision).
- **`qwen2.5-coder:14b`**: 14B specialized code reasoning engine.
- **`qwen3-coder:30b`**: 30B heavy code reasoning MoE.

---

### 2. Reasoning Runaway Suppression (Critical for `qwen3.5`)
Reasoning models like `qwen3.5:9b-q6` output internal chain-of-thought in a `thinking` block before emitting `content`. Without calibration, complex prompts can spend 4,000+ tokens purely in internal reflection, resulting in `done_reason: length` and an empty `content` block.

**Mandatory Countermeasures**:
1. **Calibrated System Prompt**:
   ```text
   You are a senior Rust systems programmer. Do NOT output any lengthy thinking trace or reasoning. Output only pure, complete Rust code.
   ```
   *(Reduces thinking trace by 97%, allowing immediate code output).*
2. **Headroom Budget**: Set `-NumPredict 4096` (or 8192) so the model never runs out of tokens.
3. **Zero Temperature**: Set `-Temperature 0.0` for deterministic, reproducible code synthesis.

---

### 3. File-Staged Delegation Pattern
To prevent Windows PowerShell command-line argument parsing and quote-escaping corruptions:
1. **Write the prompt to a staging file**: `scripts/prompt_<task>.txt`.
2. **Execute delegation via the PowerShell runner**:
   ```powershell
   pwsh -File scripts/local_agent_delegate.ps1 `
       -PromptFile scripts/prompt_<task>.txt `
       -OutputFile scripts/output_<task>.rs `
       -Model "qwen3.5:9b-q6" `
       -NumPredict 4096 `
       -Temperature 0.0
   ```
3. **Inspect and integrate** the generated code from `scripts/output_<task>.rs`.

---

### 4. Modular Task Decomposition
Do not ask the local model to write an entire multi-struct crate in a single prompt. Decompose into focused, single-responsibility units:
- **Phase A**: Struct definition, alignment attributes (`#[repr(C, align(64))]`), and compile-time `const _: () = assert!(...);` geometry assertions.
- **Phase B**: Core accessor and modification methods (`empty`, `is_valid`, `set_channel`, `get_channel`).
- **Phase C**: Serialization/conversion traits (`From`, `TryFrom`) and `#![no_std]` compatibility.
- **Phase D**: Unit tests (`bytemuck::bytes_of`, roundtrip serialization, edge cases).

---

### 5. Architectural Verification Gate
All code synthesized by the local model MUST pass the repository's machine contract:
1. **Compile & Unit Test**:
   ```powershell
   cargo test -p <crate_name>
   ```
2. **Pattern Conformance Review**:
   ```powershell
   cargo run -p ast_auditor -- review crates/<crate_name>
   ```
3. **Workspace Invariant Audit**:
   ```powershell
   cargo run -p ast_auditor -- audit core/ crates/
   ```

---

## Common Pitfalls & Solutions

| Pitfall | Symptom | Remediation |
| :--- | :--- | :--- |
| **Thinking Saturation** | Output file is 0 bytes; `done_reason: length` | Add strict system prompt suppressing chain-of-thought; increase `num_predict` to 4096+. |
| **Shell Quote Truncation** | PowerShell errors or truncated prompt | Always use `-PromptFile` instead of passing multiline string arguments via CLI. |
| **Serde Array Limit (>32)** | `the trait bound [u8; N]: Serialize is not satisfied` | Serde only supports arrays $\le 32$ without helper crates. Split padding larger than 32 into chunks (e.g. `_pad1: [u8; 32], _pad2: [u8; 12]`). |
| **Implicit Compiler Padding** | `bytemuck` derive fails with unaligned struct | Add explicit padding bytes (`_pad0: [u8; 2]`) to naturally align 4-byte and 8-byte members. |
