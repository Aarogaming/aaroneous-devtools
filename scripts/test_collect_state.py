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

    def test_a_seat_named_branch_wins_over_coauthor_trailers(self):
        text = "x" + chr(10) + chr(10) + "Co-Authored-By: Claude Sonnet 5.5 <n@a.com>" + chr(10)
        self.assertEqual(cs.seat_from_trailers(text, "codex/foundry-transition"), "codex")

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


class Rejections(unittest.TestCase):
    def test_a_comment_is_attributed_to_the_seat_it_says_it_is_from(self):
        self.assertEqual(cs.comment_seat("### Review\n**Seat / Agent**: Antigravity (Gemini 2.5 Pro)\n"), "gemini")
        self.assertEqual(cs.comment_seat("Seat / agent: Mimo (mimo-v2.6-flash-free, opencode)"), "mimo")
        self.assertEqual(cs.comment_seat("Codex independent delta re-review at abc"), "codex")
        self.assertEqual(cs.comment_seat("no seat is named here"), "")

    def test_rejection_reasons_are_stated_in_words_an_author_can_act_on(self):
        self.assertIn("not valid JSON", cs.explain_problem("invalid JSON: Expecting value: line 1 column 1 (char 0)"))
        self.assertIn("missing fields", cs.explain_problem("missing fields: ['head']"))
        self.assertIn("different repository or PR", cs.explain_problem("attestation subject a/b#9 does not match c/d#8"))
        self.assertEqual(cs.explain_problem("something new"), "something new")

    def test_each_rejected_comment_is_attributed_and_explained(self):
        pr = {"head": "a" * 40, "comments": [
            {"id": 11, "body": "**Seat / Agent**: Antigravity (Gemini)\nhead aaaaaaaa\n```attestation\nverdict: ACCEPT\n```"},
            {"id": 12, "body": "Seat / agent: Mimo\nhead aaaaaaaa\nVERDICT: accept, whole diff"},
            {"id": 13, "body": "unrelated chatter"},
        ]}
        row = {"malformed": [{"comment": 0, "problem": "invalid JSON: Expecting value"}, {"review": 5, "problem": "x"}],
               "legacy": [{"comment": 1}]}
        found = cs.rejected_verdicts(pr, row)
        self.assertEqual([(f["comment_id"], f["seat"]) for f in found], [(11, "gemini"), (12, "mimo")])
        self.assertIn("not valid JSON", found[0]["reason"])
        self.assertEqual(found[1]["reason"], cs.LEGACY_REASON)

    def test_a_comment_that_does_not_cite_the_current_head_is_history_not_a_repair(self):
        pr = {"head": "a" * 40, "comments": [{"id": 1, "body": "Seat / agent: Mimo\nhead bbbbbbbb\nVERDICT: accept"}]}
        self.assertEqual(cs.rejected_verdicts(pr, {"legacy": [{"comment": 0}]}), [])

    def test_a_seat_name_further_down_a_comment_does_not_attribute_it(self):
        self.assertEqual(cs.comment_seat("My note.\nMimo reviewed this earlier.\n", strict=True), "")
        self.assertEqual(cs.comment_seat("Mimo reviewed this earlier.\n", strict=False), "mimo")
        self.assertEqual(cs.comment_seat("Gemini whole-diff packet\n", strict=True), "gemini")
        self.assertEqual(cs.comment_seat("intro\nSeat / agent: Codex\n", strict=True), "codex")

    def test_only_the_latest_rejections_per_seat_are_kept(self):
        comments = [{"id": i, "body": f"Seat / agent: Mimo\naaaaaaaa\nVERDICT {i}"} for i in range(5)]
        found = cs.rejected_verdicts({"head": "a" * 40, "comments": comments}, {"legacy": [{"comment": i} for i in range(5)]})
        self.assertEqual([f["comment_id"] for f in found], [3, 4])

    def test_an_out_of_range_comment_index_is_ignored_not_fatal(self):
        self.assertEqual(cs.rejected_verdicts({"comments": []}, {"malformed": [{"comment": 4, "problem": "x"}]}), [])

    def test_rejected_verdicts_reach_the_dispatcher_state(self):
        snapshot = {"prs": [{"number": 7, "head": "a" * 40, "base": "main",
                             "comments": [{"id": 5, "body": "Seat / agent: Gemini\naaaaaaaa\n```attestation\nx\n```"}]}]}
        meta = {7: {"title": "t", "isDraft": False, "mergeStateStatus": "CLEAN", "updatedAt": "2026-10-04T11:00:00Z"}}
        rows = [{"pr": 7, "covered": False, "malformed": [{"comment": 0, "problem": "invalid JSON: x"}]}]
        state = cs.build_state(snapshot, rows, meta, {7: 0}, {7: "claude"}, ["claude", "gemini"], [], "2026-10-04T12:00:00Z")
        self.assertEqual(state["prs"][0]["rejected_verdicts"][0]["seat"], "gemini")


class Build(unittest.TestCase):
    def test_unresolved_threads_are_counted_per_pr(self):
        graphql = {"data": {"repository": {"pullRequests": {"nodes": [
            {"number": 1, "reviewThreads": {"nodes": [{"isResolved": True}, {"isResolved": False}, {"isResolved": False}]}},
            {"number": 2, "reviewThreads": {"nodes": []}},
        ]}}}}
        self.assertEqual(cs.unresolved_by_pr(graphql), {1: 2, 2: 0})

    def test_every_implementer_reaches_the_dispatcher_state(self):
        snapshot = {"prs": [{"number": 7, "head": "a" * 40, "base": "main", "implementers": ["claude", "mimo"]}]}
        meta = {7: {"title": "t", "isDraft": False, "mergeStateStatus": "CLEAN", "updatedAt": "2026-10-04T11:00:00Z"}}
        state = cs.build_state(snapshot, [{"pr": 7, "covered": False}], meta, {7: 0}, {7: "claude"},
                               ["claude", "mimo", "gemini"], [], "2026-10-04T12:00:00Z")
        self.assertEqual(state["prs"][0]["implementers"], ["claude", "mimo"])

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
