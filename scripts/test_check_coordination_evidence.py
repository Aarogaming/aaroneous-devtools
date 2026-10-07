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
            rust = cce.parse_rust_statuses(handle.read())
        self.assertIsNotNone(rust, "multithreader.rs no longer has the matches!(cols[3], ...) status list; update the checker")
        with open(os.path.join(ROOT, "governance", "QUEUE_STATUSES.json"), encoding="utf-8") as handle:
            declared = json.load(handle)["statuses"]
        self.assertEqual(sorted(rust), sorted(declared))
        self.assertNotIn("Proposed", declared)

    def test_the_rust_list_is_read_from_the_parser_source_text(self):
        sample = 'if cols.len() < 5 || !matches!(cols[3], "Ready" | "Claimed" | "Review" | "Done" | "Blocked")\n{ continue; }'
        self.assertEqual(cce.parse_rust_statuses(sample), ["Ready", "Claimed", "Review", "Done", "Blocked"])
        self.assertIsNone(cce.parse_rust_statuses("fn other() {}"))


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

    def make_repo(self, repo, json_statuses, rust_statuses):
        os.makedirs(os.path.join(repo, "governance"))
        os.makedirs(os.path.join(repo, "foundation", "crates", "foundation_cli", "src"))
        with open(os.path.join(repo, "governance", "QUEUE_STATUSES.json"), "w", encoding="utf-8") as handle:
            json.dump({"version": 1, "statuses": json_statuses}, handle)
        listed = " | ".join(f'"{w}"' for w in rust_statuses)
        parser = os.path.join(repo, cce.RUST_PARSER)
        with open(parser, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(f"if cols.len() < 5 || !matches!(cols[3], {listed})\n{{ continue; }}\n")
        with open(os.path.join(repo, "governance", "COORDINATION_QUEUE.md"), "w", encoding="utf-8", newline="\n") as handle:
            handle.write(row("C1", "Ready") + "\n")
        self.git(repo, "init", "-q")
        self.git(repo, "add", "-A")
        self.git(repo, "commit", "-qm", "base")
        self.git(repo, "tag", "base")
        with open(os.path.join(repo, "governance", "COORDINATION_QUEUE.md"), "a", encoding="utf-8", newline="\n") as handle:
            handle.write(row("C2", "Ready") + "\n")
        self.git(repo, "commit", "-qam", "row")

    def test_a_vocabulary_that_disagrees_with_the_rust_parser_fails_the_check(self):
        with tempfile.TemporaryDirectory() as repo:
            self.make_repo(repo, STATUSES + ["Proposed"], STATUSES)
            result = self.run_check(repo)
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn("disagree", result.stdout)
            self.assertIn("Proposed", result.stdout)

    def test_a_vocabulary_equal_to_the_rust_parser_passes(self):
        with tempfile.TemporaryDirectory() as repo:
            self.make_repo(repo, STATUSES, list(reversed(STATUSES)))
            result = self.run_check(repo)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_losing_a_readable_rust_parser_list_fails_instead_of_switching_the_check_off(self):
        with tempfile.TemporaryDirectory() as repo:
            self.make_repo(repo, STATUSES, STATUSES)
            os.remove(os.path.join(repo, cce.RUST_PARSER))
            self.git(repo, "commit", "-qam", "remove parser")
            result = self.run_check(repo)
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn("was readable at base but not at HEAD", result.stdout)

    def test_a_parser_nobody_could_read_before_either_warns_loudly_and_does_not_fail(self):
        with tempfile.TemporaryDirectory() as repo:
            self.make_repo(repo, STATUSES, STATUSES)
            parser = os.path.join(repo, cce.RUST_PARSER)
            with open(parser, "w", encoding="utf-8", newline="\n") as handle:
                handle.write("fn queue_ids() {}\n")
            self.git(repo, "commit", "-qam", "unreadable shape")
            self.git(repo, "tag", "-f", "base")
            with open(os.path.join(repo, "governance", "COORDINATION_QUEUE.md"), "a", encoding="utf-8", newline="\n") as handle:
                handle.write(row("C3", "Ready") + "\n")
            self.git(repo, "commit", "-qam", "row")
            result = self.run_check(repo)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("was not compared with the native parser", result.stderr.replace("\n", " "))

    def test_deleting_the_vocabulary_file_fails_instead_of_switching_the_check_off(self):
        with tempfile.TemporaryDirectory() as repo:
            self.make_repo(repo, STATUSES, STATUSES)
            os.remove(os.path.join(repo, "governance", "QUEUE_STATUSES.json"))
            self.git(repo, "commit", "-qam", "delete vocabulary")
            result = self.run_check(repo)
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn("readable at base but not at HEAD", result.stdout)

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



class EndToEndWithNonAsciiRows(unittest.TestCase):
    """Run the real script against a throwaway git repository whose added row contains characters the Windows default
    code page cannot decode."""

    SCRIPT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "check_coordination_evidence.py")

    def git(self, repo, *args):
        subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid", "-C", repo, *args],
                       check=True, capture_output=True)

    def check_added_row(self, row):
        with tempfile.TemporaryDirectory() as repo:
            os.makedirs(os.path.join(repo, "governance"))
            queue = os.path.join(repo, "governance", "COORDINATION_QUEUE.md")
            with open(queue, "wb") as handle:
                handle.write(b"| C1 | s | Ready | o | b | t | e |\n")
            self.git(repo, "init", "-q")
            self.git(repo, "add", "-A")
            self.git(repo, "commit", "-qm", "base")
            self.git(repo, "tag", "base")
            with open(queue, "ab") as handle:
                handle.write(row.encode("utf-8") + b"\n")
            self.git(repo, "commit", "-qam", "row")
            return subprocess.run([sys.executable, self.SCRIPT, "base", "HEAD"], cwd=repo, capture_output=True,
                                  text=True, encoding="utf-8", errors="replace")

    def test_an_unevidenced_done_row_with_a_right_double_quote_still_fails(self):
        result = self.check_added_row("| C2 | scope \u201dquoted\u201d | Done | Claude | br | 2026-10-06 | all good |")
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertNotIn("UnicodeDecodeError", result.stderr)

    def test_an_evidenced_done_row_with_a_right_double_quote_passes(self):
        result = self.check_added_row("| C2 | scope \u201dquoted\u201d | Done | Claude | br | 2026-10-06 | exit code 0 |")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("passed", result.stdout)
        self.assertNotIn("UnicodeDecodeError", result.stderr)


if __name__ == "__main__":
    unittest.main()
