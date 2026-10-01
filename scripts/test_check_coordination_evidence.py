import unittest

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


if __name__ == "__main__":
    unittest.main()
