# Seat status: host compute availability

Version: 1 (2026-10-09). Lane C112, owner direction 2026-10-09.

## Purpose

Each seat declares whether its **host** has spare compute, so peers can tell —
between active user sessions, with nobody at the console — that a machine is
free to take work.

This is not lane capacity. `foundation schedule-review` derives global primary
lane capacity from the queue; this directory only describes a host. A host can
be compute-available while its seat holds claims, and compute-busy while it
holds none. The `Open claims` cell exposes that distinction rather than asking
a reader to infer it.

## One seat, one file

Every seat has one file named `<seat-id>.md`. A file contains exactly one row
using the table below. The filename and its `Seat` cell must match.

| Seat | Host | Availability | Open claims | Measured at (UTC) | Note |
| --- | --- | --- | --- | --- | --- |
| example-seat | example-host | available | - | 2026-01-01T00:00:00Z | Example only; do not copy this row into a seat file |

- **Availability** is exactly `available`, `busy`, or `offline`.
- **Measured at** is UTC ISO-8601: `YYYY-MM-DDTHH:MM:SSZ`.
- **Open claims** is informative only; use `-` when there are none.

## Authority and refreshes

Only a seat writes its own file. The owner-authorized C112 exception in
[`AGENTS.md`](../../AGENTS.md) allows a seat to refresh its own existing file
without a new queue claim, but it does **not** allow any other edit. Initial
file creation, protocol changes, and removal remain ordinary claimed work.

Readers — a future steward projection and HTTP mirror — consume these files
and never write them. This avoids maintaining another hard-coded seat roster:
the validator reads `governance/identities.json` when present and warns, rather
than fails, when registration has not yet caught up.

## Freshness

Refresh on a state change and at most hourly. The validator fails when an
`available` declaration is older than 120 minutes: stale free-compute evidence
is a false offer. Stale `busy` and `offline` declarations only warn.

Live telemetry (GPU utilisation, load, uptime) is not committed here. It would
dirty the checkout each poll; it belongs to the future HTTP leg instead.

## Verify

```bash
python3 scripts/check_seat_status.py governance/seat-status
python3 scripts/test_check_seat_status.py
```
