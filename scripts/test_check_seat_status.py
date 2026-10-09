#!/usr/bin/env python3
"""Stdlib tests for the C112 per-seat status validator."""
import json
import os
import sys
import tempfile
import unittest
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import check_seat_status as css

HEADER = "| " + " | ".join(css.HEADER) + " |"
SEP = "| " + " | ".join(["---"] * 6) + " |"
AS_OF = datetime(2026, 10, 9, 7, 0, tzinfo=timezone.utc)
FRESH = "2026-10-09T06:00:00Z"
STALE = "2026-10-09T00:00:00Z"


def row(seat="ubuntu-worker-01", state="available", measured=FRESH, claims="C111, C112", note="RTX idle"):
    return f"| {seat} | host.example | {state} | {claims} | {measured} | {note} |"


class StatusTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.directory = os.path.join(self.tmp.name, "seat-status")
        os.mkdir(self.directory)

    def write(self, seat="ubuntu-worker-01", content=None):
        content = content or f"# Status\n\n{HEADER}\n{SEP}\n{row(seat)}\n"
        with open(os.path.join(self.directory, seat + ".md"), "w", encoding="utf-8") as f:
            f.write(content)

    def check(self, **kwargs):
        kwargs.setdefault("as_of", AS_OF)
        kwargs.setdefault("identities", None)
        return css.check(self.directory, **kwargs)

    def test_valid_file(self):
        self.write()
        self.assertEqual(self.check(), ([], []))

    def test_each_state_is_valid(self):
        for state in css.STATES:
            with self.subTest(state=state):
                self.write(content=f"{HEADER}\n{SEP}\n{row(state=state)}\n")
                errors, _ = self.check()
                self.assertEqual(errors, [])
                os.unlink(os.path.join(self.directory, "ubuntu-worker-01.md"))

    def test_filename_must_match_seat_cell(self):
        self.write("wrong-name", content=f"{HEADER}\n{SEP}\n{row()}\n")
        errors, _ = self.check()
        self.assertTrue(any("must equal filename" in e for e in errors), errors)

    def test_unknown_state_fails(self):
        self.write(content=f"{HEADER}\n{SEP}\n{row(state='In progress')}\n")
        errors, _ = self.check()
        self.assertTrue(any("Availability" in e for e in errors), errors)

    def test_bad_timestamp_fails(self):
        self.write(content=f"{HEADER}\n{SEP}\n{row(measured='yesterday')}\n")
        errors, _ = self.check()
        self.assertTrue(any("UTC" in e for e in errors), errors)

    def test_stale_available_fails(self):
        self.write(content=f"{HEADER}\n{SEP}\n{row(measured=STALE)}\n")
        errors, _ = self.check(max_age_minutes=120)
        self.assertTrue(any("stale" in e for e in errors), errors)

    def test_stale_busy_warns(self):
        self.write(content=f"{HEADER}\n{SEP}\n{row(state='busy', measured=STALE)}\n")
        errors, warnings = self.check(max_age_minutes=120)
        self.assertEqual(errors, [])
        self.assertTrue(any("stale" in w for w in warnings), warnings)

    def test_future_beyond_skew_fails(self):
        self.write(content=f"{HEADER}\n{SEP}\n{row(measured='2026-10-09T07:10:00Z')}\n")
        errors, _ = self.check(skew_minutes=5)
        self.assertTrue(any("future" in e for e in errors), errors)

    def test_second_data_row_fails(self):
        self.write(content=f"{HEADER}\n{SEP}\n{row()}\n{row()}\n")
        errors, _ = self.check()
        self.assertTrue(any("exactly one" in e for e in errors), errors)

    def test_no_files_fails(self):
        errors, _ = self.check()
        self.assertTrue(any("no <seat-id>" in e for e in errors), errors)

    def test_readme_is_not_a_status_file(self):
        with open(os.path.join(self.directory, "README.md"), "w") as f:
            f.write("# docs\n")
        errors, _ = self.check()
        self.assertTrue(errors)

    def test_identity_mismatch_warns_only(self):
        self.write()
        identities = os.path.join(self.tmp.name, "identities.json")
        with open(identities, "w") as f:
            json.dump({"seats": {"claude": {}}}, f)
        errors, warnings = self.check(identities=identities)
        self.assertEqual(errors, [])
        self.assertTrue(any("no identities" in w for w in warnings), warnings)
        self.assertTrue(any("no status" in w for w in warnings), warnings)

    def test_escaped_pipe_is_one_cell(self):
        self.write(content=f"{HEADER}\n{SEP}\n{row(note=r'kernel\\|control')}\n")
        self.assertEqual(self.check(), ([], []))

    def test_no_hard_coded_roster(self):
        with open(css.__file__, encoding="utf-8") as f:
            source = f.read()
        for forbidden in ("KNOWN_SEATS =", '"claude"', '"mimo"', '"codex"', '"gemini"'):
            self.assertNotIn(forbidden, source)

    def test_cli_exit_codes(self):
        self.write()
        self.assertEqual(css.main([self.directory, "--as-of", "2026-10-09T07:00:00Z"]), 0)
        self.assertEqual(css.main([self.directory, "--as-of", "bad"]), 1)

    def test_shipped_directory(self):
        directory = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "governance", "seat-status")
        if not os.path.isdir(directory):
            self.skipTest("not in checkout")
        errors, _ = css.check(directory, as_of=AS_OF, identities=None)
        self.assertEqual(errors, [])

if __name__ == "__main__":
    unittest.main()
