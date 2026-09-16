# Rust Ecosystem Catalogue

These ecosystems are optional foundation capabilities. Add one only in an isolated foundation crate
with a stated use case, feature boundary, license review, reproducible test, and no Aaroneous
product dependency.

| Ecosystem | Foundation use | Adoption rule |
| --- | --- | --- |
| Candle | Local model evaluation, embeddings, and bounded inference experiments | Keep model paths explicit and outputs versioned. |
| Rig | Agent workflow experiments and tool-call evaluation | Use only outside product execution; record prompts and fixtures. |
| Burn | Training and portable model experiments | Isolate accelerator/toolchain requirements. |
| ONNX Runtime (`ort`) | Interoperability evaluation | Treat model/runtime binaries as caller-provided support inputs. |
| Tokio / Axum | Optional local control plane or report service | No mandatory daemon; command-line path remains supported. |
| ratatui | Terminal inspection UI | Read-only view over versioned reports. |
| cargo-deny / cargo-audit | Supply-chain evidence | Run through declared commands and archive reports. |

The initial foundation CLI intentionally has no optional technology-stack dependency. Capability
crates are added one at a time after their contract and maintenance cost are approved.
