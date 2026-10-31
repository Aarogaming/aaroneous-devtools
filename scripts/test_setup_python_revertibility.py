import os
import unittest
from unittest.mock import patch
import merge_gate as mg
import test_merge_gate as fixtures

OLD_PIN = "5fda3b95a4ea91299a34e894583c3862153e4b97"
NEW_PIN = "6d4fd22b20465ccc0f7354476588e8df7c9ca0e0"
OLD = "jobs:\n  test:\n    steps:\n      - uses: actions/setup-python@" + OLD_PIN + " # v7.0.0\n        with:\n          python-version: '3.12'\n"
NEW = OLD.replace(OLD_PIN, NEW_PIN).replace("# v7.0.0", "# v7.1.0")
PATH = ".github/workflows/python-scripts.yml"


class SetupPinText(unittest.TestCase):
    def test_exact_substitution_with_or_without_version_comment(self):
        self.assertTrue(mg.setup_python_pin_update(OLD, NEW))
        self.assertTrue(mg.setup_python_pin_update(OLD.replace(" # v7.0.0", ""), NEW.replace(" # v7.1.0", "")))

    def test_other_changes_hold(self):
        for bad in (OLD, NEW + "\n", NEW.replace("3.12", "3.13"),
                    NEW.replace(NEW_PIN, "f" * 40), NEW.replace("# v7.1.0", "# v8"),
                    NEW.replace("\n", "\r\n"), NEW.replace("with:", "env:")):
            with self.subTest(bad=bad):
                self.assertFalse(mg.setup_python_pin_update(OLD, bad))


class SetupPinGit(unittest.TestCase):
    write_lock = fixtures.RevertibilityFromGit.write_lock
    classify = fixtures.RevertibilityFromGit.classify

    def setUp(self):
        fixtures.RevertibilityFromGit.setUp(self)
        os.makedirs(os.path.join(self.directory, ".github", "workflows"))
        self.write_workflow(OLD)
        self.git("add", ".")
        self.git("commit", "-qm", "setup before")
        self.base = self.git("rev-parse", "HEAD")
        guard = patch.dict(mg.WORKFLOW_PIN_BASE_BLOBS, {PATH: self.git("rev-parse", self.base + ":" + PATH)})
        guard.start()
        self.addCleanup(guard.stop)
        self.write_workflow(NEW)
        self.git("add", ".")
        self.git("commit", "-qm", "setup after")
        self.head = self.git("rev-parse", "HEAD")

    def write_workflow(self, text):
        with open(os.path.join(self.directory, PATH), "w", encoding="utf-8", newline="") as f:
            f.write(text)

    def test_regular_pinned_blobs_ignore_dirty_checkout(self):
        self.write_workflow("dirty")
        self.assertTrue(self.classify([PATH]))
        self.assertIsNone(self.classify([PATH, "governance/x.md"]))
        self.assertIsNone(self.classify([PATH], head="HEAD"))

    def test_unknown_template_and_modes_hold(self):
        with patch.dict(mg.WORKFLOW_PIN_BASE_BLOBS, {PATH: "0" * 40}):
            self.assertIsNone(self.classify([PATH]))
        self.git("update-index", "--chmod=+x", PATH)
        self.git("commit", "-qm", "mode")
        self.assertIsNone(self.classify([PATH], head=self.git("rev-parse", "HEAD")))

    def test_changed_input_and_symlink_hold(self):
        self.write_workflow(NEW.replace("3.12", "3.13"))
        self.git("add", ".")
        self.git("commit", "-qm", "new input")
        self.assertIsNone(self.classify([PATH], head=self.git("rev-parse", "HEAD")))
        blob = self.git("hash-object", "-w", "--", PATH)
        self.git("update-index", "--cacheinfo", "120000," + blob + "," + PATH)
        self.git("commit", "-qm", "symlink")
        self.assertIsNone(self.classify([PATH], head=self.git("rev-parse", "HEAD")))
