import os
import unittest
from unittest.mock import patch
import merge_gate as mg
from test_merge_gate import RevertibilityFromGit

OLD = 'jobs:\n  test:\n    steps:\n      - uses: actions/checkout@' + '1' * 40 + '\n      - run: echo safe\n'
NEW = OLD.replace('1' * 40, '2' * 40)
PATH = '.github/workflows/foundation-ci.yml'


class PinText(unittest.TestCase):
    def test_pin(self):
        self.assertTrue(mg.workflow_pin_update(OLD, NEW))
        self.assertTrue(mg.workflow_pin_update(NEW, OLD))

    def test_block_scalar_cannot_forge_steps(self):
        a = "jobs:\n  test:\n    steps:\n      - run: |\n          steps:\n            - uses: actions/checkout@" + "1" * 40 + "\n"
        self.assertFalse(mg.workflow_pin_update(a, a.replace("1" * 40, "2" * 40)))

    def test_other_changes_fail_closed(self):
        for bad in (OLD, NEW + '\n', NEW.replace('safe', 'unsafe'),
                    NEW.replace('2' * 40, 'v7'), NEW.replace('2' * 40, '0' * 40),
                    NEW.replace('actions/checkout', 'other/checkout'),
                    NEW.replace('uses:', 'run:'), NEW.replace('\n', '\r\n'),
                    NEW.replace('      - uses', '        - uses')):
            with self.subTest(bad=bad):
                self.assertFalse(mg.workflow_pin_update(OLD, bad))


class PinGit(unittest.TestCase):
    def setUp(self):
        RevertibilityFromGit.setUp(self)
        os.makedirs(os.path.join(self.directory, '.github', 'workflows'))
        self.write_workflow(OLD)
        self.git('add', '.')
        self.git('commit', '-qm', 'workflow before')
        self.base = self.git('rev-parse', 'HEAD')
        guard = patch.dict(mg.WORKFLOW_PIN_BASE_BLOBS, {PATH: self.git('rev-parse', self.base + ':' + PATH)})
        guard.start()
        self.addCleanup(guard.stop)
        self.write_workflow(NEW)
        self.git('add', '.')
        self.git('commit', '-qm', 'pin')
        self.head = self.git('rev-parse', 'HEAD')

    write_lock = RevertibilityFromGit.write_lock
    classify = RevertibilityFromGit.classify

    def write_workflow(self, text):
        with open(os.path.join(self.directory, PATH), 'w', encoding='utf-8', newline='') as f:
            f.write(text)

    def test_pinned_regular_blobs(self):
        self.write_workflow('dirty checkout')
        self.assertTrue(self.classify([PATH]))
        self.assertIsNone(self.classify([PATH, 'governance/x.md']))
        self.assertIsNone(self.classify([PATH], head='HEAD'))
        self.assertIsNone(self.classify([PATH], head='0' * 40))

    def test_executable_and_deleted_fail_closed(self):
        self.git('update-index', '--chmod=+x', PATH)
        self.git('commit', '-qm', 'mode')
        self.assertIsNone(self.classify([PATH], head=self.git('rev-parse', 'HEAD')))
        self.git('rm', '-f', PATH)
        self.git('commit', '-qm', 'deleted')
        self.assertIsNone(self.classify([PATH], head=self.git('rev-parse', 'HEAD')))

    def test_symlink_fails_closed(self):
        blob = self.git('hash-object', '-w', '--', PATH)
        self.git('update-index', '--cacheinfo', '120000,' + blob + ',' + PATH)
        self.git('commit', '-qm', 'symlink')
        self.assertIsNone(self.classify([PATH], head=self.git('rev-parse', 'HEAD')))

    def test_unknown_baseline_template_stays_unclassified(self):
        with patch.dict(mg.WORKFLOW_PIN_BASE_BLOBS, {PATH: "0" * 40}):
            self.assertIsNone(self.classify([PATH]))
