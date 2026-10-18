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
    def test_a_rejection_naming_another_merge_base_is_stale_but_not_silent(self):
        # PLATFORM_SPEC.md: a record is valid only while the merge base equals its base_sha, so a rejection against an
        # old base does not block (Codex P1 on #114). It used to vanish without a word, which is how a mistyped base_sha
        # lost a reviewer's request for changes (probe: accept + needs_revision with a wrong base_sha, no reasons).
        wrong = "w" * 40
        for verdict in ("needs_revision", "changes_requested"):
            result = ev(H1, [rec(H1), rec(H1, reviewer="codex", verdict=verdict, base=wrong)])
            self.assertTrue(result["covered"], verdict)
            self.assertEqual(len(result["reasons"]), 1)
            self.assertIn("a rejection by codex names base_sha wwwwwwwwwwww", result["reasons"][0])
            self.assertIn("ignored as stale", result["reasons"][0])
            # merge_gate finds a standing rejection by these words in the reasons; the note must not contain them
            self.assertFalse(any("needs_revision" in r or "changes_requested" in r for r in result["reasons"]))
        right = ev(H1, [rec(H1), rec(H1, reviewer="codex", verdict="needs_revision")])
        self.assertFalse(right["covered"])
        self.assertEqual(right["reasons"], ["standing needs_revision at head by codex"])

    def test_a_rejection_at_another_head_does_not_block(self):
        self.assertTrue(ev(H1, [rec(H1), rec(H2, reviewer="codex", verdict="needs_revision", base="w" * 40)])["covered"])

    def test_an_accept_that_does_not_count_says_why(self):
        wrong = ev(H1, [rec(H1, reviewer="gemini", base="e" * 40)])
        self.assertFalse(wrong["covered"])
        self.assertIn("names base_sha eeeeeeeeeeee, but the merge base is dddddddddddd", wrong["reasons"][-1])
        own = ev(H1, [rec(H1, reviewer="claude")])
        self.assertIn("the accept by claude is not independent of its implementers ['claude']", own["reasons"][-1])
        # new wording must not trip merge_gate's substring test for a standing rejection
        for result in (wrong, own):
            self.assertFalse(any("changes_requested" in r or "needs_revision" in r for r in result["reasons"]))

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

    def test_the_same_seat_spelled_two_ways_is_one_accepting_seat(self):
        # #96 at ecab144a: two Gemini comments, reviewer "Gemini" and "gemini", read as accept_seats ['Gemini', 'gemini'].
        records = [rec(H1, reviewer="Gemini", impl=("Claude",)), rec(H1, reviewer="gemini", impl=("Claude",)),
                   rec(H1, reviewer="Antigravity", impl=("Claude",))]
        decision = ev(H1, records)
        self.assertTrue(decision["covered"])
        self.assertEqual(decision["accept_seats"], ["gemini"])
        for spelling in ("Gemini/Antigravity", "gemini (antigravity)", " GEMINI "):
            self.assertEqual(ev(H1, records + [rec(H1, reviewer=spelling, impl=("Claude",))])["accept_seats"], ["gemini"], spelling)
        self.assertEqual(rc.seat_name("Claude Sonnet 5.5"), "claude")
        self.assertEqual(rc.seat_name("Mimo (OpenCode)"), "mimo")
        self.assertEqual(rc.seat_name(""), "")
        two = ev(H1, records + [rec(H1, reviewer="Codex", impl=("Claude",))])
        self.assertEqual(two["accept_seats"], ["codex", "gemini"])

    def test_a_model_family_name_without_the_seat_word_names_that_seat(self):
        # Probe: reviewers "Opus" and "Sonnet" against implementer "claude" were two independent seats and covered.
        self.assertEqual(rc.seat_names("Opus 5.5"), {"claude"})
        self.assertEqual(rc.seat_names("claude-opus-4"), {"claude"})
        self.assertEqual(rc.seat_names("gpt-5.5"), {"codex"})
        self.assertEqual(rc.seat_names("OpenAI Codex"), {"codex"})
        for reviewer in ("Opus", "Sonnet 5.5", "Fable 5.1", "Haiku", "Anthropic"):
            self.assertFalse(ev(H1, [rec(H1, reviewer=reviewer, impl=("claude",))])["covered"], reviewer)
        self.assertFalse(ev(H1, [rec(H1, reviewer="gpt-5.5", impl=("Codex",))])["covered"])
        self.assertFalse(ev(H1, [rec(H1, reviewer="Codex", impl=("GPT-5.5",))])["covered"])
        self.assertFalse(ev(H1, [rec(H1, reviewer="Claude", impl=("Opus 5.5",))])["covered"])
        self.assertTrue(ev(H1, [rec(H1, reviewer="Gemini", impl=("Opus 5.5",))])["covered"])

    def test_two_labels_for_one_model_family_are_one_accepting_seat(self):
        two = ev(H1, [rec(H1, reviewer="Opus", impl=("codex",)), rec(H1, reviewer="Sonnet", impl=("codex",))])
        self.assertEqual(two["accept_seats"], ["claude"])

    def test_an_ambiguous_reviewer_naming_multiple_seats_fails_closed_and_contributes_no_accept_seats(self):
        # Codex P2 on #111: an attestation claiming multiple seats cannot satisfy independence or contribute multiple seats.
        self.assertIsNone(rc.canonical_reviewer_seat("Opus/gpt-5.5"))
        self.assertIsNone(rc.canonical_reviewer_seat("Claude, Mimo"))
        self.assertEqual(rc.canonical_reviewer_seat("Opus 5.5"), "claude")
        self.assertEqual(rc.canonical_reviewer_seat("gpt-5.5"), "codex")
        self.assertEqual(rc.canonical_reviewer_seat("Gemini/Antigravity"), "gemini")

        decision = ev(H1, [rec(H1, reviewer="Opus/gpt-5.5", impl=("mimo",))])
        self.assertFalse(decision["covered"])
        self.assertEqual(decision["accept_seats"], [])

        decision2 = ev(H1, [rec(H1, reviewer="Claude, Mimo", impl=("gemini",))])
        self.assertFalse(decision2["covered"])
        self.assertEqual(decision2["accept_seats"], [])

    def test_model_ids_and_punctuation_forms_name_the_same_seat(self):
        # Skeptic probe on #99: all of these were independent of the reviewer they name, because only the first
        # space-separated word was compared.
        for reviewer, implementers in (("Gemini", ("gemini-3.8-flash",)), ("Gemini", ("gemini_3_flash",)),
                                       ("Claude", ("claude-sonnet-5-5",)), ("Claude", ("Claude, Mimo",)),
                                       ("Claude", ("claude+mimo",)), ("Mimo", ("mimo-v2.6-flash-free",)),
                                       ("Codex", ("codex:session-01a1",)), ("Claude", ("Claude\u200b",)),
                                       ("Claude", ("CLAUDE.",)), ("Gemini", ("Gemini3",)), ("Claude", ("Anthropic Claude",))):
            self.assertFalse(ev(H1, [rec(H1, reviewer=reviewer, impl=implementers)])["covered"], (reviewer, implementers))
        # A string naming two seats covers neither of them, and still leaves a third free to review.
        self.assertTrue(ev(H1, [rec(H1, reviewer="Codex", impl=("Claude, Mimo",))])["covered"])
        self.assertFalse(ev(H1, [rec(H1, reviewer="Mimo", impl=("Claude, Mimo",))])["covered"])
        self.assertEqual(rc.seat_names("Gemini/Antigravity"), {"gemini"})
        self.assertEqual(rc.seat_names("Claude, Mimo"), {"claude", "mimo"})
        self.assertEqual(rc.seat_names("Local Qwen"), {"mimo"})
        self.assertEqual(rc.seat_names("ubuntu-worker-01"), {"ubuntu-worker-01"})
        self.assertEqual(rc.seat_names("Ubuntu"), {"ubuntu-worker-01"})
        self.assertEqual(rc.seat_names("ubuntu-worker-01-seat[bot]"), {"ubuntu-worker-01"})
        self.assertEqual(rc.seat_names("someone else"), {"someone"})
        self.assertEqual(rc.seat_names("!!!"), set())

    def test_an_implementer_in_another_spelling_is_never_independent(self):
        # Codex's counterexample on #98: reviewer 'Gemini/Antigravity' with implementer 'gemini' cannot self-cover, and
        # neither can 'gemini' with ['Gemini'] or 'Claude' with ['Claude Sonnet 5.5'].
        for reviewer, implementers in (("Gemini/Antigravity", ("gemini",)), ("gemini", ("Gemini",)),
                                       ("Claude", ("Claude Sonnet 5.5",)), ("Antigravity", ("Gemini",)),
                                       ("mimo", ("Mimo (OpenCode)", "Claude"))):
            decision = ev(H1, [rec(H1, reviewer=reviewer, impl=implementers)])
            self.assertFalse(decision["covered"], (reviewer, implementers))
        genuine = ev(H1, [rec(H1, reviewer="codex", impl=("Gemini", "Claude Sonnet 5.5"))])
        self.assertTrue(genuine["covered"])
        mixed = ev(H1, [rec(H1, reviewer="gemini", impl=("Gemini",)), rec(H1, reviewer="Codex", impl=("Gemini",))])
        self.assertEqual(mixed["accept_seats"], ["codex"])
        self.assertTrue(mixed["covered"])

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
        self.assertTrue(text.startswith("PR    head      covered  structured  auth  legacy(not counted)  reasons"))

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


TRAILER_CLAUDE = "docs: change\n\nCo-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>\n"
NO_TRAILER = "docs: address review findings\n"


class StandInComments(unittest.TestCase):
    """The shared login posts for every seat. "Author Update (operating in lieu of Codex)" is written by a seat that is
    not Codex and does not say which."""

    def row(self, bodies):
        pr = {"number": 1, "head": H1, "merge_base": MB, "comments": [{"id": i, "body": b} for i, b in enumerate(bodies)]}
        return rc.report_snapshot({"repo": "o/r", "prs": [pr]}, is_ancestor=always)[0]

    def test_the_real_phrasing_is_found_and_names_the_seat_being_covered_for(self):
        self.assertEqual(rc.stand_in_seats([{"body": "### Author Update (operating in lieu of Codex) / PR #60 Intake"}]), ["codex"])
        self.assertEqual(rc.stand_in_seats([{"body": "Posting on behalf of Antigravity"}]), ["gemini"])
        self.assertEqual(rc.stand_in_seats([{"body": "x\n\nlater: in lieu of Claude"}]), [], "only the first line is read")
        self.assertEqual(rc.stand_in_seats([{"body": "Claude seat: not in lieu of anyone"}]), [])
        self.assertEqual(rc.stand_in_seats(None), [])

    def test_such_a_comment_is_reported_on_the_row_and_does_not_change_coverage(self):
        row = self.row(["### Author Update (operating in lieu of Codex) / PR #47"])
        self.assertTrue(any("in lieu of codex" in r for r in row["reasons"]), row["reasons"])
        self.assertFalse(any("needs_revision" in r or "changes_requested" in r for r in row["reasons"]))
        self.assertEqual(self.row(["plain comment"])["reasons"], ["no accept at the current head"])

    def test_the_comment_is_not_attributed_to_the_seat_it_stands_in_for(self):
        import collect_state as cs
        self.assertEqual(cs.comment_seat("### Author Update (operating in lieu of Codex) / PR #47", strict=True), "")


class GitSaysWhoImplemented(unittest.TestCase):
    """An attestation's `implementers` list is the reviewer's word; git history is not. Probe: Gemini committed to #116,
    the commit had no trailer, and Gemini's attestation listed only claude, so the accept counted as independent."""

    def row(self, record, commit_log="", branch="proposal/x", reviews=()):
        pr = {"number": 1, "head": H1, "merge_base": MB, "comments": [block(record)], "commit_log": commit_log,
              "branch": branch}
        return rc.report_snapshot({"repo": "o/r", "prs": [pr]}, is_ancestor=always)[0]

    def record(self, reviewer, implementers=("claude",)):
        return rec(H1, reviewer=reviewer, impl=implementers, repo="o/r", pr=1)

    def test_a_seat_named_in_a_trailer_cannot_review_even_when_the_attestation_omits_it(self):
        log = "\x1e" + TRAILER_CLAUDE + "\x1e" + "fix\n\nCo-Authored-By: Gemini <g@example.invalid>\n"
        row = self.row(self.record("gemini"), log)
        self.assertFalse(row["covered"])
        self.assertTrue(any("git history names ['gemini']" in r for r in row["reasons"]), row["reasons"])
        # the same history does not stop a seat that did not write it
        self.assertTrue(self.row(self.record("codex"), log)["covered"])

    def test_a_seat_named_branch_prefix_counts_as_an_implementer(self):
        row = self.row(self.record("codex", ("claude",)), "\x1e" + NO_TRAILER, branch="codex/output-lifecycle")
        self.assertFalse(row["covered"])

    def test_aliases_resolve_to_the_seat(self):
        log = "\x1e" + "x\n\nCo-Authored-By: Antigravity <a@example.invalid>\n"
        self.assertFalse(self.row(self.record("Gemini"), log)["covered"])

    def test_a_commit_that_names_no_seat_is_reported_but_does_not_block(self):
        log = "\x1e" + TRAILER_CLAUDE + "\x1e" + NO_TRAILER
        row = self.row(self.record("gemini"), log)
        self.assertTrue(row["covered"], "advisory only: git cannot show the author of that commit")
        self.assertTrue(any(r.startswith("1 commit(s) in base..head name no implementing seat") for r in row["reasons"]))
        self.assertFalse(any("needs_revision" in r or "changes_requested" in r for r in row["reasons"]))

    def test_a_seat_named_branch_attributes_every_commit(self):
        row = self.row(self.record("gemini"), "\x1e" + NO_TRAILER, branch="claude/x")
        self.assertTrue(row["covered"])
        self.assertEqual(row["reasons"], [])

    def test_without_git_facts_nothing_changes(self):
        self.assertTrue(self.row(self.record("gemini"))["covered"])
        self.assertEqual(self.row(self.record("gemini"))["reasons"], [])

    def test_a_commit_author_named_for_a_seat_attributes_the_commit(self):
        log = "\x1e" + "git-author: Claude <noreply@anthropic.com>\n" + NO_TRAILER
        self.assertEqual(rc.git_implementers(log, "docs/x"), {"claude"})
        self.assertEqual(rc.unattributed_commits(log, "docs/x"), 0)
        other = "\x1e" + "git-author: Aarog <aarog@example.invalid>\n" + NO_TRAILER
        self.assertEqual(rc.git_implementers(other, "docs/x"), set())
        self.assertEqual(rc.unattributed_commits(other, "docs/x"), 1)

    def test_the_helpers_read_a_real_git_log(self):
        import shutil
        import subprocess
        import tempfile
        root = tempfile.mkdtemp(prefix="git-implementers-")
        self.addCleanup(shutil.rmtree, root, True)

        def git(*args):
            result = subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid", "-C", root, *args],
                                    capture_output=True, text=True, encoding="utf-8")
            self.assertEqual(result.returncode, 0, result.stderr)
            return result.stdout

        git("init", "-q", "-b", "main")
        for index, message in enumerate((TRAILER_CLAUDE, NO_TRAILER, "third\n\nContribution-ID: C1/mimo-x\n")):
            with open(f"{root}/f{index}.txt", "w", encoding="utf-8", newline="\n") as handle:
                handle.write("x\n")
            git("add", ".")
            git("commit", "-q", "-m", message)
        log = git("log", "--format=%x1egit-author: %an <%ae>%n%B", "HEAD")
        self.assertEqual(len(rc.commit_chunks(log)), 3)
        self.assertEqual(rc.git_implementers(log, "proposal/x"), {"claude", "mimo"})
        self.assertEqual(rc.unattributed_commits(log, "proposal/x"), 1)
        self.assertEqual(rc.unattributed_commits(log, "codex/x"), 0)


class EvidenceSupersession(unittest.TestCase):
    """Merge-gate box 5: a seat's later record with commands covers its own earlier record without them."""

    @staticmethod
    def counted(*specs):
        out = []
        for comment, reviewer, commands in specs:
            record = rec(H1, reviewer=reviewer)
            record["comment"] = comment
            if commands:
                record["commands"] = ["cargo test -> exit 0"]
            out.append(record)
        return out

    def test_a_later_record_with_commands_covers_the_same_seats_earlier_record(self):
        counted = self.counted((3, "claude", False), (7, "claude", True), (6, "gemini", True))
        self.assertEqual(rc.unevidenced_verdicts(counted), [])

    def test_a_seat_that_never_cited_a_command_still_blocks(self):
        counted = self.counted((3, "claude", False), (6, "gemini", True))
        self.assertEqual(rc.unevidenced_verdicts(counted), [3])

    def test_another_seats_commands_do_not_cover_this_seat(self):
        counted = self.counted((3, "claude", False), (6, "gemini", True), (7, "gemini", True))
        self.assertEqual(rc.unevidenced_verdicts(counted), [3])

    def test_spelling_variants_of_one_seat_are_one_seat(self):
        counted = self.counted((3, "Claude", False), (7, "claude", True))
        self.assertEqual(rc.unevidenced_verdicts(counted), [])

    def test_an_ambiguous_reviewer_is_never_covered_by_anyone_elses_commands(self):
        counted = self.counted((3, "claude/gemini", False), (6, "gemini", True), (7, "claude", True))
        self.assertEqual(rc.unevidenced_verdicts(counted), [3])

    def test_two_unresolvable_reviewers_do_not_cover_each_other(self):
        counted = self.counted((3, "claude/gemini", False), (7, "claude/gemini", True))
        self.assertEqual(rc.unevidenced_verdicts(counted), [3])

    def test_usable_evidence_is_a_command_with_an_exit_code_not_anything_truthy(self):
        good = [
            ["cargo test -> exit 0 (all ok)"],
            ["git diff --check -> exit 0", "cargo clippy -> exit 101 (failed)"],
            ["cargo -> exit -1"],
            ["x (exit 0)"], ["cargo test: exit 2"], ["cargo test -> exit 0."], ["cargo test, exit 1"],  # other separators in use
            ["cargo test -> exit 0failed -> exit 1"], ["cargo test -> exit 0" + chr(10)], ["cargo test -> exit0"],  # a later complete code, a newline, `exit0`
            [{"cmd": "cargo audit --file foundation/Cargo.lock", "rc": 0}],  # the shape one seat already signs
            [{"garbage": True}, "git diff --check -> exit 0"],  # one real entry among noise still counts
            ["cargo test -> exit0,54 integration tests"],  # real records write the code with no space
            [{"command": "python -m unittest discover -s scripts", "exit_code": 0}],  # PLATFORM_SPEC section 3
            [{"command": "cargo test", "exit_code": 101, "passed": 5, "failed": 1, "count": 6, "log_ref": "tmp/x.log",
              "log_sha256": "0" * 64, "log": "tail"}],
        ]
        for commands in good:
            self.assertTrue(rc.usable_evidence(commands), commands)
        bad = [
            None, [], {}, "", "cargo test -> exit 0",  # not a list
            {"garbage": True}, [{"garbage": True}], [123], [None], [["cargo test -> exit 0"]],  # Codex's counterexample and kin
            {"cargo test -> exit 0": True},  # a dict iterates as its keys, which must not be read as entries
            ["cargo test"], ["cargo test -> exited 0"], ["exit 0"], [" -> exit 0"], ["cargo preexit 0"],  # no command or no exit code
            [{"cmd": "x", "rc": True}], [{"cmd": "x", "rc": "0"}], [{"cmd": "x", "rc": 0.5}],  # rc must be an integer
            [{"cmd": "", "rc": 0}], [{"cmd": "  ", "rc": 0}], [{"cmd": 1, "rc": 0}],  # cmd must be text
            [{"cmd": "x", "rc": 0, "extra": 1}], [{"cmd": "x"}], [{"rc": 0}],  # exactly those two keys
            [{"command": "x"}], [{"exit_code": 0}], [{"command": "x", "exit_code": True}],  # the spec shape needs both, real ints
            [{"command": "x", "exit_code": "0"}], [{"command": " ", "exit_code": 0}], [{"command": 1, "exit_code": 0}],
            [{"command": "x", "exit_code": 0, "surprise": 1}],  # only the documented extras are allowed
            ["x -> exited 0"], ["x preexit0"],
            ["cargo test -> exit 0failed"], ["cargo test -> exit 0_"], ["cargo test -> exit 12abc"], ["cargo test -> exit -1x"],  # Codex on #226
            ["cargo test -> exit 0.5"], ["the test might exit 0"], ["cargo test exit 0"], ["cargo test -> exit 1.25 seconds"],  # Codex on #226
            ["cargo test -> exit 0failed", "x -> exit"],  # a malformed entry does not become usable next to another bad one
        ]
        for commands in bad:
            self.assertFalse(rc.usable_evidence(commands), commands)

    def test_a_malformed_record_neither_satisfies_box_5_nor_covers_another(self):
        for garbage in ({"garbage": True}, [{"garbage": True}], "cargo test -> exit 0", ["cargo test"], [{"cmd": "x", "rc": True}]):
            bare = rec(H1, reviewer="codex")
            bare["comment"] = 1
            covering = rec(H1, reviewer="codex")
            covering["comment"] = 2
            covering["commands"] = garbage
            # Codex's counterexample on #226: the malformed record used to clear the bare one and pass box 5 itself.
            self.assertEqual(rc.unevidenced_verdicts([bare, covering]), [1, 2], garbage)
        real = rec(H1, reviewer="codex")
        real["comment"] = 3
        real["commands"] = ["cargo test -> exit 0"]
        bare = rec(H1, reviewer="codex")
        bare["comment"] = 1
        self.assertEqual(rc.unevidenced_verdicts([bare, real]), [])

    def test_nothing_counted_means_nothing_missing_and_all_bare_means_all_missing(self):
        self.assertEqual(rc.unevidenced_verdicts([]), [])
        counted = self.counted((3, "claude", False), (6, "gemini", False))
        self.assertEqual(rc.unevidenced_verdicts(counted), [3, 6])


if __name__ == "__main__":
    unittest.main()
