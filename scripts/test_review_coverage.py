import json
import os
import sys
import types
import unittest
from unittest import mock

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


if __name__ == "__main__":
    unittest.main()
