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

    def test_a_non_independent_reviewer_may_still_record_a_rejection(self):
        record, error = ma.build_record("o/r", 7, HEAD, BASE, "claude", ["claude"], "changes_requested")
        self.assertIsNone(error)
        self.assertEqual(record["verdict"], "changes_requested")


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


if __name__ == "__main__":
    unittest.main()
