import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import merge_gate as mg  # noqa: E402

COVERED = {"covered": True}
UNCOVERED = {"covered": False}


class Plan(unittest.TestCase):
    def test_docs_only_changes_get_the_light_gate(self):
        self.assertEqual(mg.gate_plan(["governance/PLATFORM_SPEC.md", "governance/identities.json"])["level"], "light")

    def test_anything_else_gets_the_full_gate(self):
        for paths in (["foundation/crates/foundation_cli/src/main.rs"], ["Cargo.lock"], ["scripts/x.py"],
                      ["governance/a.md", "build.rs"], []):
            self.assertEqual(mg.gate_plan(paths)["level"], "full", paths)

    def test_unlisted_doc_directories_are_not_trusted(self):
        self.assertEqual(mg.gate_plan(["random/notes.md"])["level"], "full")


class Decision(unittest.TestCase):
    def test_covered_clean_main_pr_is_eligible(self):
        d = mg.decide(COVERED, ["governance/x.md"], "main", True)
        self.assertTrue(d["eligible"])
        self.assertEqual(d["reasons"], [])

    def test_uncovered_pr_is_blocked_with_the_condition_named(self):
        d = mg.decide(UNCOVERED, ["governance/x.md"], "main", True)
        self.assertFalse(d["eligible"])
        self.assertTrue(d["reasons"][0].startswith("a:"))

    def test_trial_merge_conflict_blocks(self):
        d = mg.decide(COVERED, ["governance/x.md"], "main", False)
        self.assertTrue(any(r.startswith("b:") for r in d["reasons"]))

    def test_non_main_base_blocks(self):
        d = mg.decide(COVERED, ["governance/x.md"], "codex/foundry-transition", True)
        self.assertTrue(any(r.startswith("d:") for r in d["reasons"]))

    def test_protected_paths_need_owner_confirmation(self):
        for path in ("AGENTS.md", ".github/workflows/ci.yml"):
            d = mg.decide(COVERED, [path], "main", True)
            self.assertFalse(d["eligible"], path)
            self.assertTrue(any(r.startswith("f:") for r in d["reasons"]))
        self.assertTrue(mg.decide(COVERED, ["AGENTS.md"], "main", True, owner_confirmed_protected=True)["eligible"])

    def test_every_blocking_reason_is_reported_together(self):
        d = mg.decide(UNCOVERED, ["AGENTS.md"], "codex/x", False)
        self.assertEqual(sorted(r[0] for r in d["reasons"]), ["a", "b", "d", "f"])


if __name__ == "__main__":
    unittest.main()
