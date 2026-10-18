# Management Components (Operational Runners)

Components in this tier own active development environment operations, persistent services, and local worker management.

## Managed Areas

- **`local_worker`**: Adaptive local Ollama/Qwen worker service, reservation locking, quiet-window backoff, and scan-cycle processing.
- **`maintenance_pipeline`**: Atomic file update mechanisms, automated fix loops, and recovery pipelines.
- **`system_telemetry`**: Host monitors (GPU VRAM, thermal levels, system load, disk space) for the Ubuntu operations center and Windows R&D station.
