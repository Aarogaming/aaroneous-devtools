import os
import subprocess
import types
import sys
import tempfile
import unittest
from unittest import mock

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


class DoneReceipts(unittest.TestCase):
    def test_done_receipts_names_the_seat_that_completed_each_card_and_done_keys_is_unchanged(self):
        head = "c" * 40
        body = f"[done:fix_threads:7:{head}] Claude seat: all threads answered"
        snapshot = {"prs": [{"number": 7, "head": head, "comments": [{"id": 1, "body": body}]}]}
        self.assertEqual(cs.done_receipts(snapshot, {}, {7: ["claude"]}), {f"fix_threads:7:{head[:8]}": "claude"})
        self.assertEqual(cs.done_keys(snapshot, {}, {7: ["claude"]}), [f"fix_threads:7:{head[:8]}"])
        self.assertEqual(cs.done_receipts(snapshot, {}, {7: ["codex"]}), {})


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

    def test_a_different_head_sharing_the_first_eight_characters_is_not_this_head(self):
        head = "a" * 8 + "1" * 32
        other = "a" * 8 + "2" * 32
        self.assertTrue(cs.cites_head("see " + head, head))
        self.assertTrue(cs.cites_head("short form " + head[:8], head))
        self.assertTrue(cs.cites_head("sha " + head[:12] + " reviewed", head))
        self.assertFalse(cs.cites_head("reviewed " + other, head))
        self.assertFalse(cs.cites_head("no sha here", head))
        self.assertFalse(cs.cites_head("see " + head, ""))

    def test_a_comment_citing_a_colliding_head_is_not_a_repair_for_this_head(self):
        head = "a" * 8 + "1" * 32
        other = "a" * 8 + "2" * 32
        pr = {"head": head, "comments": [{"id": 1, "body": "Seat / agent: Mimo\nhead " + other + "\nVERDICT: accept"}]}
        self.assertEqual(cs.rejected_verdicts(pr, {"legacy": [{"comment": 0}]}), [])
        pr["comments"][0]["body"] = "Seat / agent: Mimo\nhead " + head + "\nVERDICT: accept"
        self.assertEqual(len(cs.rejected_verdicts(pr, {"legacy": [{"comment": 0}]})), 1)

    def test_a_mistyped_head_that_names_no_commit_is_reported_back_to_its_author(self):
        head = "ce3a58c6f8324020e942ecddc86dd7c2dfff87b0"
        typo = "ce3a58cf0b4458f7cfd36a83a0ee1cf7bcba4c10"
        body = "**Reviewer / Seat**: Antigravity\n**Head Reviewed**: `" + typo + "`\n```attestation\n{\"version\": 1}\n```\n"
        pr = {"head": head, "comments": [{"id": 9, "body": body}]}
        row = {"malformed": [{"comment": 0, "problem": "missing fields: ['head']"}]}
        self.assertEqual(cs.rejected_verdicts(pr, row), [])  # no existence check supplied: unchanged behaviour
        found = cs.rejected_verdicts(pr, row, commit_exists=lambda token: False)
        self.assertEqual([(f["comment_id"], f["seat"]) for f in found], [(9, "gemini")])
        self.assertIn(typo, found[0]["reason"])
        self.assertIn(head, found[0]["reason"])
        self.assertIn("suspected head mismatch, not proof", found[0]["reason"])
        self.assertNotIn("not a commit in this repository", found[0]["reason"])

    def test_a_failed_lookup_is_not_an_accusation(self):
        head = "ce3a58c6f8324020e942ecddc86dd7c2dfff87b0"
        typo = "ce3a58cf0b4458f7cfd36a83a0ee1cf7bcba4c10"
        pr = {"head": head, "comments": [{"id": 9, "body": "Seat / agent: Gemini\nhead " + typo}]}
        row = {"legacy": [{"comment": 0}]}
        self.assertEqual(cs.rejected_verdicts(pr, row, commit_exists=lambda token: None), [])
        self.assertEqual(len(cs.rejected_verdicts(pr, row, commit_exists=lambda token: False)), 1)

    def test_the_git_lookup_separates_present_absent_and_failed(self):
        with tempfile.TemporaryDirectory() as repo:
            for args in (["init", "-q"], ["-c", "user.name=t", "-c", "user.email=t@example.invalid", "commit", "-q",
                                          "--allow-empty", "-m", "a"]):
                subprocess.run(["git", "-C", repo, *args], check=True, capture_output=True)
            present = subprocess.run(["git", "-C", repo, "rev-parse", "HEAD"], check=True, capture_output=True,
                                     text=True).stdout.strip()
            exists = cs.git_commit_exists(repo)
            self.assertIs(exists(present), True)
            self.assertIs(exists("0" * 39 + "1"), False)
            self.assertIsNone(cs.git_commit_exists(os.path.join(repo, "no", "such", "dir"))(present))
            with mock.patch.object(cs.rc, "git", side_effect=FileNotFoundError("git")):
                self.assertIsNone(cs.git_commit_exists(repo)(present))

    def test_a_near_miss_that_is_a_real_commit_or_a_different_prefix_stays_history(self):
        head = "ce3a58c6f8324020e942ecddc86dd7c2dfff87b0"
        real_other = "ce3a58cf0b4458f7cfd36a83a0ee1cf7bcba4c10"
        unrelated = "1234567890abcdef1234567890abcdef12345678"
        row = {"legacy": [{"comment": 0}]}
        pr = {"head": head, "comments": [{"id": 1, "body": "Seat / agent: Mimo\nhead " + real_other + "\nVERDICT: accept"}]}
        self.assertEqual(cs.rejected_verdicts(pr, row, commit_exists=lambda token: True), [])
        pr = {"head": head, "comments": [{"id": 2, "body": "Seat / agent: Mimo\nhead " + unrelated + "\nVERDICT: accept"}]}
        self.assertEqual(cs.rejected_verdicts(pr, row, commit_exists=lambda token: False), [])

    def test_a_seat_name_right_after_a_completion_receipt_attributes_the_comment(self):
        head = "a" * 40
        receipt = f"[done:fix_threads:62:{head}]"
        for tail, seat in ((" Claude Sonnet 5.5 \u2014 all threads answered", "claude"), (" Claude seat: done", "claude"),
                           (": Mimo: done", "mimo"), (" Seat / agent: Codex", "codex"),
                           (" thanks Mimo for the review", ""), (" all done, Claude", ""), ("", "")):
            self.assertEqual(cs.comment_seat(receipt + tail, strict=True), seat, tail)

    def test_a_receipt_with_the_seat_after_it_completes_the_card_only_for_an_eligible_seat(self):
        head = "a" * 40

        def done(seat_text, implementers):
            snap = {"prs": [{"number": 62, "head": head,
                             "comments": [{"id": 1, "body": f"[done:fix_threads:62:{head}] {seat_text} \u2014 all threads answered"}]}]}
            return cs.done_keys(snap, {}, {62: implementers})

        self.assertEqual(done("Claude Sonnet 5.5", ["claude"]), ["fix_threads:62:aaaaaaaa"])
        self.assertEqual(done("Mimo", ["claude"]), [])  # not the author seat
        self.assertEqual(done("Claude Sonnet 5.5", []), [])  # unknown implementers fail closed

    def test_the_titles_the_seats_give_their_reviews_attribute_the_comment(self):
        for body, seat in (("Independent Codex verdict: ACCEPT this attribution delta only", "codex"),
                           ("### Codex bounded independent review: verification incomplete", "codex"),
                           ("## Gemini whole-diff packet", "gemini"),
                           ("Independent Mimo review at the current head", "mimo"),
                           ("independent claude re-check", "claude")):
            self.assertEqual(cs.comment_seat(body, strict=True), seat, body)

    def test_a_title_that_only_mentions_a_seat_does_not_attribute_the_comment(self):
        for body in ("Independent review by Codex", "Independently, Codex said it passes", "Review of Codex's change",
                     "### Independent review", "Independent verdict: ACCEPT"):
            self.assertEqual(cs.comment_seat(body, strict=True), "", body)

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

    def graphql(self, prs, more_prs=False):
        return {"data": {"repository": {"pullRequests": {"pageInfo": {"hasNextPage": more_prs}, "nodes": prs}}}}

    def pr_threads(self, number, resolved, unresolved, more):
        nodes = [{"isResolved": True}] * resolved + [{"isResolved": False}] * unresolved
        return {"number": number, "reviewThreads": {"pageInfo": {"hasNextPage": more}, "nodes": nodes}}

    def test_a_pr_with_a_further_page_of_threads_is_never_reported_as_having_none(self):
        full_page_all_resolved = self.pr_threads(1, 100, 0, True)
        self.assertEqual(cs.unresolved_by_pr(self.graphql([full_page_all_resolved])), {1: 1})
        self.assertEqual(cs.unresolved_by_pr(self.graphql([self.pr_threads(2, 97, 3, True)])), {2: 4})

    def test_a_complete_page_is_counted_exactly(self):
        self.assertEqual(cs.unresolved_by_pr(self.graphql([self.pr_threads(1, 99, 1, False), self.pr_threads(2, 0, 0, False)])),
                         {1: 1, 2: 0})

    def test_more_than_a_hundred_open_prs_stops_instead_of_dropping_the_rest(self):
        with self.assertRaises(SystemExit) as caught:
            cs.unresolved_by_pr(self.graphql([self.pr_threads(1, 0, 0, False)], more_prs=True))
        self.assertIn("100 open pull requests", str(caught.exception))

    AUTHOR_REPLY = "Claude Sonnet 5.5 (author). Valid, fixed at abc1234: the format is now quoted."
    REVIEWER_FOLLOW_UP = "Reviewer / Seat: Codex. Still missing: the case with an empty list is not covered."

    def thread(self, resolved, total, last_login, body=None):
        node = {"author": {"login": last_login}}
        if body is not None:
            node["body"] = body
        comments = {"totalCount": total, "nodes": [node]} if last_login is not None else {}
        return {"isResolved": resolved, "comments": comments}

    def test_a_thread_is_answered_only_when_the_pr_sides_seat_has_the_last_word(self):
        author = ("claude",)
        self.assertTrue(cs.thread_unanswered(self.thread(False, 1, "Aarogaming", self.AUTHOR_REPLY), author))  # only the opener
        self.assertTrue(cs.thread_unanswered(self.thread(False, 3, "chatgpt-codex-connector[bot]", "x"), author))  # bot last
        self.assertFalse(cs.thread_unanswered(self.thread(False, 2, "Aarogaming", self.AUTHOR_REPLY), author))  # author answered
        self.assertTrue(cs.thread_unanswered(self.thread(False, 3, "Aarogaming", self.REVIEWER_FOLLOW_UP), author))  # reviewer last
        self.assertTrue(cs.thread_unanswered(self.thread(False, 2, "Aarogaming", "Still not fixed, see line 12"), author))  # no seat
        self.assertTrue(cs.thread_unanswered(self.thread(False, 2, "Aarogaming", self.AUTHOR_REPLY), ()))  # implementers unknown
        self.assertTrue(cs.thread_unanswered({"isResolved": False}, author))  # unknown comment data: the old behaviour
        self.assertTrue(cs.thread_unanswered(self.thread(False, 2, "", "x"), author))

    def test_a_co_implementer_answer_also_counts_as_the_pr_sides(self):
        reply = "Mimo seat: fixed in abc1234."
        self.assertFalse(cs.thread_unanswered(self.thread(False, 2, "Aarogaming", reply), ("claude", "mimo")))
        self.assertTrue(cs.thread_unanswered(self.thread(False, 2, "Aarogaming", reply), ("claude",)))

    def test_unanswered_counts_only_unresolved_threads_the_pr_side_has_not_answered(self):
        threads = [self.thread(True, 2, "Aarogaming", self.AUTHOR_REPLY), self.thread(False, 2, "Aarogaming", self.AUTHOR_REPLY),
                   self.thread(False, 1, "x[bot]", "p"), self.thread(False, 3, "Aarogaming", self.REVIEWER_FOLLOW_UP)]
        graphql = self.graphql([{"number": 1, "reviewThreads": {"pageInfo": {"hasNextPage": False}, "nodes": threads}}])
        self.assertEqual(cs.unresolved_by_pr(graphql), {1: 3})
        self.assertEqual(cs.unanswered_by_pr(graphql, {1: ["claude"]}), {1: 2})
        self.assertEqual(cs.unanswered_by_pr(graphql), {1: 3})  # without implementers nothing is answered

    def test_a_further_page_of_threads_counts_one_extra_unanswered_thread(self):
        graphql = self.graphql([{"number": 1, "reviewThreads": {"pageInfo": {"hasNextPage": True},
                                                                  "nodes": [self.thread(False, 2, "Aarogaming", self.AUTHOR_REPLY)]}}])
        self.assertEqual(cs.unanswered_by_pr(graphql, {1: ["claude"]}), {1: 1})

    def test_the_state_carries_the_unanswered_count_and_defaults_to_the_old_behaviour(self):
        snapshot = {"prs": [{"number": 7, "head": "a" * 40, "base": "main", "implementers": ["claude"]}]}
        meta = {7: {"title": "t", "isDraft": False, "mergeStateStatus": "CLEAN", "updatedAt": "2026-10-04T11:00:00Z"}}
        args = (snapshot, [{"pr": 7, "covered": False}], meta, {7: 3}, {7: "claude"}, ["claude"], [], "2026-10-04T12:00:00Z")
        self.assertEqual(cs.build_state(*args)["prs"][0]["threads_unanswered"], 3)
        self.assertEqual(cs.build_state(*args, unanswered={7: 0})["prs"][0]["threads_unanswered"], 0)

    def test_the_standing_rejecting_verdict_is_read_from_the_coverage_reasons(self):
        self.assertEqual(cs.standing_blocker(["standing needs_revision at head by Claude", "no accept at the current head"]),
                         {"verdict": "needs_revision", "by": "Claude"})
        self.assertEqual(cs.standing_blocker(["standing changes_requested at head by Mimo"]),
                         {"verdict": "changes_requested", "by": "Mimo"})
        for none in ([], None, ["no accept at the current head"], ["accept at head exists but is not chained to a whole accept"]):
            self.assertIsNone(cs.standing_blocker(none))

    def test_the_state_carries_the_standing_verdict(self):
        snapshot = {"prs": [{"number": 7, "head": "a" * 40, "base": "main", "implementers": ["codex"]}]}
        meta = {7: {"title": "t", "isDraft": False, "mergeStateStatus": "CLEAN", "updatedAt": "2026-10-04T11:00:00Z"}}
        rows = [{"pr": 7, "covered": False, "reasons": ["standing needs_revision at head by Claude"]}]
        state = cs.build_state(snapshot, rows, meta, {7: 0}, {7: "codex"}, ["codex"], [], "2026-10-04T12:00:00Z")
        self.assertEqual(state["prs"][0]["standing_blocker"], {"verdict": "needs_revision", "by": "Claude"})

    def test_only_the_author_completes_an_address_review_card(self):
        self.assertTrue(cs.eligible_completer("address_review", "codex", ["codex"]))
        self.assertFalse(cs.eligible_completer("address_review", "claude", ["codex"]))
        self.assertFalse(cs.eligible_completer("address_review", "codex", []))

    def collect_with_an_unsigned_accept(self, **kwargs):
        # collect() against a stubbed GitHub: one PR whose only accept is an unsigned attestation block.
        import json
        from unittest import mock
        head, base = "a" * 40, "d" * 40
        record = {"repo": "o/r", "pr": 1, "head": head, "base_sha": base, "scope": "whole", "reviewer": "gemini",
                  "implementers": ["claude"], "verdict": "accept"}
        snapshot = {"repo": "o/r", "prs": [{"number": 1, "head": head, "base": "main", "merge_base": base,
                                            "reviews": [], "base_events": [],
                                            "comments": [{"id": 5, "body": "```attestation\n" + json.dumps(record) + "\n```"}]}]}
        meta = [{"number": 1, "title": "t", "isDraft": False, "mergeStateStatus": "CLEAN", "updatedAt": "2026-10-06T00:00:00Z",
                 "headRefName": "claude/x", "additions": 1, "deletions": 0, "changedFiles": 1}]
        graphql = {"data": {"repository": {"pullRequests": {"pageInfo": {"hasNextPage": False}, "nodes": [
            {"number": 1, "reviewThreads": {"pageInfo": {"hasNextPage": False}, "nodes": []}}]}}}}

        def gh_json(*args):
            if args[0] == "pr":
                return meta
            if args[0] == "api":
                return graphql
            return {"defaultBranchRef": {"name": "main"}}

        done = types.SimpleNamespace(stdout="", returncode=0)
        with mock.patch.object(cs.rc, "live_snapshot", return_value=snapshot), \
                mock.patch.object(cs.rc, "gh_json", side_effect=gh_json), \
                mock.patch.object(cs.rc, "git", return_value=done), \
                mock.patch.object(cs.rc, "live_ancestry", return_value=lambda a, b: True):
            return cs.collect("o/r", ".", ["claude", "gemini"], [], {"gemini": "gemini"}, **kwargs)

    def test_a_snapshot_the_caller_already_took_is_not_read_again(self):
        # The steward took one snapshot for ingest and collect() took a second: two passes of one gh call per open PR.
        from unittest import mock
        state = self.collect_with_an_unsigned_accept(keys=None, require_signatures=False)
        self.assertEqual(len(state["prs"]), 1)
        sentinel = {"repo": "o/r", "prs": []}
        with mock.patch.object(cs.rc, "live_snapshot", side_effect=AssertionError("read the snapshot again")),                 mock.patch.object(cs.rc, "gh_json", side_effect=lambda *args: [] if args[0] == "pr" else
                                  {"defaultBranchRef": {"name": "main"}}),                 mock.patch.object(cs, "unresolved_by_pr", return_value={}),                 mock.patch.object(cs, "unanswered_by_pr", return_value={}):
            self.assertEqual(cs.collect("o/r", ".", ["claude"], [], snapshot=sentinel)["prs"], [])

    def test_signature_enforcement_reaches_the_coverage_decision(self):
        keys = {"seats": {"gemini": {"keys": ["ssh-ed25519 AAAA test"]}}}
        unenforced = self.collect_with_an_unsigned_accept(keys=keys, require_signatures=False)
        self.assertTrue(unenforced["prs"][0]["covered"])
        enforced = self.collect_with_an_unsigned_accept(keys=keys, require_signatures=True)
        self.assertFalse(enforced["prs"][0]["covered"], "an unsigned accept must stop counting once enforcement is on")

    def test_enforcement_without_registered_keys_fails_closed(self):
        for keys in (None, {}, {"seats": {}}):
            with self.assertRaises(SystemExit):
                self.collect_with_an_unsigned_accept(keys=keys, require_signatures=True)

    def test_receipts_for_cards_that_complete_on_github_are_not_read(self):
        head = "c" * 40
        for kind, seat in (("resolve_threads", "Gemini"), ("address_review", "Codex"), ("identify_implementers", "Claude"),
                           ("not_mergeable", "Gemini")):
            body = f"[done:{kind}:7:{head}] {seat} seat: done"
            snapshot = {"prs": [{"number": 7, "head": head, "comments": [{"id": 1, "body": body}]}]}
            self.assertEqual(cs.done_receipts(snapshot, {}, {7: ["codex" if kind == "resolve_threads" else "codex"]}), {}, kind)
        body = f"[done:fix_threads:7:{head}] Claude seat: done"
        snapshot = {"prs": [{"number": 7, "head": head, "comments": [{"id": 1, "body": body}]}]}
        self.assertEqual(cs.done_receipts(snapshot, {}, {7: ["claude"]}), {f"fix_threads:7:{head[:8]}": "claude"})

    def test_only_a_non_implementer_completes_a_resolve_threads_card(self):
        self.assertTrue(cs.eligible_completer("resolve_threads", "gemini", ["claude"]))
        self.assertFalse(cs.eligible_completer("resolve_threads", "claude", ["claude"]))
        self.assertFalse(cs.eligible_completer("resolve_threads", "gemini", []))
        self.assertFalse(cs.eligible_completer("resolve_threads", "", ["claude"]))

    def test_the_threads_query_asks_for_both_page_infos(self):
        self.assertEqual(cs.THREADS_QUERY.count("pageInfo{hasNextPage}"), 2)

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


HEAD = "a" * 40
KEY = "needs_review:9:aaaaaaaa"


def receipt(kind="needs_review", pr=9, head=HEAD):
    return f"[done:{kind}:{pr}:{head}]"


class Done(unittest.TestCase):
    def snap(self, body, head=HEAD, number=9, implementers=("claude",)):
        return {"prs": [{"number": number, "head": head, "implementers": list(implementers),
                         "comments": [{"body": body}]}]}

    def test_an_explicit_receipt_bound_to_the_full_head_from_a_named_seat_completes_the_card(self):
        self.assertEqual(cs.done_keys(self.snap(receipt() + " Seat / agent: Mimo\nreview posted")), [KEY])
        self.assertEqual(cs.done_keys(self.snap(receipt("ready_to_merge") + " OWNER MERGE BRIEF (Claude seat)")),
                         ["ready_to_merge:9:aaaaaaaa"])

    def test_the_eight_character_card_reference_never_completes_anything(self):
        self.assertEqual(cs.done_keys(self.snap("[card:needs_review:9:aaaaaaaa] Mimo seat: done")), [])
        self.assertEqual(cs.done_keys(self.snap("[card:needs_review:9:aaaaaaaa done] Mimo seat")), [])

    def test_a_receipt_quoted_or_mentioned_further_down_is_a_reference_not_a_receipt(self):
        for body in ("NOT DONE: please investigate " + receipt(),
                     "Seat / agent: Mimo\n> " + receipt() + "\nquoting the card",
                     "Seat / agent: Mimo\nsee `" + receipt() + "` above"):
            self.assertEqual(cs.done_keys(self.snap(body)), [], body)

    def test_a_receipt_for_another_pr_or_another_head_does_not_complete_this_card(self):
        self.assertEqual(cs.done_keys(self.snap(receipt(pr=8) + " Mimo seat done")), [])
        self.assertEqual(cs.done_keys(self.snap(receipt(head="b" * 40) + " Mimo seat done")), [])
        self.assertEqual(cs.done_keys(self.snap(receipt() + " Mimo seat done", head="b" * 40)), [])

    def test_a_receipt_whose_first_line_says_the_work_was_not_done_completes_nothing(self):
        for text in ("Mimo seat: NOT DONE, review incomplete", "Mimo seat: blocked on fetching the head",
                     "Mimo seat: could not run the tests"):
            self.assertEqual(cs.done_keys(self.snap(receipt() + " " + text)), [], text)

    def test_contradictory_provenance_a_different_full_head_with_the_same_prefix_completes_nothing(self):
        current = "12345678" + "a" * 32
        old = "12345678" + "b" * 32
        clean = receipt(head=current) + " Mimo seat: reviewed"
        self.assertEqual(cs.done_keys(self.snap(clean, head=current)), ["needs_review:9:12345678"])
        mixed = receipt(head=current) + " Mimo seat: reviewed the older head " + old
        self.assertEqual(cs.done_keys(self.snap(mixed, head=current)), [])
        probe = receipt(head=current) + " Mimo seat: NOT DONE, reviewed old head " + old
        self.assertEqual(cs.done_keys(self.snap(probe, head=current)), [])

    def test_an_unattributed_comment_never_completes_a_card(self):
        self.assertEqual(cs.done_keys(self.snap(receipt() + " done")), [])

    def test_an_implementer_cannot_complete_a_review_of_its_own_work(self):
        self.assertEqual(cs.done_keys(self.snap(receipt() + " Claude seat: reviewed", implementers=("claude",))), [])
        self.assertEqual(cs.done_keys(self.snap(receipt() + " Mimo seat: reviewed", implementers=("claude",))), [KEY])

    def test_unknown_implementers_fail_closed_for_reviews(self):
        body = receipt() + " Mimo seat: reviewed"
        self.assertEqual(cs.done_keys(self.snap(body, implementers=())), [])
        self.assertEqual(cs.done_keys(self.snap(body, implementers=("claude",))), [KEY])

    def test_only_the_author_seat_completes_its_own_thread_fixes(self):
        body = receipt("fix_threads") + " {} seat: answered every thread"
        self.assertEqual(cs.done_keys(self.snap(body.format("Mimo"), implementers=("codex",))), [])
        self.assertEqual(cs.done_keys(self.snap(body.format("Codex"), implementers=("codex",))), ["fix_threads:9:aaaaaaaa"])

    def test_a_repair_receipt_counts_only_from_the_seat_whose_verdict_was_rejected(self):
        snap = self.snap(receipt("repair_verdict") + " Gemini seat: reposted the verdict as JSON")
        self.assertEqual(cs.done_keys(snap), [])
        self.assertEqual(cs.done_keys(snap, {9: {"mimo"}}), [])
        self.assertEqual(cs.done_keys(snap, {9: {"gemini"}}), ["repair_verdict:9:aaaaaaaa"])

    def test_build_state_wires_the_repair_owner_and_the_implementers_into_completion(self):
        comments = [
            {"id": 1, "body": "Seat / agent: Gemini\nhead aaaaaaaa\nVERDICT: accept"},
            {"id": 2, "body": receipt("repair_verdict") + " Mimo seat: not mine to repair"},
        ]
        snapshot = {"prs": [{"number": 9, "head": HEAD, "base": "main", "comments": comments}]}
        meta = {9: {"title": "t", "isDraft": False, "mergeStateStatus": "CLEAN", "updatedAt": "2026-10-04T11:00:00Z"}}
        rows = [{"pr": 9, "covered": False, "legacy": [{"comment": 0}]}]
        args = ({9: 0}, {9: "claude"}, ["claude", "gemini"], [], "2026-10-04T12:00:00Z")
        self.assertEqual(cs.build_state(snapshot, rows, meta, *args)["done"], [])
        snapshot["prs"][0]["comments"][1]["body"] = receipt("repair_verdict") + " Gemini seat: reposted"
        self.assertEqual(cs.build_state(snapshot, rows, meta, *args)["done"], ["repair_verdict:9:aaaaaaaa"])

    def test_a_completion_suppresses_only_its_own_card_end_to_end(self):
        pr = {"number": 9, "title": "t", "head": HEAD, "base": "main", "draft": False, "mergeable_state": "CLEAN",
              "updated_at": "2026-10-04T11:00:00Z", "seat": "claude", "implementers": ["claude"],
              "threads_unresolved": 0, "covered": False}
        state = {"now": "2026-10-04T12:00:00Z", "agents": [{"name": "mimo"}], "prs": [pr], "backlog": []}
        self.assertEqual(len(nt.work_items(dict(state, done=[]))), 1)
        reference_only = self.snap("NOT DONE: please investigate [card:needs_review:9:aaaaaaaa]")
        self.assertEqual(len(nt.work_items(dict(state, done=cs.done_keys(reference_only)))), 1)
        old = "aaaaaaaa" + "b" * 32
        contradictory = self.snap(receipt() + " Mimo seat: NOT DONE, reviewed old head " + old)
        self.assertEqual(len(nt.work_items(dict(state, done=cs.done_keys(contradictory)))), 1)
        good = self.snap(receipt() + " Mimo seat: review posted")
        self.assertEqual(nt.work_items(dict(state, done=cs.done_keys(good))), [])


if __name__ == "__main__":
    unittest.main()
