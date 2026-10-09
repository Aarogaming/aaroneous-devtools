#!/usr/bin/env python3
"""Validate per-seat host-compute declarations in governance/seat-status (C112).

A seat owns and refreshes only its own existing <seat-id>.md file. This checker
has no hard-coded seat roster: it validates file shape, state vocabulary and
freshness, then advisory-cross-checks identities.json when it is available.

Usage: check_seat_status.py [--max-age-minutes N] [--skew-minutes N]
                            [--as-of UTC_ISO] [--identities PATH] DIRECTORY
"""
import argparse
import json
import os
import re
import sys
from datetime import datetime, timedelta, timezone

HEADER = ["Seat", "Host", "Availability", "Open claims", "Measured at (UTC)", "Note"]
STATES = ("available", "busy", "offline")
SEAT_RE = re.compile(r"^[a-z0-9][a-z0-9._-]*$")
TIME_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
DEFAULT_MAX_AGE_MINUTES = 120
DEFAULT_SKEW_MINUTES = 5


def split_cells(line):
    line = line.rstrip("\n")
    if not line.startswith("|") or not line.endswith("|"):
        raise ValueError("row must start and end with '|'")
    return [x.strip() for x in re.split(r"(?<!\\)\|", line)[1:-1]]


def parse_time(value):
    if not TIME_RE.fullmatch(value or ""):
        return None
    try:
        return datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError:
        return None


def load_identities(path):
    if not path or not os.path.exists(path):
        return None
    try:
        with open(path, encoding="utf-8") as f:
            data = json.load(f)
    except (OSError, ValueError):
        return None
    seats = data.get("seats") if isinstance(data, dict) else None
    return set(seats) if isinstance(seats, dict) else None


def table_rows(path):
    """Return the sole data row cells, or an error string."""
    try:
        with open(path, encoding="utf-8") as f:
            lines = f.read().splitlines()
    except OSError as exc:
        return None, f"cannot read: {exc}"
    header_at = None
    for i, line in enumerate(lines):
        if line.startswith("|"):
            try:
                cells = split_cells(line)
            except ValueError:
                continue
            if cells == HEADER:
                header_at = i
                break
    if header_at is None:
        return None, "no exact status-table header"
    if header_at + 2 >= len(lines):
        return None, "status table needs a separator and one data row"
    try:
        sep = split_cells(lines[header_at + 1])
        row = split_cells(lines[header_at + 2])
    except ValueError as exc:
        return None, str(exc)
    if len(sep) != len(HEADER) or not all(re.fullmatch(r":?-{3,}:?", x) for x in sep):
        return None, "invalid table separator"
    if len(row) != len(HEADER):
        return None, f"data row has {len(row)} cells, expected {len(HEADER)}"
    for line in lines[header_at + 3:]:
        if line.startswith("|"):
            return None, "a seat file must contain exactly one data row"
    return row, None


def check(directory, max_age_minutes=DEFAULT_MAX_AGE_MINUTES, skew_minutes=DEFAULT_SKEW_MINUTES,
          as_of=None, identities=None):
    errors, warnings = [], []
    as_of = as_of or datetime.now(timezone.utc)
    if not os.path.isdir(directory):
        return [f"not a directory: {directory}"], warnings
    paths = sorted(
        os.path.join(directory, name) for name in os.listdir(directory)
        if name.endswith(".md") and name != "README.md" and os.path.isfile(os.path.join(directory, name))
    )
    if not paths:
        return [f"no <seat-id>.md files in {directory}"], warnings
    seen = {}
    for path in paths:
        filename_seat = os.path.basename(path)[:-3]
        row, problem = table_rows(path)
        label = os.path.basename(path)
        if problem:
            errors.append(f"{label}: {problem}")
            continue
        seat, host, state, claims, measured, note = row
        if not SEAT_RE.fullmatch(filename_seat):
            errors.append(f"{label}: filename is not a lowercase seat identifier")
        if seat != filename_seat:
            errors.append(f"{label}: Seat cell {seat!r} must equal filename {filename_seat!r}")
        if not SEAT_RE.fullmatch(seat):
            errors.append(f"{label}: seat {seat!r} is not a lowercase seat identifier")
        if seat in seen:
            errors.append(f"{label}: duplicate seat {seat!r}; first in {seen[seat]}")
        else:
            seen[seat] = label
        if not host:
            errors.append(f"{label}: Host is empty")
        if state not in STATES:
            errors.append(f"{label}: Availability {state!r} is not one of {STATES}")
        if not claims:
            errors.append(f"{label}: Open claims is empty; use '-'")
        if not note:
            warnings.append(f"{label}: Note is empty")
        ts = parse_time(measured)
        if ts is None:
            errors.append(f"{label}: measured-at {measured!r} is not UTC YYYY-MM-DDTHH:MM:SSZ")
            continue
        age = as_of - ts
        if age < timedelta(minutes=-skew_minutes):
            errors.append(f"{label}: measured-at is {-age.total_seconds()/60:.1f} minutes in the future")
        elif age > timedelta(minutes=max_age_minutes):
            minutes = age.total_seconds() / 60
            if state == "available":
                errors.append(f"{label}: available declaration is stale ({minutes:.0f}m > {max_age_minutes}m)")
            else:
                warnings.append(f"{label}: {state} declaration is stale ({minutes:.0f}m)")
    registered = load_identities(identities)
    if registered is not None:
        for seat in seen:
            if seat not in registered:
                warnings.append(f"{seat!r} has a status file but no identities.json registration")
        for seat in sorted(registered - set(seen)):
            warnings.append(f"registered seat {seat!r} has no status file")
    return errors, warnings


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("directory")
    p.add_argument("--max-age-minutes", type=int, default=DEFAULT_MAX_AGE_MINUTES)
    p.add_argument("--skew-minutes", type=int, default=DEFAULT_SKEW_MINUTES)
    p.add_argument("--as-of")
    p.add_argument("--identities")
    args = p.parse_args(argv)
    as_of = None
    if args.as_of:
        as_of = parse_time(args.as_of)
        if as_of is None:
            print(f"ERROR: --as-of {args.as_of!r} is not UTC YYYY-MM-DDTHH:MM:SSZ")
            return 1
    identities = args.identities
    if identities is None:
        candidate = os.path.join(os.path.dirname(os.path.abspath(args.directory)), "identities.json")
        identities = candidate if os.path.exists(candidate) else None
    errors, warnings = check(args.directory, args.max_age_minutes, args.skew_minutes, as_of, identities)
    for msg in warnings:
        print(f"WARNING: {msg}")
    for msg in errors:
        print(f"ERROR: {msg}")
    if errors:
        print(f"{args.directory}: {len(errors)} error(s), {len(warnings)} warning(s)")
        return 1
    print(f"{args.directory}: OK ({len(warnings)} warning(s))")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
