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
import shlex
import sys
from datetime import datetime, timezone

STALE_HOURS = 72
AUDIT_EVERY = 6
LEASE_TTL = 6
DEFAULT_ROLES = ["implementer", "reviewer", "auditor", "skeptic"]

# Each time the same item is handed out again (by anyone) the framing changes, so a repeat is never the
# same prompt: the lens index is the number of times the item has already been handed out.
LENSES = ("direct", "skeptic", "minimal", "handoff")
LENS_TEXT = {
    "direct": "Do the task as stated.",
    "skeptic": "Earlier handouts of this item did not finish. Assume the obvious path hides a problem: find the "
               "two weakest claims in the diff or thread and verify each by running something, not reading.",
    "minimal": "Produce the smallest artifact that changes the state (one comment, one command output, one "
               "attestation) and stop. Do not widen scope.",
    "handoff": "Write for a seat with no memory of this PR: starting state, what you ran with exit codes, what "
               "remains. Leave it continuable.",
}


def parse_time(text):
    return datetime.fromisoformat(text.replace("Z", "+00:00")).astimezone(timezone.utc)


def item_key(kind, pr, head):
    return f"{kind}:{pr}:{(head or '')[:8]}"


def work_items(state):
    """Ranked list of concrete work items derived from the snapshot. Lower rank is more urgent.

    Items already completed for the same PR head (their key is in state["done"], filled from
    ``[done:...]`` receipts agents leave in their output) are skipped, so a finished task is not
    handed out again; a new head makes a new key and the item reappears.
    """
    now = parse_time(state["now"])
    done = set(state.get("done", []))
    default_branch = state.get("default_branch", "main")
    items = []
    for pr in state["prs"]:
        number, draft = pr["number"], pr.get("draft", False)
        clean = pr.get("mergeable_state") == "CLEAN"
        threads = pr.get("threads_unresolved", 0)
        age_h = (now - parse_time(pr["updated_at"])).total_seconds() / 3600
        base = {"pr": number, "title": pr.get("title", ""), "seat": pr.get("seat", ""), "head": pr.get("head", ""),
                "implementers": list(pr.get("implementers") or ([pr["seat"]] if pr.get("seat") else [])),
                "base_ref": pr.get("base", default_branch), "base_sha": pr.get("base_sha", ""),
                "size": pr.get("size", ""), "mergeable_state": pr.get("mergeable_state", ""),
                "rejected_unattributed": [r for r in pr.get("rejected_verdicts", []) if not r.get("seat")]}
        unanswered = pr.get("threads_unanswered", threads)
        chain_start = len(items)
        if threads and unanswered:
            items.append(dict(base, kind="fix_threads", rank=1, owner_seat=pr.get("seat"),
                              detail=f"{unanswered} of {threads} unresolved review thread(s) not yet answered"))
        elif threads:
            # Every open thread has an answer from the PR's side: nothing is left for the author to fix. What remains is
            # a reviewer reading the replies and resolving. Handing the author the same card every tick never completes.
            items.append(dict(base, kind="resolve_threads", rank=2, owner_seat=None,
                              detail=f"{threads} unresolved review thread(s), all answered; waiting for a reviewer to resolve"))
        elif pr.get("standing_blocker"):
            # A reviewer's rejecting verdict stands at this head. Another review of the same head cannot help and the
            # author was never told: the next move is the author's, and it clears when a new head is reviewed.
            blocker = pr["standing_blocker"]
            items.append(dict(base, kind="address_review", rank=1, owner_seat=pr.get("seat"), blocker=blocker,
                              detail=f"{blocker['verdict']} by {blocker['by']} stands at the current head"))
        elif not pr.get("covered", False):
            where = " (a draft: flipping it ready waits until it is covered)" if draft else ""
            items.append(dict(base, kind="needs_review", rank=2,
                              detail="no covered whole-diff accept at the current head" + where))
        elif draft:
            items.append(dict(base, kind="draft", rank=4, detail="draft: merge option hidden until ready for review"))
        elif pr.get("covered") and clean and pr.get("base", default_branch) != default_branch:
            items.append(dict(base, kind="stacked", rank=6,
                              detail=f"covered and CLEAN but stacked on {pr.get('base')}: lands after its base, not into {default_branch}"))
        elif pr.get("covered") and clean:
            items.append(dict(base, kind="ready_to_merge", rank=3, detail="covered, CLEAN, no unresolved threads"))
        elif not clean:
            items.append(dict(base, kind="not_mergeable", rank=2, detail=f"mergeable_state={pr.get('mergeable_state')}"))
        if not base["implementers"]:
            # Nobody is known to have written this PR. An author-only card would go to whichever seat is first, and a
            # review card could go to the real author. Withhold those and ask for the facts instead.
            chain = items[chain_start:]
            if any(i["kind"] in NEEDS_KNOWN_IMPLEMENTERS for i in chain):
                items[chain_start:] = [i for i in chain if i["kind"] not in NEEDS_KNOWN_IMPLEMENTERS]
                items.append(dict(base, kind="identify_implementers", rank=1, owner_seat=None,
                                  detail="implementers unknown (no seat trailer, no seat-named branch): "
                                         "author-only and independence-dependent cards are withheld until they are known"))
        rejected = pr.get("rejected_verdicts", [])
        # An implementer's verdict can never count, so repairing it would be wasted work: skip those seats.
        implementers = set(base["implementers"]) | {pr.get("seat")}
        attributed = sorted({r["seat"] for r in rejected if r.get("seat") and r["seat"] not in implementers})
        if attributed and not pr.get("covered", False):
            seat = attributed[0]
            mine = [r for r in rejected if r.get("seat") == seat]
            items.append(dict(base, kind="repair_verdict", rank=1, owner_seat=seat, rejected=mine,
                              detail=f"{len(mine)} verdict comment(s) by {seat} were rejected, so the PR is not covered"))
        if age_h > STALE_HOURS:
            items.append(dict(base, kind="stale", rank=5, detail=f"no update for {int(age_h)}h"))
    # A receipt closes a card for a head, but resolving threads is observable on GitHub: the item disappears when the
    # threads are resolved, and a receipt must not hide it while one is still open.
    items = [i for i in items if i["kind"] in OBSERVABLE_KINDS
             or item_key(i["kind"], i["pr"], i["head"]) not in done]
    # A stale overlay on a PR that already has its primary action (merge brief or draft flip) is noise.
    primary = {i["pr"] for i in items if i["kind"] in ("ready_to_merge", "draft", "stacked")}
    items = [i for i in items if not (i["kind"] == "stale" and i["pr"] in primary)]
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
        "address_review": ("Answer the standing " + str((item.get("blocker") or {}).get("verdict", "rejecting verdict"))
                           + " by " + str((item.get("blocker") or {}).get("by", "a reviewer")) + " on " + ref,
                           "a new head that fixes what the reviewer found, or a reply naming each finding and the commit or evidence that answers it; "
                           "the card clears when the new head is reviewed"),
        "identify_implementers": ("Identify who implemented " + ref + " from git, or say it cannot be established",
                                  "a comment naming each implementing seat with the git evidence (trailer or branch), or stating that none can be "
                                  "established; until the implementers are known no review, fix or resolve card is issued for this PR"),
        "resolve_threads": ("Check the replies on every unresolved review thread of " + ref + " and resolve each one that is addressed",
                            "every thread is resolved after you checked the commit its reply names, or carries your reply saying what is still missing"),
        "needs_review": ("Post an independent whole-diff review of " + ref + " at its current head, with an attestation block",
                         "a packet with commands run and exit codes, plus one `attestation` JSON block; review_coverage.py then reads covered=True or a stated reason"),
        "not_mergeable": ("Make " + ref + " mergeable: " + item.get("detail", ""),
                          "mergeable_state is CLEAN at the new head; new head stated, earlier reviews marked as not carrying"),
        "ready_to_merge": ("Prepare the owner merge brief for " + ref,
                           "a one-command merge line with the full head SHA, the evidence for each merge condition, and the order relative to other ready PRs"),
        "draft": ("Ask the author, or prepare the one-line command, to mark " + ref + " ready for review",
                  "the PR is no longer a draft, or a comment names who must flip it"),
        "repair_verdict": ("Repair your rejected verdict on " + ref + ": the review you posted did not count",
                           "review_coverage.py reads a structured verdict from you at the current head, or states a different reason"),
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
        "marker": f"[done:{kind}:{pr}:{item.get('head', '').lower()}]" if pr else "",
        "acceptance": acceptance + (
            f" Start your comment with the receipt [done:{kind}:{pr}:{item.get('head', '').lower()}] (the full 40-character head) "
            "and name your seat on the first line, so this task is not re-issued; if you could not complete it, do not use "
            "that receipt and say what blocked you." if pr and kind not in OBSERVABLE_KINDS else ""),
        "if_blocked": "say exactly which step is blocked and by whom, then take the next item on the ladder; do not report 'pending' as the outcome",
    }


# Kinds whose completion is visible on GitHub (see collect_state.OBSERVABLE_KINDS); no receipt is requested for them.
OBSERVABLE_KINDS = ("resolve_threads", "address_review", "identify_implementers")
# Kinds that are meaningless without a known author or known independence: they are withheld while the implementers
# are unknown, and an identify_implementers item takes their place.
NEEDS_KNOWN_IMPLEMENTERS = ("fix_threads", "resolve_threads", "address_review", "needs_review")


def implementers_field(item, base_sha, head):
    """What a reviewer should put in `implementers`. A known list is printed as before. When the implementers are not
    known the old text printed `implementers=[]`, which the coverage tool rejects ("implementers must be a non-empty
    list"), so a seat that followed the card posted a review that could never count. Say how to find them instead."""
    names = item.get("implementers") or []
    if names:
        return repr(list(names))
    return ("<NOT KNOWN, so find them: git log --format='%(trailers:key=Co-Authored-By,key=Contribution-ID,only=true,unfold=true)' "
            f"{base_sha or '<merge-base>'}..{head}, plus a seat-named branch prefix; the list must not be empty, so if git "
            "names no seat, post a comment saying the implementers are unknown instead of an accept block>")


def recipe_for(item):
    """Exact commands for the item with the real SHAs filled in. Never contains a pasteable attestation block:
    a pasted block counts as that seat's verdict, so only the field names are listed."""
    pr, head, base_ref, base_sha = item.get("pr"), item.get("head", ""), item.get("base_ref", "main"), item.get("base_sha", "")
    if item["kind"] == "needs_review" and pr:
        return [
            f"git fetch --prune origin && git fetch origin pull/{pr}/head   # head must be {head}",
            f"git diff --check {base_sha or '<merge-base>'}...{head}   # expect exit 0",
            # A base branch name is chosen by whoever opened the PR and git allows ' $ ( ) ; & in ref names, so it is
            # quoted; a seat pastes this line into a shell.
            f"git merge-tree --write-tree {shlex.quote('origin/' + base_ref)} {head}   # expect exit 0 (clean trial merge)",
            "run the PR's own tests if it changes code (python -m unittest discover -s scripts -p 'test_*.py')",
            f"post ONE attestation block with fields: repo, pr={pr}, head={head}, base_sha={base_sha or '<merge-base>'}, "
            f"scope=whole, reviewer=<your seat>, implementers={implementers_field(item, base_sha, head)}, verdict",
        ]
    if item["kind"] == "repair_verdict" and pr:
        return [
            "read why each of your comments was rejected (REJECTED lines above)",
            f"post a new comment, built from a file so backslash sequences are not mangled, holding ONE fenced block tagged "
            f"attestation whose body is a JSON object (not YAML) with exactly these fields: repo (this repository), "
            f"pr={pr}, head={head}, base_sha={base_sha or '<merge-base>'}, scope=whole, reviewer=<your seat>, "
            f"implementers={implementers_field(item, base_sha, head)}, verdict",
            "keep your analysis as prose above the block",
        ]
    if item["kind"] == "identify_implementers" and pr:
        return [f"git log --format='%(trailers:key=Co-Authored-By,key=Contribution-ID,only=true,unfold=true)' "
                f"{base_sha or '<merge-base>'}..{head}   # trailers naming a seat",
                "read the head branch name: a seat-named prefix such as codex/ names its owner",
                "post a comment naming each implementing seat with that evidence, or saying none can be established; "
                "the author may add a Co-Authored-By trailer in a new commit"]
    if item["kind"] == "address_review" and pr:
        return ["read the reviewer's latest comment on the PR and list each finding it names",
                "fix each finding in a commit, or reply with the evidence that answers it",
                "push the new head and say so on the PR; the reviewer re-reviews the new head"]
    if item["kind"] == "resolve_threads" and pr:
        return ["list the unresolved threads on the PR with their replies",
                "for each thread, read the reply and open the commit it names",
                "resolve the thread only if it is actually addressed; otherwise reply with what is still missing"]
    if item["kind"] == "fix_threads" and pr:
        return ["list the unresolved threads on the PR", "fix each in a commit",
                "reply on each thread naming the commit; resolve only what is addressed"]
    return []


def enrich_card(card, item, name, state, renewed, upcoming):
    """Fresh, forward-format context: where the item stands now, what changed since this agent last saw it, the
    exact commands, a framing that differs from every earlier handout of the same item, and what comes next."""
    history = state.get("history", {})
    key = item_key(item["kind"], item["pr"], item.get("head", "")) if item.get("pr") else None
    handed = history.get("item_attempts", {}).get(key, 0) if key else 0
    index = max(handed - 1, 0) if renewed else handed
    card["lens"] = LENSES[index % len(LENSES)]
    card["lens_text"] = LENS_TEXT[card["lens"]]
    card["attempt"] = index + 1
    if item.get("pr"):
        head8 = item.get("head", "")[:8]
        card["state"] = (f"PR #{item['pr']} head {item.get('head', '')[:12]} on {item.get('base_ref', 'main')}; "
                         f"{item.get('size') or 'size unknown'}; mergeable={item.get('mergeable_state') or '?'}; "
                         f"implementers={','.join(item.get('implementers') or []) or 'unknown'}")
        last = history.get("last_head", {}).get(f"{name}|{item['pr']}")
        if last is None:
            card["new_since_last"] = "first time this seat is handed this PR"
        elif last != head8:
            card["new_since_last"] = f"head moved {last} -> {head8} since your last card for this PR; earlier accepts do not carry"
        else:
            card["new_since_last"] = f"same head {head8} as your last card; do not repeat the same output, use the lens below"
    else:
        card["state"] = item.get("detail", "")
        card["new_since_last"] = ""
    card["recipe"] = recipe_for(item)
    if item.get("rejected"):
        card["rejected"] = [{"comment_id": r.get("comment_id"), "reason": r.get("reason")} for r in item["rejected"]]
    if item.get("rejected_unattributed") and item["kind"] == "needs_review":
        card["rejected_note"] = [{"comment_id": r.get("comment_id"), "reason": r.get("reason")}
                                 for r in item["rejected_unattributed"]]
    card["then"] = upcoming
    return card


def audit_item(state, tick):
    """Periodic audit bound to an explicit subject (repo, PR, merge commit, head); None when unknown."""
    merged = state.get("last_merged")
    if not merged or not state.get("repo"):
        return None
    subject = (f"{state['repo']}#{merged['number']} (merge commit {str(merged.get('merge_commit', ''))[:12]}, "
               f"head {str(merged.get('head', ''))[:12]})")
    return {
        "kind": "audit",
        "id": "audit-newest-merged",
        "rank": 8,
        "task": f"Re-run review_coverage.py against {subject} and compare its structured verdict with the prose verdicts in its comments; report any disagreement",
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


def standby_card(agent, role, tick):
    return {
        "agent": agent["name"], "role": role, "tick": tick, "priority": 9,
        "task": "Standby: every open item is currently leased to another seat",
        "why_now": "no free item; duplicating work or inventing a task would be noise",
        "inputs": {"pr": None, "kind": "standby"}, "marker": "",
        "acceptance": "no artifact required; take the next free card when a lease completes or expires",
        "if_blocked": "not applicable; do not report 'pending' as the outcome",
    }


def build_cards(state, tick):
    agents = state["agents"]
    urgent = work_items(state)
    urgent = [i for i in urgent if i["kind"] != "stacked"]
    substantive = [i for i in urgent if i["kind"] != "stale"]
    only_owner_left = bool(substantive) and all(i["kind"] in ("ready_to_merge", "draft") for i in substantive)
    ladder = list(urgent) + [dict(i, kind="backlog", rank=7) for i in backlog_items(state)]
    if not ladder:
        audit = audit_item(state, tick)
        if tick % AUDIT_EVERY != 0 or audit is None:
            return [
                healthy_idle_card(agent, (agent.get("roles") or DEFAULT_ROLES)[(tick + i) % len(agent.get("roles") or DEFAULT_ROLES)], tick, state)
                for i, agent in enumerate(agents)
            ]
        ladder = [audit]
    cards = []
    taken = set()
    leases = state.get("leases", {})
    ttl = state.get("lease_ttl", LEASE_TTL)

    def key_of(item):
        return item_key(item["kind"], item["pr"], item.get("head", "")) if item.get("pr") else None

    def allowed(item, name):
        if item["kind"] in ("needs_review", "resolve_threads") and name in set(item.get("implementers") or []) | {item.get("seat")}:
            return False
        return not (item["kind"] in ("fix_threads", "repair_verdict", "address_review")
                    and item.get("owner_seat") not in (None, "", name))

    history = state.get("history", {})

    def burned(item, name):
        """This agent already let a lease on this exact item expire."""
        k = key_of(item)
        return bool(k) and history.get("expired", {}).get(f"{name}|{k}", 0) >= 1

    def someone_else_can(item, name):
        return any(a["name"] != name and allowed(item, a["name"]) and not burned(item, a["name"]) for a in agents)

    def score(agent):
        rec = history.get("agents", {}).get(agent["name"], {})
        return rec.get("done", 0) - rec.get("expired", 0)

    # Ignore leases whose holder is no longer allowed to hold the item (ownership or role changed).
    leases = {k: v for k, v in leases.items()
              if not any(key_of(i) == k and not allowed(i, v["agent"]) for i in ladder)}

    # Agents that actually finish work are served first (done minus expired leases); ties keep list order.
    order = sorted(range(len(agents)), key=lambda i: (-score(agents[i]), i))
    for index in order:
        agent = agents[index]
        roles = agent.get("roles") or DEFAULT_ROLES
        role = roles[(tick + index) % len(roles)]
        name = agent["name"]
        leased_to_others = {id(i) for i in ladder if key_of(i) in leases and leases[key_of(i)]["agent"] != name}
        eligible = [
            item for item in ladder
            # Rotation: an agent that let this item's lease expire does not get it again while another
            # eligible agent exists; it is not handed the same thing twice.
            if id(item) not in leased_to_others and allowed(item, name)
            and not (burned(item, name) and someone_else_can(item, name))
        ]
        # A lease is honoured only while the item is still eligible for its holder (ownership or role may
        # have changed since it was granted); otherwise the holder just draws a fresh card.
        held = [i for i in eligible if key_of(i) in leases and leases[key_of(i)]["agent"] == name]
        if held:
            # Continuity: an agent keeps the card it holds a lease on until it completes or the lease expires.
            item = held[0]
        else:
            # Highest-ranked free item first; the tick only breaks ties within that rank. When nothing is
            # free the agent stands by (no duplicate, no invented work).
            free = [i for i in eligible if id(i) not in taken]
            item = None
            if free:
                top = min(i["rank"] for i in free)
                tier = [i for i in free if i["rank"] == top]
                item = tier[(tick + index) % len(tier)]
            if item is None:
                card = standby_card(agent, role, tick)
                card["owner_blocked"] = only_owner_left
                cards.append((index, card))
                continue
        taken.add(id(item))
        card = card_for(agent, role, tick, item)
        card["owner_blocked"] = only_owner_left
        key = key_of(item)
        later = [i for i in ladder if id(i) not in taken and id(i) not in leased_to_others and allowed(i, name)]
        upcoming = (f"{later[0]['kind']} on PR #{later[0]['pr']}" if later and later[0].get("pr")
                    else "no further item is free for you; stand by")
        enrich_card(card, item, name, state, bool(key and key in leases and leases[key]["agent"] == name), upcoming)
        if key:
            lease = leases.get(key)
            card["lease"] = {"key": key, "expires_tick": lease["expires"] if lease else tick + ttl,
                             "renewed": bool(lease)}
        cards.append((index, card))
    return [card for _, card in sorted(cards, key=lambda pair: pair[0])]


def render(cards):
    out = []
    for c in cards:
        head = (f"[{c['agent']}] role={c['role']} tick={c['tick']} p{c['priority']}"
                + (f" lens={c['lens']} attempt={c['attempt']}" if c.get("lens") else "")
                + (f" lease->tick {c['lease']['expires_tick']}" if c.get("lease") else ""))
        lines = [head, f"  TASK: {c['task']}"]
        if c.get("state"):
            lines.append(f"  STATE NOW: {c['state']}")
        if c.get("new_since_last"):
            lines.append(f"  NEW SINCE YOUR LAST CARD: {c['new_since_last']}")
        if c.get("lens_text"):
            lines.append(f"  APPROACH: {c['lens_text']}")
        for r in c.get("rejected") or []:
            lines.append(f"  REJECTED: comment {r['comment_id']}: {r['reason']}")
        for r in c.get("rejected_note") or []:
            lines.append(f"  NOTE: comment {r['comment_id']} on this PR was not counted ({r['reason']}); if it is yours, repost it")
        for n, step in enumerate(c.get("recipe") or [], 1):
            lines.append(f"  STEP {n}: {step}")
        lines.append(f"  DONE WHEN: {c['acceptance']}")
        if c.get("then"):
            lines.append(f"  THEN: {c['then']}")
        lines.append(f"  IF BLOCKED: {c['if_blocked']}")
        out.append("\n".join(lines))
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
