import json
import os
import re
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import check_coordination_evidence as cce
from check_coordination_evidence import review_rows_without_contribution_id as warn


class ReviewRowWarning(unittest.TestCase):
    def test_flags_review_row_without_id(self):
        row = "+| C1 | scope | Review | owner | branch | 2026-10-01 | notes |"
        self.assertEqual(warn([row]), [row])

    def test_accepts_review_row_with_id(self):
        row = "+| C1 | scope | Review | owner | branch | 2026-10-01 | Contribution-ID: C1/x-v1 |"
        self.assertEqual(warn([row]), [])

    def test_ignores_other_statuses_and_context_lines(self):
        self.assertEqual(warn(["+| C1 | scope | Claimed | o | b | d | n |"]), [])
        self.assertEqual(warn(["| C1 | scope | Review | o | b | d | n |"]), [])


ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STATUSES = ["Ready", "Claimed", "Review", "Done", "Blocked"]


def row(lane, status, scope="scope"):
    return f"| {lane} | {scope} | {status} | owner | branch | 2026-10-05 | notes |"


class LaneStatusRule(unittest.TestCase):
    def test_a_lane_row_is_read_the_way_the_native_parser_reads_it(self):
        self.assertEqual(cce.lane_status(row("C103", "Claimed")), ("C103", "Claimed"))
        for not_a_lane in ("| ID | Scope | Status | Owner | Branch | Started UTC | Evidence |",
                           "| --- | --- | --- | --- | --- | --- | --- |",
                           "prose line | with | pipes | in | it",
                           " " + row("C103", "Claimed"),
                           "| C1 | too |"):
            self.assertIsNone(cce.lane_status(not_a_lane), not_a_lane)

    def test_an_unrecognised_status_in_an_added_row_is_flagged(self):
        diff = ["+" + row("C102", "Proposed"), "+" + row("C103", "Blocked"), "+" + row("C104", "In progress")]
        found = cce.status_violations(diff, STATUSES)
        self.assertEqual([(lane, status) for lane, status, _ in found], [("C102", "Proposed"), ("C104", "In progress")])

    def test_every_recognised_status_passes_and_removed_or_context_lines_are_ignored(self):
        diff = ["+" + row("C1", st) for st in STATUSES] + ["-" + row("C2", "Proposed"), " " + row("C3", "Proposed"),
                                                          "+++ b/governance/COORDINATION_QUEUE.md"]
        self.assertEqual(cce.status_violations(diff, STATUSES), [])

    def test_status_matching_is_exact_because_the_parser_is(self):
        diff = ["+" + row("C1", "ready"), "+" + row("C2", "Ready "), "+" + row("C3", "Review")]
        self.assertEqual([lane for lane, _, _ in cce.status_violations(diff, STATUSES)], ["C1"])

    def test_an_unescaped_pipe_inside_a_cell_shifts_the_columns_and_is_flagged(self):
        broken = "| C89 | profile = \"kernel\\|control\\|tooling\" | Ready | Unowned | - | - | notes |"
        found = cce.status_violations(["+" + broken], STATUSES)
        self.assertEqual([(lane, status) for lane, status, _ in found], [("C89", "control\\")])


class VocabularyMatchesTheNativeParser(unittest.TestCase):
    """The allowed words live in two places, this JSON file and the Rust parser. A contract test keeps them equal
    so a change to one that forgets the other fails here instead of silently skipping queue rows."""

    def test_the_json_list_equals_the_list_in_multithreader_rs(self):
        source = os.path.join(ROOT, "foundation", "crates", "foundation_cli", "src", "multithreader.rs")
        self.assertTrue(os.path.exists(source), "the Rust parser this contract is checked against is missing")
        with open(source, encoding="utf-8") as handle:
            text = handle.read()
        match = re.search(r"matches!\(\s*cols\[3\]\s*,((?:\s*\"[A-Za-z ]+\"\s*\|?)+)\)", text)
        self.assertIsNotNone(match, "multithreader.rs no longer has the matches!(cols[3], ...) status list; update this test")
        rust = re.findall(r'"([A-Za-z ]+)"', match.group(1))
        with open(os.path.join(ROOT, "governance", "QUEUE_STATUSES.json"), encoding="utf-8") as handle:
            declared = json.load(handle)["statuses"]
        self.assertEqual(sorted(rust), sorted(declared))
        self.assertNotIn("Proposed", declared)


class EndToEnd(unittest.TestCase):
    """Run the real script against a throwaway git repository."""

    def git(self, cwd, *args):
        subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid", *args],
                       cwd=cwd, check=True, capture_output=True, text=True)

    def run_check(self, repo):
        script = os.path.join(ROOT, "scripts", "check_coordination_evidence.py")
        return subprocess.run([sys.executable, script, "base", "HEAD"], cwd=repo, capture_output=True, text=True)

    def test_a_proposed_row_fails_and_a_blocked_row_passes(self):
        header = "| ID | Scope | Status | Owner | Branch | Started UTC | Evidence |\n| --- | --- | --- | --- | --- | --- | --- |\n"
        with tempfile.TemporaryDirectory() as repo:
            os.makedirs(os.path.join(repo, "governance"))
            with open(os.path.join(repo, "governance", "QUEUE_STATUSES.json"), "w", encoding="utf-8") as handle:
                json.dump({"version": 1, "statuses": STATUSES}, handle)
            queue = os.path.join(repo, "governance", "COORDINATION_QUEUE.md")
            with open(queue, "w", encoding="utf-8", newline="\n") as handle:
                handle.write(header + row("C1", "Ready") + "\n")
            self.git(repo, "init", "-q")
            self.git(repo, "add", "-A")
            self.git(repo, "commit", "-qm", "base")
            self.git(repo, "tag", "base")
            with open(queue, "a", encoding="utf-8", newline="\n") as handle:
                handle.write(row("C2", "Proposed") + "\n")
            self.git(repo, "commit", "-qam", "proposed")
            failed = self.run_check(repo)
            self.assertEqual(failed.returncode, 1, failed.stdout + failed.stderr)
            self.assertIn("C2", failed.stdout)
            self.assertIn("Proposed", failed.stdout)
            with open(queue, "w", encoding="utf-8", newline="\n") as handle:
                handle.write(header + row("C1", "Ready") + "\n" + row("C2", "Blocked") + "\n")
            self.git(repo, "commit", "-qam", "blocked")
            passed = self.run_check(repo)
            self.assertEqual(passed.returncode, 0, passed.stdout + passed.stderr)

    def test_without_a_vocabulary_file_the_check_warns_loudly_and_does_not_fail(self):
        with tempfile.TemporaryDirectory() as repo:
            os.makedirs(os.path.join(repo, "governance"))
            queue = os.path.join(repo, "governance", "COORDINATION_QUEUE.md")
            with open(queue, "w", encoding="utf-8", newline="\n") as handle:
                handle.write(row("C1", "Ready") + "\n")
            self.git(repo, "init", "-q")
            self.git(repo, "add", "-A")
            self.git(repo, "commit", "-qm", "base")
            self.git(repo, "tag", "base")
            with open(queue, "a", encoding="utf-8", newline="\n") as handle:
                handle.write(row("C2", "Proposed") + "\n")
            self.git(repo, "commit", "-qam", "proposed")
            result = self.run_check(repo)
            self.assertEqual(result.returncode, 0)
            self.assertIn("were not checked", result.stderr)


if __name__ == "__main__":
    unittest.main()
