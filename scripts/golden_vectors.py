#!/usr/bin/env python3
"""Golden vectors for the steward contract (lane C99).

The vectors are the contract a Rust re-implementation (the proposed Guild component) must reproduce byte for
byte: given the same input they must yield the same output. They live as plain JSON under
scripts/golden/steward/ so the contract does not depend on Python:

  <name>.in.json   {"tick": N, "state": {...}}   input to next_task.build_cards
  <name>.out.json  [card, ...]                    exact expected cards
  chain.in.json    [{"type", "data", "now"}, ...] events appended in order to an empty log
  chain.out.json   {"lines": [...], "tip": "<sha256 of last line>"}  exact log lines and tip

  Rule vectors, each a list of cases and the list of exact results, in the same order:
  attribution  collect_state.comment_seat    heads      collect_state.cites_head
  receipts     collect_state.done_receipts   rejected   collect_state.rejected_verdicts
  stalls       steward_run.stalled_seats and attribution_weak
  coverage     review_coverage.evaluate      audit      next_task.build_cards across ticks

`golden_vectors.py --write` regenerates the files from the scenarios below; `--check` fails if the committed
files are not what the current code produces. Regenerating is a deliberate act: a changed vector is a contract
change and needs review like any other.
"""

import argparse
import json
import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import collect_state as cs  # noqa: E402
import next_task as nt  # noqa: E402
import review_coverage as rc  # noqa: E402
import steward_run as sr  # noqa: E402

DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "golden", "steward")
NOW = "2026-10-05T00:00:00Z"
AGENTS = [{"name": "mimo"}, {"name": "gemini"}, {"name": "codex"}, {"name": "claude"}]


def pr(number, **kw):
    base = {
        "number": number, "title": f"pr {number}", "head": f"{number:02d}" * 20, "base": "main",
        "draft": False, "mergeable_state": "CLEAN", "updated_at": "2026-10-04T23:00:00Z",
        "seat": "claude", "implementers": ["claude"], "threads_unresolved": 0, "covered": False,
        "base_sha": "b" * 40, "size": "+10/-2 in 1 file(s)",
    }
    base.update(kw)
    return base


def state(prs, **kw):
    base = {"now": NOW, "agents": AGENTS, "prs": prs, "backlog": [], "done": [], "default_branch": "main"}
    base.update(kw)
    return base


def key(kind, number):
    return f"{kind}:{number}:{(f'{number:02d}' * 20)[:8]}"


SCENARIOS = {
    "review_excludes_every_implementer": (0, state([pr(1, implementers=["claude", "mimo"], seat="claude")])),
    "fix_threads_goes_only_to_the_author": (0, state([pr(2, threads_unresolved=2, seat="codex", implementers=["codex"])])),
    "uncovered_draft_gets_a_review_card": (0, state([pr(3, draft=True)])),
    "covered_draft_gets_the_flip_ready_card": (0, state([pr(4, draft=True, covered=True)])),
    "stacked_pr_gets_no_merge_brief": (0, state([pr(5, covered=True, base="proposal/other")])),
    "covered_clean_pr_gets_the_merge_brief": (0, state([pr(6, covered=True)])),
    "lens_advances_with_each_handout": (0, state([pr(7)], history={"item_attempts": {key("needs_review", 7): 2}})),
    "held_lease_keeps_holder_and_lens": (3, state(
        [pr(8)], leases={key("needs_review", 8): {"agent": "gemini", "expires": 9}},
        history={"item_attempts": {key("needs_review", 8): 1}, "last_head": {"gemini|8": "08080808"}})),
    "expired_holder_rotates_to_another_seat": (7, state(
        [pr(9)], history={"expired": {f"mimo|{key('needs_review', 9)}": 1}, "item_attempts": {key("needs_review", 9): 1}})),
    "head_moved_since_last_card": (0, state([pr(10)], history={"last_head": {"mimo|10": "aaaaaaaa", "gemini|10": "aaaaaaaa"}})),
    "finishers_are_served_first": (0, state(
        [pr(11)], agents=[{"name": "mimo"}, {"name": "gemini"}],
        history={"agents": {"gemini": {"done": 4, "expired": 0}, "mimo": {"done": 0, "expired": 3}}})),
    "rejected_verdict_goes_back_to_its_author": (0, state([pr(14, rejected_verdicts=[{"comment_id": 77, "seat": "gemini", "reason": "the attestation block is not valid JSON; it must be one JSON object, not YAML or prose"}])])),
    "everything_leased_means_standby": (2, state(
        [pr(12)], leases={key("needs_review", 12): {"agent": "mimo", "expires": 9}})),
    "healthy_idle": (1, state([pr(13, covered=True, base="proposal/other")])),
}

CHAIN_EVENTS = [
    {"type": "ingest", "data": {"tick": 1, "per_seat": {"mimo": 2}, "unattributed": 0}, "now": "2026-10-05T00:00:00Z"},
    {"type": "lease_grant", "data": {"tick": 1, "key": "needs_review:1:01010101", "agent": "gemini", "expires": 7},
     "now": "2026-10-05T00:00:01Z"},
    {"type": "cards", "data": {"tick": 1, "agents": ["gemini"], "tasks": ["Post a review — déjà vu"]},
     "now": "2026-10-05T00:00:02Z"},
]


H = "c" * 40
OTHER = "c" * 8 + "d" * 32
MB = "b" * 40


def receipt(kind, number, head, rest):
    return f"[done:{kind}:{number}:{head}] {rest}"


ATTRIBUTION = [
    {"body": "Seat / agent: Mimo (mimo-v2.6-flash-free, opencode)", "strict": False},
    {"body": "### Review\n**Seat / Agent**: Antigravity (Gemini 2.5 Pro)\n", "strict": False},
    {"body": "My note.\nMimo reviewed this earlier.\n", "strict": True},
    {"body": "Mimo reviewed this earlier.\n", "strict": False},
    {"body": "Gemini whole-diff packet\n", "strict": True},
    {"body": "intro\nSeat / agent: Codex\n", "strict": True},
    {"body": "<!-- codex-pull-request-review-summary -->\n\n## Codex Review Summary\n\nCodex reacts with an eyes emoji.\n", "strict": True},
    {"body": "### Independent Review Packet: o/r#63\n\n**Reviewer / Seat**: Antigravity (`gemini-3.8-flash`)\n", "strict": True},
    {"body": "no seat is named here", "strict": True},
    {"body": "Claude seat: done", "strict": True},
]

HEADS = [
    {"body": f"head {H}", "head": H},
    {"body": f"head {OTHER}", "head": H},
    {"body": "head cccccccc", "head": H},
    {"body": "head ccccccc", "head": H},
    {"body": "head " + "C" * 40, "head": H},
    {"body": "no hash here", "head": H},
    {"body": f"head {H}", "head": ""},
]


def snapshot_for(number, comments, head=H):
    return {"prs": [{"number": number, "head": head, "comments": [{"id": i + 1, "body": b} for i, b in enumerate(comments)]}]}


RECEIPTS = [
    {"name": "author seat completes thread fixes",
     "snapshot": snapshot_for(7, [receipt("fix_threads", 7, H, "Claude seat: all threads answered")]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "a negative first line is not a completion",
     "snapshot": snapshot_for(7, [receipt("fix_threads", 7, H, "Claude seat: blocked by the owner")]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "NOT DONE is negative in any case",
     "snapshot": snapshot_for(7, [receipt("fix_threads", 7, H, "Claude seat: NOT DONE")]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "a quoted receipt is not a posted one",
     "snapshot": snapshot_for(7, ["> " + receipt("fix_threads", 7, H, "Claude seat: ok")]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "a receipt for another head does not complete the card",
     "snapshot": snapshot_for(7, [receipt("fix_threads", 7, "d" * 40, "Claude seat: ok")]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "a different head sharing the first eight characters conflicts",
     "snapshot": snapshot_for(7, [receipt("fix_threads", 7, H, "Claude seat: ok " + OTHER)]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "a receipt naming another PR does not count",
     "snapshot": snapshot_for(7, [receipt("fix_threads", 8, H, "Claude seat: ok")]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "only the author completes thread fixes",
     "snapshot": snapshot_for(7, [receipt("fix_threads", 7, H, "Codex seat: ok")]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "an unattributed receipt never completes",
     "snapshot": snapshot_for(7, [receipt("fix_threads", 7, H, "all threads answered")]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "a reviewer who is not an implementer completes a review",
     "snapshot": snapshot_for(7, [receipt("needs_review", 7, H, "Gemini seat: reviewed")]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "an implementer cannot complete a review",
     "snapshot": snapshot_for(7, [receipt("needs_review", 7, H, "Claude seat: reviewed")]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "unknown implementers fail closed for reviews",
     "snapshot": snapshot_for(7, [receipt("needs_review", 7, H, "Gemini seat: reviewed")]),
     "repair_owners": {}, "implementers_by_pr": {"7": []}},
    {"name": "only the seat whose verdict was rejected completes the repair",
     "snapshot": snapshot_for(7, [receipt("repair_verdict", 7, H, "Gemini seat: reposted"),
                                  receipt("repair_verdict", 7, H, "Mimo seat: reposted")]),
     "repair_owners": {"7": ["gemini"]}, "implementers_by_pr": {"7": ["claude"]}},
    {"name": "the first eligible receipt names the seat",
     "snapshot": snapshot_for(7, [receipt("needs_review", 7, H, "Gemini seat: first"),
                                  receipt("needs_review", 7, H, "Codex seat: second")]),
     "repair_owners": {}, "implementers_by_pr": {"7": ["claude"]}},
]


def row_for(*legacy_indexes, malformed=()):
    return {"legacy": [{"comment": i} for i in legacy_indexes],
            "malformed": [{"comment": i, "problem": p} for i, p in malformed]}


def rejected_pr(bodies, head=H):
    return {"head": head, "comments": [{"id": 100 + i, "body": b} for i, b in enumerate(bodies)]}


REJECTED = [
    {"name": "a legacy comment citing the current head goes back to its seat",
     "pr": rejected_pr(["Seat / agent: Gemini\nhead " + H + "\nVERDICT: accept"]), "row": row_for(0)},
    {"name": "a comment that does not cite the current head is history",
     "pr": rejected_pr(["Seat / agent: Gemini\nhead " + "e" * 40 + "\nVERDICT: accept"]), "row": row_for(0)},
    {"name": "a head sharing eight characters is not this head",
     "pr": rejected_pr(["Seat / agent: Mimo\nhead " + OTHER + "\nVERDICT: accept"]), "row": row_for(0)},
    {"name": "at most the latest two per seat are kept",
     "pr": rejected_pr([f"Seat / agent: Mimo\nhead {H}\nVERDICT: accept {n}" for n in range(5)]), "row": row_for(0, 1, 2, 3, 4)},
    {"name": "a malformed block carries a plain-words reason",
     "pr": rejected_pr(["Seat / agent: Gemini\nhead " + H + "\n```attestation\n{}\n```"]),
     "row": row_for(malformed=[(0, "missing fields: ['head', 'reviewer']")])},
    {"name": "a malformed block with invalid JSON",
     "pr": rejected_pr(["Seat / agent: Mimo\nhead " + H]), "row": row_for(malformed=[(0, "invalid JSON: Expecting value")])},
    {"name": "an unattributed rejected comment is kept with no seat",
     "pr": rejected_pr(["head " + H + "\nVERDICT: accept"]), "row": row_for(0)},
]


def stall_events(spec):
    """spec: one entry per tick, {"issued": [...], "produced": {seat: n}, "unattributed": n}."""
    events = []
    for tick, entry in enumerate(spec, 1):
        events.append({"type": "ingest", "data": {"tick": tick, "per_seat": entry.get("produced", {}),
                                                 "unattributed": entry.get("unattributed", 0)}})
        events.append({"type": "cards", "data": {"tick": tick, "agents": entry.get("issued", [])}})
    return events


EVERYONE = ["claude", "mimo", "codex", "gemini"]
STALLS = [
    {"name": "three silent ticks stall every seat that was issued cards",
     "events": stall_events([{"issued": EVERYONE}] * 3), "agents": EVERYONE, "stall_after": 3, "window": 3},
    {"name": "one artifact in the last tick clears that seat",
     "events": stall_events([{"issued": EVERYONE}, {"issued": EVERYONE}, {"issued": EVERYONE, "produced": {"mimo": 1}}]),
     "agents": EVERYONE, "stall_after": 3, "window": 3},
    {"name": "fewer ticks than the window judges nobody",
     "events": stall_events([{"issued": EVERYONE}] * 2), "agents": EVERYONE, "stall_after": 3, "window": 3},
    {"name": "a seat that was not issued a card in every tick is not stalled",
     "events": stall_events([{"issued": ["mimo"]}, {"issued": EVERYONE}, {"issued": EVERYONE}]),
     "agents": EVERYONE, "stall_after": 3, "window": 3},
    {"name": "unattributed comments make attribution weak",
     "events": stall_events([{"issued": EVERYONE, "unattributed": 3}] * 3), "agents": EVERYONE, "stall_after": 3, "window": 3},
    {"name": "exactly half attributed is not weak",
     "events": stall_events([{"issued": EVERYONE, "unattributed": 3, "produced": {"mimo": 3}}]),
     "agents": EVERYONE, "stall_after": 1, "window": 1},
    {"name": "fewer than three comments is never weak",
     "events": stall_events([{"issued": EVERYONE, "unattributed": 2}]), "agents": EVERYONE, "stall_after": 1, "window": 1},
]


def rec(head, scope="whole", reviewer="mimo", implementers=("claude",), verdict="accept", base=MB, frm=None):
    record = {"repo": "o/r", "pr": 1, "head": head, "base_sha": base, "scope": scope, "reviewer": reviewer,
              "implementers": list(implementers), "verdict": verdict}
    if frm:
        record["from"] = frm
    return record


A, B, C = "a" * 40, "b" * 39 + "1", "c" * 40
COVERAGE = [
    {"name": "a whole accept at the head covers", "head": A, "merge_base": MB, "records": [rec(A)], "ancestry": []},
    {"name": "a delta chained from a whole accept covers", "head": B, "merge_base": MB,
     "records": [rec(A), rec(B, "delta", frm=A)], "ancestry": [[A, B]]},
    {"name": "a delta with no whole origin does not cover", "head": B, "merge_base": MB,
     "records": [rec(B, "delta", frm=A)], "ancestry": [[A, B]]},
    {"name": "a delta whose from is not an ancestor does not cover", "head": B, "merge_base": MB,
     "records": [rec(A), rec(B, "delta", frm=A)], "ancestry": []},
    {"name": "two chained deltas cover", "head": C, "merge_base": MB,
     "records": [rec(A), rec(B, "delta", frm=A), rec(C, "delta", frm=B)], "ancestry": [[A, B], [B, C]]},
    {"name": "an accept at an older head does not carry", "head": B, "merge_base": MB, "records": [rec(A)], "ancestry": []},
    {"name": "an implementer cannot accept its own work", "head": A, "merge_base": MB,
     "records": [rec(A, reviewer="claude")], "ancestry": []},
    {"name": "a standing needs_revision at the head blocks", "head": A, "merge_base": MB,
     "records": [rec(A), rec(A, reviewer="gemini", verdict="needs_revision")], "ancestry": []},
    {"name": "a record bound to another merge base is invalid", "head": A, "merge_base": MB,
     "records": [rec(A, base="d" * 40)], "ancestry": []},
    {"name": "a moved merge base invalidates the record", "head": A, "merge_base": "e" * 40,
     "records": [rec(A)], "ancestry": []},
]


def evaluate_case(case):
    pairs = {(a, b) for a, b in case["ancestry"]}
    return rc.evaluate(case["head"], case["merge_base"], json.loads(json.dumps(case["records"])),
                       lambda a, b: a == b or (a, b) in pairs)


AUDIT_TICKS = list(range(0, 14))


def audit_scenario(tick):
    """Healthy state with a known newest merge, so the periodic audit has a subject to be issued for."""
    return state([pr(13, covered=True, base="proposal/other")], repo="o/r",
                 last_merged={"number": 54, "merge_commit": "m" * 40, "head": "h" * 40}), tick


def build_rule_vectors():
    out = {}
    out["attribution.in.json"] = ATTRIBUTION
    out["attribution.out.json"] = [cs.comment_seat(c["body"], strict=c["strict"]) for c in ATTRIBUTION]
    out["heads.in.json"] = HEADS
    out["heads.out.json"] = [cs.cites_head(c["body"], c["head"]) for c in HEADS]
    out["receipts.in.json"] = RECEIPTS
    out["receipts.out.json"] = [
        cs.done_receipts(c["snapshot"], {int(k): v for k, v in c["repair_owners"].items()},
                         {int(k): v for k, v in c["implementers_by_pr"].items()}) for c in RECEIPTS]
    out["rejected.in.json"] = REJECTED
    out["rejected.out.json"] = [cs.rejected_verdicts(c["pr"], c["row"]) for c in REJECTED]
    out["stalls.in.json"] = STALLS
    out["stalls.out.json"] = [
        {"stalled": sr.stalled_seats(c["events"], c["agents"], c["stall_after"]),
         "attribution_weak": sr.attribution_weak(
             [e for e in c["events"] if e["type"] == "ingest"], c["window"])} for c in STALLS]
    out["coverage.in.json"] = COVERAGE
    out["coverage.out.json"] = [evaluate_case(c) for c in COVERAGE]
    out["audit.in.json"] = {"ticks": AUDIT_TICKS, "state": audit_scenario(0)[0]}
    out["audit.out.json"] = [
        {"tick": t, "tasks": [c["task"] for c in nt.build_cards(json.loads(json.dumps(audit_scenario(t)[0])), t)]}
        for t in AUDIT_TICKS]
    return out


def build_outputs():
    out = {}
    for name, (tick, st) in SCENARIOS.items():
        out[f"{name}.in.json"] = {"tick": tick, "state": st}
        out[f"{name}.out.json"] = nt.build_cards(json.loads(json.dumps(st)), tick)
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "events.jsonl")
        for event in CHAIN_EVENTS:
            sr.append_event(path, event["type"], event["data"], event["now"])
        with open(path, encoding="utf-8") as handle:
            lines = [line.rstrip("\n") for line in handle if line.strip()]
        out["chain.in.json"] = CHAIN_EVENTS
        out["chain.out.json"] = {"lines": lines, "tip": sr.digest(lines[-1])}
    out.update(build_rule_vectors())
    return out


def dump(obj):
    return json.dumps(obj, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--write", action="store_true")
    group.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    outputs = build_outputs()
    if args.write:
        os.makedirs(DIR, exist_ok=True)
        for name, obj in outputs.items():
            with open(os.path.join(DIR, name), "w", encoding="utf-8", newline="\n") as handle:
                handle.write(dump(obj))
        print(f"wrote {len(outputs)} files to {DIR}")
        return 0
    stale = []
    for name, obj in outputs.items():
        path = os.path.join(DIR, name)
        if not os.path.exists(path) or open(path, encoding="utf-8").read() != dump(obj):
            stale.append(name)
    if stale:
        print("committed vectors differ from current code: " + ", ".join(stale))
        return 1
    print(f"{len(outputs)} vectors match")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
