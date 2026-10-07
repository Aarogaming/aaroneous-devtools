"""Generate adversarial text cases and the Python oracle's answers for the Rust differential test (lane C99).

  python scripts/differential_cases.py COUNT SEED OUT.json
  STEWARD_DIFF_FILE=OUT.json cargo test -p steward --test differential -- --ignored --nocapture   (run in foundation/)

Not part of the gate: the committed golden vectors are the contract. This is how divergences between the two
languages are found (the first run, 110,000 bodies, found six: see the vectors tagged as found by it). Each new
divergence is then pinned as a vector.
"""

import json, random, sys
sys.path.insert(0, 'scripts')
import collect_state as cs, review_coverage as rc, steward_run as sr, merge_gate as mg
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
DIRS = ["", "governance/", "docs/", "architecture/", "scripts/", "inventories/", "contracts/", ".github/workflows/", "foundation/crates/x/src/", "notes/", "a/b/", "governance//", "docs/."]
NAMES2 = ["a.md", "b.json", "c.py", "d.toml", "e.txt", "f.sh", "g.ps1", "h.rs", "AGENTS.md", "AGENTS.md.bak", "SECURITY.md", "CONTRIBUTING.md", "AUTOMATED_MERGE_POLICY.md", "identities.json", ".gitignore", "..hidden", "x.tar.gz", "noext", "trail.", "A.MD", "caf\u00e9.md", "ci.yml", "a b.md"]
def random_path():
    path = random.choice(DIRS) + random.choice(NAMES2)
    if random.random() < 0.1: path = random.choice(["AGENTS.md", "governance/AUTOMATED_MERGE_POLICY.md", "governance/identities.json"])
    return path
gate_tier = []
for _ in range(int(sys.argv[1]) // 5):
    paths = [random_path() for _ in range(random.randint(0, 4))]
    gate_tier.append({"paths": paths, "out": {"tier": list(mg.tier_of(paths)), "protected": mg.touches_protected(paths),
                                              "plan_level": mg.gate_plan(paths)["level"], "category": mg.category_of(paths)}})
gate_ci = []
for _ in range(int(sys.argv[1]) // 5):
    rollup = []
    for _ in range(random.randint(0, 4)):
        item = {}
        if random.random() < 0.85: item["conclusion"] = random.choice(["SUCCESS", "success", "FAILURE", "SKIPPED", "", None, "NEUTRAL", "stale"])
        if random.random() < 0.85: item["status"] = random.choice(["COMPLETED", "completed", "IN_PROGRESS", "QUEUED", "", None, "WAITING"])
        rollup.append(item)
    gate_ci.append({"rollup": rollup, "out": mg.ci_state(rollup)})
TOKENS = ["tripped", " ", "=", "true", "false", "True", "1", '"', "'", "#", "\n", "[", "]", "[a]", ".", "a", "tripped = true\n", "tripped = false\n", "tripped=false", "x = 1\n", "\t", "{", "}", ",", "0x1", "1e3", "inf", "nan", "\"false\"", "tripped.sub = true\n", "[t]\ntripped = true\n", "\r\n", "\u00e9", "'" "'" "'", '\"\"\"']
gate_breaker = []
LINES = ["tripped = true", "tripped = false", "tripped = false # note", "# tripped = true", "x = 1", "[sec]", "[sec]\ntripped = true", "tripped = 1", "tripped = 'true'", "tripped = \"false\"", "TRIPPED = true", "tripped=true", "a.tripped = true", "tripped = [true]", "tripped = {x = 1}", "\"tripped\" = false", "'tripped' = true", "tripped = true\ntripped = false", "", "  tripped  =  false  ", "tripped = tru", "tripped = true,", "[[arr]]", "tripped = 0x1", "tripped = +true"]
for _ in range(int(sys.argv[1]) // 5):
    if random.random() < 0.7:
        text = "\n".join(random.choice(LINES) for _ in range(random.randint(1, 4))) + random.choice(["\n", "", "\r\n"])
        if random.random() < 0.3 and text:
            i = random.randrange(len(text))
            text = text[:i] + random.choice(TOKENS) + text[i + 1:]
    else:
        text = "".join(random.choice(TOKENS) for _ in range(random.randint(0, 8)))
    gate_breaker.append({"text": text, "out": mg.breaker_state(text)})
def junk(depth=0):
    kind = random.random()
    if depth > 2 or kind < 0.35: return random.choice([True, False, None, 0, 1, "", "x", 2.5])
    if kind < 0.6: return [junk(depth + 1) for _ in range(random.randint(0, 3))]
    return {random.choice(["data", "repository", "pullRequest", "reviewThreads", "nodes", "pageInfo", "hasNextPage", "isResolved"]): junk(depth + 1) for _ in range(random.randint(0, 3))}
def threads_response():
    nodes = [random.choice([{"isResolved": True}, {"isResolved": False}, {"isResolved": 0}, {"isResolved": None}, {}, "x", 3, {"isResolved": "yes"}]) for _ in range(random.randint(0, 4))]
    page = random.choice([{"hasNextPage": False}, {"hasNextPage": True}, {"hasNextPage": None}, {"hasNextPage": 0}, {}, None])
    node_value = random.choice([nodes, nodes, nodes, {}, "", "ab", None, {"a": 1}])
    data = {"data": {"repository": {"pullRequest": {"reviewThreads": {"nodes": node_value, "pageInfo": page}}}}}
    if random.random() < 0.15: return junk()
    if random.random() < 0.1: del data["data"]["repository"]["pullRequest"]["reviewThreads"]["pageInfo"]
    return data
gate_threads = []
for _ in range(int(sys.argv[1]) // 5):
    data = threads_response()
    gate_threads.append({"data": data, "out": mg.open_threads(data)})
GREEN = json.load(open("scripts/golden/steward/gate_checklist.in.json", encoding="utf-8"))["green"]
ODD = {
    "ci": [None, {}, {"state": "missing", "detail": "no CI status found"}, {"state": "failure", "detail": "1 failed"}, {"state": "pending", "detail": ""}, {"state": "success"}, {"state": "success", "detail": 7}],
    "trial_merge_ok": [True, False, None, 1, "yes", 0],
    "base_ref": ["main", "proposal/x", "", 5],
    "default_branch": ["main", "master"],
    "base_tip": ["t" * 40, "", None, 12, "short"],
    "head": ["h" * 40, "abc", ""],
    "accept_seats": [["a", "b"], ["a", "a"], ["", ""], [], "ab", None, ["a", None], ["claude", "codex", "x"], [" a ", "a"]],
    "covered": [True, False, None, 1, "yes"],
    "threads_open": [0, 1, 3, None, False, True, 0.0, "0", -1],
    "standing_changes_requested": [True, False, None, 0, "no"],
    "counted_verdicts": [0, 1, 2, None, True, 2.5, "2", -1],
    "verdicts_missing_evidence": [[], [3], ["a", None], None, 0, "", {"a": 1}, [[1, 2]], ["it's"]],
    "revertible": [True, False, None, "yes", 1],
    "breaker_tripped": [True, False, None, 0, ""],
    "trust": [None, [5, 5], [4, 5], [0, 0], [5, 0], [-1, -2], [10, 3]],
    "category": ["scripts/*.py", "mixed", None, "caf\u00e9"],
    "tier": [["B", ""], ["A", "touches owner-only paths: ['x']"], ["unclassified", "why"], ["B", 1], "B", None, ["B"], ["b", ""]],
}
checklist_cases = []
for _ in range(int(sys.argv[1]) // 5):
    facts = json.loads(json.dumps(GREEN))
    for _ in range(random.randint(0, 3)):
        key = random.choice(list(ODD) + list(GREEN))
        if random.random() < 0.25:
            facts.pop(key, None)
        elif key in ODD:
            facts[key] = random.choice(ODD[key])
    try:
        lines, would, first = mg.checklist(json.loads(json.dumps(facts)))
        out = {"lines": lines, "would_merge": would, "first_blocker": first}
    except Exception:
        out = {"error": True}
    checklist_cases.append({"facts": facts, "out": out})
decide_cases = []
for _ in range(int(sys.argv[1]) // 5):
    paths = [random_path() for _ in range(random.randint(0, 3))]
    row = {"covered": random.choice([True, False, None, 0, 1, "", "x", []])}
    base = random.choice(["main", "proposal/x", "", "Main"])
    trial = random.choice([True, False, None, 0, 1])
    owner = random.choice([True, False, 0, 1, None])
    result = mg.decide(row, paths, base, trial, owner)
    _, eligible, first = mg.reconcile(result["eligible"], json.loads(json.dumps(GREEN)))
    decide_cases.append({"row": row, "paths": paths, "base": base, "trial": trial, "owner": owner,
                         "out": {"decide": result, "reconciled": {"eligible": eligible, "first_blocker": first}}})
json.dump({"text": cases, "stalls": stalls, "pushes": push_cases, "coverage": coverage, "gate_checklist": checklist_cases, "gate_decide": decide_cases, "gate_tier": gate_tier, "gate_ci": gate_ci, "gate_breaker": gate_breaker, "gate_threads": gate_threads}, open(sys.argv[3], "w", encoding="utf-8"), ensure_ascii=False)
print(len(cases), len(stalls), len(push_cases))
