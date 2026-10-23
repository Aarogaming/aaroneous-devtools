import os
import sys
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(__file__))
import queue_collisions as qc  # noqa: E402


def row(lane, status="Ready", scope="scope"):
    return f"| {lane} | {scope} | {status} | claude | branch | 2026-10-07T00:00:00Z | evidence |"


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


if __name__ == "__main__":
    unittest.main()
