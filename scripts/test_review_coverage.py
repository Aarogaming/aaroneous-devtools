import json
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import review_coverage as rc  # noqa: E402

MB = "m" * 40
H1, H2, H3 = "a" * 40, "b" * 40, "c" * 40


def always(_a, _b):
    return True


def never(_a, _b):
    return False


def rec(head, scope="whole", verdict="accept", reviewer="mimo", base=MB, frm=None,
        impl=("claude",), repo="o/r", pr=1):
    record = {
        "repo": repo,
        "pr": pr,
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


def block(value):
    return {"body": "```attestation\n" + json.dumps(value) + "\n```"}


def ev(head, records, anc=always, mb=MB):
    return rc.evaluate(head, mb, records, anc)


class Coverage(unittest.TestCase):
    def test_whole_accept_at_head_covers(self):
        self.assertTrue(ev(H1, [rec(H1)])["covered"])

    def test_delta_chained_from_whole_covers(self):
        self.assertTrue(ev(H2, [rec(H1), rec(H2, "delta", frm=H1)])["covered"])

    def test_delta_without_a_whole_origin_does_not_cover(self):
        decision = ev(H2, [rec(H2, "delta", frm=H1)])
        self.assertFalse(decision["covered"])
        self.assertIn("not chained", " ".join(decision["reasons"]))

    def test_a_delta_from_the_merge_base_does_not_seed_coverage_but_the_reason_says_to_relabel_it_whole(self):
        decision = ev(H2, [rec(H2, "delta", frm=MB)])
        self.assertFalse(decision["covered"])
        self.assertIn("not chained to a whole accept", " ".join(decision["reasons"]))
        self.assertIn('re-post it with scope "whole"', " ".join(decision["reasons"]))

    def test_the_relabel_hint_appears_only_for_a_delta_that_starts_at_the_merge_base(self):
        other = ev(H2, [rec(H2, "delta", frm=H1)])
        self.assertFalse(other["covered"])
        self.assertNotIn("re-post", " ".join(other["reasons"]))
        implementer = ev(H2, [rec(H2, "delta", frm=MB, reviewer="claude")])
        self.assertNotIn("re-post", " ".join(implementer["reasons"]))

    def test_the_same_review_labelled_whole_covers(self):
        self.assertTrue(ev(H2, [rec(H2, "whole")])["covered"])

    def test_accept_at_an_older_head_does_not_carry(self):
        decision = ev(H2, [rec(H1)])
        self.assertFalse(decision["covered"])
        self.assertIn("no accept at the current head", decision["reasons"])

    def test_chain_of_two_deltas_covers(self):
        records = [rec(H1), rec(H2, "delta", frm=H1), rec(H3, "delta", frm=H2)]
        self.assertTrue(ev(H3, records)["covered"])

    def test_delta_whose_from_is_not_an_ancestor_is_rejected(self):
        # A rebased or flattened head names an old SHA but does not descend from it.
        decision = ev(H2, [rec(H1), rec(H2, "delta", frm=H1)], anc=never)
        self.assertFalse(decision["covered"])

    def test_moved_merge_base_invalidates(self):
        self.assertFalse(ev(H1, [rec(H1)], mb="n" * 40)["covered"])

    def test_same_merge_base_stays_valid_when_base_tip_moves(self):
        self.assertTrue(ev(H1, [rec(H1)])["covered"])

    def test_reviewer_who_implemented_does_not_count(self):
        self.assertFalse(ev(H1, [rec(H1, reviewer="claude")])["covered"])

    def test_standing_changes_requested_blocks(self):
        decision = ev(H1, [rec(H1), rec(H1, verdict="changes_requested", reviewer="codex")])
        self.assertFalse(decision["covered"])
        self.assertIn("changes_requested", " ".join(decision["reasons"]))


class Parsing(unittest.TestCase):
    def test_structured_block_is_parsed_and_prose_is_not_counted(self):
        comments = [
            {"body": "intro\n```attestation\n" + json.dumps(rec(H1)) + "\n```\nVERDICT: ACCEPT"},
            {"body": "REVIEW PACKET\nwhole-diff accept at " + H1 + "\nVERDICT: ACCEPT-EVIDENCE"},
        ]
        structured, problems, legacy = rc.parse_attestations(comments)
        self.assertEqual((len(structured), problems, len(legacy)), (1, [], 1))
        self.assertTrue(legacy[0]["looks_whole"])

    def test_malformed_blocks_are_reported_not_counted(self):
        comments = [
            {"body": "```attestation\n{not json}\n```"},
            block({"head": H1}),
            block(rec(H2, "delta")),
        ]
        structured, problems, _ = rc.parse_attestations(comments)
        self.assertEqual(structured, [])
        self.assertEqual(len(problems), 3)

    def test_missing_or_empty_implementers_is_rejected(self):
        no_impl = rec(H1)
        del no_impl["implementers"]
        empty = rec(H1)
        empty["implementers"] = []
        structured, problems, _ = rc.parse_attestations([block(no_impl), block(empty)])
        self.assertEqual(structured, [])
        self.assertEqual(len(problems), 2)

    def test_non_object_json_is_malformed_not_a_crash(self):
        structured, problems, _ = rc.parse_attestations([block(None), block([]), block("text"), block(5)])
        self.assertEqual(structured, [])
        self.assertEqual(len(problems), 4)
        self.assertIn("not a JSON object", problems[0]["problem"])


class Plumbing(unittest.TestCase):
    def test_flatten_pages_merges_slurped_arrays(self):
        pages = [[{"a": 1}], [{"a": 2}, {"a": 3}], []]
        self.assertEqual(rc.flatten_pages(pages), [{"a": 1}, {"a": 2}, {"a": 3}])

    def test_pr_list_requests_every_open_pr(self):
        args = rc.pr_list_args("o/r")
        self.assertIn("--limit", args)
        self.assertGreaterEqual(int(args[args.index("--limit") + 1]), 1000)

    def test_checkout_matches_the_named_repository(self):
        name = "Aarogaming/aaroneous-devtools"
        self.assertTrue(rc.checkout_matches("https://github.com/" + name + ".git\n", name))
        self.assertTrue(rc.checkout_matches("git@github.com:" + name + ".git", name))
        self.assertFalse(rc.checkout_matches("https://github.com/Aarogaming/aaroneous.git", name))

    def test_repo_requires_an_explicit_checkout(self):
        with self.assertRaises(SystemExit):
            rc.main(["--repo", "o/r"])


class Snapshot(unittest.TestCase):
    def test_report_marks_prose_only_prs_uncovered_and_counts_legacy(self):
        snapshot = {
            "repo": "o/r",
            "prs": [
                {
                    "number": 7,
                    "head": H1,
                    "merge_base": MB,
                    "comments": [{"body": "whole-diff accept at " + H1 + "\nVERDICT: ACCEPT"}],
                },
                {"number": 8, "head": H1, "merge_base": MB, "comments": [block(rec(H1, pr=8))]},
            ],
        }
        rows = {r["pr"]: r for r in rc.report_snapshot(snapshot)}
        self.assertFalse(rows[7]["covered"])
        self.assertEqual(rows[7]["legacy_packets_not_counted"], 1)
        self.assertTrue(rows[8]["covered"])
        self.assertIn("#8", rc.render(list(rows.values())))

    def test_snapshot_ancestry_pairs_gate_delta_chains(self):
        comments = [block(rec(H1)), block(rec(H2, "delta", frm=H1))]
        pr = {"number": 1, "head": H2, "merge_base": MB, "comments": comments}
        flattened = rc.report_snapshot({"repo": "o/r", "prs": [dict(pr, ancestry=[])]})[0]
        linked = rc.report_snapshot({"repo": "o/r", "prs": [dict(pr, ancestry=[[H1, H2]])]})[0]
        self.assertFalse(flattened["covered"])
        self.assertTrue(linked["covered"])


class Subject(unittest.TestCase):
    def snapshot(self, record, number=46):
        pr = {"number": number, "head": H1, "merge_base": MB, "comments": [block(record)]}
        return {"repo": "right/repo", "prs": [pr]}

    def test_matching_repo_and_pr_counts(self):
        row = rc.report_snapshot(self.snapshot(rec(H1, repo="right/repo", pr=46)))[0]
        self.assertTrue(row["covered"])

    def test_attestation_for_another_repository_is_not_counted(self):
        row = rc.report_snapshot(self.snapshot(rec(H1, repo="wrong/repo", pr=46)))[0]
        self.assertFalse(row["covered"])
        self.assertIn("does not match", row["malformed"][0]["problem"])

    def test_attestation_for_another_pr_is_not_counted(self):
        row = rc.report_snapshot(self.snapshot(rec(H1, repo="right/repo", pr=999)))[0]
        self.assertFalse(row["covered"])
        self.assertEqual(len(row["malformed"]), 1)

    def test_pr_number_as_string_still_matches(self):
        row = rc.report_snapshot(self.snapshot(rec(H1, repo="right/repo", pr="46")))[0]
        self.assertTrue(row["covered"])

    def test_snapshot_without_a_repository_is_rejected(self):
        with self.assertRaises(ValueError):
            rc.report_snapshot({"prs": []})


if __name__ == "__main__":
    unittest.main()
