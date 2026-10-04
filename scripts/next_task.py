#!/usr/bin/env python3
"""Deterministic task-card generator (lane C99, report-only).

Static scheduled prompts make agents converge on identical "nothing changed" reports. This script
turns real state into one specific task card per agent per tick, so the prompt an agent receives is
generated from what is actually open, uncovered, stale or blocked.

It never edits the queue, claims a lane, posts a comment or merges. It reads a state snapshot and
prints cards. When urgent work exists the cards name it; when it does not, the work ladder falls
through to a ranked backlog. If the snapshot is genuinely healthy (nothing urgent, no backlog) the
result is an explicit ``healthy_idle`` card carrying the evidence that was checked, which is a valid
outcome (STEWARD_HEARTBEAT.md: a verified healthy no-op, never invented work). A periodic audit
spot-check is issued every ``AUDIT_EVERY`` ticks so healthy state is re-verified, not merely assumed.

State snapshot (JSON), per PR: number, title, head, base, draft, mergeable_state, updated_at (ISO),
seat (the implementing seat), threads_unresolved (int), covered (bool, from review_coverage.py),
needs_review_from_anyone (derived here). Top level: now (ISO), agents (list of
{name, roles}), backlog (optional list of {id, task, acceptance}), owner_blocked (derived).

Usage:
  next_task.py --snapshot STATE.json --tick N [--json]
"""

import argparse
import json
import sys
from datetime import datetime, timezone

STALE_HOURS = 72
AUDIT_EVERY = 6
DEFAULT_ROLES = ["implementer", "reviewer", "auditor", "skeptic"]


def parse_time(text):
    return datetime.fromisoformat(text.replace("Z", "+00:00")).astimezone(timezone.utc)


def work_items(state):
    """Ranked list of concrete work items derived from the snapshot. Lower rank is more urgent."""
    now = parse_time(state["now"])
    items = []
    for pr in state["prs"]:
        number, draft = pr["number"], pr.get("draft", False)
        clean = pr.get("mergeable_state") == "CLEAN"
        threads = pr.get("threads_unresolved", 0)
        age_h = (now - parse_time(pr["updated_at"])).total_seconds() / 3600
        base = {"pr": number, "title": pr.get("title", ""), "seat": pr.get("seat", "")}
        if threads:
            items.append(dict(base, kind="fix_threads", rank=1, owner_seat=pr.get("seat"),
                              detail=f"{threads} unresolved review thread(s)"))
        elif not pr.get("covered", False) and not draft:
            items.append(dict(base, kind="needs_review", rank=2, detail="no covered whole-diff accept at the current head"))
        elif draft:
            items.append(dict(base, kind="draft", rank=4, detail="draft: merge option hidden until ready for review"))
        elif pr.get("covered") and clean:
            items.append(dict(base, kind="ready_to_merge", rank=3, detail="covered, CLEAN, no unresolved threads"))
        elif not clean:
            items.append(dict(base, kind="not_mergeable", rank=2, detail=f"mergeable_state={pr.get('mergeable_state')}"))
        if age_h > STALE_HOURS:
            items.append(dict(base, kind="stale", rank=5, detail=f"no update for {int(age_h)}h"))
    return sorted(items, key=lambda i: (i["rank"], i["pr"]))


def backlog_items(state):
    return list(state.get("backlog", []))


def card_for(agent, role, tick, item):
    kind = item["kind"]
    pr = item.get("pr")
    ref = f"PR #{pr}" if pr else item.get("id", "backlog")
    tasks = {
        "fix_threads": ("Fix and answer every unresolved review thread on " + ref,
                        "every thread has a reply naming the commit that addresses it, and is resolved only if addressed"),
        "needs_review": ("Post an independent whole-diff review of " + ref + " at its current head, with an attestation block",
                         "a packet with commands run and exit codes, plus one `attestation` JSON block; review_coverage.py then reads covered=True or a stated reason"),
        "not_mergeable": ("Make " + ref + " mergeable: " + item.get("detail", ""),
                          "mergeable_state is CLEAN at the new head; new head stated, earlier reviews marked as not carrying"),
        "ready_to_merge": ("Prepare the owner merge brief for " + ref,
                           "a one-command merge line with the full head SHA, the evidence for each merge condition, and the order relative to other ready PRs"),
        "draft": ("Ask the author, or prepare the one-line command, to mark " + ref + " ready for review",
                  "the PR is no longer a draft, or a comment names who must flip it"),
        "stale": ("Nudge or triage " + ref + ": " + item.get("detail", ""),
                  "a comment with the specific missing step, the owner of that step and its acceptance condition"),
    }
    if kind in tasks:
        task, acceptance = tasks[kind]
    else:
        task, acceptance = item["task"], item.get("acceptance", "a retained artifact (commit, comment or report) with evidence")
    return {
        "agent": agent["name"],
        "role": role,
        "tick": tick,
        "priority": item.get("rank", 9),
        "task": task,
        "why_now": item.get("detail", "ranked next on the backlog"),
        "inputs": {"pr": pr, "kind": kind},
        "acceptance": acceptance,
        "if_blocked": "say exactly which step is blocked and by whom, then take the next item on the ladder; do not report 'pending' as the outcome",
    }


def audit_item(state, tick):
    return {
        "kind": "audit",
        "id": "audit-newest-merged",
        "rank": 8,
        "task": "Re-run review_coverage.py against the newest merged PR and compare its structured verdict with the prose verdicts in its comments; report any disagreement",
        "acceptance": "a comment or report listing agreements and disagreements, each with the comment id and head SHA",
        "detail": f"periodic audit (every {AUDIT_EVERY} ticks): re-verify healthy state",
    }


def healthy_idle_card(agent, role, tick, state):
    prs = state["prs"]
    return {
        "agent": agent["name"],
        "role": role,
        "tick": tick,
        "priority": 9,
        "task": "Healthy idle: no action required this tick",
        "why_now": "verified healthy: nothing urgent and no backlog",
        "inputs": {"pr": None, "kind": "healthy_idle"},
        "evidence": {
            "open_prs": len(prs),
            "covered": sum(1 for p in prs if p.get("covered")),
            "unresolved_threads": sum(p.get("threads_unresolved", 0) for p in prs),
            "not_clean": sum(1 for p in prs if p.get("mergeable_state") != "CLEAN"),
            "backlog": len(state.get("backlog", [])),
        },
        "acceptance": "no artifact required; do not invent work or notify",
        "if_blocked": "not applicable; if state changes the next card will say so; do not report 'pending' as the outcome",
        "owner_blocked": False,
    }


def build_cards(state, tick):
    agents = state["agents"]
    urgent = work_items(state)
    only_owner_left = bool(urgent) and all(i["kind"] in ("ready_to_merge", "draft") for i in urgent)
    ladder = list(urgent) + [dict(i, kind="backlog", rank=7) for i in backlog_items(state)]
    if not ladder:
        if tick % AUDIT_EVERY != 0:
            return [
                healthy_idle_card(agent, (agent.get("roles") or DEFAULT_ROLES)[(tick + i) % len(agent.get("roles") or DEFAULT_ROLES)], tick, state)
                for i, agent in enumerate(agents)
            ]
        ladder = [audit_item(state, tick)]
    cards = []
    for index, agent in enumerate(agents):
        roles = agent.get("roles") or DEFAULT_ROLES
        role = roles[(tick + index) % len(roles)]
        eligible = [
            item for item in ladder
            if not (item["kind"] == "needs_review" and item.get("seat") == agent["name"])
            and not (item["kind"] == "fix_threads" and item.get("owner_seat") not in (None, "", agent["name"]))
        ] or [audit_item(state, tick)]
        item = eligible[(tick + index) % len(eligible)]
        card = card_for(agent, role, tick, item)
        card["owner_blocked"] = only_owner_left
        cards.append(card)
    return cards


def render(cards):
    out = []
    for c in cards:
        out.append(
            f"[{c['agent']}] role={c['role']} tick={c['tick']} p{c['priority']}\n"
            f"  TASK: {c['task']}\n  WHY NOW: {c['why_now']}\n  ACCEPT: {c['acceptance']}\n  IF BLOCKED: {c['if_blocked']}"
        )
    return "\n\n".join(out)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--snapshot", required=True)
    parser.add_argument("--tick", type=int, required=True)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    with open(args.snapshot, encoding="utf-8") as handle:
        state = json.load(handle)
    cards = build_cards(state, args.tick)
    print(json.dumps(cards, indent=2) if args.json else render(cards))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
