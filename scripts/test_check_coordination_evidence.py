import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

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
