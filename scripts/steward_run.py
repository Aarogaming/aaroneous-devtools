#!/usr/bin/env python3
"""One steward tick as a data-flow pipeline with a tamper-evident local event log (lane C99).

  collect state -> ingest new agent output -> append events -> detect stalls -> issue task cards

The log is an append-only JSONL file where every line carries the SHA-256 of the previous line, so
editing or removing history *before the last event* is detectable (``verify_chain``). Editing or
deleting the final event is not visible to the chain alone, so each append also writes the tip digest
to a ``<log>.tip`` sidecar and every run prints it for anchoring somewhere the log's writer cannot
quietly rewrite (a PR comment, another seat's record); ``--expect-tip`` verifies against such an
external digest. The sidecar alone only catches casual edits. It records which cards were issued and
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
SEAT_IN_TEXT = cs.SEAT_IN_TEXT


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


def tip_path(path):
    return path + ".tip"


def current_tip(path):
    """(count, digest of the last line) for the log, or (0, GENESIS) when empty."""
    if not os.path.exists(path):
        return 0, GENESIS
    with open(path, encoding="utf-8") as handle:
        lines = [l.rstrip("\n") for l in handle if l.strip()]
    return len(lines), (digest(lines[-1]) if lines else GENESIS)


def verify_tip(path, expected=None):
    """Check the log tip against the sidecar and, when given, an external digest. Returns (ok, problem)."""
    count, tip = current_tip(path)
    if expected and tip != expected:
        return False, "tip digest does not match the externally anchored value (last events edited or removed)"
    if os.path.exists(tip_path(path)):
        with open(tip_path(path), encoding="utf-8") as handle:
            recorded = json.load(handle)
        if recorded.get("count") != count or recorded.get("tip") != tip:
            return False, "tip sidecar disagrees with the log (last events edited, removed or appended outside the tool)"
    return True, ""


def append_event(path, event_type, data, now):
    events = read_events(path)
    prev = GENESIS
    if events:
        with open(path, encoding="utf-8") as handle:
            lines = [l.rstrip("\n") for l in handle if l.strip()]
        prev = digest(lines[-1])
    event = {"seq": len(events) + 1, "ts": now, "type": event_type, "data": data, "prev": prev}
    line = json.dumps(event, sort_keys=True, ensure_ascii=False)
    with open(path, "a", encoding="utf-8", newline="\n") as handle:
        handle.write(line + "\n")
    with open(tip_path(path), "w", encoding="utf-8") as handle:
        json.dump({"count": event["seq"], "tip": digest(line)}, handle)
    return event


def seat_from_comment(body):
    return cs.comment_seat(body)


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
        elif event["type"] in ("lease_done", "lease_expired", "lease_released"):
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


def history_from_events(events):
    """Replay the log into what the dispatcher needs to vary and rotate: how often each item was handed out, who
    let a lease expire, who finishes work, and the last head each agent was given per PR."""
    history = {"attempts": {}, "expired": {}, "item_attempts": {}, "last_head": {}, "agents": {}}
    for event in events:
        data = event["data"]
        kind = event["type"]
        if kind not in ("lease_grant", "lease_expired", "lease_done"):
            continue
        agent, key = data["agent"], data["key"]
        rec = history["agents"].setdefault(agent, {"done": 0, "expired": 0})
        pair = f"{agent}|{key}"
        if kind == "lease_grant":
            history["attempts"][pair] = history["attempts"].get(pair, 0) + 1
            history["item_attempts"][key] = history["item_attempts"].get(key, 0) + 1
            parts = key.split(":")
            if len(parts) == 3:
                history["last_head"][f"{agent}|{parts[1]}"] = parts[2]
        elif kind == "lease_expired":
            history["expired"][pair] = history["expired"].get(pair, 0) + 1
            rec["expired"] += 1
        else:
            rec["done"] += 1
    return history


def attribution_weak(events, window, minimum=3):
    """True when most recently ingested comments could not be attributed to a seat, so a missing
    artifact is not evidence of a stall (the text heuristic cannot see it)."""
    ingests = [e for e in events if e["type"] == "ingest"][-window:]
    attributed = sum(sum(e["data"].get("per_seat", {}).values()) for e in ingests)
    total = attributed + sum(e["data"].get("unattributed", 0) for e in ingests)
    return total >= minimum and attributed / total < 0.5


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


def next_tick(events):
    """The tick after the last one recorded in the log (1 for a fresh log), so a persistent log numbers its own
    ticks and leases expire on a real clock instead of on whatever the caller types."""
    ticks = [e["data"].get("tick", 0) for e in events if e["type"] == "cards"]
    return (max(ticks) + 1) if ticks else 1


def write_outbox(outbox_dir, result, now):
    """One file per seat holding only that seat's current card (plus its escalation, if any), replaced
    atomically each tick. A seat's loop prompt can be one line: read <outbox>/<seat>.md and do what it says.
    The outbox is delivery only; it grants nothing and the log stays the record."""
    os.makedirs(outbox_dir, exist_ok=True)
    escalations = {e["agent"]: e for e in result["escalations"]}
    written = []
    for card in result["cards"]:
        seat = card["agent"]
        parts = [f"Steward card for {seat}, generated {now}, tick {card['tick']}.",
                 "This replaces any earlier card. If it says standby or healthy idle, do nothing and stay quiet.",
                 "", nt.render([card])]
        if seat in escalations:
            esc = escalations[seat]
            parts += ["", f"ESCALATION: {esc['task']}", f"  ACCEPT: {esc['acceptance']}"]
        path = os.path.join(outbox_dir, f"{seat}.md")
        tmp = path + ".tmp"
        with open(tmp, "w", encoding="utf-8", newline="\n") as handle:
            handle.write("\n".join(parts) + "\n")
        os.replace(tmp, path)
        written.append(path)
    return written


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
    # Replay AFTER the completion/expiry events above are appended, so a lease that expires this tick already
    # counts as burned and the item rotates to another seat instead of going straight back to its holder.
    snapshot_state = dict(snapshot_state, leases=active, history=history_from_events(read_events(log_path)))
    cards = nt.build_cards(snapshot_state, tick)
    for card in cards:
        lease = card.get("lease")
        if lease and not lease["renewed"]:
            append_event(log_path, "lease_grant",
                         {"tick": tick, "key": lease["key"], "agent": card["agent"], "expires": lease["expires_tick"]}, now)
    # Only cards that require an artifact count toward stall accounting; healthy-idle and standby cards
    # say no output is expected, so silence on them is not a stall.
    actionable = [c for c in cards if c["inputs"]["kind"] not in ("healthy_idle", "standby")]
    append_event(log_path, "cards", {
        "tick": tick,
        "agents": [c["agent"] for c in actionable],
        "tasks": [c["task"] for c in cards],
    }, now)
    events = read_events(log_path)
    names = [a["name"] for a in snapshot_state["agents"]]
    carried = {(c["agent"], c["lease"]["key"]) for c in cards if c.get("lease")}
    for key, lease in active.items():
        if (lease["agent"], key) not in carried:
            append_event(log_path, "lease_released", {"tick": tick, "key": key, "agent": lease["agent"],
                                                       "reason": "no longer eligible for its holder"}, now)
    escalations = []
    expired_agents = {lease["agent"] for lease in expired.values()}
    weak = attribution_weak(events, stall_after)
    stalled = [] if weak else stalled_seats(events, names, stall_after)
    for agent in sorted(set(stalled) | expired_agents):
        mine = [c["task"] for c in cards if c["agent"] == agent]
        escalations.append(escalation_card(agent, stall_after, mine, agent in expired_agents))
        append_event(log_path, "stall", {"tick": tick, "agent": agent, "after": stall_after,
                                         "expired_lease": agent in expired_agents}, now)
    return {"cards": cards, "escalations": escalations, "ingest": ingested,
            "leases": {"active": len(active), "completed": sorted(completed), "expired": sorted(expired)},
            "attribution_weak": weak}


def render(result):
    out = [nt.render(result["cards"])]
    for e in result["escalations"]:
        out.append(f"[{e['agent']}] ESCALATION\n  TASK: {e['task']}\n  ACCEPT: {e['acceptance']}")
    per = ", ".join(f"{s}={n}" for s, n in sorted(result["ingest"]["per_seat"].items())) or "none"
    out.append(f"INGESTED new artifacts by seat: {per}; unattributed: {result['ingest']['unattributed']}")
    if result.get("attribution_weak"):
        out.append("NOTE: most recent comments could not be attributed to a seat; engagement stalls are not judged this tick (have seats put 'Seat / agent: NAME' or a card marker in their comments)")
    lz = result["leases"]
    out.append(f"LEASES active={lz['active']} completed={len(lz['completed'])} expired={len(lz['expired'])}")
    return "\n\n".join(out)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", required=True)
    parser.add_argument("--repo-dir", required=True)
    parser.add_argument("--identities")
    parser.add_argument("--log", required=True)
    parser.add_argument("--tick", type=int, help="default: one after the last tick recorded in --log")
    parser.add_argument("--outbox", help="directory; writes <seat>.md with each seat's current card")
    parser.add_argument("--stall-after", type=int, default=3)
    parser.add_argument("--expect-tip", help="externally anchored SHA-256 of the log's last line")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    if os.path.exists(args.log):
        ok, problem = verify_chain(args.log)
        if not ok:
            raise SystemExit(f"event log failed verification: {problem}")
        ok, problem = verify_tip(args.log, args.expect_tip)
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
    tick = args.tick if args.tick is not None else next_tick(read_events(args.log))
    result = run_tick(state, snapshot, args.log, tick, now, args.stall_after)
    if args.outbox:
        result["outbox"] = write_outbox(args.outbox, result, now)
    count, tip = current_tip(args.log)
    result["tip"] = {"count": count, "sha256": tip}
    print(json.dumps(result, indent=2) if args.json else render(result) + f"\n\nLOG TIP seq={count} sha256={tip} (anchor this externally)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
