#!/usr/bin/env python3
"""Autonomous host self-updater for aaroneous-devtools (lane C173).

Synchronizes the host repository with upstream `origin/main`, validates
worktree cleanliness, executes release compilation, restarts systemd user
services upon success, and appends a structured receipt to the state log.

Zero ambient destruction:
- Refuses to update if the worktree contains uncommitted modifications or staged changes.
- Refuses to update if the checkout is on a non-main branch.
- Refuses to force-merge; requires strict fast-forward ancestry.
- If compilation fails, preserves running services and leaves old binaries intact.

Usage:
  python3 scripts/host_updater.py [--checkout /path/to/aaroneous-devtools]
                                  [--product-checkout /path/to/aaroneous]
                                  [--remote origin] [--branch main]
                                  [--rebuild-cmd "cargo build --manifest-path foundation/Cargo.toml --release"]
                                  [--restart-services aaroneous-worker.service,aaroneous-devtools-mcp.service]
                                  [--receipt-log ~/.local/state/aaroneous-worker/host-updates.jsonl]
                                  [--dry-run]
"""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional, Tuple


def run_git(cmd: List[str], repo_dir: Path) -> Tuple[int, str, str]:
    """Execute a git command within an explicit repository directory."""
    res = subprocess.run(
        ["git"] + cmd,
        cwd=repo_dir,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return res.returncode, res.stdout.strip(), res.stderr.strip()


def get_current_branch(repo_dir: Path) -> str:
    code, stdout, _ = run_git(["rev-parse", "--abbrev-ref", "HEAD"], repo_dir)
    if code != 0:
        return ""
    return stdout


def is_worktree_dirty(repo_dir: Path) -> bool:
    """Return True if there are modified, staged, or uncommitted tracked changes."""
    code, stdout, _ = run_git(["status", "--porcelain"], repo_dir)
    if code != 0:
        return True
    for line in stdout.splitlines():
        # Lines starting with '??' are untracked files; lines starting with ' M', 'M ', 'A ', 'D ' etc. are tracked edits
        status = line[:2]
        if status != "??" and any(c in status for c in "MARCDU"):
            return True
    return False


def get_head_sha(repo_dir: Path) -> str:
    code, stdout, _ = run_git(["rev-parse", "HEAD"], repo_dir)
    if code != 0:
        return ""
    return stdout


def fetch_upstream(repo_dir: Path, remote: str, branch: str) -> Tuple[bool, str]:
    code, stdout, stderr = run_git(["fetch", remote, branch], repo_dir)
    if code != 0:
        return False, f"git fetch {remote} {branch} failed: {stderr or stdout}"
    return True, ""


def get_remote_sha(repo_dir: Path, remote: str, branch: str) -> str:
    code, stdout, _ = run_git(["rev-parse", f"{remote}/{branch}"], repo_dir)
    if code != 0:
        return ""
    return stdout


def is_ancestor(repo_dir: Path, ancestor_sha: str, descendant_sha: str) -> bool:
    code, _, _ = run_git(["merge-base", "--is-ancestor", ancestor_sha, descendant_sha], repo_dir)
    return code == 0


def evaluate_readiness(
    repo_dir: Path, remote: str, branch: str
) -> Tuple[bool, str, Dict[str, Any]]:
    """Evaluate whether the repo is safe and ready to be fast-forward updated."""
    if not repo_dir.is_dir() or not (repo_dir / ".git").exists():
        return False, f"directory {repo_dir} is not a valid git repository", {}

    current_branch = get_current_branch(repo_dir)
    if current_branch != branch:
        return False, f"on branch '{current_branch}', expected '{branch}'; update refused", {}

    if is_worktree_dirty(repo_dir):
        return False, "worktree has uncommitted tracked changes; update refused to prevent data loss", {}

    ok, err = fetch_upstream(repo_dir, remote, branch)
    if not ok:
        return False, err, {}

    local_sha = get_head_sha(repo_dir)
    remote_sha = get_remote_sha(repo_dir, remote, branch)
    if not local_sha or not remote_sha:
        return False, "failed to resolve HEAD or remote SHA", {}

    if local_sha == remote_sha:
        return False, "already up to date", {"local_sha": local_sha, "remote_sha": remote_sha}

    if not is_ancestor(repo_dir, local_sha, remote_sha):
        return False, f"remote {remote_sha} is not a descendant of local {local_sha}; non-fast-forward update refused", {
            "local_sha": local_sha,
            "remote_sha": remote_sha,
        }

    return True, "ready to fast-forward update", {
        "local_sha": local_sha,
        "remote_sha": remote_sha,
    }


def fast_forward_merge(repo_dir: Path, remote: str, branch: str) -> Tuple[bool, str]:
    code, stdout, stderr = run_git(["merge", "--ff-only", f"{remote}/{branch}"], repo_dir)
    if code != 0:
        return False, f"git merge --ff-only failed: {stderr or stdout}"
    return True, stdout


def run_diff_check(repo_dir: Path) -> Tuple[bool, str]:
    code, stdout, stderr = run_git(["diff", "--check"], repo_dir)
    if code != 0:
        return False, f"git diff --check failed: {stderr or stdout}"
    return True, stdout


def run_rebuild(rebuild_cmd: str, repo_dir: Path) -> Tuple[bool, str, float]:
    start = time.monotonic()
    res = subprocess.run(
        rebuild_cmd,
        shell=True,
        cwd=repo_dir,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    elapsed = time.monotonic() - start
    if res.returncode != 0:
        err = res.stderr.strip() or res.stdout.strip()
        return False, f"build failed with exit code {res.returncode}: {err}", elapsed
    return True, res.stdout.strip(), elapsed


def restart_user_services(services: List[str]) -> Dict[str, bool]:
    outcomes: Dict[str, bool] = {}
    for svc in services:
        svc = svc.strip()
        if not svc:
            continue
        res = subprocess.run(["systemctl", "--user", "restart", svc], capture_output=True)
        outcomes[svc] = res.returncode == 0
    return outcomes


def record_receipt(receipt_path: Path, entry: Dict[str, Any]) -> None:
    receipt_path.parent.mkdir(parents=True, exist_ok=True)
    with receipt_path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(entry, sort_keys=True) + "\n")


def sync_repository(
    repo_dir: Path,
    remote: str,
    branch: str,
    rebuild_cmd: Optional[str] = None,
    restart_services: Optional[List[str]] = None,
    receipt_log: Optional[Path] = None,
    dry_run: bool = False,
) -> Dict[str, Any]:
    """Execute a single synchronization pass on a repository."""
    ready, reason, meta = evaluate_readiness(repo_dir, remote, branch)
    result: Dict[str, Any] = {
        "repo": str(repo_dir),
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "ready": ready,
        "reason": reason,
        "local_sha": meta.get("local_sha", ""),
        "remote_sha": meta.get("remote_sha", ""),
        "status": "noop" if not ready else "pending",
    }

    if not ready:
        return result

    if dry_run:
        result["status"] = "dry-run"
        return result

    # Perform fast-forward
    ok, merge_msg = fast_forward_merge(repo_dir, remote, branch)
    if not ok:
        result["status"] = "merge-failed"
        result["error"] = merge_msg
        return result

    # Verify no whitespace / carriage return anomalies
    ok, diff_err = run_diff_check(repo_dir)
    if not ok:
        result["status"] = "diff-check-failed"
        result["error"] = diff_err
        return result

    # Perform rebuild if specified
    if rebuild_cmd:
        ok, build_out, duration_secs = run_rebuild(rebuild_cmd, repo_dir)
        result["build_duration_secs"] = round(duration_secs, 2)
        if not ok:
            result["status"] = "build-failed"
            result["error"] = build_out
            if receipt_log:
                record_receipt(receipt_log, result)
            return result

    # Restart services if specified
    if restart_services:
        service_results = restart_user_services(restart_services)
        result["service_restarts"] = service_results

    result["status"] = "success"
    if receipt_log:
        record_receipt(receipt_log, result)

    return result


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Autonomous host self-updater for aaroneous-devtools (lane C173)"
    )
    parser.add_argument(
        "--checkout",
        type=Path,
        default=Path("/home/aaroneous/aaroneous-devtools"),
        help="Path to devtools repository clone",
    )
    parser.add_argument(
        "--product-checkout",
        type=Path,
        default=Path("/home/aaroneous/aaroneous"),
        help="Optional path to Aaroneous product repository clone to also synchronize",
    )
    parser.add_argument(
        "--remote",
        default="origin",
        help="Remote name to track (default: origin)",
    )
    parser.add_argument(
        "--branch",
        default="main",
        help="Branch to synchronize (default: main)",
    )
    parser.add_argument(
        "--rebuild-cmd",
        default="cargo build --manifest-path foundation/Cargo.toml --release",
        help="Compilation command executed when updates land",
    )
    parser.add_argument(
        "--restart-services",
        default="aaroneous-worker.service,aaroneous-devtools-mcp.service",
        help="Comma-separated systemd user services to restart upon landed update",
    )
    parser.add_argument(
        "--receipt-log",
        type=Path,
        default=Path(os.path.expanduser("~/.local/state/aaroneous-worker/host-updates.jsonl")),
        help="JSONL file to append update execution receipts",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Evaluate status without modifying worktrees, building, or restarting services",
    )
    return parser.parse_args(argv)


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)
    restart_svcs = [s.strip() for s in args.restart_services.split(",") if s.strip()]

    print(f"[{datetime.now(timezone.utc).isoformat()}] host_updater: checking {args.checkout}")
    devtools_res = sync_repository(
        repo_dir=args.checkout,
        remote=args.remote,
        branch=args.branch,
        rebuild_cmd=args.rebuild_cmd,
        restart_services=restart_svcs,
        receipt_log=args.receipt_log,
        dry_run=args.dry_run,
    )
    print(f"host_updater: devtools -> status={devtools_res['status']} ({devtools_res['reason']})")
    if devtools_res.get("error"):
        print(f"host_updater: devtools error: {devtools_res['error']}", file=sys.stderr)

    # Optional product sync (git fast-forward only, no cargo build or service restart)
    if args.product_checkout and args.product_checkout.is_dir():
        print(f"[{datetime.now(timezone.utc).isoformat()}] host_updater: checking product {args.product_checkout}")
        product_res = sync_repository(
            repo_dir=args.product_checkout,
            remote=args.remote,
            branch=args.branch,
            rebuild_cmd=None,
            restart_services=None,
            receipt_log=args.receipt_log,
            dry_run=args.dry_run,
        )
        print(f"host_updater: product -> status={product_res['status']} ({product_res['reason']})")
        if product_res.get("error"):
            print(f"host_updater: product error: {product_res['error']}", file=sys.stderr)

    if devtools_res["status"] in ("merge-failed", "diff-check-failed", "build-failed"):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
