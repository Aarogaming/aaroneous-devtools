# Host service operations: persistent workers, MCP, and autonomous updater (lane C173)

This document specifies the operational design, systemd service lifecycle, and autonomous synchronization system on the Linux host (`ubuntu-worker-01`, `100.109.8.12`).

## 1. Systemd host service layout

All user services run under `systemd --user` for account `aaroneous` on `ubuntu-worker-01`:

| Service / Unit | Type | Target / Command | Purpose |
| --- | --- | --- | --- |
| `aaroneous-worker.service` | `simple` | `foundation local-worker-serve --config ~/.local/state/aaroneous-worker/worker.toml` | Continuous PLC scan loop: polls approved jobs, executes model calls, evaluates validators, and routes into the active pipeline. |
| `aaroneous-devtools-mcp.service` | `simple` | `python3 scripts/devtools_mcp_server.py --port 8767` | LAN + Loopback Model Context Protocol server exposing devtools operations tools. |
| `aaroneous-feeder.timer` / `.service` | `timer` / `oneshot` | `aaroneous-feed.sh` (hourly) | Bounded C96 mechanical fixture job feeder. |
| `aaroneous-updater.timer` / `.service` | `timer` / `oneshot` | `python3 scripts/host_updater.py` (every 15 minutes) | Autonomous updater: pulls `origin/main`, rebuilds release binaries, and safely restarts services on landed changes. |

## 2. Autonomous self-updater (`scripts/host_updater.py`)

The host updater automatically keeps local services in sync with upstream changes on `origin/main` without manual intervention, while upholding strict safety invariants.

### Safety invariants
1. **Worktree cleanliness preservation**: The updater checks `git status --porcelain`. If any tracked files contain uncommitted edits or staged changes, the update is skipped immediately with a clear log reason. Local in-progress work is never clobbered or discarded.
2. **Strict fast-forward only**: The updater verifies `git merge-base --is-ancestor HEAD origin/main`. Diverged branches or non-fast-forward commits are refused.
3. **Branch binding**: The updater only runs against the designated target branch (`main`).
4. **Pre-deployment compilation gate**: After fast-forwarding, the updater runs `cargo build --manifest-path foundation/Cargo.toml --release`. If compilation fails, the running services continue executing their existing binary, and the error is recorded in the receipt log.
5. **Atomic restart & receipt emission**: On successful compilation, dependent user services (`aaroneous-worker.service`, `aaroneous-devtools-mcp.service`) are restarted, and an append-only structured record is written to `~/.local/state/aaroneous-worker/host-updates.jsonl`.

### Execution receipt schema
```json
{
  "build_duration_secs": 18.42,
  "local_sha": "021c8d808209233c39af6c6ebaa1325ea4c18915",
  "ready": true,
  "reason": "ready to fast-forward update",
  "remote_sha": "172c89304918e9d626bf553f1f25a7e795ce7968",
  "repo": "/home/aaroneous/aaroneous-devtools",
  "service_restarts": {
    "aaroneous-devtools-mcp.service": true,
    "aaroneous-worker.service": true
  },
  "status": "success",
  "timestamp": "2026-10-28T19:35:00.000000+00:00"
}
```

## 3. Operational runbook

### Manual synchronization run
```bash
python3 scripts/host_updater.py --dry-run
python3 scripts/host_updater.py
```

### Inspecting service logs
```bash
journalctl --user -u aaroneous-updater.service -n 50 --no-pager
journalctl --user -u aaroneous-worker.service -n 50 --no-pager
```

### Checking timer status
```bash
systemctl --user list-timers aaroneous-updater.timer
```

### Pausing or stopping
```bash
systemctl --user stop aaroneous-updater.timer
systemctl --user stop aaroneous-worker.service
```
