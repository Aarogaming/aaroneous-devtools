"""Generate adversarial text cases and the Python oracle's answers for the Rust differential test (lane C99).

  python scripts/differential_cases.py COUNT SEED OUT.json
  STEWARD_DIFF_FILE=OUT.json cargo test -p steward --test differential -- --ignored --nocapture   (run in foundation/)

Not part of the gate: the committed golden vectors are the contract. This is how divergences between the two
languages are found (the first run, 110,000 bodies, found six: see the vectors tagged as found by it). Each new
divergence is then pinned as a vector.
"""

import json, random, sys
sys.path.insert(0, 'scripts')
import collect_state as cs, review_coverage as rc
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
json.dump(cases, open(sys.argv[3], "w", encoding="utf-8"), ensure_ascii=False)
print(len(cases))
