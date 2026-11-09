#!/usr/bin/env python3
"""Unit tests for scripts/host_updater.py (lane C173)."""

import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import MagicMock, patch

import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import host_updater as hu


class TestHostUpdater(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.repo_dir = Path(self.temp_dir.name)
        (self.repo_dir / ".git").mkdir()

    def tearDown(self):
        self.temp_dir.cleanup()

    def test_evaluate_readiness_invalid_repo(self):
        non_repo = self.repo_dir / "not_git"
        non_repo.mkdir()
        ok, reason, _ = hu.evaluate_readiness(non_repo, "origin", "main")
        self.assertFalse(ok)
        self.assertIn("not a valid git repository", reason)

    @patch("host_updater.get_current_branch", return_value="feature-branch")
    def test_evaluate_readiness_wrong_branch(self, _mock_branch):
        ok, reason, _ = hu.evaluate_readiness(self.repo_dir, "origin", "main")
        self.assertFalse(ok)
        self.assertIn("on branch 'feature-branch', expected 'main'", reason)

    @patch("host_updater.get_current_branch", return_value="main")
    @patch("host_updater.is_worktree_dirty", return_value=True)
    def test_evaluate_readiness_dirty_worktree(self, _mock_dirty, _mock_branch):
        ok, reason, _ = hu.evaluate_readiness(self.repo_dir, "origin", "main")
        self.assertFalse(ok)
        self.assertIn("worktree has uncommitted tracked changes", reason)

    @patch("host_updater.get_current_branch", return_value="main")
    @patch("host_updater.is_worktree_dirty", return_value=False)
    @patch("host_updater.fetch_upstream", return_value=(False, "network timeout"))
    def test_evaluate_readiness_fetch_failure(self, _mock_fetch, _mock_dirty, _mock_branch):
        ok, reason, _ = hu.evaluate_readiness(self.repo_dir, "origin", "main")
        self.assertFalse(ok)
        self.assertIn("network timeout", reason)

    @patch("host_updater.get_current_branch", return_value="main")
    @patch("host_updater.is_worktree_dirty", return_value=False)
    @patch("host_updater.fetch_upstream", return_value=(True, ""))
    @patch("host_updater.get_head_sha", return_value="abc123456789")
    @patch("host_updater.get_remote_sha", return_value="abc123456789")
    def test_evaluate_readiness_already_up_to_date(
        self, _mock_remote_sha, _mock_head_sha, _mock_fetch, _mock_dirty, _mock_branch
    ):
        ok, reason, meta = hu.evaluate_readiness(self.repo_dir, "origin", "main")
        self.assertFalse(ok)
        self.assertEqual(reason, "already up to date")
        self.assertEqual(meta["local_sha"], "abc123456789")

    @patch("host_updater.get_current_branch", return_value="main")
    @patch("host_updater.is_worktree_dirty", return_value=False)
    @patch("host_updater.fetch_upstream", return_value=(True, ""))
    @patch("host_updater.get_head_sha", return_value="local11111111")
    @patch("host_updater.get_remote_sha", return_value="remote22222222")
    @patch("host_updater.is_ancestor", return_value=False)
    def test_evaluate_readiness_diverged_branch_refused(
        self, _mock_ancestor, _mock_remote_sha, _mock_head_sha, _mock_fetch, _mock_dirty, _mock_branch
    ):
        ok, reason, _ = hu.evaluate_readiness(self.repo_dir, "origin", "main")
        self.assertFalse(ok)
        self.assertIn("non-fast-forward update refused", reason)

    @patch("host_updater.get_current_branch", return_value="main")
    @patch("host_updater.is_worktree_dirty", return_value=False)
    @patch("host_updater.fetch_upstream", return_value=(True, ""))
    @patch("host_updater.get_head_sha", return_value="local11111111")
    @patch("host_updater.get_remote_sha", return_value="remote22222222")
    @patch("host_updater.is_ancestor", return_value=True)
    def test_evaluate_readiness_ready_to_update(
        self, _mock_ancestor, _mock_remote_sha, _mock_head_sha, _mock_fetch, _mock_dirty, _mock_branch
    ):
        ok, reason, meta = hu.evaluate_readiness(self.repo_dir, "origin", "main")
        self.assertTrue(ok)
        self.assertEqual(reason, "ready to fast-forward update")
        self.assertEqual(meta["local_sha"], "local11111111")
        self.assertEqual(meta["remote_sha"], "remote22222222")

    @patch("host_updater.evaluate_readiness")
    def test_sync_repository_dry_run(self, mock_eval):
        mock_eval.return_value = (True, "ready to fast-forward update", {
            "local_sha": "aaa", "remote_sha": "bbb"
        })
        res = hu.sync_repository(self.repo_dir, "origin", "main", dry_run=True)
        self.assertEqual(res["status"], "dry-run")
        self.assertTrue(res["ready"])

    @patch("host_updater.evaluate_readiness")
    @patch("host_updater.fast_forward_merge", return_value=(True, "Fast-forward"))
    @patch("host_updater.run_diff_check", return_value=(True, ""))
    @patch("host_updater.run_rebuild", return_value=(False, "compiler syntax error", 5.0))
    def test_sync_repository_build_failure(
        self, _mock_rebuild, _mock_diff, _mock_merge, mock_eval
    ):
        mock_eval.return_value = (True, "ready to fast-forward update", {
            "local_sha": "aaa", "remote_sha": "bbb"
        })
        receipt_file = self.repo_dir / "receipts.jsonl"
        res = hu.sync_repository(
            self.repo_dir,
            "origin",
            "main",
            rebuild_cmd="cargo build --release",
            receipt_log=receipt_file,
        )
        self.assertEqual(res["status"], "build-failed")
        self.assertIn("compiler syntax error", res["error"])
        self.assertTrue(receipt_file.exists())
        with receipt_file.open() as f:
            lines = f.readlines()
            self.assertEqual(len(lines), 1)
            record = json.loads(lines[0])
            self.assertEqual(record["status"], "build-failed")

    @patch("host_updater.evaluate_readiness")
    @patch("host_updater.fast_forward_merge", return_value=(True, "Fast-forward"))
    @patch("host_updater.run_diff_check", return_value=(True, ""))
    @patch("host_updater.run_rebuild", return_value=(True, "Finished release", 12.3))
    @patch("host_updater.restart_user_services", return_value={"svc1": True, "svc2": True})
    def test_sync_repository_full_success(
        self, mock_restart, _mock_rebuild, _mock_diff, _mock_merge, mock_eval
    ):
        mock_eval.return_value = (True, "ready to fast-forward update", {
            "local_sha": "aaa", "remote_sha": "bbb"
        })
        receipt_file = self.repo_dir / "receipts.jsonl"
        res = hu.sync_repository(
            self.repo_dir,
            "origin",
            "main",
            rebuild_cmd="cargo build --release",
            restart_services=["svc1", "svc2"],
            receipt_log=receipt_file,
        )
        self.assertEqual(res["status"], "success")
        self.assertEqual(res["build_duration_secs"], 12.3)
        self.assertTrue(res["service_restarts"]["svc1"])
        mock_restart.assert_called_once_with(["svc1", "svc2"])
        with receipt_file.open() as f:
            record = json.loads(f.readline())
            self.assertEqual(record["status"], "success")

    def test_record_receipt(self):
        receipt_path = self.repo_dir / "nested" / "updates.jsonl"
        hu.record_receipt(receipt_path, {"test": "val", "count": 42})
        self.assertTrue(receipt_path.exists())
        with receipt_path.open() as f:
            data = json.loads(f.readline())
            self.assertEqual(data["test"], "val")
            self.assertEqual(data["count"], 42)

    def test_parse_args_defaults(self):
        args = hu.parse_args([])
        self.assertEqual(args.remote, "origin")
        self.assertEqual(args.branch, "main")
        self.assertFalse(args.dry_run)
        self.assertIn("aaroneous-worker.service", args.restart_services)


if __name__ == "__main__":
    unittest.main()
