import json
import os
import sys
import types
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(__file__))
import review_coverage as rc  # noqa: E402

MB = "d" * 40
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


class Subprocesses(unittest.TestCase):
    """gh and git output is UTF-8. Decoding it with the platform default (cp1252 on Windows) raised UnicodeDecodeError on
    any comment holding an emoji, so the coverage tool could not run at all on this repository's real comments."""

    def test_gh_and_git_output_is_decoded_as_utf8_with_replacement(self):
        calls = []

        def fake_run(command, **kwargs):
            calls.append((command[0], kwargs))
            return types.SimpleNamespace(stdout="[]", returncode=0)

        with mock.patch.object(rc.subprocess, "run", fake_run):
            rc.gh_json("pr", "list")
            rc.git(".", "status")
        self.assertEqual([c[0] for c in calls], ["gh", "git"])
        for _, kwargs in calls:
            self.assertEqual((kwargs.get("encoding"), kwargs.get("errors")), ("utf-8", "replace"))


class Parsing(unittest.TestCase):
    def test_a_lower_case_prose_verdict_is_reported_as_legacy_not_silently_ignored(self):
        codex = "Independent Codex verdict: ACCEPT PR #69 delta only. Base " + H1 + "\nreviewed."
        structured, problems, legacy = rc.parse_attestations([{"body": codex}])
        self.assertEqual((structured, problems, len(legacy)), ([], [], 1))
        self.assertEqual(legacy[0]["heads_mentioned"], [H1])
        self.assertFalse(legacy[0]["looks_whole"])

    def test_ordinary_uses_of_the_word_verdict_are_not_legacy_packets(self):
        for body in ("No verdict yet, still reading the diff.", "The verdict is pending.\nAccept later.",
                     "verdict. Acceptable work", "Our verdict on the jury: acceptable"):
            self.assertEqual(rc.parse_attestations([{"body": body}])[2], [], body)
        for body in ("Verdict: needs_revision", "my verdict - Rejected at this head", "**Verdict**: **ACCEPT-EVIDENCE**"):
            self.assertEqual(len(rc.parse_attestations([{"body": body}])[2]), 1, body)

    def test_structured_block_is_parsed_and_prose_is_not_counted(self):
        comments = [
            {"body": "intro\n```attestation\n" + json.dumps(rec(H1)) + "\n```\nVERDICT: ACCEPT"},
            {"body": "REVIEW PACKET\nwhole-diff accept at " + H1 + "\nVERDICT: ACCEPT-EVIDENCE"},
        ]
        structured, problems, legacy = rc.parse_attestations(comments)
        self.assertEqual((len(structured), problems, len(legacy)), (1, [], 1))
        self.assertTrue(legacy[0]["looks_whole"])

    def test_a_complete_record_in_a_json_fence_is_reported_by_name_and_never_counted(self):
        # Codex's whole accept of #57 (comment 6016730072) used a json fence; it was not an attestation, not a problem
        # and, on a base without the prose-verdict rule, not even legacy: nothing told its author why it did not count.
        for label in ("json", "", "JSON", "yaml", "Attestation", "ATTESTATION", " attestation", "attestation v1"):
            comments = [{"body": "Reviewer / Seat: Codex\n\n```" + label + "\n" + json.dumps(rec(H1)) + "\n```\nscope: all files"}]
            structured, problems, legacy = rc.parse_attestations(comments)
            self.assertEqual(structured, [], label)
            self.assertEqual(len(problems), 1, label)
            self.assertIn(H1, problems[0]["problem"])
            self.assertIn("only a fence labelled attestation is read", problems[0]["problem"])
            self.assertEqual(legacy, [], label)

    def test_quoting_the_format_or_other_json_is_not_a_problem(self):
        template = {key: "<" + key + ">" for key in rc.REQUIRED}
        partial = {"head": H1, "verdict": "accept"}
        comments = [{"body": "```json\n" + json.dumps(template) + "\n```"},
                    {"body": "```json\n" + json.dumps(partial) + "\n```"},
                    {"body": "```json\n{not json}\n```"},
                    {"body": "```python\nprint(1)\n```"},
                    {"body": "```attestation\n" + json.dumps(rec(H1)) + "\n```"}]
        structured, problems, legacy = rc.parse_attestations(comments)
        # The partial record is still reported as an unstructured verdict (the existing prose rule), never as a problem.
        self.assertEqual((len(structured), problems, [item["comment"] for item in legacy]), (1, [], [1]))

    def test_a_mislabelled_record_does_not_cover_the_pr(self):
        snapshot = {"repo": "o/r", "prs": [{"number": 1, "head": H1, "merge_base": MB, "implementers": ["claude"],
                                            "comments": [{"body": "```json\n" + json.dumps(rec(H1)) + "\n```"}]}]}
        row = rc.report_snapshot(snapshot, is_ancestor=always)[0]
        self.assertFalse(row["covered"])
        self.assertEqual(len(row["malformed"]), 1)

    def test_the_default_report_names_every_rejected_block_not_only_json(self):
        placeholder = dict(rec(H1), head="<full 40-char head SHA>")
        snapshot = {"repo": "o/r", "prs": [
            {"number": 1, "head": H1, "merge_base": MB, "implementers": ["claude"],
             "comments": [block(placeholder), {"body": "```attestation\nreviewer: Gemini\nverdict: ACCEPT\n```"}]},
            {"number": 2, "head": H2, "merge_base": MB, "implementers": ["claude"], "comments": [block(rec(H2, pr=2))]}]}
        text = rc.render(rc.report_snapshot(snapshot, is_ancestor=always))
        self.assertIn("#1: 2 attestation block(s) not counted", text)
        self.assertIn("head must be a full 40-character lowercase SHA", text)
        self.assertIn("invalid JSON", text)
        self.assertNotIn("#2: ", text.replace("#2    ", ""))
        self.assertTrue(text.startswith("PR    head      covered  structured  legacy(not counted)  reasons"))

    def test_the_default_report_caps_the_lines_per_pr(self):
        comments = [block(dict(rec(H1), head="x")) for _ in range(rc.MAX_PROBLEM_LINES + 3)]
        snapshot = {"repo": "o/r", "prs": [{"number": 1, "head": H1, "merge_base": MB, "implementers": ["claude"],
                                            "comments": comments}]}
        text = rc.render(rc.report_snapshot(snapshot, is_ancestor=always))
        self.assertIn("... and 3 more (see --json)", text)
        self.assertEqual(text.count("    comment "), rc.MAX_PROBLEM_LINES)

    def test_a_placeholder_or_short_sha_is_reported_never_counted(self):
        # The template comment on #46 (5981785915) is an attestation-labelled block whose head and base_sha are
        # "<full 40-char head SHA>" and "<full merge-base SHA>"; it parsed as a structured record.
        for field in ("head", "base_sha"):
            for bad in ("<full 40-char head SHA>", "abcdef12", "a" * 39, "a" * 41, "", None, 7):
                structured, problems, _ = rc.parse_attestations([block(dict(rec(H1), **{field: bad}))])
                self.assertEqual(structured, [], (field, bad))
                self.assertEqual(len(problems), 1, (field, bad))
                self.assertIn(field + " must be a full 40-character lowercase SHA", problems[0]["problem"])
        structured, problems, _ = rc.parse_attestations([block(rec(H2, "delta", frm="<covered head>"))])
        self.assertEqual((structured, len(problems)), ([], 1))
        self.assertIn("from must be", problems[0]["problem"])

    def test_an_upper_case_sha_is_the_same_commit_and_now_matches(self):
        upper = dict(rec(H1), head=H1.upper(), base_sha=MB.upper())
        structured, problems, _ = rc.parse_attestations([block(upper)])
        self.assertEqual(problems, [])
        self.assertEqual((structured[0]["head"], structured[0]["base_sha"]), (H1, MB))
        self.assertTrue(ev(H1, structured)["covered"])

    def test_an_accept_in_the_wrong_case_is_reported_and_never_covers(self):
        # Gemini's blocks on #62 and #64 used ACCEPT. It parsed, matched nothing and produced no problem: the PR kept
        # reading "no accept at the current head" with nothing saying why.
        for verdict in ("ACCEPT", "Accept", "accepted", "ACCEPT-EVIDENCE", "approve", ""):
            structured, problems, _ = rc.parse_attestations([block(rec(H1, verdict=verdict))])
            self.assertEqual(structured, [], verdict)
            self.assertEqual(len(problems), 1, verdict)
            self.assertIn("unknown verdict", problems[0]["problem"])
            self.assertIn("lower case", problems[0]["problem"])

    def test_a_rejection_in_the_wrong_case_still_blocks(self):
        for verdict in ("NEEDS_REVISION", "Needs_Revision", " changes_requested ", "CHANGES_REQUESTED"):
            structured, problems, _ = rc.parse_attestations([block(rec(H1, verdict=verdict, reviewer="codex"))])
            self.assertEqual(problems, [], verdict)
            self.assertEqual(structured[0]["verdict"] in rc.REJECT, True, verdict)
            decision = ev(H1, [rec(H1), structured[0]])
            self.assertFalse(decision["covered"], verdict)
            self.assertTrue(any("standing" in r for r in decision["reasons"]), verdict)

    def test_the_exact_verdict_words_are_unchanged(self):
        for verdict in ("accept", "needs_revision", "changes_requested"):
            structured, problems, _ = rc.parse_attestations([block(rec(H1, verdict=verdict))])
            self.assertEqual((len(structured), problems), (1, []), verdict)
        structured, problems, _ = rc.parse_attestations([block(dict(rec(H1), verdict=["accept"]))])
        self.assertEqual((structured, len(problems)), ([], 1))

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
