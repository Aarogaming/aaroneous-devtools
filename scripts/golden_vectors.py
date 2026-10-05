#!/usr/bin/env python3
"""Golden vectors for the steward contract (lane C99).

The vectors are the contract a Rust re-implementation (the proposed Guild component) must reproduce byte for
byte: given the same input they must yield the same output. They live as plain JSON under
scripts/golden/steward/ so the contract does not depend on Python:

  <name>.in.json   {"tick": N, "state": {...}}   input to next_task.build_cards
  <name>.out.json  [card, ...]                    exact expected cards
  chain.in.json    [{"type", "data", "now"}, ...] events appended in order to an empty log
  chain.out.json   {"lines": [...], "tip": "<sha256 of last line>"}  exact log lines and tip

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
import next_task as nt  # noqa: E402
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
