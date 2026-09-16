## Second tranche — disconnected Python surface

Transferred from Aaroneous on 2026-09-16 after exact-path and source-reference inspection:

- `tools/`: local/cloud agent clients and an autonomous host-editing loop.
- `core/hypervisor/utils/mine_tensors.py`: a machine-specific GGUF indexer with hard-coded paths.
- `sdk/python/`: an un-packaged Python bridge and orchestrator with no live framework reference.
- `data/extensions/python/` and `data/extensions/win32_intercept/`: disconnected extension stubs.

These are retained only as historical migration input. They may not be restored as a framework dependency without a reviewed Rust interface and explicit compatibility contract.
