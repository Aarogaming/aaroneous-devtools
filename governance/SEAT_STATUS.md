# Seat status: host compute availability

Version: 1 (2026-10-09). Lane C112, owner direction 2026-10-09.

## What this file is

Each seat publishes whether its **host** has spare compute, so the other seats
can tell — between active user sessions, with nobody at the console — that a
machine is free to take work.

It is deliberately **not** lane capacity. How many primary lanes are open is a
global number derived by `foundation schedule-review`
(`contracts/schedule-review-v1.schema.json`) from the queue, and is never
inferred from this file. A seat can be compute-available while holding claims,
and compute-busy while holding none; the `Open claims` column exists so a
reader can see both facts side by side instead of guessing.

## One writer, two readers

Only a seat writes **its own row**. Readers — the steward, and the HTTP mirror
specified but not yet claimed in C112 Phase 3 — consume this file and never
write it.

Two writers is how `KNOWN_SEATS` diverged: `scripts/collect_state.py:33` and
`scripts/review_coverage.py:146` hold different copies of the seat roster, and
`antigravity` is in one but not the other. This file must not become a third
roster, which is why `scripts/check_seat_status.py` hard-codes no seat list and
only ever *warns* about identity membership.

## Declared, not measured

The table records **declared** state, refreshed when it changes and at most
hourly. Live telemetry (GPU utilisation, load, uptime) is never committed here:
polling it into the checkout would dirty the tree on every tick and violates
the standing rule against generated artifacts in shared paths.

The measured values belong to the HTTP leg (C112 Phase 3), which is specified
in the queue row but not claimed. Until it exists, read them directly from the
host.

## Table

Column meanings:

- **Seat** — the seat name. Must match `governance/identities.json` once the
  seat is registered. A mismatch is a warning, never an error: registration is
  an owner-reviewed PR and may legitimately lag this file.
- **Availability** — exactly one of `available`, `busy`, `offline`.
- **Open claims** — queue IDs the seat holds, or `-`. Informational only.
- **Measured at** — UTC ISO-8601, `YYYY-MM-DDTHH:MM:SSZ`.

| Seat | Host | Availability | Open claims | Measured at (UTC) | Note |
| --- | --- | --- | --- | --- | --- |
| ubuntu-worker-01 | aaroneoushome (100.109.8.12) | available | C111, C112 | 2026-10-09T05:30:00Z | RTX 4060 idle and the heavy Cargo slot free at measurement; host compute only, lane capacity is schedule-review's output |

## Freshness

`scripts/check_seat_status.py` **fails** when an `available` row is older than
the freshness window (default 120 minutes), because a stale claim to be free is
a false offer.

Stale `busy` and `offline` rows are **warnings**, not failures: a seat that
stopped reporting while busy is not advertising anything untrue, and failing CI
for it would push seats to refresh rows they cannot honestly refresh.

## Updating your row

A seat edits only its own row, and only under a claim that owns this file:

1. Set `Measured at` to the current UTC time (`date -u +%Y-%m-%dT%H:%M:%SZ`).
2. Update `Availability` if your host state actually changed.
3. Refresh on state change and at most hourly — not every tick.

Do not edit another seat's row. Release or hand off instead, per the 24-hour
claim rule in `governance/COORDINATION_QUEUE.md`.

## Verification

```bash
python3 scripts/test_check_seat_status.py            # exit 0
python3 scripts/check_seat_status.py governance/SEAT_STATUS.md   # exit 0
```
