#!/usr/bin/env python3
"""Deterministic task-card generator (lane C98, report-only).

Static scheduled prompts make agents converge on identical "nothing changed" reports. This script
turns real state into one specific task card per agent per tick, so the prompt an agent receives is
generated from what is actually open, uncovered, stale or blocked.

It never edits the queue, claims a lane, posts a comment or merges. It reads a state snapshot and
prints cards. A card is never "idle": when no urgent work exists the work ladder falls through to a
ranked backlog, and when even that is empty to a deterministic audit task.

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
        "detail": "no urgent work: deterministic audit task so the tick produces an artifact",
    }


def build_cards(state, tick):
    agents = state["agents"]
    urgent = work_items(state)
    only_owner_left = bool(urgent) and all(i["kind"] in ("ready_to_merge", "draft") for i in urgent)
    ladder = list(urgent) + [dict(i, kind="backlog", rank=7) for i in backlog_items(state)]
    if not ladder:
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
