import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import collect_state as cs  # noqa: E402
import next_task as nt  # noqa: E402


class Seats(unittest.TestCase):
    def test_coauthor_trailer_names_the_seat(self):
        text = "feat: x\n\nContribution-ID: C99/claude-v3\n\nCo-Authored-By: Claude Sonnet 5.5 <n@a.com>\n"
        self.assertEqual(cs.seat_from_trailers(text), "claude")

    def test_branch_prefix_is_the_fallback(self):
        self.assertEqual(cs.seat_from_trailers("no trailers", "codex/maintenance-progress-lane"), "codex")

    def test_unknown_seat_is_empty_not_a_guess(self):
        self.assertEqual(cs.seat_from_trailers("plain commit", "feature/thing"), "")


class AllSeats(unittest.TestCase):
    def test_every_seat_in_the_range_is_collected_and_deduplicated(self):
        text = "\n".join([
            "feat: a", "", "Co-Authored-By: Claude Sonnet 5.5 <n@a.com>",
            "---", "fix: b", "", "Co-Authored-By: Codex <c@a.com>", "Contribution-ID: C1/gemini-v1",
            "---", "fix: c", "", "Co-Authored-By: Claude Sonnet 5.5 <n@a.com>",
        ])
        self.assertEqual(cs.seats_from_trailers(text), ["claude", "codex", "gemini"])

    def test_the_branch_owner_comes_first_and_is_not_duplicated(self):
        text = "Co-Authored-By: Claude Sonnet 5.5 <n@a.com>\nCo-Authored-By: Codex <c@a.com>"
        self.assertEqual(cs.seats_from_trailers(text, "codex/foundry"), ["codex", "claude"])

    def test_primary_seat_is_the_first_of_the_list(self):
        self.assertEqual(cs.seat_from_trailers("Co-Authored-By: Mimo <m@a.com>", ""), "mimo")
        self.assertEqual(cs.seat_from_trailers("plain", "feature/x"), "")


class Build(unittest.TestCase):
    def test_unresolved_threads_are_counted_per_pr(self):
        graphql = {"data": {"repository": {"pullRequests": {"nodes": [
            {"number": 1, "reviewThreads": {"nodes": [{"isResolved": True}, {"isResolved": False}, {"isResolved": False}]}},
            {"number": 2, "reviewThreads": {"nodes": []}},
        ]}}}}
        self.assertEqual(cs.unresolved_by_pr(graphql), {1: 2, 2: 0})

    def test_state_feeds_the_dispatcher_end_to_end(self):
        snapshot = {"prs": [{"number": 7, "head": "a" * 40, "base": "main"}]}
        meta = {7: {"title": "t", "isDraft": False, "mergeStateStatus": "CLEAN", "updatedAt": "2026-10-04T11:00:00Z"}}
        state = cs.build_state(snapshot, [{"pr": 7, "covered": False}], meta, {7: 0}, {7: "claude"},
                               ["claude", "mimo"], [], "2026-10-04T12:00:00Z")
        self.assertEqual(state["default_branch"], "main")
        cards = nt.build_cards(state, 0)
        self.assertEqual({c["agent"] for c in cards}, {"claude", "mimo"})
        reviewer = next(c for c in cards if c["agent"] == "mimo")
        self.assertIn("independent whole-diff review of PR #7", reviewer["task"])
        implementer = next(c for c in cards if c["agent"] == "claude")
        self.assertNotIn("independent whole-diff review of PR #7", implementer["task"])


class Done(unittest.TestCase):
    def snap(self, body, head="a" * 40):
        return {"prs": [{"number": 9, "head": head, "comments": [{"body": body}]}]}

    def test_explicit_marker_marks_the_card_done(self):
        keys = cs.done_keys(self.snap("done [card:needs_review:9:aaaaaaaa]"))
        self.assertIn("needs_review:9:aaaaaaaa", keys)

    def test_merge_brief_naming_the_head_counts_as_done(self):
        keys = cs.done_keys(self.snap("OWNER MERGE BRIEF ... head aaaaaaaa..."))
        self.assertIn("ready_to_merge:9:aaaaaaaa", keys)

    def test_merge_brief_for_an_older_head_does_not_count(self):
        keys = cs.done_keys(self.snap("OWNER MERGE BRIEF ... head cccccccc...", head="b" * 40))
        self.assertEqual(keys, [])


if __name__ == "__main__":
    unittest.main()
