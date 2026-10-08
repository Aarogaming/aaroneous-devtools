import os
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


ROW_DONE = "| C1 | s | Done | o | b | t | exit code 0 |"
ROW_CLAIMED = "| C2 | s | Claimed | o | b | t | e |"
ROW_READY = "| C3 | s | Ready | o | b | t | e |"
BASE_QUEUE = "\n".join(["| ID | Scope | Status |", "| --- | --- | --- |", ROW_DONE, ROW_CLAIMED, ROW_READY]) + "\n"


class LaneRowFindings(unittest.TestCase):
    def findings(self, head_text):
        return cce.lane_row_findings(BASE_QUEUE, head_text)

    def test_an_unchanged_queue_has_no_findings(self):
        self.assertEqual(self.findings(BASE_QUEUE), ([], []))

    def test_a_removed_live_lane_fails_and_a_removed_done_lane_only_warns(self):
        failures, warnings = self.findings(BASE_QUEUE.replace(ROW_CLAIMED + "\n", ""))
        self.assertEqual((failures, warnings), (["C2: row removed (it was Claimed)"], []))
        failures, warnings = self.findings(BASE_QUEUE.replace(ROW_DONE + "\n", ""))
        self.assertEqual((failures, warnings), ([], ["C1: row removed (it was Done)"]))

    def test_a_second_row_with_an_existing_lane_id_fails(self):
        failures, _ = self.findings(BASE_QUEUE + ROW_CLAIMED + "\n")
        self.assertEqual(failures, ["C2: lane ID appears more than once"])

    def test_a_duplicate_that_already_existed_at_the_base_is_not_blamed_on_this_change(self):
        dirty = BASE_QUEUE + ROW_READY + "\n"
        self.assertEqual(cce.lane_row_findings(dirty, dirty), ([], []))

    def test_a_done_lane_cannot_change_status_but_the_normal_order_is_allowed(self):
        failures, _ = self.findings(BASE_QUEUE.replace("| C1 | s | Done |", "| C1 | s | Ready |"))
        self.assertEqual(failures, ["C1: Done row changed to Ready"])
        progressed = BASE_QUEUE.replace("| C2 | s | Claimed |", "| C2 | s | Review |").replace(
            "| C3 | s | Ready |", "| C3 | s | Claimed |")
        self.assertEqual(self.findings(progressed), ([], []))

    def test_new_lanes_and_edited_text_are_fine(self):
        grown = BASE_QUEUE + "| C4 | new | Ready | o | b | t | e |\n"
        self.assertEqual(self.findings(grown), ([], []))
        self.assertEqual(self.findings(BASE_QUEUE.replace("| C2 | s |", "| C2 | longer scope |")), ([], []))

    def test_an_escaped_pipe_in_a_cell_does_not_change_the_status(self):
        row = "| C89 | profile = `kernel\\|control\\|tooling` | Ready | Unowned |"
        self.assertEqual(cce.lane_rows(row), ({"C89": "Ready"}, []))
        self.assertEqual(cce.split_cells("| a\\\\| b |"), ["", " a\\\\", " b ", ""])

    def test_an_unreadable_status_is_reported_not_crashed_on(self):
        failures, _ = self.findings(BASE_QUEUE.replace(ROW_CLAIMED + "\n", "| C2 |\n"))
        self.assertEqual(failures, [])
        failures, _ = cce.lane_row_findings(BASE_QUEUE, BASE_QUEUE.replace(ROW_CLAIMED + "\n", ""))
        self.assertTrue(failures)


class EndToEndLaneLoss(unittest.TestCase):
    """The real script against a throwaway repository: the three changes that used to exit 0."""

    SCRIPT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "check_coordination_evidence.py")

    def git(self, repo, *args):
        subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid", "-C", repo, *args],
                       check=True, capture_output=True)

    def run_change(self, head_text):
        with tempfile.TemporaryDirectory() as repo:
            os.makedirs(os.path.join(repo, "governance"))
            queue = os.path.join(repo, "governance", "COORDINATION_QUEUE.md")
            with open(queue, "w", encoding="utf-8", newline="\n") as handle:
                handle.write(BASE_QUEUE)
            self.git(repo, "init", "-q")
            self.git(repo, "add", "-A")
            self.git(repo, "commit", "-qm", "base")
            self.git(repo, "tag", "base")
            with open(queue, "w", encoding="utf-8", newline="\n") as handle:
                handle.write(head_text)
            self.git(repo, "commit", "-qam", "change")
            return subprocess.run([sys.executable, self.SCRIPT, "base", "HEAD"], cwd=repo, capture_output=True,
                                  text=True, encoding="utf-8", errors="replace")

    def test_deleting_a_claimed_lane_fails(self):
        result = self.run_change(BASE_QUEUE.replace(ROW_CLAIMED + "\n", ""))
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("C2: row removed (it was Claimed)", result.stdout)

    def test_a_duplicate_lane_id_fails(self):
        result = self.run_change(BASE_QUEUE + ROW_CLAIMED + "\n")
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("C2: lane ID appears more than once", result.stdout)

    def test_regressing_a_done_lane_fails(self):
        result = self.run_change(BASE_QUEUE.replace("| C1 | s | Done |", "| C1 | s | Ready |"))
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)

    def test_deleting_a_done_lane_passes_with_a_warning(self):
        result = self.run_change(BASE_QUEUE.replace(ROW_DONE + "\n", ""))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("C1: row removed (it was Done)", result.stderr)

    def test_a_normal_progression_passes(self):
        result = self.run_change(BASE_QUEUE.replace("| C3 | s | Ready |", "| C3 | s | Claimed |"))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


PROGRAM_HEAD = "| Priority | ID | Bounded work / source | Dependency | Acceptance |\n| --- | --- | --- | --- | --- |\n"


def program_text(*rows):
    return PROGRAM_HEAD + "".join(f"| {tier} | {ident} | work | dep | acceptance |\n" for tier, ident in rows)


class IdOverlapFindings(unittest.TestCase):
    def findings(self, base_ids, head_ids, base_program=(), head_program=()):
        def queue(ids):
            return "\n".join(f"| {i} | s | Ready | o | b | t | e |" for i in ids) + "\n"
        return cce.id_overlap_findings(queue(base_ids), queue(head_ids), program_text(*base_program),
                                       program_text(*head_program))

    def test_program_ids_read_the_tiered_rows_only(self):
        text = program_text(("P1", "M5"), ("Done", "M2"), ("Retired", "M9")) + "| Capacity | Use |\n| --- | --- |\n"
        self.assertEqual(cce.program_ids(text), {"M5", "M2", "M9"})

    def test_an_id_added_to_the_worklist_that_the_queue_owns_fails(self):
        failures, _ = self.findings(["C7"], ["C7"], [], [("P2", "C7")])
        self.assertEqual(len(failures), 1)
        self.assertIn("C7: the ID is now in both", failures[0])

    def test_an_id_added_to_the_queue_that_the_worklist_holds_fails(self):
        failures, _ = self.findings([], ["C8"], [("P1", "C8")], [("P1", "C8")])
        self.assertEqual(len(failures), 1)

    def test_an_overlap_that_was_already_there_only_warns(self):
        failures, warnings = self.findings(["C106"], ["C106"], [("P2", "C106")], [("P2", "C106")])
        self.assertEqual(failures, [])
        self.assertEqual(len(warnings), 1)

    def test_distinct_ids_and_a_move_between_the_files_are_fine(self):
        self.assertEqual(self.findings(["C1"], ["C1"], [("P1", "M1")], [("P1", "M1")]), ([], []))
        # moving a row: out of the queue and into the worklist in one change
        self.assertEqual(self.findings(["C9"], [], [], [("P1", "C9")]), ([], []))


class EndToEndIdOverlap(unittest.TestCase):
    SCRIPT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "check_coordination_evidence.py")

    def run_change(self, head_queue, head_program):
        with tempfile.TemporaryDirectory() as repo:
            os.makedirs(os.path.join(repo, "governance"))
            queue = os.path.join(repo, "governance", "COORDINATION_QUEUE.md")
            program = os.path.join(repo, "governance", "MULTITHREAD_PROGRAM.md")

            def write(path, text):
                with open(path, "w", encoding="utf-8", newline="\n") as handle:
                    handle.write(text)

            def git(*args):
                subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid", "-C", repo, *args],
                               check=True, capture_output=True)

            write(queue, BASE_QUEUE)
            write(program, program_text(("P1", "M1")))
            git("init", "-q")
            git("add", "-A")
            git("commit", "-qm", "base")
            git("tag", "base")
            write(queue, head_queue)
            write(program, head_program)
            git("commit", "-qam", "change")
            return subprocess.run([sys.executable, self.SCRIPT, "base", "HEAD"], cwd=repo, capture_output=True, text=True,
                                  encoding="utf-8", errors="replace")

    def test_a_worklist_only_change_that_duplicates_a_queue_id_fails(self):
        # the queue file is untouched, which used to end the check with "nothing to check"
        result = self.run_change(BASE_QUEUE, program_text(("P1", "M1"), ("P2", "C2")))
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("C2: the ID is now in both", result.stdout)

    def test_a_change_with_no_overlap_passes(self):
        result = self.run_change(BASE_QUEUE, program_text(("P1", "M1"), ("P2", "M2")))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


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
