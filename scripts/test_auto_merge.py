import os
import sys
import types
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(__file__))
import auto_merge as am  # noqa: E402

ALLOWED = am.ALLOWED_REPO


def decision(pr, eligible, reasons=(), head="a" * 40):
    return {"pr": pr, "head": head, "eligible": eligible, "reasons": list(reasons)}


def fake_merge_result(returncode, stderr=""):
    result = types.SimpleNamespace(returncode=returncode, stderr=stderr, stdout="")
    return result


class RepoGuard(unittest.TestCase):
    def test_refuses_any_repo_other_than_devtools(self):
        with mock.patch.object(am.mg, "evaluate_all") as ev:
            exit_code, lines = am.run("Aarogaming/aaroneous", "/tmp/x", "governance/identities.json", None, False)
        ev.assert_not_called()
        self.assertEqual(exit_code, 1)
        self.assertTrue(any("refusing" in l and "aaroneous" in l for l in lines), lines)

    def test_the_allowed_repo_is_the_devtools_repo(self):
        self.assertEqual(ALLOWED, "Aarogaming/aaroneous-devtools")


class DryRun(unittest.TestCase):
    def test_dry_run_never_calls_merge(self):
        with mock.patch.object(am.mg, "evaluate_all", return_value=[decision(1, True)]):
            merge_fn = mock.Mock()
            exit_code, lines = am.run(ALLOWED, "/tmp/x", "governance/identities.json", None, True, merge_fn=merge_fn)
        merge_fn.assert_not_called()
        self.assertEqual(exit_code, 0)
        self.assertTrue(any("ELIGIBLE" in l for l in lines))
        self.assertTrue(any("dry run" in l for l in lines))


class Merging(unittest.TestCase):
    def test_eligible_prs_are_merged_blocked_ones_are_not(self):
        decisions = [decision(1, True), decision(2, False, reasons=["a: not covered"])]
        merge_fn = mock.Mock(return_value=fake_merge_result(0))
        with mock.patch.object(am.mg, "evaluate_all", return_value=decisions):
            exit_code, lines = am.run(ALLOWED, "/tmp/x", "governance/identities.json", None, False, merge_fn=merge_fn)
        merge_fn.assert_called_once_with(ALLOWED, 1, "a" * 40)
        self.assertEqual(exit_code, 0)
        self.assertTrue(any("BLOCKED" in l and "not covered" in l for l in lines))
        self.assertTrue(any("merged #1" in l for l in lines))

    def test_a_failed_merge_call_is_reported_and_fails_the_run(self):
        merge_fn = mock.Mock(return_value=fake_merge_result(1, stderr="already merged"))
        with mock.patch.object(am.mg, "evaluate_all", return_value=[decision(3, True)]):
            exit_code, lines = am.run(ALLOWED, "/tmp/x", "governance/identities.json", None, False, merge_fn=merge_fn)
        self.assertEqual(exit_code, 1)
        self.assertTrue(any("merge failed" in l and "already merged" in l for l in lines), lines)

    def test_only_pr_is_forwarded_to_evaluate_all(self):
        with mock.patch.object(am.mg, "evaluate_all", return_value=[]) as ev:
            am.run(ALLOWED, "/tmp/x", "governance/identities.json", 42, True)
        ev.assert_called_once_with(ALLOWED, "/tmp/x", "governance/identities.json", 42, False)

    def test_head_change_is_refused_and_reported(self):
        checked_head, live_head = "a" * 40, "b" * 40

        def guarded_merge(repo, pr, expected_head):
            self.assertEqual(expected_head, checked_head)
            return fake_merge_result(1, "head changed") if expected_head != live_head else fake_merge_result(0)

        with mock.patch.object(am.mg, "evaluate_all", return_value=[decision(7, True, head=checked_head)]):
            code, lines = am.run(ALLOWED, "/tmp/x", "ids.json", None, False, merge_fn=guarded_merge)
        self.assertEqual(code, 1)
        self.assertFalse(any("merged #7" in line for line in lines))

    def test_cli_merge_requires_full_evaluated_sha(self):
        head = "abcdef01" * 5
        with mock.patch.object(am.subprocess, "run", return_value=fake_merge_result(0)) as run:
            am.merge(ALLOWED, 7, head)
        argv = run.call_args.args[0]
        self.assertEqual(argv[argv.index("--match-head-commit") + 1], head)


if __name__ == "__main__":
    unittest.main()
