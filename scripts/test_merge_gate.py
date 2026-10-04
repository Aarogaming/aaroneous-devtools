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


def good_facts(**kw):
    facts = {
        "head": "a" * 40, "base_ref": "main", "base_tip": "b" * 40, "default_branch": "main",
        "ci": {"state": "success", "detail": "2 checks: SUCCESS"}, "trial_merge_ok": True,
        "accept_seats": ["mimo", "codex"], "covered": True, "threads_open": 0,
        "counted_verdicts": 2, "verdicts_missing_evidence": [], "revertible": True,
        "breaker_tripped": False, "trust": (5, 5), "category": "governance/*.md", "tier": ("B", ""),
    }
    facts.update(kw)
    return facts


class Checklist(unittest.TestCase):
    def test_all_boxes_green_means_would_merge(self):
        lines, ok, first = mg.checklist(good_facts())
        self.assertTrue(ok)
        self.assertIsNone(first)
        self.assertTrue(all(l.startswith("[x]") for l in lines), lines)

    def test_each_failing_condition_stops_the_loop_at_its_box(self):
        cases = {
            "1": {"ci": {"state": "failure", "detail": "x"}},
            "2": {"trial_merge_ok": False},
            "2a": {"base_ref": "codex/foundry-transition"},
            "3": {"accept_seats": ["mimo"]},
            "4": {"threads_open": 2},
            "5": {"verdicts_missing_evidence": [4]},
            "6": {"revertible": False},
            "7": {"breaker_tripped": True},
            "8": {"trust": (2, 5)},
        }
        for box_no, change in cases.items():
            lines, ok, first = mg.checklist(good_facts(**change))
            self.assertFalse(ok, box_no)
            self.assertEqual(first, box_no)

    def test_an_unevaluable_box_is_blocked_and_fails_the_gate(self):
        for change, text in (({"ci": None}, "CI"), ({"revertible": None}, "classification"),
                             ({"breaker_tripped": None}, "circuit"), ({"threads_open": None}, "thread")):
            lines, ok, first = mg.checklist(good_facts(**change))
            self.assertFalse(ok)
            self.assertTrue(any(l.startswith("[blocked:") for l in lines), lines)

    def test_pending_ci_is_not_success(self):
        self.assertEqual(mg.ci_state([{"status": "IN_PROGRESS", "conclusion": ""}])["state"], "pending")
        self.assertEqual(mg.ci_state([])["state"], "missing")
        self.assertEqual(mg.ci_state([{"status": "COMPLETED", "conclusion": "SUCCESS"}])["state"], "success")
        self.assertEqual(mg.ci_state([{"status": "COMPLETED", "conclusion": "SUCCESS"},
                                      {"status": "COMPLETED", "conclusion": "FAILURE"}])["state"], "failure")

    def test_tier_a_paths_are_owner_only(self):
        for path in (".github/workflows/ci.yml", "AGENTS.md", "governance/AUTOMATED_MERGE_POLICY.md"):
            self.assertEqual(mg.tier_of([path])[0], "A", path)
            lines, ok, first = mg.checklist(good_facts(tier=mg.tier_of([path])))
            self.assertFalse(ok)
            self.assertTrue(first.startswith("0"))
        self.assertEqual(mg.tier_of(["governance/PLATFORM_SPEC.md"])[0], "B")

    def test_two_seats_must_also_be_covered(self):
        lines, ok, first = mg.checklist(good_facts(covered=False))
        self.assertFalse(ok)
        self.assertEqual(first, "3")

    def test_standing_changes_requested_fails_the_threads_box(self):
        lines, ok, first = mg.checklist(good_facts(standing_changes_requested=True))
        self.assertEqual(first, "4")


if __name__ == "__main__":
    unittest.main()
