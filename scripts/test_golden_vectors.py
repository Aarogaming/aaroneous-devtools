"""The committed golden vectors are the steward contract. This test reads only the JSON files, never the scenario
definitions, so it checks the code against the contract and not the generator against itself."""

import glob
import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import golden_vectors as gv  # noqa: E402
import next_task as nt  # noqa: E402
import steward_run as sr  # noqa: E402

DIR = gv.DIR


def load(name):
    with open(os.path.join(DIR, name), encoding="utf-8") as handle:
        return json.load(handle)


class Golden(unittest.TestCase):
    def test_every_scenario_input_reproduces_its_committed_cards_exactly(self):
        inputs = sorted(glob.glob(os.path.join(DIR, "*.in.json")))
        names = [os.path.basename(p)[: -len(".in.json")] for p in inputs if not p.endswith("chain.in.json")]
        self.assertGreaterEqual(len(names), 10)
        for name in names:
            case = load(f"{name}.in.json")
            self.assertEqual(nt.build_cards(case["state"], case["tick"]), load(f"{name}.out.json"), name)

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
