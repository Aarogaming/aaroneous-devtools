import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import merge_gate as mg  # noqa: E402

COVERED = {"covered": True}
UNCOVERED = {"covered": False}


LOCK_BEFORE = '''version = 4

[[package]]
name = "demo"
version = "0.1.0"
dependencies = ["rustls"]

[[package]]
name = "rustls"
version = "0.23.45"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
dependencies = ["ring"]
'''
LOCK_AFTER = LOCK_BEFORE.replace('"0.23.45"', '"0.23.46"').replace('"' + 'a' * 64 + '"', '"' + 'b' * 64 + '"')


class LockfileRevertibility(unittest.TestCase):
    def test_only_registry_version_and_checksum_changes_are_qualified(self):
        self.assertTrue(mg.registry_version_checksum_update(LOCK_BEFORE, LOCK_AFTER))
        self.assertTrue(mg.registry_version_checksum_update(LOCK_AFTER, LOCK_BEFORE))

    def test_unknown_lockfile_shapes_stay_unclassified(self):
        bad_versions = [
            LOCK_AFTER.replace('dependencies = ["ring"]', 'dependencies = ["other"]'),
            LOCK_AFTER.replace('name = "rustls"', 'name = "other"'),
            LOCK_AFTER.replace('registry+https://github.com/rust-lang/crates.io-index', 'git+https://example.invalid/repo'),
            LOCK_AFTER.replace('registry+https://github.com/rust-lang/crates.io-index', 'registry+https://example.invalid/index'),
            LOCK_AFTER.replace('version = 4', 'version = 3'),
            LOCK_AFTER.replace('version = 4', 'version = 5'),
            LOCK_AFTER.replace('version = 4', 'version = true'),
            LOCK_AFTER.replace('"0.23.46"', '"invalid"'),
            LOCK_AFTER.replace('"' + 'b' * 64 + '"', '"not-a-checksum"'),
            LOCK_AFTER.replace('"' + 'b' * 64 + '"', '"' + 'a' * 64 + '"'),
            LOCK_AFTER.replace('"0.23.46"', '"0.23.45"'),
            LOCK_AFTER.replace('[[package]]\nname = "demo"\nversion = "0.1.0"\ndependencies = ["rustls"]\n\n', ''),
            LOCK_AFTER + '\n[[package]]\nname = "extra"\nversion = "1.0.0"\n',
            LOCK_AFTER + '\n[metadata]\nnew = "unqualified"\n',
            'version = 4\npackage = "not a package list"',
            'malformed = [',
            LOCK_BEFORE,
        ]
        for candidate in bad_versions:
            with self.subTest(candidate=candidate):
                self.assertFalse(mg.registry_version_checksum_update(LOCK_BEFORE, candidate))

    def test_no_changed_local_or_git_package_is_qualified(self):
        for source in ('', 'source = "git+https://example.invalid/repo"\n'):
            old = LOCK_BEFORE.replace('source = "registry+https://github.com/rust-lang/crates.io-index"\n', source)
            new = LOCK_AFTER.replace('source = "registry+https://github.com/rust-lang/crates.io-index"\n', source)
            self.assertFalse(mg.registry_version_checksum_update(old, new))

    def test_input_bound_and_incomplete_documents_fail_closed(self):
        for old, new in (("#" * (2 * 1024 * 1024 + 1), LOCK_AFTER), ("", ""), ('version = 4', 'version = 4')):
            self.assertFalse(mg.registry_version_checksum_update(old, new))


class RevertibilityFromGit(unittest.TestCase):
    def setUp(self):
        import subprocess
        import tempfile
        import shutil
        self.directory = tempfile.mkdtemp(prefix="merge-lock-revert-")
        self.addCleanup(shutil.rmtree, self.directory, True)

        def git(*args):
            result = subprocess.run([
                "git", "-C", self.directory, "-c", "user.name=test", "-c", "user.email=test@example.com",
                "-c", "commit.gpgsign=false", *args,
            ], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            return result.stdout.strip()

        self.git = git
        git("init", "-q", "-b", "main")
        os.makedirs(os.path.join(self.directory, "foundation"))
        self.write_lock(LOCK_BEFORE)
        git("add", ".")
        git("commit", "-q", "-m", "before")
        self.base = git("rev-parse", "HEAD")
        self.write_lock(LOCK_AFTER)
        git("add", ".")
        git("commit", "-q", "-m", "after")
        self.head = git("rev-parse", "HEAD")

    def write_lock(self, text):
        with open(os.path.join(self.directory, "foundation", "Cargo.lock"), "w", encoding="utf-8", newline="\n") as handle:
            handle.write(text)

    def classify(self, paths=None, base=None, head=None):
        return mg.repository_revertibility(self.directory, base or self.base, head or self.head,
                                            paths if paths is not None else ["foundation/Cargo.lock"])

    def test_pinned_blob_comparison_ignores_dirty_checkout(self):
        self.write_lock('broken = [')
        self.assertTrue(self.classify())
        self.assertTrue(self.classify(["foundation/Cargo.lock", "governance/COORDINATION_QUEUE.md"]))

    def test_unrelated_paths_cannot_piggyback_on_a_lock_update(self):
        for path in ("foundation/Cargo.toml", "foundation/src/lib.rs", ".github/workflows/ci.yml", "other/Cargo.lock"):
            self.assertIsNone(self.classify(["foundation/Cargo.lock", path]))
        self.assertIsNone(self.classify([]))

    def test_missing_and_symbolic_objects_stay_unknown(self):
        self.assertIsNone(self.classify(head="0" * 40))
        self.assertIsNone(self.classify(head="HEAD"))
        self.assertIsNone(self.classify(base="missing-ref"))

    def test_git_replacement_cannot_disguise_a_dependency_change(self):
        self.write_lock(LOCK_AFTER.replace('dependencies = ["ring"]', 'dependencies = ["other"]'))
        self.git("add", "foundation/Cargo.lock")
        self.git("commit", "-q", "-m", "unqualified dependency")
        bad = self.git("rev-parse", "HEAD")
        self.git("replace", bad, self.head)
        self.assertIsNone(self.classify(head=bad))
        self.assertTrue(self.classify(head=self.head))

    def test_symlink_mode_and_dependency_changes_stay_unknown(self):
        blob = self.git("hash-object", "-w", "--", "foundation/Cargo.lock")
        self.git("update-index", "--cacheinfo", f"120000,{blob},foundation/Cargo.lock")
        self.git("commit", "-q", "-m", "symlink")
        self.assertIsNone(self.classify(head=self.git("rev-parse", "HEAD")))
        self.write_lock(LOCK_AFTER.replace('dependencies = ["ring"]', 'dependencies = ["other"]'))
        self.git("add", "foundation/Cargo.lock")
        self.git("commit", "-q", "-m", "dependency change")
        self.assertIsNone(self.classify(head=self.git("rev-parse", "HEAD")))

    def test_checklist_still_requires_every_other_gate(self):
        reversible = self.classify()
        self.assertTrue(reversible)
        for change in ({"ci": {"state": "failure", "detail": "failed"}}, {"accept_seats": ["gemini"]},
                       {"verdicts_missing_evidence": [3]}, {"breaker_tripped": True}, {"threads_open": 1}):
            lines, eligible, first = mg.checklist(good_facts(revertible=reversible, **change))
            self.assertFalse(eligible, lines)
            self.assertIsNotNone(first)

    def test_live_fact_collection_uses_the_proof_without_filling_other_boxes(self):
        from unittest import mock
        self.git("update-ref", "refs/remotes/origin/main", self.base)
        pr = {"number": 222, "head": self.head, "base": "main", "draft": False}
        row = {"covered": True, "accept_seats": ["claude", "gemini"], "counted_verdicts": 2,
               "verdicts_missing_evidence": [3], "reasons": []}
        with mock.patch.object(mg.rc, "gh_json", return_value={}):
            facts = mg.gather_facts("owner/repo", self.directory, pr, row,
                                    ["foundation/Cargo.lock", "governance/COORDINATION_QUEUE.md"], True, "main")
        self.assertIs(facts["revertible"], True)
        self.assertEqual(facts["verdicts_missing_evidence"], [3])
        self.assertEqual(facts["ci"]["state"], "missing")
        self.assertIsNone(facts["threads_open"])
        self.assertIsNone(facts["breaker_tripped"])
        self.assertFalse(mg.checklist(facts)[1])


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
    """changed_paths against a real throwaway repository."""

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

    def test_a_non_ascii_or_spaced_path_is_listed_as_itself(self):
        # Probe: git quoted the e-acute workflow path as a backslash-escaped octal string, which broke
        # downstream consumers that matched on raw path strings.
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
        decision = mg.decide({"covered": True}, paths, "main", True)
        self.assertTrue(decision["eligible"])

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

    def test_renaming_a_file_reports_both_the_old_and_new_path(self):
        directory, git = self.make_repo()
        git("mv", "governance/identities.json", "governance/other.json")
        git("commit", "-q", "-m", "rename")
        paths = mg.changed_paths(directory, "main", git("rev-parse", "HEAD"))
        self.assertIn("governance/identities.json", paths)
        self.assertIn("governance/other.json", paths)
        self.assertTrue(mg.decide(COVERED, paths, "main", True)["eligible"])

    def test_an_ordinary_edit_is_listed_once(self):
        directory, git = self.make_repo()
        with open(os.path.join(directory, "governance", "notes.md"), "w", encoding="utf-8", newline="\n") as handle:
            handle.write("notes\n")
        git("add", ".")
        git("commit", "-q", "-m", "notes")
        paths = mg.changed_paths(directory, "main", git("rev-parse", "HEAD"))
        self.assertEqual(paths, ["governance/notes.md"])


class AdditiveIdentityRegistration(unittest.TestCase):
    """The owner-decision 2026-10-11 exemption: a seat filling in its own already-provisioned login/key
    is paperwork, not a trust decision, and bypasses condition 3 — but only for exactly that shape."""

    BASE_DOC = ('{"owner": ["Aarogaming"], "seats": {'
                '"claude": {"logins": [], "keys": ["key-claude"]}, '
                '"mimo": {"logins": [], "keys": []}'
                '}}\n')

    def make_repo(self, base_doc=None):
        import subprocess
        import tempfile
        directory = tempfile.mkdtemp(prefix="merge-gate-identity-test-")
        self.addCleanup(__import__("shutil").rmtree, directory, True)

        def git(*args):
            result = subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.com", "-C", directory, *args],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            return result.stdout.strip()

        git("init", "-q", "-b", "main")
        os.makedirs(os.path.join(directory, "governance"))
        with open(os.path.join(directory, "governance", "identities.json"), "w", encoding="utf-8",
                  newline="\n") as handle:
            handle.write(base_doc if base_doc is not None else self.BASE_DOC)
        git("add", ".")
        git("commit", "-q", "-m", "base")
        git("update-ref", "refs/remotes/origin/main", "HEAD")
        git("checkout", "-q", "-b", "pr")
        return directory, git

    def write_and_commit(self, directory, git, doc):
        with open(os.path.join(directory, "governance", "identities.json"), "w", encoding="utf-8",
                  newline="\n") as handle:
            handle.write(doc)
        git("add", ".")
        git("commit", "-q", "-m", "identities update")
        return git("rev-parse", "HEAD")

    def test_appending_a_login_to_an_existing_seat_is_exempt(self):
        directory, git = self.make_repo()
        head = self.write_and_commit(directory, git, (
            '{"owner": ["Aarogaming"], "seats": {'
            '"claude": {"logins": ["aaroneous-claude-seat[bot]"], "keys": ["key-claude"]}, '
            '"mimo": {"logins": [], "keys": []}'
            '}}\n'))
        self.assertTrue(mg.additive_identity_registration(
            directory, "main", head, [mg.IDENTITIES_PATH]))

    def test_appending_a_key_to_a_previously_empty_seat_is_exempt(self):
        directory, git = self.make_repo()
        head = self.write_and_commit(directory, git, (
            '{"owner": ["Aarogaming"], "seats": {'
            '"claude": {"logins": [], "keys": ["key-claude"]}, '
            '"mimo": {"logins": [], "keys": ["key-mimo"]}'
            '}}\n'))
        self.assertTrue(mg.additive_identity_registration(
            directory, "main", head, [mg.IDENTITIES_PATH]))

    def test_a_second_file_in_the_diff_is_not_exempt(self):
        directory, git = self.make_repo()
        with open(os.path.join(directory, "governance", "identities.json"), "w", encoding="utf-8",
                  newline="\n") as handle:
            handle.write('{"owner": ["Aarogaming"], "seats": {'
                         '"claude": {"logins": ["x"], "keys": ["key-claude"]}, "mimo": {"logins": [], "keys": []}}}\n')
        with open(os.path.join(directory, "governance", "notes.md"), "w", encoding="utf-8", newline="\n") as handle:
            handle.write("notes\n")
        git("add", ".")
        git("commit", "-q", "-m", "two files")
        head = git("rev-parse", "HEAD")
        self.assertFalse(mg.additive_identity_registration(
            directory, "main", head, [mg.IDENTITIES_PATH, "governance/notes.md"]))

    def test_adding_a_brand_new_seat_is_not_exempt(self):
        directory, git = self.make_repo()
        head = self.write_and_commit(directory, git, (
            '{"owner": ["Aarogaming"], "seats": {'
            '"claude": {"logins": [], "keys": ["key-claude"]}, "mimo": {"logins": [], "keys": []}, '
            '"antigravity": {"logins": [], "keys": ["key-new"]}'
            '}}\n'))
        self.assertFalse(mg.additive_identity_registration(
            directory, "main", head, [mg.IDENTITIES_PATH]))

    def test_removing_a_seat_is_not_exempt(self):
        directory, git = self.make_repo()
        head = self.write_and_commit(directory, git,
            '{"owner": ["Aarogaming"], "seats": {"claude": {"logins": [], "keys": ["key-claude"]}}}\n')
        self.assertFalse(mg.additive_identity_registration(
            directory, "main", head, [mg.IDENTITIES_PATH]))

    def test_removing_an_existing_key_is_not_exempt(self):
        # A rotation (drop the old key, add a new one) is a credential change, not a job application.
        directory, git = self.make_repo()
        head = self.write_and_commit(directory, git, (
            '{"owner": ["Aarogaming"], "seats": {'
            '"claude": {"logins": [], "keys": ["key-claude-NEW"]}, "mimo": {"logins": [], "keys": []}'
            '}}\n'))
        self.assertFalse(mg.additive_identity_registration(
            directory, "main", head, [mg.IDENTITIES_PATH]))

    def test_changing_the_owner_list_is_not_exempt(self):
        directory, git = self.make_repo()
        head = self.write_and_commit(directory, git, (
            '{"owner": ["Aarogaming", "someone-else"], "seats": {'
            '"claude": {"logins": [], "keys": ["key-claude"]}, "mimo": {"logins": [], "keys": []}'
            '}}\n'))
        self.assertFalse(mg.additive_identity_registration(
            directory, "main", head, [mg.IDENTITIES_PATH]))

    def test_malformed_json_on_either_side_is_not_exempt(self):
        directory, git = self.make_repo(base_doc="not json\n")
        head = self.write_and_commit(directory, git, '{"owner": ["Aarogaming"], "seats": {}}\n')
        self.assertFalse(mg.additive_identity_registration(
            directory, "main", head, [mg.IDENTITIES_PATH]))

    def test_a_no_op_rewrite_is_still_exempt(self):
        # Reordering or re-serialising without removing anything is vacuously additive.
        directory, git = self.make_repo()
        head = self.write_and_commit(directory, git, (
            '{"seats": {"mimo": {"keys": [], "logins": []}, '
            '"claude": {"keys": ["key-claude"], "logins": []}}, "owner": ["Aarogaming"]}\n'))
        self.assertTrue(mg.additive_identity_registration(
            directory, "main", head, [mg.IDENTITIES_PATH]))

    def test_wrong_path_is_not_exempt(self):
        directory, git = self.make_repo()
        head = self.write_and_commit(directory, git,
            '{"owner": ["Aarogaming"], "seats": {"claude": {"logins": ["x"], "keys": ["key-claude"]}, '
            '"mimo": {"logins": [], "keys": []}}}\n')
        self.assertFalse(mg.additive_identity_registration(directory, "main", head, ["governance/other.json"]))
        self.assertFalse(mg.additive_identity_registration(directory, "main", head, []))


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

    def test_no_path_is_owner_gated_anymore(self):
        # Owner-only path gating (former Tier A) was dropped by owner decision 2026-10-09: nothing
        # in aaroneous-devtools is blocked by path alone; the mechanical conditions are the only gate.
        for path in ("AGENTS.md", ".github/workflows/ci.yml", "SECURITY.md", "CONTRIBUTING.md",
                     ".github/CODEOWNERS", "governance/AUTOMATED_MERGE_POLICY.md", "governance/identities.json"):
            d = mg.decide(COVERED, [path], "main", True)
            self.assertTrue(d["eligible"], path)
            self.assertEqual(d["reasons"], [], path)

    def test_every_blocking_reason_is_reported_together(self):
        d = mg.decide(UNCOVERED, ["AGENTS.md"], "codex/x", False)
        self.assertEqual(sorted(r[0] for r in d["reasons"]), ["a", "b", "d"])


def good_facts(**kw):
    facts = {
        "head": "a" * 40, "base_ref": "main", "base_tip": "b" * 40, "default_branch": "main",
        "ci": {"state": "success", "detail": "2 checks: SUCCESS"}, "trial_merge_ok": True,
        "accept_seats": ["mimo", "codex"], "covered": True, "threads_open": 0,
        "counted_verdicts": 2, "verdicts_missing_evidence": [], "revertible": True,
        "breaker_tripped": False, "category": "governance/*.md", "is_draft": False,
        "standing_changes_requested": False, "identity_registration_exempt": False,
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
            "2b": {"is_draft": True},
            "3": {"accept_seats": ["mimo"]},
            "4": {"threads_open": 2},
            "5": {"verdicts_missing_evidence": [4]},
            "6": {"revertible": False},
            "7": {"breaker_tripped": True},
        }
        for box_no, change in cases.items():
            lines, ok, first = mg.checklist(good_facts(**change))
            self.assertFalse(ok, box_no)
            self.assertEqual(first, box_no)

    def test_an_unevaluable_box_is_blocked_and_fails_the_gate(self):
        for change, text in (({"ci": None}, "CI"), ({"revertible": None}, "classification"),
                             ({"breaker_tripped": None}, "circuit"), ({"threads_open": None}, "thread"),
                             ({"is_draft": None}, "draft")):
            lines, ok, first = mg.checklist(good_facts(**change))
            self.assertFalse(ok)
            self.assertTrue(any(l.startswith("[blocked:") for l in lines), lines)

    def test_pending_ci_is_not_success(self):
        self.assertEqual(mg.ci_state([{"status": "IN_PROGRESS", "conclusion": ""}])["state"], "pending")
        self.assertEqual(mg.ci_state([])["state"], "missing")
        self.assertEqual(mg.ci_state([{"status": "COMPLETED", "conclusion": "SUCCESS"}])["state"], "success")
        self.assertEqual(mg.ci_state([{"status": "COMPLETED", "conclusion": "SUCCESS"},
                                      {"status": "COMPLETED", "conclusion": "FAILURE"}])["state"], "failure")

    def test_only_the_newest_run_of_each_check_counts(self):
        def run(name, started, status, conclusion, workflow="CI"):
            return {"name": name, "workflowName": workflow, "startedAt": started, "status": status, "conclusion": conclusion}
        old_fail = run("advisories", "2026-10-14T19:09:00Z", "COMPLETED", "FAILURE")
        new_pass = run("advisories", "2026-10-17T02:45:00Z", "COMPLETED", "SUCCESS")
        # A failure that was re-run successfully no longer blocks, whatever order GitHub lists the runs in.
        self.assertEqual(mg.ci_state([old_fail, new_pass])["state"], "success")
        self.assertEqual(mg.ci_state([new_pass, old_fail])["state"], "success")
        # The newest run decides: a later failure, a later unfinished run or a worse tie still blocks.
        self.assertEqual(mg.ci_state([new_pass, run("advisories", "2026-10-18T00:00:00Z", "COMPLETED", "FAILURE")])["state"], "failure")
        self.assertEqual(mg.ci_state([old_fail, run("advisories", "2026-10-18T00:00:00Z", "IN_PROGRESS", "")])["state"], "pending")
        self.assertEqual(mg.ci_state([new_pass, run("advisories", "2026-10-17T02:45:00Z", "COMPLETED", "FAILURE")])["state"], "failure")
        # Another workflow's check of the same name, another name, or an unnamed entry is never replaced.
        self.assertEqual(mg.ci_state([new_pass, run("advisories", "2026-10-01T00:00:00Z", "COMPLETED", "FAILURE", "Other")])["state"], "failure")
        self.assertEqual(mg.ci_state([new_pass, run("lint", "2026-10-01T00:00:00Z", "COMPLETED", "FAILURE")])["state"], "failure")
        self.assertEqual(mg.ci_state([new_pass, {"status": "COMPLETED", "conclusion": "FAILURE"}])["state"], "failure")
        # Runs without startedAt are ordered by completedAt; a legacy context is matched by its context name.
        self.assertEqual(mg.ci_state([
            {"name": "x", "completedAt": "2026-10-14T00:00:00Z", "status": "COMPLETED", "conclusion": "FAILURE"},
            {"name": "x", "completedAt": "2026-10-17T00:00:00Z", "status": "COMPLETED", "conclusion": "SUCCESS"}])["state"], "success")
        self.assertEqual(mg.ci_state([
            {"context": "ci/legacy", "startedAt": "2026-10-14T00:00:00Z", "status": "COMPLETED", "conclusion": "FAILURE"},
            {"context": "ci/legacy", "startedAt": "2026-10-17T00:00:00Z", "status": "COMPLETED", "conclusion": "SUCCESS"}])["state"], "success")

    def test_two_seats_must_also_be_covered(self):
        lines, ok, first = mg.checklist(good_facts(covered=False))
        self.assertFalse(ok)
        self.assertEqual(first, "3")

    def test_standing_changes_requested_fails_the_threads_box(self):
        lines, ok, first = mg.checklist(good_facts(standing_changes_requested=True))
        self.assertEqual(first, "4")

    def test_identity_exemption_bypasses_boxes_3_and_5_with_zero_verdicts(self):
        lines, ok, first = mg.checklist(good_facts(
            identity_registration_exempt=True, accept_seats=[], covered=False,
            counted_verdicts=0, verdicts_missing_evidence=[]))
        self.assertTrue(ok)
        self.assertIsNone(first)
        box3 = [line for line in lines if " 3. " in line][0]
        box5 = [line for line in lines if " 5. " in line][0]
        self.assertTrue(box3.startswith("[x]"), box3)
        self.assertIn("exempt", box3)
        self.assertTrue(box5.startswith("[x]"), box5)
        self.assertIn("exempt", box5)

    def test_identity_exemption_does_not_bypass_other_boxes(self):
        lines, ok, first = mg.checklist(good_facts(
            identity_registration_exempt=True, accept_seats=[], covered=False, threads_open=2))
        self.assertFalse(ok)
        self.assertEqual(first, "4")

    def test_identity_exemption_must_be_exactly_true(self):
        for value in (False, None, 1, "true"):
            lines, ok, first = mg.checklist(good_facts(
                identity_registration_exempt=value, accept_seats=["mimo"], covered=True))
            self.assertFalse(ok, value)
            self.assertEqual(first, "3", value)

    def test_identity_exemption_absent_falls_through_to_the_ordinary_bar(self):
        facts = good_facts(accept_seats=["mimo"], covered=True)
        del facts["identity_registration_exempt"]
        lines, ok, first = mg.checklist(facts)
        self.assertFalse(ok)
        self.assertEqual(first, "3")


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


class ChecklistReadsFactsStrictly(unittest.TestCase):
    """A missing fact or one of the wrong type blocks its box; it never reads as fine. Probe: removing
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
        for key in ("standing_changes_requested", "verdicts_missing_evidence", "counted_verdicts", "threads_open",
                    "breaker_tripped", "covered", "accept_seats", "revertible"):
            self.assertFalse(self.merges(**{key: MISSING}), key)

    def test_a_value_of_the_wrong_type_blocks(self):
        for key, values in {
            "threads_open": (False, 0.0, True, "0"),
            "counted_verdicts": ("2", 2.5, True),
            "breaker_tripped": (0, "", None, "false"),
            "standing_changes_requested": (0, None, "no"),
            "verdicts_missing_evidence": (None, 0, ""),
            "covered": (1, "yes"),
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


MISSING = object()


if __name__ == "__main__":
    unittest.main()
