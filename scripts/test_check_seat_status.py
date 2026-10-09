#!/usr/bin/env python3
"""Tests for scripts/check_seat_status.py (lane C112).

Stdlib only, matching the rest of scripts/test_*.py. Run with:

    python3 scripts/test_check_seat_status.py
    python3 -m unittest discover -s scripts -p 'test_*.py'
"""

import json
import os
import sys
import tempfile
import unittest
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import check_seat_status as css  # noqa: E402

HEADER = "| " + " | ".join(css.EXPECTED_HEADER) + " |"
SEPARATOR = "| " + " | ".join(["---"] * len(css.EXPECTED_HEADER)) + " |"

# Freshness is evaluated relative to this fixed instant so the suite never depends on the wall clock.
AS_OF = datetime(2026, 10, 9, 6, 0, 0, tzinfo=timezone.utc)
FRESH = "2026-10-09T05:30:00Z"   # 30 minutes before AS_OF
STALE = "2026-10-09T00:00:00Z"   # 360 minutes before AS_OF


def make_row(seat="ubuntu-worker-01", host="aaroneoushome (100.109.8.12)", availability="available",
             claims="C111, C112", measured=FRESH, note="RTX 4060 idle"):
    return f"| {seat} | {host} | {availability} | {claims} | {measured} | {note} |"


class SeatStatusCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.path = os.path.join(self._tmp.name, "SEAT_STATUS.md")

    def write_status(self, *rows):
        body = "\n".join([HEADER, SEPARATOR, *rows]) + "\n"
        with open(self.path, "w", encoding="utf-8") as fh:
            fh.write(body)
        return self.path

    def write_identities(self, seats):
        path = os.path.join(self._tmp.name, "identities.json")
        with open(path, "w", encoding="utf-8") as fh:
            json.dump({"owner": ["Aarogaming"],
                       "seats": {s: {"logins": [], "keys": []} for s in seats}}, fh)
        return path

    def check(self, identities=None, **kwargs):
        kwargs.setdefault("as_of", AS_OF)
        return css.check(self.path, identities=identities, **kwargs)

    def assertNoErrors(self, errors, warnings=None):
        self.assertEqual(errors, [], f"expected no errors, got: {errors}")
        if warnings is not None:
            self.assertEqual(warnings, [], f"expected no warnings, got: {warnings}")

    # --- happy path -------------------------------------------------------

    def test_valid_file_passes_clean(self):
        self.write_status(make_row())
        errors, warnings = self.check(identities=None)
        self.assertNoErrors(errors, warnings)

    def test_all_three_availability_states_are_accepted(self):
        self.write_status(
            make_row(seat="ubuntu-worker-01", availability="available"),
            make_row(seat="claude", availability="busy"),
            make_row(seat="codex", availability="offline", claims="-", note="powered down"),
        )
        errors, _ = self.check(identities=None)
        self.assertNoErrors(errors)

    def test_escaped_pipe_in_a_cell_keeps_six_columns(self):
        # The queue parser splits on unescaped pipes; a plain split would read this as extra cells.
        self.write_status(make_row(note=r"kernel\|control\|tooling"))
        errors, warnings = self.check(identities=None)
        self.assertNoErrors(errors, warnings)

    # --- vocabulary -------------------------------------------------------

    def test_unknown_availability_is_an_error(self):
        self.write_status(make_row(availability="In progress"))
        errors, _ = self.check(identities=None)
        self.assertTrue(any("availability" in e for e in errors), errors)

    def test_availability_is_case_sensitive(self):
        self.write_status(make_row(availability="Available"))
        errors, _ = self.check(identities=None)
        self.assertTrue(errors)

    def test_seat_name_shape_is_enforced(self):
        self.write_status(make_row(seat="Ubuntu Worker 01"))
        errors, _ = self.check(identities=None)
        self.assertTrue(any("seat name" in e for e in errors), errors)

    # --- structure --------------------------------------------------------

    def test_wrong_column_count_is_an_error(self):
        self.write_status("| ubuntu-worker-01 | host | available |")
        errors, _ = self.check(identities=None)
        self.assertTrue(any("cells" in e for e in errors), errors)

    def test_empty_table_is_an_error(self):
        with open(self.path, "w", encoding="utf-8") as fh:
            fh.write(f"{HEADER}\n{SEPARATOR}\n")
        errors, _ = self.check(identities=None)
        self.assertTrue(any("no data rows" in e for e in errors), errors)

    def test_missing_header_is_an_error(self):
        with open(self.path, "w", encoding="utf-8") as fh:
            fh.write("some prose, no table here\n")
        errors, _ = self.check(identities=None)
        self.assertTrue(any("no table header" in e for e in errors), errors)

    def test_missing_file_is_an_error_not_a_crash(self):
        errors, _ = css.check(os.path.join(self._tmp.name, "absent.md"), as_of=AS_OF)
        self.assertTrue(errors)
        self.assertIn("cannot read", errors[0])

    def test_header_mismatch_is_an_error(self):
        bad = "| " + " | ".join(["Seat", "Host", "State", "Claims", "At", "Note"]) + " |"
        with open(self.path, "w", encoding="utf-8") as fh:
            fh.write(f"{bad}\n{SEPARATOR}\n{make_row()}\n")
        errors, _ = self.check(identities=None)
        self.assertTrue(any("header mismatch" in e for e in errors), errors)

    def test_duplicate_seat_is_an_error(self):
        self.write_status(make_row(), make_row(measured="2026-10-09T05:45:00Z"))
        errors, _ = self.check(identities=None)
        self.assertTrue(any("duplicates seat" in e for e in errors), errors)

    def test_empty_open_claims_cell_is_an_error(self):
        self.write_status(make_row(claims=""))
        errors, _ = self.check(identities=None)
        self.assertTrue(any("Open claims" in e for e in errors), errors)

    # --- timestamps and freshness ----------------------------------------

    def test_unparseable_timestamp_is_an_error(self):
        self.write_status(make_row(measured="yesterday"))
        errors, _ = self.check(identities=None)
        self.assertTrue(any("UTC YYYY-MM-DDTHH:MM:SSZ" in e for e in errors), errors)

    def test_local_offset_timestamp_is_rejected(self):
        # Only Z is accepted: a local time would silently drift the freshness window.
        self.write_status(make_row(measured="2026-10-09T05:30:00+00:00"))
        errors, _ = self.check(identities=None)
        self.assertTrue(errors)

    def test_future_timestamp_beyond_skew_is_an_error(self):
        self.write_status(make_row(measured="2026-10-09T06:30:00Z"))
        errors, _ = self.check(identities=None)
        self.assertTrue(any("in the future" in e for e in errors), errors)

    def test_future_within_skew_is_allowed(self):
        self.write_status(make_row(measured="2026-10-09T06:03:00Z"))
        errors, _ = self.check(identities=None, skew_minutes=5)
        self.assertNoErrors(errors)

    def test_stale_available_is_an_error(self):
        self.write_status(make_row(measured=STALE))
        errors, _ = self.check(identities=None, max_age_minutes=120)
        self.assertTrue(any("freshness window" in e for e in errors), errors)

    def test_stale_busy_is_a_warning_not_an_error(self):
        # A seat that stopped reporting while busy is not making a false offer.
        self.write_status(make_row(availability="busy", measured=STALE))
        errors, warnings = self.check(identities=None, max_age_minutes=120)
        self.assertNoErrors(errors)
        self.assertTrue(any("stale" in w for w in warnings), warnings)

    def test_stale_offline_is_a_warning_not_an_error(self):
        self.write_status(make_row(availability="offline", measured=STALE))
        errors, warnings = self.check(identities=None, max_age_minutes=120)
        self.assertNoErrors(errors)
        self.assertTrue(any("stale" in w for w in warnings), warnings)

    def test_freshness_window_is_configurable(self):
        # 31 minutes old passes a 60-minute window and fails a 10-minute one.
        self.write_status(make_row(measured="2026-10-09T05:30:00Z"))
        errors, _ = self.check(identities=None, max_age_minutes=60)
        self.assertNoErrors(errors)
        errors, _ = self.check(identities=None, max_age_minutes=10)
        self.assertTrue(errors)

    def test_as_of_is_honoured_for_reproducible_runs(self):
        # Measured at 05:30: judged fresh as of 05:45, stale as of 09:00.
        self.write_status(make_row(measured="2026-10-09T05:30:00Z"))
        errors, _ = css.check(self.path, identities=None, as_of=datetime(2026, 10, 9, 5, 45,
                                                                        tzinfo=timezone.utc))
        self.assertNoErrors(errors)
        errors, _ = css.check(self.path, identities=None, as_of=datetime(2026, 10, 9, 9, 0,
                                                                        tzinfo=timezone.utc))
        self.assertTrue(errors)

    # --- identity cross-check: advisory by design -------------------------

    def test_seat_missing_from_identities_is_a_warning_only(self):
        self.write_status(make_row())
        identities = self.write_identities(["claude", "mimo", "codex", "gemini"])
        errors, warnings = self.check(identities=identities)
        self.assertNoErrors(errors)
        self.assertTrue(any("no entry" in w for w in warnings), warnings)

    def test_registered_seat_without_a_row_is_a_warning_only(self):
        self.write_status(make_row())
        identities = self.write_identities(["claude", "mimo", "codex", "gemini", "ubuntu-worker-01"])
        errors, warnings = self.check(identities=identities)
        self.assertNoErrors(errors)
        self.assertTrue(any("no row" in w for w in warnings), warnings)

    def test_identity_mismatch_never_fails_the_check(self):
        # Registration is an owner-reviewed PR and may legitimately lag the ledger.
        self.write_status(make_row())
        identities = self.write_identities(["claude"])
        errors, _ = self.check(identities=identities)
        self.assertNoErrors(errors)

    def test_absent_identities_file_skips_the_cross_check(self):
        self.write_status(make_row())
        errors, _ = self.check(identities=os.path.join(self._tmp.name, "missing.json"))
        self.assertNoErrors(errors)

    def test_no_hard_coded_seat_roster(self):
        # The checker must not carry its own seat list: read the module source to be sure.
        with open(css.__file__, encoding="utf-8") as fh:
            source = fh.read()
        for forbidden in ("KNOWN_SEATS =", '"claude"', '"mimo"', '"codex"', '"gemini"'):
            self.assertNotIn(forbidden, source,
                             f"{forbidden} would be a third copy of the seat roster")

    def test_shipped_ledger_validates(self):
        shipped = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                               "governance", "SEAT_STATUS.md")
        if not os.path.exists(shipped):
            self.skipTest("governance/SEAT_STATUS.md not present in this checkout")
        # Identities are intentionally not passed: registration lags the ledger by design.
        errors, warnings = css.check(shipped, as_of=datetime(2026, 10, 9, 6, 0, tzinfo=timezone.utc),
                                     identities=None)
        self.assertNoErrors(errors, warnings)


class CommandLineCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.path = os.path.join(self._tmp.name, "SEAT_STATUS.md")

    def write_status(self, row):
        with open(self.path, "w", encoding="utf-8") as fh:
            fh.write(f"{HEADER}\n{SEPARATOR}\n{row}\n")

    def test_valid_file_exits_zero(self):
        self.write_status(make_row())
        self.assertEqual(css.main([self.path, "--as-of", "2026-10-09T06:00:00Z"]), 0)

    def test_invalid_file_exits_one(self):
        self.write_status(make_row(availability="In progress"))
        self.assertEqual(css.main([self.path, "--as-of", "2026-10-09T06:00:00Z"]), 1)

    def test_bad_as_of_exits_one(self):
        self.write_status(make_row())
        self.assertEqual(css.main([self.path, "--as-of", "not-a-time"]), 1)


if __name__ == "__main__":
    unittest.main()
