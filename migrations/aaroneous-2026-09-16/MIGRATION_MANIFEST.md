# Migration manifest: Aaroneous tooling separation

Date: 2026-09-16
Source repository: Aarogaming/aaroneous

## Transferred

- `dev/tools/`: agent loops, workstation diagnostics, local runners, experimental project analysis, generated map output, and legacy protocol material.
- `.agents/skills/local-agent-delegate/`: local model delegation instructions.
- `opencode.json`: personal OpenCode/local-model configuration.
- `scripts/generate_modelfiles.ps1`, `scripts/local_agent_delegate.ps1`, `scripts/register_ollama_models.ps1`, and `scripts/update_qwen_templates.ps1`: local Ollama and LM Studio model operations.
- `scripts/rosetta_stone_oracle.py`: experimental model-data generator.

## Remains in Aaroneous

- Rust workspace code and product-facing interfaces.
- Canonical verification entry points pending Rust-native replacement: `scripts/agent_check.sh`, `scripts/agent_check.ps1`, `scripts/verify_immune_system.sh`, and the text-encoding migration scripts.

The copied material is a transfer record. It is not approved to become a dependency of the Aaroneous build, test, CI, release, or runtime paths.
