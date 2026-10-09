#!/usr/bin/env python3
"""Validate governance/SEAT_STATUS.md, the host-compute availability ledger (lane C112).

SEAT_STATUS.md answers one question: is a seat's *host* free to take work, between active user
sessions. This checker exists to stop that file from becoming another silently-wrong roster, so it
validates structure, vocabulary and freshness, and only ever *warns* about identity membership.

It deliberately hard-codes no seat list. The seat roster already exists twice in this repository
(`scripts/collect_state.py` KNOWN_SEATS and `scripts/review_coverage.py`), the two copies already
disagree, and a third copy here would reproduce the defect instead of preventing it. Identity
membership is therefore read from `governance/identities.json` when that file is present, and a
mismatch is reported as a warning because registering a seat is an owner-reviewed PR that may
legitimately lag this ledger.

Freshness semantics: a stale `available` row is an error, because advertising free compute on old
evidence is a false offer. A stale `busy` or `offline` row is a warning, because a seat that stopped
reporting is not claiming anything untrue.

Usage:
  check_seat_status.py [--max-age-minutes N] [--skew-minutes N] [--as-of ISO]
                       [--identities PATH] FILE

Exits 1 and prints each error; warnings are printed but do not fail the check.
"""

import argparse
import json
import os
import re
import sys
from datetime import datetime, timedelta, timezone

EXPECTED_HEADER = ["Seat", "Host", "Availability", "Open claims", "Measured at (UTC)", "Note"]
AVAILABILITY = ("available", "busy", "offline")
SEAT_SHAPE = re.compile(r"^[a-z0-9][a-z0-9._-]*$")
TIMESTAMP_SHAPE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
DEFAULT_MAX_AGE_MINUTES = 120
DEFAULT_SKEW_MINUTES = 5


def split_cells(line):
    """Split one markdown table row on unescaped pipes.

    Cells may contain a backslash-escaped pipe (`kernel\\|control`), which the queue parser and
    check_coordination_evidence.py both honour; a plain split would misread them.
    """
    s = line.rstrip("\n")
    if not s.startswith("|") or not s.endswith("|"):
        raise ValueError("row must start and end with '|'")
    return [c.strip() for c in re.split(r"(?<!\\)\|", s)[1:-1]]


def parse_timestamp(text):
    """Return an aware UTC datetime, or None when the value is not a usable UTC timestamp."""
    if not TIMESTAMP_SHAPE.match(text or ""):
        return None
    try:
        return datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError:
        return None


def load_identities(path):
    """Return the seat names registered in identities.json, or None when unavailable.

    None means "cannot cross-check", which is not an error: the file may be absent in a bare
    checkout, and an unreadable identities.json is not this ledger's defect to report as its own.
    """
    if not path or not os.path.exists(path):
        return None
    try:
        with open(path, encoding="utf-8") as fh:
            data = json.load(fh)
    except (OSError, ValueError):
        return None
    seats = data.get("seats") if isinstance(data, dict) else None
    if isinstance(seats, dict):
        return set(seats.keys())
    return None


def find_table(lines):
    """Locate the status table. Returns (header_index, cells) or (None, None)."""
    for i, line in enumerate(lines):
        if not line.startswith("|"):
            continue
        try:
            cells = split_cells(line)
        except ValueError:
            continue
        if cells and cells[0] == "Seat":
            return i, cells
    return None, None


def check(path, max_age_minutes=DEFAULT_MAX_AGE_MINUTES, skew_minutes=DEFAULT_SKEW_MINUTES,
          as_of=None, identities=None):
    """Return (errors, warnings), each a list of human-readable strings."""
    errors, warnings = [], []
    as_of = as_of or datetime.now(timezone.utc)

    try:
        with open(path, encoding="utf-8") as fh:
            text = fh.read()
    except OSError as exc:
        return [f"cannot read {path}: {exc}"], []

    lines = text.splitlines()
    idx, header = find_table(lines)
    if idx is None:
        return [f"no table header found; expected a row beginning with '| Seat |'"], []

    if header != EXPECTED_HEADER:
        errors.append(
            "header mismatch: expected {!r}, found {!r}".format(EXPECTED_HEADER, header)
        )

    if idx + 1 >= len(lines):
        return errors + ["header is the last line; separator and data rows are missing"], warnings
    try:
        sep = split_cells(lines[idx + 1])
    except ValueError as exc:
        return errors + [f"separator row malformed: {exc}"], warnings
    if len(sep) != len(EXPECTED_HEADER) or not all(re.fullmatch(r":?-{3,}:?", c) for c in sep):
        errors.append(f"row {idx + 2} is not a valid separator row: {lines[idx + 1]!r}")
        return errors, warnings

    seen = {}
    rows = 0
    for n, line in enumerate(lines[idx + 2:], start=idx + 3):
        if not line.startswith("|"):
            break
        rows += 1
        try:
            cells = split_cells(line)
        except ValueError as exc:
            errors.append(f"row {n} malformed: {exc}")
            continue
        if len(cells) != len(EXPECTED_HEADER):
            errors.append(
                f"row {n} has {len(cells)} cells, expected {len(EXPECTED_HEADER)}"
            )
            continue

        seat, host, availability, claims, measured, note = cells

        if not SEAT_SHAPE.match(seat):
            errors.append(f"row {n} seat name {seat!r} is not a lowercase seat identifier")
        elif seat in seen:
            errors.append(f"row {n} duplicates seat {seat!r} (first seen on row {seen[seat]})")
        else:
            seen[seat] = n

        if not host:
            errors.append(f"row {n} ({seat or '?'}) has an empty Host")

        if availability not in AVAILABILITY:
            errors.append(
                f"row {n} ({seat or '?'}) availability {availability!r} is not one of {AVAILABILITY}"
            )

        if not claims:
            errors.append(f"row {n} ({seat or '?'}) has an empty Open claims cell (use '-' if none)")

        if not note:
            warnings.append(f"row {n} ({seat or '?'}) has an empty Note")

        ts = parse_timestamp(measured)
        if ts is None:
            errors.append(
                f"row {n} ({seat or '?'}) measured-at {measured!r} is not UTC "
                f"YYYY-MM-DDTHH:MM:SSZ"
            )
            continue

        age = as_of - ts
        if age < timedelta(minutes=-skew_minutes):
            errors.append(
                f"row {n} ({seat or '?'}) measured-at {measured} is {-age.total_seconds() / 60:.1f} "
                f"minutes in the future (skew allowance {skew_minutes} minutes)"
            )
            continue

        if age > timedelta(minutes=max_age_minutes):
            age_min = age.total_seconds() / 60.0
            if availability == "available":
                errors.append(
                    f"row {n} ({seat}) claims 'available' but was measured {age_min:.0f} minutes "
                    f"ago, over the {max_age_minutes}-minute freshness window"
                )
            else:
                warnings.append(
                    f"row {n} ({seat}) is stale ({age_min:.0f} minutes, state {availability!r}); "
                    f"only an 'available' claim must be fresh"
                )

    if rows == 0:
        errors.append("the status table has no data rows")

    registered = load_identities(identities)
    if registered is not None:
        for seat in seen:
            if seat not in registered:
                warnings.append(
                    f"seat {seat!r} has an availability row but no entry in "
                    f"{identities or 'governance/identities.json'} (owner-reviewed registration "
                    f"still pending)"
                )
        for seat in sorted(registered):
            if seat not in seen:
                warnings.append(
                    f"registered seat {seat!r} has no row in {os.path.basename(path)}; a seat that "
                    f"never announces is indistinguishable from a seat that is gone"
                )

    return errors, warnings


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("file", help="path to SEAT_STATUS.md")
    parser.add_argument("--max-age-minutes", type=int, default=DEFAULT_MAX_AGE_MINUTES,
                        help=f"freshness window for an 'available' row "
                             f"(default {DEFAULT_MAX_AGE_MINUTES})")
    parser.add_argument("--skew-minutes", type=int, default=DEFAULT_SKEW_MINUTES,
                        help=f"tolerated clock skew into the future "
                             f"(default {DEFAULT_SKEW_MINUTES})")
    parser.add_argument("--as-of", metavar="ISO",
                        help="evaluate freshness as of this UTC time instead of now "
                             "(YYYY-MM-DDTHH:MM:SSZ), for reproducible runs")
    parser.add_argument("--identities", default=None,
                        help="path to identities.json (default: alongside the file's parent)")
    args = parser.parse_args(argv)

    as_of = None
    if args.as_of:
        as_of = parse_timestamp(args.as_of)
        if as_of is None:
            print(f"ERROR: --as-of {args.as_of!r} is not UTC YYYY-MM-DDTHH:MM:SSZ")
            return 1

    identities = args.identities
    if identities is None:
        candidate = os.path.join(os.path.dirname(os.path.abspath(args.file)), "identities.json")
        identities = candidate if os.path.exists(candidate) else None

    errors, warnings = check(
        args.file,
        max_age_minutes=args.max_age_minutes,
        skew_minutes=args.skew_minutes,
        as_of=as_of,
        identities=identities,
    )

    for message in warnings:
        print(f"WARNING: {message}")
    for message in errors:
        print(f"ERROR: {message}")

    if errors:
        print(f"{args.file}: {len(errors)} error(s), {len(warnings)} warning(s)")
        return 1
    print(f"{args.file}: OK ({len(warnings)} warning(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
