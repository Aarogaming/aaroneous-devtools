"""Generate adversarial text cases and the Python oracle's answers for the Rust differential test (lane C99).

  python scripts/differential_cases.py COUNT SEED OUT.json
  STEWARD_DIFF_FILE=OUT.json cargo test -p steward --test differential -- --ignored --nocapture   (run in foundation/)

Not part of the gate: the committed golden vectors are the contract. This is how divergences between the two
languages are found (the first run, 110,000 bodies, found six: see the vectors tagged as found by it). Each new
divergence is then pinned as a vector.
"""

import json, random, sys
sys.path.insert(0, 'scripts')
import collect_state as cs, review_coverage as rc, steward_run as sr
random.seed(int(sys.argv[2]))
H = "c" * 40
pieces = ["Seat / agent:", "Seat/agent", "Reviewer", "Reviewer / Seat", "Reviewer seat", "Claude", "claude", "CODEX", "Gemini", "Mimo", "Antigravity", "Qwen",
  "xGemini", "Geminis", "Independent", "#", "###", "#######", " ", "  ", "\t", "\n", "\r\n", "\r", " ", " ", "\x0b", "\x0c", "\x1c", "\x1f", "\x85", " ", "　", "​",
  "-", "_", "(", ")", ":", ".", "seat", "SEAT", "é", "́", "ͅ", "Ⓐ", "‿", "‍", "ั", "K", "ſ", "İ", "٧", "7", "[done:", "fix_threads", "needs_review", "repair_verdict", ":7:", ":8:", ":٧:",
  H, H.upper(), "c" * 8, "c" * 7, "c" * 41, "c" * 8 + "d" * 32, "deadbeef", "verdict", "not done", "blocked", "NOT DONE", "failed", "]", ">", "Claude seat", "Codex seat: ok", "Gemini seat: ok"]
def text(): return "".join(random.choice(pieces) for _ in range(random.randint(1, 9)))
cases = []
for _ in range(int(sys.argv[1])):
    body = text()
    if random.random() < 0.5: body = f"[done:{random.choice(['fix_threads','needs_review','repair_verdict','resolve_threads'])}:{random.choice(['7','8'])}:{random.choice([H,'d'*40,H.upper()])}] " + body
    implementers = random.choice([["claude"], ["claude", "codex"], [], ["gemini"]])
    owners = random.choice([[], ["gemini"], ["mimo"]])
    snap = {"prs": [{"number": 7, "head": H, "comments": [{"id": 1, "body": body}]}]}
    out = {
        "seats": sorted(rc.seat_names(body)),
        "heads": cs.cites_head(body, H),
        "attr_strict": cs.comment_seat(body, strict=True),
        "attr_loose": cs.comment_seat(body, strict=False),
        "receipts": cs.done_receipts(snap, {7: owners}, {7: implementers}),
    }
    cases.append({"body": body, "implementers": implementers, "owners": owners, "out": out})
AGENTS = ["claude", "mimo", "codex", "gemini"]
def random_events():
    events = []
    for tick in range(random.randint(0, 6)):
        if random.random() < 0.15:
            tick = random.choice([tick, tick + 1])  # duplicate or skipped tick numbers
        if random.random() < 0.8:
            events.append({"type": "ingest", "data": {"tick": tick,
                "per_seat": {a: random.choice([0, 0, 1, 3]) for a in AGENTS if random.random() < 0.4},
                "pushes": {a: random.choice([0, 1, 2]) for a in AGENTS if random.random() < 0.3},
                "unattributed": random.choice([0, 0, 1, 2, 5])}})
        if random.random() < 0.9:
            events.append({"type": "cards", "data": {"tick": tick, "agents": [a for a in AGENTS if random.random() < 0.8]}})
        if random.random() < 0.1:
            events.append({"type": "lease_grant", "data": {"tick": tick}})
        if random.random() < 0.05:
            events.append({"type": "ingest", "data": {"per_seat": {"claude": 1}}})  # no tick
    return events
stalls, push_cases = [], []
for _ in range(int(sys.argv[1]) // 5):
    events = random_events()
    after, window = random.choice([-2, -1, 0, 1, 2, 3, 4, 9]), random.choice([-2, -1, 0, 1, 2, 3, 9])
    stalls.append({"events": events, "agents": AGENTS, "stall_after": after, "window": window,
                   "out": {"stalled": sr.stalled_seats(events, AGENTS, after),
                           "weak": sr.attribution_weak([e for e in events if e["type"] == "ingest"], window)}})
    prs = [{"number": n, "head": random.choice(["a" * 40, "b" * 40, "c" * 40, ""]),
            "seat": random.choice(["claude", "Claude", "codex", "", "GEMINI"])} for n in random.sample(range(1, 9), random.randint(0, 5))]
    previous = random.choice([None, {}, {str(n): random.choice(["a" * 40, "b" * 40]) for n in range(1, 9) if random.random() < 0.6}])
    heads, credited = sr.pushes(prs, previous)
    push_cases.append({"prs": prs, "previous": previous, "out": {"heads": heads, "credited": credited}})
HEADS = ["a" * 40, "b" * 40, "c" * 40, "d" * 40]
NAMES = ["claude", "Claude", "claude-sonnet-5-5", "codex", "Gemini", "Antigravity", "mimo", "Qwen", "Claude, Mimo", "someone else", "gemini/antigravity", "Opus 5.5", "sonnet", "gpt-5.5", "Opus/gpt-5.5", "OpenAI Codex"]
coverage = []
for _ in range(int(sys.argv[1]) // 5):
    records = []
    for _ in range(random.randint(0, 5)):
        scope = random.choice(["whole", "whole", "delta"])
        record = {"repo": "o/r", "pr": 1, "head": random.choice(HEADS), "base_sha": random.choice(["m" * 40, "m" * 40, "n" * 40]),
                  "scope": scope, "reviewer": random.choice(NAMES),
                  "implementers": random.choice([["claude"], ["claude", "codex"], ["Gemini"], ["Claude, Mimo"], ["claude-sonnet-5-5"]]),
                  "verdict": random.choice(["accept", "accept", "accept", "needs_revision", "changes_requested"])}
        if scope == "delta":
            record["from"] = random.choice(HEADS)
        records.append(record)
    pairs = [[a, b] for a in HEADS for b in HEADS if a != b and random.random() < 0.4]
    merge_base = random.choice(["m" * 40, "m" * 40, "n" * 40, "", "unknown", None])
    head = random.choice(HEADS)
    known = {(a, b) for a, b in pairs}
    out = rc.evaluate(head, merge_base, records, lambda a, b: a == b or (a, b) in known)
    coverage.append({"head": head, "merge_base": merge_base, "records": records, "ancestry": pairs, "out": out})
json.dump({"text": cases, "stalls": stalls, "pushes": push_cases, "coverage": coverage}, open(sys.argv[3], "w", encoding="utf-8"), ensure_ascii=False)
print(len(cases), len(stalls), len(push_cases))
