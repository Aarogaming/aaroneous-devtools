"""The committed golden vectors are the steward contract. This test reads only the JSON files, never the scenario
definitions, so it checks the code against the contract and not the generator against itself."""

import glob
import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import collect_state as cs  # noqa: E402
import golden_vectors as gv  # noqa: E402
import merge_gate as mg  # noqa: E402
import next_task as nt  # noqa: E402
import review_coverage as rc  # noqa: E402
import steward_run as sr  # noqa: E402

DIR = gv.DIR
RULE_FAMILIES = ("attribution", "heads", "receipts", "rejected", "stalls", "pushes", "seats", "coverage", "audit",
                 "gate_tier", "gate_ci", "gate_breaker", "gate_threads", "gate_checklist", "gate_decide")


def load(name):
    with open(os.path.join(DIR, name), encoding="utf-8") as handle:
        return json.load(handle)


class Golden(unittest.TestCase):
    def test_every_scenario_input_reproduces_its_committed_cards_exactly(self):
        inputs = sorted(glob.glob(os.path.join(DIR, "*.in.json")))
        names = [os.path.basename(p)[: -len(".in.json")] for p in inputs if not p.endswith("chain.in.json")]
        names = [n for n in names if n not in RULE_FAMILIES]
        self.assertGreaterEqual(len(names), 10)
        for name in names:
            case = load(f"{name}.in.json")
            self.assertEqual(nt.build_cards(case["state"], case["tick"]), load(f"{name}.out.json"), name)

    def test_every_rule_vector_reproduces_its_committed_results_exactly(self):
        attribution = load("attribution.in.json")
        self.assertEqual([cs.comment_seat(c["body"], strict=c["strict"]) for c in attribution], load("attribution.out.json"))
        heads = load("heads.in.json")
        self.assertEqual([cs.cites_head(c["body"], c["head"]) for c in heads], load("heads.out.json"))
        receipts = load("receipts.in.json")
        self.assertEqual(
            [cs.done_receipts(c["snapshot"], {int(k): v for k, v in c["repair_owners"].items()},
                              {int(k): v for k, v in c["implementers_by_pr"].items()}) for c in receipts],
            load("receipts.out.json"))
        rejected = load("rejected.in.json")
        self.assertEqual([cs.rejected_verdicts(c["pr"], c["row"]) for c in rejected], load("rejected.out.json"))
        stalls = load("stalls.in.json")
        self.assertEqual(
            [{"stalled": sr.stalled_seats(c["events"], c["agents"], c["stall_after"]),
              "attribution_weak": sr.attribution_weak([e for e in c["events"] if e["type"] == "ingest"], c["window"])}
             for c in stalls], load("stalls.out.json"))
        pushes = load("pushes.in.json")
        self.assertEqual([dict(zip(("heads", "credited"), sr.pushes(c["prs"], c["previous"]))) for c in pushes],
                         load("pushes.out.json"))
        seats = load("seats.in.json")
        self.assertEqual([sorted(rc.seat_names(s)) for s in seats], load("seats.out.json"))
        coverage = load("coverage.in.json")
        results = []
        for c in coverage:
            pairs = {(a, b) for a, b in c["ancestry"]}
            results.append(rc.evaluate(c["head"], c["merge_base"], c["records"], lambda a, b: a == b or (a, b) in pairs))
        self.assertEqual(results, load("coverage.out.json"))
        audit = load("audit.in.json")
        self.assertEqual(
            [{"tick": t, "tasks": [x["task"] for x in nt.build_cards(json.loads(json.dumps(audit["state"])), t)]}
             for t in audit["ticks"]], load("audit.out.json"))

    def test_every_merge_gate_vector_reproduces_its_committed_results_exactly(self):
        tier = load("gate_tier.in.json")
        self.assertEqual(
            [{"tier": list(mg.tier_of(c["paths"])), "protected": mg.touches_protected(c["paths"]),
              "plan_level": mg.gate_plan(c["paths"])["level"], "category": mg.category_of(c["paths"])} for c in tier],
            load("gate_tier.out.json"))
        self.assertEqual([mg.ci_state(c["rollup"]) for c in load("gate_ci.in.json")], load("gate_ci.out.json"))
        self.assertEqual([mg.breaker_state(c["text"]) for c in load("gate_breaker.in.json")], load("gate_breaker.out.json"))
        self.assertEqual([mg.open_threads(c["data"]) for c in load("gate_threads.in.json")], load("gate_threads.out.json"))
        checklist = load("gate_checklist.in.json")
        results = []
        for case in checklist["cases"]:
            facts = json.loads(json.dumps(checklist["green"]))
            facts.update(case["override"])
            lines, would, first = mg.checklist(facts)
            results.append({"lines": lines, "would_merge": would, "first_blocker": first})
        self.assertEqual(results, load("gate_checklist.out.json"))
        decided = []
        for c in load("gate_decide.in.json"):
            result = mg.decide(c["row"], c["paths"], c["base"], c["trial"], c["owner"])
            _, eligible, first = mg.reconcile(result["eligible"], json.loads(json.dumps(checklist["green"])))
            decided.append({"decide": result, "reconciled_with_all_green_checklist": {"eligible": eligible, "first_blocker": first}})
        self.assertEqual(decided, load("gate_decide.out.json"))

    def test_the_merge_gate_vectors_pin_what_makes_a_pr_eligible_to_merge(self):
        def named(family):
            return {c["name"]: o for c, o in zip(load(f"{family}.in.json")["cases"] if family == "gate_checklist"
                                                 else load(f"{family}.in.json"), load(f"{family}.out.json"))}

        checklist = named("gate_checklist")
        self.assertTrue(checklist["every box green would merge"]["would_merge"])
        for blocked in [n for n in checklist if n != "every box green would merge"]:
            self.assertFalse(checklist[blocked]["would_merge"], blocked)
        self.assertEqual(checklist["red CI blocks"]["first_blocker"], "1")
        self.assertEqual(checklist["a merge conflict blocks"]["first_blocker"], "2")
        self.assertEqual(checklist["a stacked base blocks"]["first_blocker"], "2a")
        self.assertEqual(checklist["one accept seat is not enough"]["first_blocker"], "3")
        self.assertEqual(checklist["an open thread blocks"]["first_blocker"], "4")
        self.assertEqual(checklist["a tripped breaker blocks"]["first_blocker"], "7")
        self.assertEqual(checklist["trust below the threshold blocks"]["first_blocker"], "8")
        self.assertTrue(checklist["tier A is owner-only"]["first_blocker"].startswith("0"))
        tier = named("gate_tier")
        self.assertEqual(tier["AGENTS.md is owner-only"]["tier"][0], "A")
        self.assertEqual(tier["workflow files are owner-only"]["tier"][0], "A")
        self.assertEqual(tier["Rust sources are unclassified, never assumed safe"]["tier"][0], "unclassified")
        self.assertEqual(tier["docs and governance markdown is tier B with the light gate"]["plan_level"], "light")
        self.assertEqual(tier["scripts are tier B but get the full gate"]["plan_level"], "full")
        self.assertEqual(tier["a lookalike prefix is also owner-only (prefix match, fails closed)"]["protected"], ["AGENTS.md.bak"])
        ci = named("gate_ci")
        self.assertEqual(ci["no rollup is missing"]["state"], "missing")
        self.assertEqual(ci["all successful"]["state"], "success")
        self.assertEqual(ci["skipped is not success"]["state"], "failure")
        self.assertEqual(ci["a running check is pending"]["state"], "pending")
        breaker = named("gate_breaker")
        self.assertIs(breaker["tripped = false"], False)
        self.assertIs(breaker["a comment is not the value"], True)
        for unreadable in ("only a comment is blocked", "a string is not a boolean", "invalid TOML is blocked",
                           "an empty file is blocked"):
            self.assertIsNone(breaker[unreadable], unreadable)
        threads = named("gate_threads")
        self.assertEqual(threads["a complete page with two open"], 2)
        for unknown in ("a full page that has a next page is unknown", "missing pagination info is unknown",
                        "a malformed response is unknown"):
            self.assertIsNone(threads[unknown], unknown)
        decide = named("gate_decide")
        self.assertTrue(decide["covered clean docs PR on main is eligible"]["decide"]["eligible"])
        self.assertEqual(len(decide["every blocking reason is reported together"]["decide"]["reasons"]), 4)
        self.assertTrue(decide["owner confirmation lifts only condition f"]["decide"]["eligible"])

    def test_the_rule_vectors_pin_the_decisions_merge_safety_depends_on(self):
        out = {name: load(f"{name}.out.json") for name in RULE_FAMILIES}
        cases = {name: load(f"{name}.in.json") for name in RULE_FAMILIES}
        by_name = lambda family: {c["name"]: o for c, o in zip(cases[family], out[family]) if isinstance(c, dict) and "name" in c}  # noqa: E731
        # a receipt completes a card only for the right head, from an eligible seat, with no negative first line
        receipts = by_name("receipts")
        self.assertEqual(receipts["author seat completes thread fixes"], {"fix_threads:7:cccccccc": "claude"})
        for refused in ("a negative first line is not a completion", "NOT DONE is negative in any case",
                        "a quoted receipt is not a posted one", "a receipt for another head does not complete the card",
                        "a different head sharing the first eight characters conflicts",
                        "a receipt naming another PR does not count", "only the author completes thread fixes",
                        "an unattributed receipt never completes", "an implementer cannot complete a review",
                        "unknown implementers fail closed for reviews"):
            self.assertEqual(receipts[refused], {}, refused)
        self.assertEqual(receipts["the first eligible receipt names the seat"], {"needs_review:7:cccccccc": "gemini"})
        self.assertEqual(receipts["only the seat whose verdict was rejected completes the repair"],
                         {"repair_verdict:7:cccccccc": "gemini"})
        # attribution
        self.assertEqual(out["attribution"][2], "")  # a seat named below the first line is not the author
        self.assertEqual(out["attribution"][6], "")  # a bot footer is not a seat
        self.assertEqual(out["attribution"][7], "gemini")  # Antigravity is the Gemini seat
        # head citation
        # the first seven pin prefix length, case and a different full head; the next five pin Python's word characters:
        # a combining mark, a Thai vowel sign and connector punctuation do not join a token, a superscript digit does
        self.assertEqual(out["heads"], [True, False, True, False, True, False, False, True, True, True, False, False])
        # rejected verdicts: history is not repaired, at most two per seat
        rejected = by_name("rejected")
        self.assertEqual(len(rejected["a legacy comment citing the current head goes back to its seat"]), 1)
        self.assertEqual(rejected["a comment that does not cite the current head is history"], [])
        self.assertEqual(rejected["a head sharing eight characters is not this head"], [])
        self.assertEqual(len(rejected["at most the latest two per seat are kept"]), 2)
        pushed = {c["name"]: r for c, r in zip(load("pushes.in.json"), load("pushes.out.json"))}
        self.assertEqual(pushed["no earlier record credits nobody and records every head"]["credited"], {})
        self.assertEqual(pushed["a moved head credits the PR's seat"]["credited"], {"claude": 1})
        self.assertEqual(pushed["two moved heads of one seat count twice"]["credited"], {"claude": 2})
        self.assertEqual(pushed["a moved head with no known seat is recorded and credits nobody"]["credited"], {})
        # stalls and weak attribution
        stalls = by_name("stalls")
        self.assertEqual(stalls["three silent ticks stall every seat that was issued cards"]["stalled"],
                         ["claude", "mimo", "codex", "gemini"])
        self.assertNotIn("mimo", stalls["one artifact in the last tick clears that seat"]["stalled"])
        self.assertEqual(stalls["fewer ticks than the window judges nobody"]["stalled"], [])
        self.assertTrue(stalls["unattributed comments make attribution weak"]["attribution_weak"])
        self.assertFalse(stalls["exactly half attributed is not weak"]["attribution_weak"])
        self.assertFalse(stalls["fewer than three comments is never weak"]["attribution_weak"])
        # seat names: a spelling is a seat, a string may name several, an unknown one keeps its first token
        named = dict(zip(load("seats.in.json"), load("seats.out.json")))
        self.assertEqual(named["Gemini/Antigravity"], ["gemini"])
        self.assertEqual(named["Claude, Mimo"], ["claude", "mimo"])
        self.assertEqual(named["gemini-3.8-flash"], ["gemini"])
        self.assertEqual(named["someone else"], ["someone"])
        self.assertEqual(named["!!!"], [])
        # coverage
        coverage = by_name("coverage")
        for refused in ("the same seat in another case or alias cannot accept its own work",
                        "an implementer named by model id is still that seat",
                        "one implementer string may name two seats and neither can accept"):
            self.assertFalse(coverage[refused]["covered"], refused)
        self.assertTrue(coverage["a third seat still covers when an implementer string names two others"]["covered"])
        self.assertEqual(coverage["one seat spelled three ways is one accepting seat"]["accept_seats"], ["gemini"])
        for covered in ("a whole accept at the head covers", "a delta chained from a whole accept covers",
                        "two chained deltas cover"):
            self.assertTrue(coverage[covered]["covered"], covered)
        for refused in ("a delta with no whole origin does not cover", "a delta whose from is not an ancestor does not cover",
                        "an accept at an older head does not carry", "an implementer cannot accept its own work",
                        "a standing needs_revision at the head blocks", "a record bound to another merge base is invalid",
                        "a moved merge base invalidates the record"):
            self.assertFalse(coverage[refused]["covered"], refused)
        # the audit card appears on multiples of AUDIT_EVERY and only then
        audit_ticks = [o["tick"] for o in out["audit"] if any("Re-run review_coverage.py" in t for t in o["tasks"])]
        self.assertEqual(audit_ticks, [t for t in range(14) if t % 6 == 0])  # the contract says every 6th tick

    def test_the_log_chain_reproduces_its_exact_bytes_and_tip(self):
        events = load("chain.in.json")
        expected = load("chain.out.json")
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "events.jsonl")
            for event in events:
                sr.append_event(path, event["type"], event["data"], event["now"])
            with open(path, encoding="utf-8") as handle:
                lines = [line.rstrip("\n") for line in handle if line.strip()]
        self.assertEqual(lines, expected["lines"])
        self.assertEqual(sr.digest(lines[-1]), expected["tip"])
        self.assertEqual(json.loads(lines[0])["prev"], sr.GENESIS)

    def test_the_committed_vectors_are_current_with_the_generator(self):
        for name, obj in gv.build_outputs().items():
            with open(os.path.join(DIR, name), encoding="utf-8") as handle:
                self.assertEqual(handle.read(), gv.dump(obj), name)

    def test_the_vectors_pin_the_behaviours_the_contract_depends_on(self):
        def cards(name):
            return {c["agent"]: c for c in load(f"{name}.out.json")}

        review = cards("review_excludes_every_implementer")
        self.assertEqual(review["mimo"]["inputs"]["kind"], "standby")
        self.assertEqual(review["gemini"]["inputs"]["kind"], "needs_review")
        self.assertEqual(cards("fix_threads_goes_only_to_the_author")["codex"]["inputs"]["kind"], "fix_threads")
        self.assertEqual(cards("uncovered_draft_gets_a_review_card")["mimo"]["inputs"]["kind"], "needs_review")
        self.assertEqual(cards("covered_draft_gets_the_flip_ready_card")["mimo"]["inputs"]["kind"], "draft")
        for card in load("stacked_pr_gets_no_merge_brief.out.json"):
            self.assertNotEqual(card["inputs"]["kind"], "ready_to_merge")
        self.assertEqual(cards("lens_advances_with_each_handout")["mimo"]["lens"], "minimal")
        self.assertEqual(cards("held_lease_keeps_holder_and_lens")["gemini"]["inputs"]["kind"], "needs_review")
        self.assertNotEqual(cards("expired_holder_rotates_to_another_seat")["mimo"]["inputs"]["kind"], "needs_review")
        self.assertEqual(cards("finishers_are_served_first")["gemini"]["inputs"]["kind"], "needs_review")
        self.assertEqual(cards("healthy_idle")["mimo"]["inputs"]["kind"], "healthy_idle")
        repair = cards("rejected_verdict_goes_back_to_its_author")
        self.assertEqual(repair["gemini"]["inputs"]["kind"], "repair_verdict")
        self.assertEqual(repair["gemini"]["rejected"][0]["comment_id"], 77)
        self.assertNotEqual(repair["mimo"]["inputs"]["kind"], "repair_verdict")



if __name__ == "__main__":
    unittest.main()
