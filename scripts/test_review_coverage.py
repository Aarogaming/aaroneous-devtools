import json
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import review_coverage as rc  # noqa: E402

MB = "m" * 40
H1, H2, H3 = "a" * 40, "b" * 40, "c" * 40


def rec(head, scope="whole", verdict="accept", reviewer="mimo", base=MB, frm=None, impl=("claude",)):
    record = {
        "repo": "o/r",
        "pr": 1,
        "head": head,
        "base_sha": base,
        "scope": scope,
        "reviewer": reviewer,
        "implementers": list(impl),
        "verdict": verdict,
    }
    if frm:
        record["from"] = frm
    return record


class Coverage(unittest.TestCase):
    def test_whole_accept_at_head_covers(self):
        self.assertTrue(rc.evaluate(H1, MB, [rec(H1)])["covered"])

    def test_delta_chained_from_whole_covers(self):
        records = [rec(H1), rec(H2, "delta", frm=H1)]
        self.assertTrue(rc.evaluate(H2, MB, records)["covered"])

    def test_delta_without_a_whole_origin_does_not_cover(self):
        decision = rc.evaluate(H2, MB, [rec(H2, "delta", frm=H1)])
        self.assertFalse(decision["covered"])
        self.assertIn("not chained", " ".join(decision["reasons"]))

    def test_accept_at_an_older_head_does_not_carry(self):
        decision = rc.evaluate(H2, MB, [rec(H1)])
        self.assertFalse(decision["covered"])
        self.assertIn("no accept at the current head", decision["reasons"])

    def test_chain_of_two_deltas_covers(self):
        records = [rec(H1), rec(H2, "delta", frm=H1), rec(H3, "delta", frm=H2)]
        self.assertTrue(rc.evaluate(H3, MB, records)["covered"])

    def test_moved_merge_base_invalidates(self):
        self.assertFalse(rc.evaluate(H1, "n" * 40, [rec(H1)])["covered"])

    def test_same_merge_base_stays_valid_when_base_tip_moves(self):
        # The base tip moving does not change the merge-base, so the record is still valid.
        self.assertTrue(rc.evaluate(H1, MB, [rec(H1)])["covered"])

    def test_reviewer_who_implemented_does_not_count(self):
        self.assertFalse(rc.evaluate(H1, MB, [rec(H1, reviewer="claude")])["covered"])

    def test_standing_changes_requested_blocks(self):
        records = [rec(H1), rec(H1, verdict="changes_requested", reviewer="codex")]
        decision = rc.evaluate(H1, MB, records)
        self.assertFalse(decision["covered"])
        self.assertIn("changes_requested", " ".join(decision["reasons"]))


class Parsing(unittest.TestCase):
    def test_structured_block_is_parsed_and_prose_is_not_counted(self):
        good = rec(H1)
        comments = [
            {"body": "intro\n```attestation\n" + json.dumps(good) + "\n```\nVERDICT: ACCEPT"},
            {"body": "REVIEW PACKET\nwhole-diff accept at " + H1 + "\nVERDICT: ACCEPT-EVIDENCE"},
        ]
        structured, problems, legacy = rc.parse_attestations(comments)
        self.assertEqual(len(structured), 1)
        self.assertEqual(problems, [])
        self.assertEqual(len(legacy), 1)
        self.assertTrue(legacy[0]["looks_whole"])

    def test_malformed_blocks_are_reported_not_counted(self):
        comments = [
            {"body": "```attestation\n{not json}\n```"},
            {"body": "```attestation\n" + json.dumps({"head": H1}) + "\n```"},
            {"body": "```attestation\n" + json.dumps(rec(H2, "delta")) + "\n```"},
        ]
        structured, problems, _ = rc.parse_attestations(comments)
        self.assertEqual(structured, [])
        self.assertEqual(len(problems), 3)


class Snapshot(unittest.TestCase):
    def test_report_marks_prose_only_prs_uncovered_and_counts_legacy(self):
        snapshot = {
            "prs": [
                {
                    "number": 7,
                    "head": H1,
                    "merge_base": MB,
                    "comments": [{"body": "whole-diff accept at " + H1 + "\nVERDICT: ACCEPT"}],
                },
                {
                    "number": 8,
                    "head": H1,
                    "merge_base": MB,
                    "comments": [{"body": "```attestation\n" + json.dumps(rec(H1)) + "\n```"}],
                },
            ]
        }
        rows = {r["pr"]: r for r in rc.report_snapshot(snapshot)}
        self.assertFalse(rows[7]["covered"])
        self.assertEqual(rows[7]["legacy_packets_not_counted"], 1)
        self.assertTrue(rows[8]["covered"])
        self.assertIn("#8", rc.render(list(rows.values())))


if __name__ == "__main__":
    unittest.main()
