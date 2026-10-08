import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import merge_gate as mg  # noqa: E402

COVERED = {"covered": True}
UNCOVERED = {"covered": False}


class DiffCheckCommand(unittest.TestCase):
    """The plan's whitespace check must see a committed error; the no-revision form never could."""

    def run_plan_check(self, plan_commands):
        import shutil
        import subprocess
        import tempfile
        directory = tempfile.mkdtemp(prefix="merge-gate-ws-")
        self.addCleanup(shutil.rmtree, directory, True)

        def git(*args):
            return subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.com", "-C", directory, *args],
                                  capture_output=True, text=True)

        git("init", "-q", "-b", "main")
        with open(os.path.join(directory, "a.txt"), "w", encoding="utf-8", newline="\n") as handle:
            handle.write("clean\n")
        git("add", ".")
        git("commit", "-q", "-m", "base")
        git("update-ref", "refs/remotes/origin/main", "HEAD")
        git("checkout", "-q", "-b", "pr")
        with open(os.path.join(directory, "a.txt"), "w", encoding="utf-8", newline="\n") as handle:
            handle.write("trailing whitespace   \n")
        git("add", ".")
        git("commit", "-q", "-m", "committed whitespace error")
        command = [part.replace("{base}", "origin/main") for part in plan_commands[0]]
        return git(*command[1:]).returncode, command

    def test_both_plans_fail_on_a_whitespace_error_committed_by_the_pr(self):
        for plan in (mg.FULL_GATE, mg.LIGHT_GATE):
            returncode, command = self.run_plan_check(plan)
            self.assertEqual(command[:3], ["git", "diff", "--check"])
            self.assertIn("origin/main...HEAD", command)
            self.assertNotEqual(returncode, 0, command)

    def test_the_old_no_revision_form_passes_a_committed_whitespace_error(self):
        # The bug being fixed, kept as the control: this is what the plan used to run.
        returncode, _ = self.run_plan_check([["git", "diff", "--check"]])
        self.assertEqual(returncode, 0)


class ChangedPaths(unittest.TestCase):
    """changed_paths against a real throwaway repository: the protected path must survive a rename."""

    def make_repo(self):
        import subprocess
        import tempfile
        directory = tempfile.mkdtemp(prefix="merge-gate-test-")
        self.addCleanup(__import__("shutil").rmtree, directory, True)

        def git(*args):
            result = subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.com", "-C", directory, *args],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            return result.stdout.strip()

        git("init", "-q", "-b", "main")
        os.makedirs(os.path.join(directory, "governance"))
        with open(os.path.join(directory, "governance", "identities.json"), "w", encoding="utf-8", newline="\n") as handle:
            handle.write('{"seats": {}, "padding": "' + "x" * 300 + '"}\n')
        git("add", ".")
        git("commit", "-q", "-m", "base")
        git("update-ref", "refs/remotes/origin/main", "HEAD")
        git("checkout", "-q", "-b", "pr")
        return directory, git

    def test_a_non_ascii_or_spaced_path_is_listed_as_itself_and_still_protected(self):
        # Probe: git quoted '.github/workflows/\u00e9.yml' as '".github/workflows/\\303\\251.yml"', no protected prefix
        # matched it, and decide() said eligible=True for a covered PR touching an owner-only workflow.
        directory, git = self.make_repo()
        os.makedirs(os.path.join(directory, ".github", "workflows"))
        os.makedirs(os.path.join(directory, "docs"))
        names = [".github/workflows/é.yml", "docs/with space.md"] + ([] if os.name == "nt" else ['docs/quo"te.md'])
        for name in names:
            with open(os.path.join(directory, name), "w", encoding="utf-8", newline="\n") as handle:
                handle.write("x\n")
        git("add", "-A")
        git("commit", "-q", "-m", "odd names")
        paths = mg.changed_paths(directory, "main", git("rev-parse", "HEAD"))
        self.assertEqual(sorted(paths), sorted(names))
        self.assertEqual(mg.touches_protected(paths), [".github/workflows/\u00e9.yml"])
        self.assertEqual(mg.tier_of(paths)[0], "A")
        decision = mg.decide({"covered": True}, paths, "main", True)
        self.assertFalse(decision["eligible"])
        self.assertIn("f: touches protected paths", decision["reasons"][0])

    def test_a_breaker_file_under_a_directory_with_a_space_is_found(self):
        directory, git = self.make_repo()
        os.makedirs(os.path.join(directory, "ops dir"))
        with open(os.path.join(directory, "ops dir", "circuit_breaker_override.toml"), "w", encoding="utf-8",
                  newline="\n") as handle:
            handle.write("tripped = true\n")
        git("add", "-A")
        git("commit", "-q", "-m", "breaker")
        git("update-ref", "refs/remotes/origin/main", "HEAD")
        self.assertEqual(mg.breaker_files(directory, "main"), ["ops dir/circuit_breaker_override.toml"])

    def test_renaming_an_owner_only_file_still_reports_the_protected_source_path(self):
        directory, git = self.make_repo()
        git("mv", "governance/identities.json", "governance/other.json")
        git("commit", "-q", "-m", "rename")
        paths = mg.changed_paths(directory, "main", git("rev-parse", "HEAD"))
        self.assertIn("governance/identities.json", paths)
        self.assertIn("governance/other.json", paths)
        self.assertEqual(mg.tier_of(paths)[0], "A")
        self.assertFalse(mg.decide(COVERED, paths, "main", True)["eligible"])

    def test_an_ordinary_edit_is_listed_once_and_stays_tier_b(self):
        directory, git = self.make_repo()
        with open(os.path.join(directory, "governance", "notes.md"), "w", encoding="utf-8", newline="\n") as handle:
            handle.write("notes\n")
        git("add", ".")
        git("commit", "-q", "-m", "notes")
        paths = mg.changed_paths(directory, "main", git("rev-parse", "HEAD"))
        self.assertEqual(paths, ["governance/notes.md"])
        self.assertEqual(mg.tier_of(paths)[0], "B")


class Plan(unittest.TestCase):
    def test_docs_only_changes_get_the_light_gate(self):
        self.assertEqual(mg.gate_plan(["governance/PLATFORM_SPEC.md", "governance/identities.json"])["level"], "light")

    def test_anything_else_gets_the_full_gate(self):
        for paths in (["foundation/crates/foundation_cli/src/main.rs"], ["Cargo.lock"], ["scripts/x.py"],
                      ["governance/a.md", "build.rs"], []):
            self.assertEqual(mg.gate_plan(paths)["level"], "full", paths)

    def test_unlisted_doc_directories_are_not_trusted(self):
        self.assertEqual(mg.gate_plan(["random/notes.md"])["level"], "full")


class Decision(unittest.TestCase):
    def test_covered_clean_main_pr_is_eligible(self):
        d = mg.decide(COVERED, ["governance/x.md"], "main", True)
        self.assertTrue(d["eligible"])
        self.assertEqual(d["reasons"], [])

    def test_uncovered_pr_is_blocked_with_the_condition_named(self):
        d = mg.decide(UNCOVERED, ["governance/x.md"], "main", True)
        self.assertFalse(d["eligible"])
        self.assertTrue(d["reasons"][0].startswith("a:"))

    def test_trial_merge_conflict_blocks(self):
        d = mg.decide(COVERED, ["governance/x.md"], "main", False)
        self.assertTrue(any(r.startswith("b:") for r in d["reasons"]))

    def test_non_main_base_blocks(self):
        d = mg.decide(COVERED, ["governance/x.md"], "codex/foundry-transition", True)
        self.assertTrue(any(r.startswith("d:") for r in d["reasons"]))

    def test_protected_paths_need_owner_confirmation(self):
        for path in ("AGENTS.md", ".github/workflows/ci.yml", "SECURITY.md", "CONTRIBUTING.md",
                     ".github/CODEOWNERS", "governance/AUTOMATED_MERGE_POLICY.md"):
            d = mg.decide(COVERED, [path], "main", True)
            self.assertFalse(d["eligible"], path)
            self.assertTrue(any(r.startswith("f:") for r in d["reasons"]))
        self.assertTrue(mg.decide(COVERED, ["AGENTS.md"], "main", True, owner_confirmed_protected=True)["eligible"])

    def test_the_seat_identity_map_is_owner_only(self):
        # It decides who counts as an independent reviewer and which keys verify; a login or key added by an
        # auto-merged PR would make that PR's author's later verdicts count.
        for paths in (["governance/identities.json"], ["governance/identities.json", "docs/a.md"]):
            self.assertEqual(mg.tier_of(paths)[0], "A", paths)
            d = mg.decide(COVERED, paths, "main", True)
            self.assertFalse(d["eligible"], paths)
            self.assertTrue(any(r.startswith("f:") for r in d["reasons"]), paths)
        self.assertTrue(mg.decide(COVERED, ["governance/identities.json"], "main", True,
                                  owner_confirmed_protected=True)["eligible"])
        # The other governance json files stay Tier B.
        self.assertEqual(mg.tier_of(["governance/other.json"])[0], "B")

    def test_the_decision_path_and_the_checklist_share_one_owner_only_list(self):
        self.assertIs(mg.PROTECTED, mg.TIER_A_PATHS)
        for path in mg.TIER_A_PATHS:
            self.assertTrue(mg.touches_protected([path]), path)
            self.assertEqual(mg.tier_of([path])[0], "A", path)

    def test_every_blocking_reason_is_reported_together(self):
        d = mg.decide(UNCOVERED, ["AGENTS.md"], "codex/x", False)
        self.assertEqual(sorted(r[0] for r in d["reasons"]), ["a", "b", "d", "f"])


def good_facts(**kw):
    facts = {
        "head": "a" * 40, "base_ref": "main", "base_tip": "b" * 40, "default_branch": "main",
        "ci": {"state": "success", "detail": "2 checks: SUCCESS"}, "trial_merge_ok": True,
        "accept_seats": ["mimo", "codex"], "covered": True, "threads_open": 0,
        "counted_verdicts": 2, "verdicts_missing_evidence": [], "revertible": True,
        "breaker_tripped": False, "trust": (5, 5), "category": "governance/*.md", "tier": ("B", ""),
        "standing_changes_requested": False,
    }
    facts.update(kw)
    return facts


class FactParsing(unittest.TestCase):
    def test_a_commented_tripped_false_does_not_hide_a_real_trip(self):
        text = "# tripped = false\ntripped = true\nreason = 'x'\n"
        self.assertIs(mg.breaker_state(text), True)

    def test_a_real_boolean_is_read_and_everything_else_fails_closed(self):
        self.assertIs(mg.breaker_state("tripped = false\n"), False)
        self.assertIs(mg.breaker_state("tripped = true\n"), True)
        for bad in ("", "reason = 'x'\n", 'tripped = "false"\n', "tripped = 0\n", "tripped = = false",
                    "# tripped = false\n"):
            self.assertIsNone(mg.breaker_state(bad), bad)

    @staticmethod
    def threads(nodes, has_next):
        info = {} if has_next is None else {"pageInfo": {"hasNextPage": has_next}}
        return {"data": {"repository": {"pullRequest": {"reviewThreads": dict(info, nodes=nodes)}}}}

    def test_a_truncated_page_of_resolved_threads_is_unknown_not_zero(self):
        resolved = [{"isResolved": True}] * 100
        self.assertIsNone(mg.open_threads(self.threads(resolved, True)))

    def test_a_complete_page_is_counted(self):
        nodes = [{"isResolved": True}, {"isResolved": False}, {"isResolved": False}]
        self.assertEqual(mg.open_threads(self.threads(nodes, False)), 2)
        self.assertEqual(mg.open_threads(self.threads([], False)), 0)

    def test_missing_pagination_info_or_a_malformed_response_is_unknown(self):
        self.assertIsNone(mg.open_threads(self.threads([{"isResolved": True}], None)))
        self.assertIsNone(mg.open_threads({}))
        self.assertIsNone(mg.open_threads({"data": None}))


class Checklist(unittest.TestCase):
    def test_all_boxes_green_means_would_merge(self):
        lines, ok, first = mg.checklist(good_facts())
        self.assertTrue(ok)
        self.assertIsNone(first)
        self.assertTrue(all(l.startswith("[x]") for l in lines), lines)

    def test_each_failing_condition_stops_the_loop_at_its_box(self):
        cases = {
            "1": {"ci": {"state": "failure", "detail": "x"}},
            "2": {"trial_merge_ok": False},
            "2a": {"base_ref": "codex/foundry-transition"},
            "3": {"accept_seats": ["mimo"]},
            "4": {"threads_open": 2},
            "5": {"verdicts_missing_evidence": [4]},
            "6": {"revertible": False},
            "7": {"breaker_tripped": True},
            "8": {"trust": (2, 5)},
        }
        for box_no, change in cases.items():
            lines, ok, first = mg.checklist(good_facts(**change))
            self.assertFalse(ok, box_no)
            self.assertEqual(first, box_no)

    def test_an_unevaluable_box_is_blocked_and_fails_the_gate(self):
        for change, text in (({"ci": None}, "CI"), ({"revertible": None}, "classification"),
                             ({"breaker_tripped": None}, "circuit"), ({"threads_open": None}, "thread")):
            lines, ok, first = mg.checklist(good_facts(**change))
            self.assertFalse(ok)
            self.assertTrue(any(l.startswith("[blocked:") for l in lines), lines)

    def test_pending_ci_is_not_success(self):
        self.assertEqual(mg.ci_state([{"status": "IN_PROGRESS", "conclusion": ""}])["state"], "pending")
        self.assertEqual(mg.ci_state([])["state"], "missing")
        self.assertEqual(mg.ci_state([{"status": "COMPLETED", "conclusion": "SUCCESS"}])["state"], "success")
        self.assertEqual(mg.ci_state([{"status": "COMPLETED", "conclusion": "SUCCESS"},
                                      {"status": "COMPLETED", "conclusion": "FAILURE"}])["state"], "failure")

    def test_tier_a_paths_are_owner_only(self):
        for path in (".github/workflows/ci.yml", "AGENTS.md", "SECURITY.md", "CONTRIBUTING.md", "governance/AUTOMATED_MERGE_POLICY.md"):
            self.assertEqual(mg.tier_of([path])[0], "A", path)
            lines, ok, first = mg.checklist(good_facts(tier=mg.tier_of([path])))
            self.assertFalse(ok)
            self.assertTrue(first.startswith("0"))
        self.assertEqual(mg.tier_of(["governance/PLATFORM_SPEC.md"])[0], "B")

    def test_two_seats_must_also_be_covered(self):
        lines, ok, first = mg.checklist(good_facts(covered=False))
        self.assertFalse(ok)
        self.assertEqual(first, "3")

    def test_standing_changes_requested_fails_the_threads_box(self):
        lines, ok, first = mg.checklist(good_facts(standing_changes_requested=True))
        self.assertEqual(first, "4")


class Reconcile(unittest.TestCase):
    """decide() alone must never print a verdict: the checklist is sufficient to block."""

    def test_tripped_breaker_blocks_an_otherwise_eligible_decide(self):
        lines, ok, first = mg.reconcile(True, good_facts(breaker_tripped=True))
        self.assertFalse(ok)
        self.assertEqual(first, "7")

    def test_missing_ci_fails_closed_even_when_decide_is_ok(self):
        facts = good_facts(ci={"state": "missing", "detail": "no CI status found"})
        lines, ok, first = mg.reconcile(True, facts)
        self.assertFalse(ok)
        self.assertEqual(first, "1")

    def test_open_threads_block_even_when_decide_is_ok(self):
        lines, ok, first = mg.reconcile(True, good_facts(threads_open=3))
        self.assertFalse(ok)
        self.assertEqual(first, "4")

    def test_green_checklist_cannot_save_a_failed_decide(self):
        lines, ok, first = mg.reconcile(False, good_facts())
        self.assertFalse(ok)

    def test_both_green_is_eligible(self):
        lines, ok, first = mg.reconcile(True, good_facts())
        self.assertTrue(ok)
        self.assertIsNone(first)


class TierClassification(unittest.TestCase):
    """tier_of must fail closed: Tier B only for positively recognised safe families."""

    def test_unrecognised_code_paths_are_unclassified_and_blocked(self):
        tier, why = mg.tier_of(["foundation/crates/foundation_cli/src/main.rs"])
        self.assertEqual(tier, "unclassified")
        lines, ok, first = mg.checklist(good_facts(tier=(tier, why)))
        self.assertFalse(ok)
        self.assertEqual(first, "0 (%s)" % why)
        self.assertIn("[blocked:", lines[0])

    def test_safe_doc_and_script_families_stay_tier_b(self):
        for paths in (
            ["governance/X.md"],
            ["governance/IDENTITY_SETUP.md"],
            ["docs/y/z.md"],
            ["architecture/spec.md"],
            ["scripts/tool.py", "scripts/test_tool.py"],
            ["inventories/REPORT.md"],
            ["contracts/schema.json"],
        ):
            self.assertEqual(mg.tier_of(paths)[0], "B", paths)

    def test_owner_only_paths_still_win_over_safe_families(self):
        tier, why = mg.tier_of(["governance/X.md", "AGENTS.md"])
        self.assertEqual(tier, "A")
        self.assertIn("AGENTS.md", why)

    def test_empty_diff_is_still_classified_b(self):
        self.assertEqual(mg.tier_of([])[0], "B")

    def test_unclassified_renders_blocked_in_checklist_output(self):
        tier, why = mg.tier_of(["crates/mystery/src/lib.rs"])
        lines, ok, first = mg.checklist(good_facts(tier=(tier, why)))
        self.assertFalse(ok)
        self.assertTrue(lines[0].startswith("[blocked:"))
        self.assertIn("never assumed safe", why)


class ChecklistReadsFactsStrictly(unittest.TestCase):
    """A missing fact or one of the wrong type blocks its box; it never reads as fine. Probe: removing `tier`,
    `standing_changes_requested` or `verdicts_missing_evidence` from an otherwise green facts dict still said would_merge,
    and so did `threads_open=False`, `breaker_tripped=0`, `accept_seats=["a", "a"]` and `accept_seats="ab"`."""

    def merges(self, **override):
        facts = good_facts()
        for key, value in override.items():
            if value is MISSING:
                del facts[key]
            else:
                facts[key] = value
        return mg.checklist(facts)[1]

    def test_the_green_facts_still_merge(self):
        self.assertTrue(self.merges())

    def test_a_missing_fact_blocks(self):
        for key in ("tier", "standing_changes_requested", "verdicts_missing_evidence", "counted_verdicts", "threads_open",
                    "breaker_tripped", "covered", "accept_seats", "revertible"):
            self.assertFalse(self.merges(**{key: MISSING}), key)

    def test_a_missing_tier_is_unclassified_not_tier_b(self):
        facts = good_facts()
        del facts["tier"]
        lines, ok, first = mg.checklist(facts)
        self.assertFalse(ok)
        self.assertIn("tier fact missing or malformed", first)

    def test_a_value_of_the_wrong_type_blocks(self):
        for key, values in {
            "threads_open": (False, 0.0, True, "0"),
            "counted_verdicts": ("2", 2.5, True),
            "breaker_tripped": (0, "", None, "false"),
            "standing_changes_requested": (0, None, "no"),
            "verdicts_missing_evidence": (None, 0, ""),
            "covered": (1, "yes"),
            "tier": ("B", ("B",), ("B", 1), None),
        }.items():
            for value in values:
                try:
                    merged = self.merges(**{key: value})
                except Exception as error:  # a crash is also not a merge, but report it
                    self.fail(f"{key}={value!r} raised {error!r}")
                self.assertFalse(merged, f"{key}={value!r}")

    def test_seats_must_be_two_distinct_non_empty_names_in_a_list(self):
        for seats in (["a", "a"], ["", ""], ["x", " x "], "ab", ("claude", "codex"), ["claude", None], ["claude"]):
            self.assertFalse(self.merges(accept_seats=seats), seats)
        self.assertTrue(self.merges(accept_seats=["claude", "codex"]))

    def test_malformed_box_3_facts_render_as_blocked_not_as_an_unchecked_box(self):
        # Codex P2 on #122: "unavailable" must be distinguishable from a valid fact reporting too few verdicts.
        for override in ({"accept_seats": "ab"}, {"accept_seats": MISSING}, {"covered": MISSING}, {"covered": 1}):
            facts = good_facts()
            for key, value in override.items():
                if value is MISSING:
                    del facts[key]
                else:
                    facts[key] = value
            lines = mg.checklist(facts)[0]
            box3 = [line for line in lines if " 3. " in line][0]
            self.assertTrue(box3.startswith("[blocked: accepting seats or coverage unavailable]"), (override, box3))
        facts = good_facts(accept_seats=["claude"])
        box3 = [line for line in mg.checklist(facts)[0] if " 3. " in line][0]
        self.assertTrue(box3.startswith("[ ]"), "a valid fact reporting one seat is an unchecked box, not blocked")

    def test_json_style_lists_for_the_tier_are_accepted(self):
        # facts read back from JSON hold lists, not tuples
        self.assertTrue(self.merges(tier=["B", ""]))


MISSING = object()


if __name__ == "__main__":
    unittest.main()
