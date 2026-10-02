import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from worktree_inventory import classify, parse_worktrees


class Inventory(unittest.TestCase):
    def test_parse_two_blocks(self):
        text = "worktree /a\nHEAD abc\nbranch refs/heads/x\n\nworktree /b\nHEAD def\ndetached\n"
        rows = parse_worktrees(text)
        self.assertEqual([r["worktree"] for r in rows], ["/a", "/b"])
        self.assertEqual(rows[0]["branch"], "refs/heads/x")

    def test_dirty_is_never_a_candidate(self):
        self.assertTrue(classify(False, True, False).startswith("DIRTY"))
        self.assertTrue(classify(True, True, False).startswith("candidate"))
        self.assertTrue(classify(True, False, False).startswith("keep"))
        self.assertTrue(classify(True, True, True).startswith("primary"))


if __name__ == "__main__":
    unittest.main()
