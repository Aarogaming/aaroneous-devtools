import os
import sys
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(__file__))
import queue_collisions as qc  # noqa: E402


def row(lane, status="Ready", scope="scope", cid=None):
    cid_part = f" Contribution-ID: {cid}." if cid else ""
    return f"| {lane} | {scope} | {status} | claude | branch | 2026-10-07T00:00:00Z | evidence.{cid_part} |"


def patch(*rows):
    return "@@ -1,2 +1,3 @@\n context\n" + "\n".join("+" + r for r in rows) + "\n-| C1 | removed | Done | x | y | z | w |\n"


class AddedRows(unittest.TestCase):
    def test_only_added_lane_rows_are_kept(self):
        text = patch(row("C100", "Blocked"), row("C101")) + "+not a row\n+| header | x |\n context | C5 | not added |\n"
        self.assertEqual(qc.added_rows(text), {"C100": [row("C100", "Blocked")], "C101": [row("C101")]})

    def test_lane_ids_with_a_letter_suffix_and_crlf_patches_are_read(self):
        text = "+" + row("C99b") + "\r\n+" + row("C99c") + "\r\n"
        self.assertEqual(list(qc.added_rows(text)), ["C99b", "C99c"])
        self.assertFalse(qc.added_rows(text)["C99b"][0].endswith("\r"))

    def test_empty_or_missing_patch_adds_nothing(self):
        self.assertEqual((qc.added_rows(""), qc.added_rows(None)), ({}, {}))

    def test_status_follows_the_third_column_and_escaped_pipes_do_not_shift_it(self):
        self.assertEqual(qc.status_of(row("C1", "Review")), "Review")
        escaped = "| C2 | a \\| b | Blocked | claude | br | t | e |"
        self.assertEqual(qc.status_of(escaped), "Blocked")
        self.assertEqual(qc.status_of("| C3 | only two |"), "?")

    def test_contribution_id_extracted_when_present(self):
        self.assertEqual(qc.contribution_id_of(row("C1", cid="C1/test-task-v1")), "C1/test-task-v1")
        self.assertIsNone(qc.contribution_id_of(row("C1")))


class Collisions(unittest.TestCase):
    def test_the_same_lane_with_different_content_is_a_collision_with_each_version_named(self):
        found = qc.collisions({
            50: {"C100": [row("C100", "Blocked")]},
            96: {"C100": [row("C100", "Blocked", scope="other wording")]},
            117: {"C100": [row("C100", "Review")]},
        })
        self.assertEqual([e["lane"] for e in found], ["C100"])
        self.assertEqual([(v["prs"], v["status"]) for v in found[0]["versions"]],
                         [([50], "Blocked"), ([96], "Blocked"), ([117], "Review")])

    def test_identical_rows_from_several_prs_are_one_version_not_a_collision(self):
        same = {"C104": [row("C104")]}
        self.assertEqual(qc.collisions({64: same, 66: same, 96: same}), [])

    def test_a_lane_in_only_one_pr_is_fine_and_lanes_sort_numerically(self):
        found = qc.collisions({
            1: {"C9": [row("C9", "A")], "C100": [row("C100", "A")], "C5": [row("C5")]},
            2: {"C9": [row("C9", "B")], "C100": [row("C100", "B")]},
        })
        self.assertEqual([e["lane"] for e in found], ["C9", "C100"])

    def test_versions_are_ordered_by_their_lowest_pr_whatever_order_the_prs_arrive_in(self):
        found = qc.collisions({117: {"C1": [row("C1", "B")]}, 50: {"C1": [row("C1", "A")]}})
        self.assertEqual([v["prs"] for v in found[0]["versions"]], [[50], [117]])

    def test_one_pr_adding_two_different_rows_for_a_lane_collides_with_itself(self):
        found = qc.collisions({7: {"C1": [row("C1", "Ready"), row("C1", "Review")]}})
        self.assertEqual(found[0]["versions"][0]["prs"], [7])
        self.assertEqual(len(found[0]["versions"]), 2)

    def test_two_versions_shared_by_several_prs_group_their_pr_numbers(self):
        found = qc.collisions({1: {"C1": [row("C1", "A")]}, 2: {"C1": [row("C1", "A")]}, 3: {"C1": [row("C1", "B")]}})
        self.assertEqual([v["prs"] for v in found[0]["versions"]], [[1, 2], [3]])


class NamespaceCollisions(unittest.TestCase):
    def test_two_unrelated_stacks_allocate_same_short_id_with_distinct_contribution_ids(self):
        # Codex test requirement: PR198 has C133/claude-model-hub-v1, PR204 has C133/codex-local-integration-authority-v1
        by_pr = {
            198: {"C133": [row("C133", scope="model hub", cid="C133/claude-model-hub-v1")]},
            204: {"C133": [row("C133", scope="local authority", cid="C133/codex-local-integration-authority-v1")]},
        }
        found = qc.namespace_collisions(by_pr)
        self.assertEqual(len(found), 1)
        self.assertEqual(found[0]["lane"], "C133")
        cids = [c["contribution_id"] for c in found[0]["claims"]]
        self.assertIn("C133/claude-model-hub-v1", cids)
        self.assertIn("C133/codex-local-integration-authority-v1", cids)

    def test_legitimate_child_updates_same_contribution(self):
        # Child branch on a stack updates status of the same Contribution-ID: NOT a collision
        by_pr = {
            10: {"C5": [row("C5", status="Ready", cid="C5/task-v1")]},
            11: {"C5": [row("C5", status="Review", cid="C5/task-v1")]},
        }
        found = qc.namespace_collisions(by_pr)
        self.assertEqual(found, [])

    def test_identical_content_in_multiple_branches(self):
        by_pr = {
            1: {"C5": [row("C5", cid="C5/task-v1")]},
            2: {"C5": [row("C5", cid="C5/task-v1")]},
        }
        self.assertEqual(qc.namespace_collisions(by_pr), [])

    def test_missing_identity_with_differing_rows_flags_collision(self):
        # Two PRs add differing rows for C5 without Contribution-ID: flagged for reconciliation
        by_pr = {
            1: {"C5": [row("C5", scope="scope A")]},
            2: {"C5": [row("C5", scope="scope B")]},
        }
        found = qc.namespace_collisions(by_pr)
        self.assertEqual(len(found), 1)
        self.assertEqual(found[0]["lane"], "C5")
        self.assertTrue(any(c["contribution_id"] == "(missing)" for c in found[0]["claims"]))


class DescriptionClaims(unittest.TestCase):
    def test_lane_of_contribution_id_needs_a_leading_short_id(self):
        self.assertEqual(qc.lane_of_contribution_id("C176/gemini-x-v1"), "C176")
        self.assertEqual(qc.lane_of_contribution_id("C99b/x"), "C99b")
        self.assertEqual(qc.lane_of_contribution_id("C5"), "C5")
        self.assertIsNone(qc.lane_of_contribution_id("declares"))
        self.assertIsNone(qc.lane_of_contribution_id("XC5/x"))
        self.assertIsNone(qc.lane_of_contribution_id(None))

    def test_declarations_are_read_from_descriptions_with_crlf_and_trailing_punctuation(self):
        bodies = {256: "Reconcile rows.\r\n\r\nContribution-ID: C176/codex-rust-ownership-reconciliation-v1. Review needed.",
                  300: "Contribution-ID: nothing useful here", 301: None}
        self.assertEqual(qc.description_claims(bodies), {"C176": {"C176/codex-rust-ownership-reconciliation-v1": [256]}})

    def test_the_issue_256_versus_250_case_is_caught_although_256_adds_no_row_for_the_lane(self):
        # Scout 2026-10-31: #250 adds a C176 row with a gemini ID; draft #256 only declares a codex C176 ID.
        by_pr = {250: {"C176": [row("C176", "Review", cid="C176/gemini-cascading-automata-reducer-v1")]},
                 256: {"C132": [row("C132", "Done")]}}
        bodies = {250: "Registers the lane.", 256: "Contribution-ID: C176/codex-rust-ownership-reconciliation-v1."}
        self.assertEqual(qc.namespace_collisions(by_pr), [])  # the row views cannot see it
        found = qc.description_collisions(by_pr, bodies)
        self.assertEqual([e["lane"] for e in found], ["C176"])
        self.assertEqual([(c["contribution_id"], c["prs"], c["sources"]) for c in found[0]["claims"]],
                         [("C176/gemini-cascading-automata-reducer-v1", [250], ["queue row"]),
                          ("C176/codex-rust-ownership-reconciliation-v1", [256], ["description"])])

    def test_the_same_id_in_a_row_and_a_description_or_across_a_stack_is_one_claim(self):
        by_pr = {10: {"C5": [row("C5", "Ready", cid="C5/task-v1")]}, 11: {"C5": [row("C5", "Review", cid="C5/task-v1")]}}
        bodies = {10: "Contribution-ID: C5/task-v1", 11: "Contribution-ID: C5/task-v1."}
        self.assertEqual(qc.description_collisions(by_pr, bodies), [])

    def test_two_different_ids_that_are_both_only_in_rows_are_left_to_the_row_views(self):
        by_pr = {1: {"C7": [row("C7", cid="C7/a-v1")]}, 2: {"C7": [row("C7", cid="C7/b-v1")]}}
        self.assertEqual(qc.description_collisions(by_pr, {1: "", 2: ""}), [])
        self.assertEqual(len(qc.namespace_collisions(by_pr)), 1)

    def test_two_descriptions_claiming_one_short_id_collide_and_lanes_sort_numerically(self):
        bodies = {1: "Contribution-ID: C100/a-v1", 2: "Contribution-ID: C100/b-v1",
                  3: "Contribution-ID: C9/a-v1", 4: "Contribution-ID: C9/b-v1"}
        found = qc.description_collisions({}, bodies)
        self.assertEqual([e["lane"] for e in found], ["C9", "C100"])
        self.assertEqual([c["sources"] for c in found[0]["claims"]], [["description"], ["description"]])

    def test_a_row_whose_contribution_id_names_another_lane_is_not_counted_for_this_lane(self):
        by_pr = {1: {"C8": [row("C8", cid="C9/other-v1")]}}
        self.assertEqual(qc.description_collisions(by_pr, {2: "Contribution-ID: C8/x-v1"}), [])

    def test_render_names_the_claims_and_says_so_when_there_is_nothing(self):
        found = qc.description_collisions({}, {1: "Contribution-ID: C3/a-v1", 2: "Contribution-ID: C3/b-v1"})
        rendered = qc.render([], [], [], found)
        self.assertIn("Description-declared claim collisions", rendered)
        self.assertIn("#1  Contribution-ID=C3/a-v1  from=description", rendered)
        self.assertIn("no description-declared claim collision", qc.render([], [], [], []))
        self.assertNotIn("description-declared", qc.render([], [], []))

    def test_declared_bodies_reads_open_pr_descriptions_and_tolerates_a_null_body(self):
        listing = [{"number": 5, "body": "Contribution-ID: C1/x-v1"}, {"number": 6, "body": None}, {"number": 7}]
        with mock.patch.object(qc.rc, "gh_json", return_value=listing) as gh:
            self.assertEqual(qc.declared_bodies("o/r"), {5: "Contribution-ID: C1/x-v1", 6: "", 7: ""})
        self.assertIn("number,body", gh.call_args.args)


class Live(unittest.TestCase):
    def run_live(self, listing, files):
        def fake(*args):
            if args[0] == "pr":
                return listing
            number = int(args[1].split("/")[4])
            return [files[number]]
        with mock.patch.object(qc.rc, "gh_json", side_effect=fake):
            return qc.live("o/r")

    def test_prs_that_do_not_touch_the_queue_are_ignored_and_a_missing_patch_is_reported_not_skipped(self):
        listing = [{"number": 1, "baseRefName": "main"}, {"number": 2, "baseRefName": "main"},
                   {"number": 3, "baseRefName": "main"}]
        files = {
            1: [{"filename": "scripts/x.py", "patch": "+| C1 | x |"}],
            2: [{"filename": qc.QUEUE}],
            3: [{"filename": qc.QUEUE, "patch": "+" + row("C7")}],
        }
        by_base, unreadable = self.run_live(listing, files)
        self.assertEqual(by_base, {"main": {3: {"C7": [row("C7")]}}})
        self.assertEqual(unreadable, [2])

    def test_prs_are_grouped_by_the_base_they_target(self):
        listing = [{"number": 1, "baseRefName": "main"}, {"number": 2, "baseRefName": "proposal/x"}]
        files = {1: [{"filename": qc.QUEUE, "patch": "+" + row("C1")}],
                 2: [{"filename": qc.QUEUE, "patch": "+" + row("C1", "Review")}]}
        by_base, _ = self.run_live(listing, files)
        self.assertEqual(sorted(by_base), ["main", "proposal/x"])

    def test_a_stacked_pr_that_updates_its_predecessors_lane_is_not_a_collision(self):
        # Codex P2 on #126: a PR into the predecessor's branch intentionally changes the row that predecessor added.
        by_base = {"main": {10: {"C5": [row("C5", "Ready")]}}, "proposal/pred": {11: {"C5": [row("C5", "Review")]}}}
        self.assertEqual(qc.collisions_by_base(by_base), [])
        by_base["main"][12] = {"C5": [row("C5", "Blocked")]}
        found = qc.collisions_by_base(by_base)
        self.assertEqual([(e["base"], e["lane"]) for e in found], [("main", "C5")])
        self.assertIn("PRs into main", qc.render(found, []))

    def test_render_names_unreadable_prs_and_says_so_when_there_is_nothing(self):
        self.assertIn("NOT CHECKED", qc.render([], [9]))
        self.assertIn("no lane row", qc.render([], []))

    def test_render_labels_same_base_and_namespace_distinctly(self):
        rendered = qc.render([], [], [])
        self.assertIn("direct same-base", rendered)
        self.assertIn("no cross-stack claim namespace collision", rendered)


if __name__ == "__main__":
    unittest.main()
