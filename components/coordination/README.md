# Coordination Components (Multi-Agent Consensus)

Components in this tier own distributed coordination between agent seats (Ubuntu worker, Windows R&D, Codex, Claude, Gemini/Antigravity).

## Coordinated Operations

- **`steward`**: Pure deterministic state reducer for task cards, claims, and event chain verification.
- **`attestation`**: Cryptographic signatures (`ed25519`) and Git commit attestation per seat identity.
- **`queue_consensus`**: Ledger arbitration for `COORDINATION_QUEUE.md` and continuum lease management.
