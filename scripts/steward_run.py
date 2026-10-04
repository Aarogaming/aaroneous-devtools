#!/usr/bin/env python3
"""One steward tick as a data-flow pipeline with a tamper-evident local event log (lane C99).

  collect state -> ingest new agent output -> append events -> detect stalls -> issue task cards

The log is an append-only JSONL file where every line carries the SHA-256 of the previous line, so
removing or editing history is detectable (``verify_chain``). It records which cards were issued and
which comments (artifacts) were ingested per seat, so engagement is measured from output, not from
polling: a seat that was handed cards but produced no artifact for ``K`` ticks gets an escalation card
that demands either the artifact or an explicit named blocker.

Report-only: reads GitHub through collect_state.py, writes only the local log file you name, never
posts, claims or merges.

Usage:
  steward_run.py --repo OWNER/NAME --repo-dir CLONE --identities governance/identities.json
                 --log steward-events.jsonl --tick N [--stall-after 3] [--json]
"""

import argparse
import hashlib
import json
import os
import re
import sys
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import collect_state as cs  # noqa: E402
import next_task as nt  # noqa: E402

GENESIS = "0" * 64
SEAT_IN_TEXT = re.compile(
    r"(?:Seat\s*/\s*agent|Reviewer(?:\s+seat)?)\W{0,6}(Claude|Mimo|Codex|Gemini)|^\s*(Claude|Codex|Gemini|Mimo)\b",
    re.I | re.M,
)


def digest(line):
    return hashlib.sha256(line.encode("utf-8")).hexdigest()


def read_events(path):
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8") as handle:
        return [json.loads(line) for line in handle if line.strip()]


def verify_chain(path):
    """Return (ok, problem). Every event must carry the digest of the previous raw line."""
    prev = GENESIS
    with open(path, encoding="utf-8") as handle:
        for number, line in enumerate(handle, 1):
            line = line.rstrip("\n")
            if not line:
                continue
            event = json.loads(line)
            if event.get("prev") != prev:
                return False, f"line {number}: chain broken (history edited or removed)"
            prev = digest(line)
    return True, ""


def append_event(path, event_type, data, now):
    events = read_events(path)
    prev = GENESIS
    if events:
        with open(path, encoding="utf-8") as handle:
            lines = [l.rstrip("\n") for l in handle if l.strip()]
        prev = digest(lines[-1])
    event = {"seq": len(events) + 1, "ts": now, "type": event_type, "data": data, "prev": prev}
    with open(path, "a", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(event, sort_keys=True, ensure_ascii=False) + "\n")
    return event


def seat_from_comment(body):
    found = SEAT_IN_TEXT.search(body or "")
    if not found:
        return ""
    return next(g for g in found.groups() if g).lower()


def seen_comment_ids(events):
    seen = set()
    for event in events:
        if event["type"] == "ingest":
            seen.update(event["data"].get("comment_ids", []))
    return seen


def ingest(snapshot, events):
    """New comments since the last ingest, attributed to a seat where the text says which."""
    seen = seen_comment_ids(events)
    fresh, per_seat = [], {}
    for pr in snapshot["prs"]:
        for comment in pr.get("comments", []):
            cid = comment.get("id")
            if cid is None or cid in seen:
                continue
            seat = seat_from_comment(comment.get("body", ""))
            fresh.append({"id": cid, "pr": pr["number"], "seat": seat})
            if seat:
                per_seat[seat] = per_seat.get(seat, 0) + 1
    return {"comment_ids": [f["id"] for f in fresh], "per_seat": per_seat, "unattributed": sum(1 for f in fresh if not f["seat"])}


def lease_table(events, tick, done):
    """Replay lease events. A lease is granted with an expiry tick; it ends when its card is completed
    (key in `done`), is released, or expires. Returns (active, expired, completed) keyed by card key."""
    granted = {}
    for event in events:
        data = event["data"]
        if event["type"] == "lease_grant":
            granted[data["key"]] = {"agent": data["agent"], "expires": data["expires"]}
        elif event["type"] in ("lease_done", "lease_expired"):
            granted.pop(data["key"], None)
    active, expired, completed = {}, {}, {}
    for key, lease in granted.items():
        if key in done:
            completed[key] = lease
        elif tick >= lease["expires"]:
            expired[key] = lease
        else:
            active[key] = lease
    return active, expired, completed


def stalled_seats(events, agents, stall_after):
    """Seats that were issued cards in each of the last K ticks but ingested no artifact in them."""
    ticks = {}
    for event in events:
        tick = event["data"].get("tick")
        if tick is None:
            continue
        slot = ticks.setdefault(tick, {"issued": set(), "produced": set()})
        if event["type"] == "cards":
            slot["issued"].update(event["data"].get("agents", []))
        if event["type"] == "ingest":
            slot["produced"].update(s for s, n in event["data"].get("per_seat", {}).items() if n)
    recent = sorted(ticks)[-stall_after:]
    if len(recent) < stall_after:
        return []
    return [
        a for a in agents
        if all(a in ticks[t]["issued"] and a not in ticks[t]["produced"] for t in recent)
    ]


def escalation_card(agent, stall_after, last_cards, expired_lease=False):
    reason = ("a lease expired without completion" if expired_lease
              else f"no artifact for {stall_after} ticks although cards were issued")
    return {
        "agent": agent,
        "role": "escalation",
        "tick": None,
        "priority": 0,
        "task": f"{agent}: {reason}: do the card now or name the blocker",
        "why_now": "engagement stall or expired lease detected from the event log",
        "inputs": {"pr": None, "kind": "stall"},
        "marker": "",
        "acceptance": "a comment that either completes one of the cards below or states exactly which step is blocked, by whom and what would unblock it",
        "if_blocked": "that statement is itself the artifact; silence is not an acceptable outcome",
        "owner_blocked": False,
        "last_cards": last_cards,
    }


def run_tick(snapshot_state, comments_snapshot, log_path, tick, now, stall_after=3):
    """Pure-ish tick: `snapshot_state` is collect_state output, `comments_snapshot` the raw snapshot."""
    events = read_events(log_path)
    ingested = ingest(comments_snapshot, events)
    append_event(log_path, "ingest", dict(ingested, tick=tick), now)
    active, expired, completed = lease_table(events, tick, set(snapshot_state.get("done", [])))
    for key, lease in completed.items():
        append_event(log_path, "lease_done", {"tick": tick, "key": key, "agent": lease["agent"]}, now)
    for key, lease in expired.items():
        append_event(log_path, "lease_expired", {"tick": tick, "key": key, "agent": lease["agent"]}, now)
    snapshot_state = dict(snapshot_state, leases=active)
    cards = nt.build_cards(snapshot_state, tick)
    for card in cards:
        lease = card.get("lease")
        if lease and not lease["renewed"]:
            append_event(log_path, "lease_grant",
                         {"tick": tick, "key": lease["key"], "agent": card["agent"], "expires": lease["expires_tick"]}, now)
    append_event(log_path, "cards", {
        "tick": tick,
        "agents": [c["agent"] for c in cards],
        "tasks": [c["task"] for c in cards],
    }, now)
    events = read_events(log_path)
    names = [a["name"] for a in snapshot_state["agents"]]
    escalations = []
    expired_agents = {lease["agent"] for lease in expired.values()}
    for agent in sorted(set(stalled_seats(events, names, stall_after)) | expired_agents):
        mine = [c["task"] for c in cards if c["agent"] == agent]
        escalations.append(escalation_card(agent, stall_after, mine, agent in expired_agents))
        append_event(log_path, "stall", {"tick": tick, "agent": agent, "after": stall_after,
                                         "expired_lease": agent in expired_agents}, now)
    return {"cards": cards, "escalations": escalations, "ingest": ingested,
            "leases": {"active": len(active), "completed": sorted(completed), "expired": sorted(expired)}}


def render(result):
    out = [nt.render(result["cards"])]
    for e in result["escalations"]:
        out.append(f"[{e['agent']}] ESCALATION\n  TASK: {e['task']}\n  ACCEPT: {e['acceptance']}")
    per = ", ".join(f"{s}={n}" for s, n in sorted(result["ingest"]["per_seat"].items())) or "none"
    out.append(f"INGESTED new artifacts by seat: {per}; unattributed: {result['ingest']['unattributed']}")
    lz = result["leases"]
    out.append(f"LEASES active={lz['active']} completed={len(lz['completed'])} expired={len(lz['expired'])}")
    return "\n\n".join(out)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", required=True)
    parser.add_argument("--repo-dir", required=True)
    parser.add_argument("--identities")
    parser.add_argument("--log", required=True)
    parser.add_argument("--tick", type=int, required=True)
    parser.add_argument("--stall-after", type=int, default=3)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    if os.path.exists(args.log):
        ok, problem = verify_chain(args.log)
        if not ok:
            raise SystemExit(f"event log failed verification: {problem}")
    remote = cs.rc.git(args.repo_dir, "remote", "get-url", "origin")
    if remote.returncode != 0 or not cs.rc.checkout_matches(remote.stdout, args.repo):
        raise SystemExit(f"--repo-dir {args.repo_dir!r} is not a clone of {args.repo}")
    identities = cs.rc.load_identities(args.identities) if args.identities else None
    keys = cs.rc.load_keys(args.identities) if args.identities else None
    snapshot = cs.rc.live_snapshot(args.repo, args.repo_dir)
    state = cs.collect(args.repo, args.repo_dir, ["claude", "mimo", "codex", "gemini"], [], identities)
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    result = run_tick(state, snapshot, args.log, args.tick, now, args.stall_after)
    print(json.dumps(result, indent=2) if args.json else render(result))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
