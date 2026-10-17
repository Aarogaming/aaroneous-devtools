import os
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import make_attestation as ma  # noqa: E402
import review_coverage as rc  # noqa: E402

HEAD = "a" * 40
BASE = "b" * 40


class Build(unittest.TestCase):
    def test_a_built_block_round_trips_through_the_coverage_parser_and_counts(self):
        record, error = ma.build_record("o/r", 7, HEAD, BASE, "gemini", ["claude"], "accept")
        self.assertIsNone(error)
        structured, problems, legacy = rc.parse_attestations([{"body": "Reviewed.\n\n" + ma.render_block(record)}])
        self.assertEqual((len(structured), problems, legacy), (1, [], []))
        self.assertEqual(structured[0]["verdict"], "accept")
        self.assertTrue(rc.independent(structured[0]))

    def test_the_block_is_json_not_yaml_and_the_verdict_is_lower_case(self):
        record, _ = ma.build_record("o/r", 7, HEAD, BASE, "codex", ["claude"], "needs_revision")
        block = ma.render_block(record)
        self.assertTrue(block.startswith("```attestation\n{"))
        self.assertIn('"verdict": "needs_revision"', block)

    def test_the_mistakes_reviewers_made_by_hand_are_refused(self):
        cases = (
            (dict(verdict="ACCEPT"), "verdict must be one of"),
            (dict(head=HEAD[:12]), "full 40-character"),
            (dict(head=HEAD.upper()), "full 40-character"),
            (dict(implementers=[]), "implementers must be a non-empty list"),
            (dict(scope="delta"), "needs --from"),
            (dict(scope="whole-ish"), "unknown scope"),
        )
        for override, expected in cases:
            args = dict(repo="o/r", pr=7, head=HEAD, base_sha=BASE, reviewer="gemini", implementers=["claude"],
                        verdict="accept")
            args.update(override)
            record, error = ma.build_record(**args)
            self.assertIsNone(record, override)
            self.assertIn(expected, error, override)

    def test_a_delta_attestation_carries_from(self):
        record, error = ma.build_record("o/r", 7, HEAD, BASE, "codex", ["claude"], "accept", "delta", "c" * 40)
        self.assertIsNone(error)
        self.assertEqual(record["from"], "c" * 40)

    def test_an_implementer_cannot_build_an_accept_for_its_own_work_however_the_seat_is_spelled(self):
        for reviewer, implementers in (("claude", ["claude"]), ("Claude Sonnet 5.5", ["claude"]),
                                       ("gemini", ["gemini", "claude"]),
                                       ("Antigravity", ["gemini"])):
            record, error = ma.build_record("o/r", 7, HEAD, BASE, reviewer, implementers, "accept")
            self.assertIsNone(record, (reviewer, implementers))
            self.assertIn("can never count", error)

    def test_a_delta_from_must_be_a_full_lower_case_sha(self):
        # Codex P2 on #112: any non-empty --from was copied in and the accept could never chain.
        for frm in ("c" * 12, ("C" * 40), "not-a-sha"):
            record, error = ma.build_record("o/r", 7, HEAD, BASE, "codex", ["claude"], "accept", "delta", frm)
            self.assertIsNone(record, frm)
            self.assertIn("--from must be the full", error)

    def test_a_non_independent_reviewer_may_still_record_a_rejection(self):
        record, error = ma.build_record("o/r", 7, HEAD, BASE, "claude", ["claude"], "changes_requested")
        self.assertIsNone(error)
        self.assertEqual(record["verdict"], "changes_requested")


class Commands(unittest.TestCase):
    """--command evidence is signed into the record, which is where the merge gate reads it (checklist box 5)."""

    GOOD = ["git diff --stat a..b -> exit 0 (1 file)", "python -m unittest -> exit code 0 (Ran 19 tests, OK)"]

    def test_commands_are_carried_in_the_record_and_survive_the_coverage_parser(self):
        record, error = ma.build_record("o/r", 7, HEAD, BASE, "gemini", ["claude"], "accept", commands=self.GOOD)
        self.assertIsNone(error)
        self.assertEqual(record["commands"], self.GOOD)
        structured, problems, _ = rc.parse_attestations([{"body": ma.render_block(record)}])
        self.assertEqual(problems, [])
        self.assertEqual(structured[0]["commands"], self.GOOD)
        self.assertTrue(structured[0]["commands"], "the gate's evidence test is `not r.get('commands')`")

    def test_without_commands_the_record_has_no_commands_key(self):
        record, _ = ma.build_record("o/r", 7, HEAD, BASE, "gemini", ["claude"], "accept")
        self.assertNotIn("commands", record)

    def test_an_entry_that_states_no_exit_code_is_refused(self):
        for bad in (["ran the tests"], ["python -m unittest -> OK"], ["exit"], ["exit code none"]):
            record, error = ma.build_record("o/r", 7, HEAD, BASE, "gemini", ["claude"], "accept", commands=bad)
            self.assertIsNone(record, bad)
            self.assertIn("exit code", error)

    def test_malformed_lists_are_refused(self):
        too_many = ["cmd -> exit 0"] * (ma.MAX_COMMANDS + 1)
        too_long = ["cmd -> exit 0 " + "x" * ma.MAX_COMMAND_LENGTH]
        for bad, expected in (([], "non-empty list"), ("cmd -> exit 0", "non-empty list"), ([""], "non-empty string"),
                              ([7], "non-empty string"), (too_many, "at most"), (too_long, "keep each under")):
            record, error = ma.build_record("o/r", 7, HEAD, BASE, "gemini", ["claude"], "accept", commands=bad)
            self.assertIsNone(record, bad)
            self.assertIn(expected, error)

    def test_a_signed_block_with_commands_verifies_and_changing_a_command_breaks_the_signature(self):
        if not shutil.which("ssh-keygen"):
            self.skipTest("ssh-keygen is not available")
        import attest_sig
        directory = tempfile.mkdtemp(prefix="make-attestation-")
        self.addCleanup(shutil.rmtree, directory, True)
        key = os.path.join(directory, "key")
        subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", key], check=True)
        public = open(key + ".pub", encoding="utf-8").read().strip()
        record, _ = ma.build_record("o/r", 7, HEAD, BASE, "gemini", ["claude"], "accept", commands=self.GOOD)
        signature = attest_sig.sign(attest_sig.canonical(record), key)
        allowed = attest_sig.allowed_signers({"seats": {"gemini": {"logins": [], "keys": [public]}}})
        self.assertTrue(attest_sig.verify(attest_sig.canonical(record), signature, "gemini", allowed))
        tampered = dict(record, commands=["python -m unittest -> exit 0 (all tests passed)"])
        self.assertFalse(attest_sig.verify(attest_sig.canonical(tampered), signature, "gemini", allowed))


class Gather(unittest.TestCase):
    """gather() against real throwaway repositories: a bare remote, a work tree, and a clone."""

    def setUp(self):
        self.root = tempfile.mkdtemp(prefix="make-attestation-")
        self.addCleanup(shutil.rmtree, self.root, True)
        self.remote = os.path.join(self.root, "github.com", "o", "r.git")
        os.makedirs(os.path.dirname(self.remote))
        self.work = os.path.join(self.root, "work")
        self.git(self.root, "init", "-q", "--bare", "-b", "main", self.remote)
        self.git(self.root, "init", "-q", "-b", "main", self.work)
        self.commit("base.txt", "base\n", "base")
        self.git(self.work, "remote", "add", "origin", self.remote)
        self.git(self.work, "push", "-q", "origin", "main")
        self.git(self.work, "checkout", "-q", "-b", "claude/feature")
        self.head = self.commit("pr.txt", "pr\n", "the change\n\nCo-Authored-By: Gemini <g@example.invalid>")
        self.git(self.work, "push", "-q", "origin", "claude/feature")
        self.clone = os.path.join(self.root, "clone")
        # the clone's origin URL must end in o/r the way a GitHub URL does, so it is cloned from a forward-slash path
        self.git(self.root, "clone", "-q", self.remote.replace(os.sep, "/"), self.clone)
        self.base = self.git(self.work, "rev-parse", "main")
        self.info = {"headRefOid": self.head, "baseRefName": "main", "headRefName": "claude/feature"}
        self.run = lambda *args: self.info

    def git(self, directory, *args):
        result = subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid", "-C", directory, *args],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.strip()

    def commit(self, name, text, message):
        with open(os.path.join(self.work, name), "w", encoding="utf-8", newline="\n") as handle:
            handle.write(text)
        self.git(self.work, "add", ".")
        self.git(self.work, "commit", "-q", "-m", message)
        return self.git(self.work, "rev-parse", "HEAD")

    def fetch_head(self):
        self.git(self.clone, "fetch", "-q", "origin", "claude/feature:refs/remotes/origin/claude/feature")

    def test_head_base_and_every_implementing_seat_come_from_the_clone(self):
        self.fetch_head()
        head, base, implementers, error = ma.gather("o/r", self.clone, 7, [], run=self.run)
        self.assertIsNone(error)
        self.assertEqual((head, base), (self.head, self.base))
        self.assertEqual(implementers, ["claude", "gemini"])

    def test_a_seat_that_pushed_without_a_trailer_is_added_once(self):
        self.fetch_head()
        _, _, implementers, _ = ma.gather("o/r", self.clone, 7, ["codex", "claude"], run=self.run)
        self.assertEqual(implementers, ["claude", "gemini", "codex"])

    def test_a_head_the_clone_does_not_have_is_reported_with_the_fetch_command(self):
        self.info["headRefOid"] = "d" * 40
        _, _, _, error = ma.gather("o/r", self.clone, 7, [], run=self.run)
        self.assertIn("git fetch origin refs/pull/7/head", error)

    def test_a_stale_base_ref_is_refreshed_before_the_merge_base_is_taken(self):
        # Codex P2 on #112: with origin/main stale, the merge base was computed against the old tip and the pasted
        # verdict did not match what coverage computes from an up-to-date clone.
        self.fetch_head()
        self.git(self.work, "checkout", "-q", "main")
        self.git(self.work, "merge", "-q", "--no-ff", "-m", "main takes the PR", "claude/feature")
        newer = self.commit("later.txt", "later\n", "main moves on")
        self.git(self.work, "push", "-q", "origin", "main")
        self.assertNotEqual(self.git(self.clone, "rev-parse", "origin/main"), newer, "the clone is stale")
        _, base, _, error = ma.gather("o/r", self.clone, 7, [], run=self.run)
        self.assertIsNone(error)
        self.assertEqual(base, self.head, "merge base of main and the head, against the refreshed main")
        self.assertEqual(self.git(self.clone, "rev-parse", "origin/main"), newer)

    def test_a_failed_base_refresh_is_an_error_not_a_guess(self):
        self.fetch_head()
        self.git(self.clone, "remote", "set-url", "origin", os.path.join(self.root, "gone.git").replace(os.sep, "/"))
        _, _, _, error = ma.gather("o/r", self.clone, 7, [], run=self.run)
        self.assertIsNotNone(error)

    def test_a_delta_from_that_is_not_an_ancestor_of_the_head_is_refused(self):
        self.fetch_head()
        ok = ma.gather("o/r", self.clone, 7, [], run=self.run, frm=self.base)
        self.assertIsNone(ok[3])
        unrelated = "e" * 40
        _, _, _, error = ma.gather("o/r", self.clone, 7, [], run=self.run, frm=unrelated)
        self.assertIn("is not an ancestor", error)

    def test_a_clone_of_another_repository_is_refused(self):
        _, _, _, error = ma.gather("o/other", self.clone, 7, [], run=self.run)
        self.assertIn("is not a clone of o/other", error)

    def test_no_identifiable_implementer_is_an_error_not_an_empty_list(self):
        self.info["headRefName"] = "feature"
        self.git(self.work, "checkout", "-q", "-b", "plain", "main")
        plain = self.commit("plain.txt", "x\n", "plain commit")
        self.git(self.work, "push", "-q", "origin", "plain")
        self.git(self.clone, "fetch", "-q", "origin", "plain:refs/remotes/origin/plain")
        self.info["headRefOid"] = plain
        _, _, implementers, error = ma.gather("o/r", self.clone, 7, [], run=self.run)
        self.assertIsNone(implementers)
        self.assertIn("--implementer", error)


@unittest.skipUnless(shutil.which("ssh-keygen"), "ssh-keygen is not installed")
class Signed(unittest.TestCase):
    """Codex P1 on #112: merge_gate drops unsigned attestations, so the tool has to be able to sign."""

    def test_a_signed_block_counts_under_signature_enforcement(self):
        import json
        import attest_sig
        with tempfile.TemporaryDirectory() as tmp:
            key = os.path.join(tmp, "key")
            made = subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", key], capture_output=True)
            self.assertEqual(made.returncode, 0, made.stderr)
            with open(key + ".pub", encoding="utf-8") as handle:
                public = handle.read().strip()
            keys = {"seats": {"codex": {"keys": [public]}}}
            from unittest import mock
            out = os.path.join(tmp, "block.md")
            with mock.patch.object(ma, "gather", return_value=(HEAD, BASE, ["claude"], None)):
                code = ma.main(["--repo", "o/r", "--repo-dir", ".", "--pr", "1", "--reviewer", "codex",
                                "--verdict", "accept", "--key", key, "--out", out])
            self.assertEqual(code, 0)
            with open(out, encoding="utf-8") as handle:
                block = handle.read()
            self.assertIn("```attestation-sig", block)
            record, error = ma.build_record("o/r", 1, HEAD, BASE, "codex", ["claude"], "accept")
            self.assertIsNone(error)
            snapshot = {"repo": "o/r", "prs": [{"number": 1, "head": HEAD, "base": "main", "merge_base": BASE,
                                                "reviews": [], "base_events": [],
                                                "comments": [{"id": 1, "body": block}]}]}
            enforced = rc.report_snapshot(snapshot, lambda a, b: True, None, keys, True)[0]
            self.assertTrue(enforced["covered"], enforced["reasons"])
            self.assertEqual(enforced["signed_attestations"], 1)
            unsigned = {"prs": [dict(snapshot["prs"][0], comments=[{"id": 2, "body": ma.render_block(record)}])],
                        "repo": "o/r"}
            self.assertFalse(rc.report_snapshot(unsigned, lambda a, b: True, None, keys, True)[0]["covered"])
            json.dumps(enforced)


if __name__ == "__main__":
    unittest.main()
