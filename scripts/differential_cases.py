"""Generate adversarial text cases and the Python oracle's answers for the Rust differential test (lane C99).

  python scripts/differential_cases.py COUNT SEED OUT.json
  STEWARD_DIFF_FILE=OUT.json cargo test -p steward --test differential -- --ignored --nocapture   (run in foundation/)

Not part of the gate: the committed golden vectors are the contract. This is how divergences between the two
languages are found (the first run, 110,000 bodies, found six: see the vectors tagged as found by it). Each new
divergence is then pinned as a vector.
"""

import json, random, sys
sys.path.insert(0, 'scripts')
import collect_state as cs, review_coverage as rc, steward_run as sr, merge_gate as mg, next_task as nt
random.seed(int(sys.argv[2]))
H = "c" * 40
pieces = ["Seat / agent:", "Seat/agent", "Reviewer", "Reviewer / Seat", "Reviewer seat", "Claude", "claude", "CODEX", "Gemini", "Mimo", "OpenCode", "Antigravity", "Qwen",
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
NAMES = ["claude", "Claude", "claude-sonnet-5-5", "codex", "Gemini", "Antigravity", "mimo", "Qwen", "Claude, Mimo", "ubuntu-worker-01", "Ubuntu Worker 01", "OpenCode", "ubuntu-worker-01 and codex", "codex, ubuntu-worker-01", "someone else", "gemini/antigravity", "Opus 5.5", "sonnet", "gpt-5.5", "Opus/gpt-5.5", "OpenAI Codex"]
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
    registered = random.choice([None, [], ["claude", "codex", "gemini", "ubuntu-worker-01"], ["claude", "mimo", "codex", "ubuntu-worker-01", "ubuntu-worker-02"], ["claude", "opencode", "codex"], ["mimo", "opencode"]])
    out = rc.evaluate(head, merge_base, records, lambda a, b: a == b or (a, b) in known, set(registered) if registered is not None else None)
    coverage.append({"head": head, "merge_base": merge_base, "records": records, "ancestry": pairs, "registered": registered, "out": out})
SEAT_LABELS = ["claude", "Claude", "opencode", "mimo", "ubuntu-worker-01", "Ubuntu Worker 01", "ubuntu_worker_01", "xubuntu-worker-01", "ubuntu-worker-011", "ubuntu-worker-01 and ubuntu-worker-02", "codex", "gpt-5.5", "Opus/gpt-5.5", "Gemini", "", " ", "Qwen", "café", "İ-worker-01", "ubuntu​-worker-01"]
MODELS = [None, "qwen3.8", "Qwen/Qwen3.8-Coder", "qwenish", "MiMo-V2.6", "mimo", "claude-sonnet-5-5", "gpt-5.5", "gemini-3.8-flash", "brandnew-9", "", "  ", " qwen3.8 ", 7, ["qwen"], "x" * 81, "x" * 80, "a\nb", "café", "org/sub/qwen2", "QWEN-7", "qwen.", "qwen-", "qwen\x00", "Kimi", "mimo ", "mimo\x1f", "\x1cmimo", " qwen3 "]
REGISTRIES = [
    {"seats": {"opencode": {"profile": {"family": "mimo"}}, "ubuntu-worker-01": {"profile": {"family": "qwen"}}, "claude": {"profile": {"family": "anthropic"}}, "codex": {}, "ubuntu-worker-02": {}}, "model_families": {"qwen": "qwen", "mimo": "mimo", "claude": "anthropic", "gpt": "openai", "gemini": "google"}},
    {"seats": {"claude": {}, "ubuntu-worker-01": {"profile": {"family": "Bad"}}, "a-b": {}}, "model_families": {"q": "qwen", "qwen": "", "Qwen": "x", "mi mo": "m"}},
    {"seats": {"mimo": {"profile": {"family": "mimo"}}, "ubuntu-worker-01": {}, "claude": {}}, "model_families": {"qwen": "qwen", "mimo": "mimo"}},
    {"seats": {"opencode": {"profile": {"family": "mimo"}}, "ubuntu-worker-01": {}, "claude": {}}, "model_families": {"qwen": "qwen", "mimo": "mimo"}},
    {"seats": {}}, {"seats": []}, {"model_families": {"qwen": "qwen"}}, [], None,
    {"seats": {"opencode": {"profile": {"family": " mimo "}}, "mimo": {"profile": "x"}}, "model_families": {"qwen": "qwen", "qwen3": "qwen3"}},
]
independence_cases = []
for _ in range(int(sys.argv[1]) // 5):
    keys = random.choice(REGISTRIES)
    counted = [{"reviewer": random.choice(SEAT_LABELS), **({} if random.random() < 0.2 else {"model": random.choice(MODELS)})} for _ in range(random.randint(0, 4))]
    registered = set(keys["seats"]) if isinstance(keys, dict) and isinstance(keys.get("seats"), dict) else None
    profiles = rc.seat_profiles(keys) if isinstance(keys, dict) else {}
    table = rc.model_table(keys) if isinstance(keys, dict) else []
    families, entries, notes = rc.independence(json.loads(json.dumps(counted)), registered, profiles, table)
    independence_cases.append({"keys": keys, "counted": counted, "out": {"families": families, "entries": entries, "notes": notes}})
DIRS = ["", "governance/", "docs/", "architecture/", "scripts/", "inventories/", "contracts/", ".github/workflows/", "foundation/crates/x/src/", "notes/", "a/b/", "governance//", "docs/."]
NAMES2 = ["a.md", "b.json", "c.py", "d.toml", "e.txt", "f.sh", "g.ps1", "h.rs", "AGENTS.md", "AGENTS.md.bak", "SECURITY.md", "CONTRIBUTING.md", "AUTOMATED_MERGE_POLICY.md", "identities.json", ".gitignore", "..hidden", "x.tar.gz", "noext", "trail.", "A.MD", "caf\u00e9.md", "ci.yml", "a b.md"]
def random_path():
    path = random.choice(DIRS) + random.choice(NAMES2)
    if random.random() < 0.1: path = random.choice(["AGENTS.md", "governance/AUTOMATED_MERGE_POLICY.md", "governance/identities.json"])
    return path
gate_plan = []
for _ in range(int(sys.argv[1]) // 5):
    paths = [random_path() for _ in range(random.randint(0, 4))]
    gate_plan.append({"paths": paths, "out": {"plan_level": mg.gate_plan(paths)["level"], "category": mg.category_of(paths)}})
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
    "accept_families": [["a", "b"], ["a", "a"], ["", ""], [], "ab", None, ["a", None], [" a ", "a"], ["google", "openai"]],
    "covered": [True, False, None, 1, "yes"],
    "threads_open": [0, 1, 3, None, False, True, 0.0, "0", -1],
    "standing_changes_requested": [True, False, None, 0, "no"],
    "counted_verdicts": [0, 1, 2, None, True, 2.5, "2", -1],
    "verdicts_missing_evidence": [[], [3], ["a", None], None, 0, "", {"a": 1}, [[1, 2]], ["it's"]],
    "revertible": [True, False, None, "yes", 1],
    "breaker_tripped": [True, False, None, 0, ""],
    "is_draft": [True, False, None, 0, 1, "yes", ""],
    "category": ["scripts/*.py", "mixed", None, "caf\u00e9"],
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
    result = mg.decide(row, paths, base, trial)
    _, eligible, first = mg.reconcile(result["eligible"], json.loads(json.dumps(GREEN)))
    decide_cases.append({"row": row, "paths": paths, "base": base, "trial": trial,
                         "out": {"decide": result, "reconciled": {"eligible": eligible, "first_blocker": first}}})
def json_value(depth=0):
    kind = random.random()
    if depth > 2 or kind < 0.4:
        return random.choice([True, False, None, 0, 1, -7, 12345678901234, "", "x", "caf\u00e9", "q\"uo\\te", "tab\there\nnew", "\u0001\u001f\u007f", "\u2028\u2029", "emoji \U0001F600", "\u0008\u000c"])
    if kind < 0.65:
        return [json_value(depth + 1) for _ in range(random.randint(0, 3))]
    return {random.choice(["b", "a", "A", "z", "\u00e9", "k\"ey", "", "10", "2"]): json_value(depth + 1) for _ in range(random.randint(0, 3))}
chain_cases = []
for _ in range(int(sys.argv[1]) // 5):
    event = {"seq": random.randint(1, 5000), "ts": "2026-10-07T00:00:00Z", "type": random.choice(["ingest", "cards", "x\u00e9"]),
             "data": json_value(), "prev": sr.digest(str(random.random()))}
    line = json.dumps(event, sort_keys=True, ensure_ascii=False)
    chain_cases.append({"event": event, "line": line, "digest": sr.digest(line)})
TIMES = ["2026-10-05T00:00:00Z", "2026-10-04T23:00:00Z", "2026-10-02T00:00:00Z", "2026-09-20T12:30:15Z", "2026-10-05T00:00:00.250Z",
         "2026-10-04T22:00:00-02:00", "2026-10-05T02:00:00+02:00", "2026-10-01T00:00:00Z", "garbage"]
SEATS = ["claude", "codex", "gemini", "mimo", "", None]
def random_pr(number):
    pr = {"number": number}
    if random.random() < 0.9: pr["head"] = random.choice(["c" * 40, "d" * 40, "abc", ""])
    if random.random() < 0.8: pr["seat"] = random.choice(SEATS)
    if random.random() < 0.5: pr["implementers"] = random.choice([[], ["claude"], ["claude", "codex"], ["gemini"]])
    if random.random() < 0.3: pr["draft"] = random.choice([True, False])
    if random.random() < 0.9: pr["mergeable_state"] = random.choice(["CLEAN", "DIRTY", "UNSTABLE", "BEHIND", None])
    if random.random() < 0.5: pr["threads_unresolved"] = random.choice([0, 0, 1, 3])
    if random.random() < 0.3: pr["threads_unanswered"] = random.choice([0, 1, 2])
    if random.random() < 0.2: pr["standing_blocker"] = random.choice([None, {"verdict": "needs_revision", "by": "Gemini"}, {}])
    if random.random() < 0.8: pr["covered"] = random.choice([True, False])
    if random.random() < 0.3: pr["base"] = random.choice(["main", "proposal/x"])
    if random.random() < 0.9: pr["updated_at"] = random.choice(TIMES[:8] if random.random() < 0.95 else TIMES)
    if random.random() < 0.3: pr["title"] = random.choice(["t", "caf\u00e9", ""])
    if random.random() < 0.2: pr["size"] = random.choice(["+3/-1", "", None])
    if random.random() < 0.2: pr["base_sha"] = random.choice(["b" * 40, ""])
    if random.random() < 0.4:
        pr["rejected_verdicts"] = [random.choice([{"comment_id": 1, "seat": "gemini", "reason": "r"}, {"comment_id": 2, "seat": "", "reason": "r"},
                                                  {"comment_id": 3, "reason": "r"}, {"comment_id": 4, "seat": "claude", "reason": "r"}])
                                   for _ in range(random.randint(1, 3))]
    return pr
work_cases = []
for _ in range(int(sys.argv[1]) // 5):
    prs = [random_pr(random.randint(1, 12)) for _ in range(random.randint(0, 4))]
    state = {"now": "2026-10-05T00:00:00Z", "prs": prs}
    if random.random() < 0.5: state["default_branch"] = random.choice(["main", "master"])
    done = []
    for pr in prs:
        for kind in random.sample(["needs_review", "fix_threads", "ready_to_merge", "draft", "stale", "stacked", "repair_verdict"], random.randint(0, 2)):
            done.append(nt.item_key(kind, pr["number"], pr.get("head", "")))
    if done or random.random() < 0.3: state["done"] = done
    try:
        out = {"items": nt.work_items(json.loads(json.dumps(state)))}
    except Exception:
        out = {"error": True}
    work_cases.append({"state": state, "out": out})
AGENT_POOL = ["claude", "codex", "gemini", "mimo"]
card_cases = []
for _ in range(int(sys.argv[1]) // 5):
    prs = [random_pr(n) for n in random.sample(range(1, 15), random.randint(0, 5))]
    for pr in prs:
        if random.random() < 0.97: pr.setdefault("updated_at", random.choice(TIMES[:8]))
    agents = [{"name": n} for n in random.sample(AGENT_POOL, random.randint(1, 4))]
    for agent in agents:
        if random.random() < 0.2: agent["roles"] = random.choice([["reviewer"], ["implementer", "skeptic"], []])
    state = {"now": "2026-10-05T00:00:00Z", "prs": prs, "agents": agents}
    if random.random() < 0.3: state["backlog"] = [{"id": f"b{k}", "task": random.choice(["do x", "café"]),
                                                     **({"acceptance": "proof"} if random.random() < 0.5 else {}),
                                                     **({"detail": "d"} if random.random() < 0.5 else {})} for k in range(random.randint(1, 2))]
    if random.random() < 0.5: state["repo"] = "o/r"
    if random.random() < 0.5: state["last_merged"] = {"number": random.randint(1, 99), "merge_commit": "m" * random.choice([5, 40]), "head": "h" * 40}
    if random.random() < 0.3: state["lease_ttl"] = random.choice([2, 4, 9])
    try:
        probe = [i for i in nt.work_items(json.loads(json.dumps(state))) if i["kind"] != "stacked" and i.get("pr")]
    except Exception:
        probe = []
    keys = [nt.item_key(i["kind"], i["pr"], i.get("head", "")) for i in probe]
    if keys and random.random() < 0.5:
        state["leases"] = {k: {"agent": random.choice(AGENT_POOL), "expires": random.randint(1, 40)}
                           for k in random.sample(keys, random.randint(1, len(keys)))}
    history = {}
    if random.random() < 0.5:
        history["agents"] = {n: {"done": random.randint(0, 3), "expired": random.randint(0, 3)} for n in AGENT_POOL if random.random() < 0.7}
    if keys and random.random() < 0.5:
        history["expired"] = {f"{random.choice(AGENT_POOL)}|{k}": random.randint(0, 2) for k in keys if random.random() < 0.6}
        history["item_attempts"] = {k: random.randint(0, 9) for k in keys if random.random() < 0.7}
    if probe and random.random() < 0.5:
        history["last_head"] = {f"{random.choice(AGENT_POOL)}|{i['pr']}": random.choice([i.get("head", "")[:8], "00000000"]) for i in probe}
    if history or random.random() < 0.2: state["history"] = history
    tick = random.randint(0, 30)
    try:
        out = {"cards": nt.build_cards(json.loads(json.dumps(state)), tick)}
    except Exception:
        out = {"error": True}
    card_cases.append({"state": state, "tick": tick, "out": out})
json.dump({"cards": card_cases, "work": work_cases, "chain": chain_cases, "text": cases, "stalls": stalls, "pushes": push_cases, "coverage": coverage, "independence": independence_cases, "gate_checklist": checklist_cases, "gate_decide": decide_cases, "gate_plan": gate_plan, "gate_ci": gate_ci, "gate_breaker": gate_breaker, "gate_threads": gate_threads}, open(sys.argv[3], "w", encoding="utf-8"), ensure_ascii=False)
print(len(cases), len(stalls), len(push_cases))
