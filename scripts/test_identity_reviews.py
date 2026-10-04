import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import review_coverage as rc  # noqa: E402

MB = "m" * 40
H1, H2 = "a" * 40, "b" * 40
MAP = {"mimo-bot": "mimo", "codex-bot": "codex", "claude-bot": "claude"}


def review(login, state="APPROVED", commit=H1, rid=1):
    return {"id": rid, "user": {"login": login}, "state": state, "commit_id": commit}


def snapshot(reviews, implementers=("claude",), head=H1):
    pr = {"number": 5, "head": head, "merge_base": MB, "comments": [], "reviews": reviews,
          "implementers": list(implementers)}
    return {"repo": "o/r", "prs": [pr]}


def row(reviews, **kw):
    return rc.report_snapshot(snapshot(reviews, **kw), None, MAP)[0]


class FormalReviews(unittest.TestCase):
    def test_approval_at_head_by_a_mapped_non_implementer_counts(self):
        r = row([review("mimo-bot")])
        self.assertTrue(r["covered"])
        self.assertEqual(r["trust"], "authenticated")
        self.assertEqual(r["authenticated_reviews"], 1)

    def test_approval_at_an_older_commit_does_not_carry(self):
        self.assertFalse(row([review("mimo-bot", commit=H2)])["covered"])

    def test_unmapped_login_never_counts_and_is_reported(self):
        r = row([review("random-user")])
        self.assertFalse(r["covered"])
        self.assertIn("not a mapped seat", r["malformed"][0]["problem"])

    def test_shared_owner_login_is_not_a_seat(self):
        self.assertFalse(row([review("Aarogaming")])["covered"])

    def test_implementer_approving_own_work_does_not_count(self):
        self.assertFalse(row([review("claude-bot")])["covered"])

    def test_changes_requested_at_head_blocks_even_with_an_approval(self):
        r = row([review("mimo-bot", rid=1), review("codex-bot", "CHANGES_REQUESTED", rid=2)])
        self.assertFalse(r["covered"])
        self.assertIn("changes_requested", " ".join(r["reasons"]))

    def test_unknown_implementers_means_independence_is_unproven(self):
        r = row([review("mimo-bot")], implementers=())
        self.assertFalse(r["covered"])
        self.assertIn("independence unproven", r["malformed"][0]["problem"])

    def test_login_matching_is_case_insensitive(self):
        self.assertTrue(row([review("MIMO-BOT")])["covered"])

    def test_no_identity_map_means_attested_only_and_formal_reviews_are_ignored(self):
        r = rc.report_snapshot(snapshot([review("mimo-bot")]), None, None)[0]
        self.assertFalse(r["covered"])
        self.assertEqual(r["trust"], "attested-only")


class IdentityMap(unittest.TestCase):
    def test_load_identities_maps_logins_to_seats_lowercased(self):
        data = {"owner": ["Aarogaming"], "seats": {"mimo": ["Mimo-Bot"], "claude": []}}
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False, encoding="utf-8") as handle:
            json.dump(data, handle)
        try:
            self.assertEqual(rc.load_identities(handle.name), {"mimo-bot": "mimo"})
        finally:
            os.unlink(handle.name)

    def test_the_committed_map_never_lists_the_owner_as_a_seat(self):
        here = os.path.dirname(os.path.abspath(__file__))
        mapping = rc.load_identities(os.path.join(here, "..", "governance", "identities.json"))
        self.assertNotIn("aarogaming", mapping)


class Encoding(unittest.TestCase):
    def test_subprocess_helpers_decode_utf8_not_the_console_codepage(self):
        # Review and comment bodies contain non-ASCII (emoji, arrows); cp1252 decoding crashed live runs.
        import subprocess
        from unittest import mock

        completed = subprocess.CompletedProcess(args=[], returncode=0, stdout='[{"b": "ok → done"}]', stderr="")
        with mock.patch.object(subprocess, "run", return_value=completed) as run:
            self.assertEqual(rc.gh_json("api", "x"), [{"b": "ok → done"}])
            self.assertEqual(run.call_args.kwargs["encoding"], "utf-8")
            rc.git(".", "status")
            self.assertEqual(run.call_args.kwargs["encoding"], "utf-8")


if __name__ == "__main__":
    unittest.main()
