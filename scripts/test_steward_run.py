import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import next_task as nt  # noqa: E402
import steward_run as sr  # noqa: E402

NOW = "2026-10-04T12:00:00Z"
AGENTS = [{"name": "claude"}, {"name": "mimo"}, {"name": "codex"}]


def state(prs):
    return {"now": NOW, "agents": AGENTS, "prs": prs, "backlog": [], "done": []}


def pr(number, head="a" * 40, **kw):
    base = {
        "number": number, "title": "t", "head": head, "draft": False, "mergeable_state": "CLEAN",
        "updated_at": "2026-10-04T11:00:00Z", "seat": "claude", "threads_unresolved": 0, "covered": False,
    }
    base.update(kw)
    return base


def snap(comments_by_pr):
    return {"prs": [{"number": n, "comments": c} for n, c in comments_by_pr.items()]}


def read_lines(path):
    with open(path, encoding="utf-8") as handle:
        return handle.read().splitlines()


def write_lines(path, lines):
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(chr(10).join(lines) + chr(10))


def tmp_log():
    handle = tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False)
    handle.close()
    os.unlink(handle.name)
    return handle.name


class Chain(unittest.TestCase):
    def test_appended_events_form_a_verifiable_chain(self):
        path = tmp_log()
        for i in range(3):
            sr.append_event(path, "x", {"i": i}, NOW)
        self.assertEqual(sr.verify_chain(path), (True, ""))
        os.unlink(path)

    def test_editing_an_event_breaks_the_chain(self):
        path = tmp_log()
        for i in range(3):
            sr.append_event(path, "x", {"i": i}, NOW)
        lines = read_lines(path)
        lines[1] = lines[1].replace('"i": 1', '"i": 99').replace('"i":1', '"i":99')
        write_lines(path, lines)
        ok, problem = sr.verify_chain(path)
        self.assertFalse(ok)
        self.assertIn("chain broken", problem)
        os.unlink(path)

    def test_removing_an_event_breaks_the_chain(self):
        path = tmp_log()
        for i in range(3):
            sr.append_event(path, "x", {"i": i}, NOW)
        lines = read_lines(path)
        write_lines(path, [lines[0], lines[2]])
        self.assertFalse(sr.verify_chain(path)[0])
        os.unlink(path)


class Ingest(unittest.TestCase):
    def test_seat_is_read_from_the_text(self):
        self.assertEqual(sr.seat_from_comment("REVIEW PACKET\nSeat / agent: Mimo (x)"), "mimo")
        self.assertEqual(sr.seat_from_comment("Codex independent review at abc"), "codex")
        self.assertEqual(sr.seat_from_comment("### Review\n- **Reviewer**: Gemini / Antigravity"), "gemini")
        self.assertEqual(sr.seat_from_comment("random words"), "")

    def test_new_comments_are_counted_once_per_seat(self):
        path = tmp_log()
        comments = {1: [{"id": 10, "body": "Codex independent review"}, {"id": 11, "body": "Seat / agent: Mimo"}]}
        first = sr.run_tick(state([pr(1)]), snap(comments), path, 1, NOW)
        self.assertEqual(first["ingest"]["per_seat"], {"codex": 1, "mimo": 1})
        second = sr.run_tick(state([pr(1)]), snap(comments), path, 2, NOW)
        self.assertEqual(second["ingest"]["comment_ids"], [])
        os.unlink(path)

    def test_unattributed_comments_are_counted_separately(self):
        path = tmp_log()
        result = sr.run_tick(state([pr(1)]), snap({1: [{"id": 5, "body": "plain note"}]}), path, 1, NOW)
        self.assertEqual(result["ingest"]["unattributed"], 1)
        os.unlink(path)


class Stalls(unittest.TestCase):
    def ticks(self, path, produced_by_tick, total):
        results = []
        for tick in range(1, total + 1):
            body = produced_by_tick.get(tick)
            comments = {1: [{"id": 100 + tick, "body": body}]} if body else {1: []}
            results.append(sr.run_tick(state([pr(1)]), snap(comments), path, tick, NOW, stall_after=3))
        return results

    def test_a_seat_that_never_produces_is_escalated_after_k_ticks(self):
        path = tmp_log()
        results = self.ticks(path, {}, 3)
        escalated = {e["agent"] for e in results[-1]["escalations"]}
        self.assertEqual(escalated, {"claude", "mimo", "codex"})
        self.assertFalse(results[0]["escalations"])
        os.unlink(path)

    def test_producing_one_artifact_clears_the_stall(self):
        path = tmp_log()
        results = self.ticks(path, {3: "Mimo independent review"}, 3)
        escalated = {e["agent"] for e in results[-1]["escalations"]}
        self.assertNotIn("mimo", escalated)
        self.assertIn("codex", escalated)
        os.unlink(path)

    def test_escalation_demands_an_artifact_or_a_named_blocker(self):
        path = tmp_log()
        card = self.ticks(path, {}, 3)[-1]["escalations"][0]
        self.assertIn("name the blocker", card["task"])
        self.assertIn("silence", card["if_blocked"])
        os.unlink(path)

    def test_the_log_stays_verifiable_after_ticks(self):
        path = tmp_log()
        self.ticks(path, {2: "Codex review"}, 4)
        self.assertEqual(sr.verify_chain(path), (True, ""))
        os.unlink(path)


class Attribution(unittest.TestCase):
    def test_weak_attribution_suppresses_stall_escalation(self):
        path = tmp_log()
        results = []
        for tick in range(1, 4):
            comments = {1: [{"id": 200 + tick * 10 + n, "body": "plain note with no seat"} for n in range(3)]}
            results.append(sr.run_tick(state([pr(1)]), snap(comments), path, tick, NOW, stall_after=3))
        self.assertTrue(results[-1]["attribution_weak"])
        self.assertEqual(results[-1]["escalations"], [])
        os.unlink(path)

    def test_good_attribution_still_escalates(self):
        path = tmp_log()
        results = []
        for tick in range(1, 4):
            comments = {1: [{"id": 300 + tick, "body": "Gemini independent review"}]}
            results.append(sr.run_tick(state([pr(1)]), snap(comments), path, tick, NOW, stall_after=3))
        self.assertFalse(results[-1]["attribution_weak"])
        self.assertTrue({e["agent"] for e in results[-1]["escalations"]} >= {"claude", "mimo", "codex"})
        os.unlink(path)


class Leases(unittest.TestCase):
    def two_prs(self, **kw):
        s = state([pr(1, seat="other"), pr(2, seat="other")])
        s.update(kw)
        return s

    def tasks_by_agent(self, result):
        return {c["agent"]: c["task"] for c in result["cards"]}

    def test_an_agent_keeps_the_card_it_holds_a_lease_on(self):
        path = tmp_log()
        runs = [sr.run_tick(self.two_prs(), snap({1: [], 2: []}), path, t, NOW) for t in range(1, 5)]
        first = self.tasks_by_agent(runs[0])
        for later in runs[1:]:
            self.assertEqual(self.tasks_by_agent(later), first)
        os.unlink(path)

    def test_a_leased_item_is_not_given_to_another_agent(self):
        path = tmp_log()
        run = sr.run_tick(self.two_prs(), snap({1: [], 2: []}), path, 1, NOW)
        reviews = [c["task"] for c in run["cards"] if "independent whole-diff review" in c["task"]]
        self.assertEqual(len(reviews), len(set(reviews)))
        later = sr.run_tick(self.two_prs(), snap({1: [], 2: []}), path, 2, NOW)
        again = [c["task"] for c in later["cards"] if "independent whole-diff review" in c["task"]]
        self.assertEqual(sorted(again), sorted(reviews))
        os.unlink(path)

    def test_a_completed_card_releases_its_lease_and_is_not_reissued(self):
        path = tmp_log()
        first = sr.run_tick(self.two_prs(), snap({1: [], 2: []}), path, 1, NOW)
        key = next(c["lease"]["key"] for c in first["cards"] if c.get("lease"))
        done = self.two_prs(done=[key])
        second = sr.run_tick(done, snap({1: [], 2: []}), path, 2, NOW)
        self.assertIn(key, second["leases"]["completed"])
        self.assertFalse(any(c.get("lease", {}).get("key") == key for c in second["cards"]))
        events = sr.read_events(path)
        self.assertTrue(any(e["type"] == "lease_done" and e["data"]["key"] == key for e in events))
        os.unlink(path)

    def test_an_expired_lease_returns_the_item_and_escalates_the_holder(self):
        path = tmp_log()
        ttl = nt.LEASE_TTL
        runs = [sr.run_tick(self.two_prs(), snap({1: [], 2: []}), path, t, NOW, stall_after=99) for t in range(1, ttl + 2)]
        last = runs[-1]
        self.assertTrue(last["leases"]["expired"])
        self.assertTrue(last["escalations"])
        self.assertIn("lease expired", last["escalations"][0]["task"])
        events = sr.read_events(path)
        self.assertTrue(any(e["type"] == "lease_expired" for e in events))
        os.unlink(path)

    def test_the_log_with_leases_still_verifies(self):
        path = tmp_log()
        for t in range(1, 9):
            sr.run_tick(self.two_prs(), snap({1: [], 2: []}), path, t, NOW)
        self.assertEqual(sr.verify_chain(path), (True, ""))
        os.unlink(path)


if __name__ == "__main__":
    unittest.main()
