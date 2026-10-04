import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import next_task as nt  # noqa: E402

NOW = "2026-10-04T12:00:00Z"
AGENTS = [
    {"name": "claude", "roles": ["implementer", "reviewer", "auditor", "skeptic"]},
    {"name": "mimo"},
    {"name": "codex"},
]


def pr(number, **kw):
    base = {
        "number": number,
        "title": f"pr {number}",
        "head": "a" * 40,
        "draft": False,
        "mergeable_state": "CLEAN",
        "updated_at": "2026-10-04T11:00:00Z",
        "seat": "claude",
        "threads_unresolved": 0,
        "covered": False,
    }
    base.update(kw)
    return base


def state(prs, backlog=None):
    return {"now": NOW, "agents": AGENTS, "prs": prs, "backlog": backlog or []}


class Cards(unittest.TestCase):
    def test_uncovered_pr_goes_to_a_reviewer_who_is_not_the_implementer(self):
        cards = nt.build_cards(state([pr(1)]), 0)
        reviewers = [c for c in cards if "independent whole-diff review of PR #1" in c["task"]]
        self.assertTrue(reviewers)
        self.assertNotIn("claude", [c["agent"] for c in reviewers])

    def test_unresolved_threads_are_the_implementers_task_first(self):
        cards = nt.build_cards(state([pr(2, threads_unresolved=3, covered=True)]), 0)
        owner = next(c for c in cards if c["agent"] == "claude")
        self.assertIn("unresolved review thread", owner["task"] + owner["why_now"])
        self.assertEqual(owner["priority"], 1)

    def test_blocked_on_owner_produces_a_merge_brief_not_a_pending_report(self):
        ready = [pr(3, covered=True), pr(4, covered=True)]
        cards = nt.build_cards(state(ready), 0)
        self.assertTrue(all(c["owner_blocked"] for c in cards))
        self.assertTrue(any("owner merge brief" in c["task"] for c in cards))

    def test_draft_pr_gets_a_ready_for_review_task(self):
        cards = nt.build_cards(state([pr(5, draft=True, covered=True)]), 0)
        self.assertTrue(any("ready for review" in c["task"] for c in cards))

    def test_stale_pr_is_triaged(self):
        stale = pr(6, covered=True, updated_at="2026-09-28T00:00:00Z", mergeable_state="DIRTY")
        kinds = {i["kind"] for i in nt.work_items(state([stale]))}
        self.assertIn("stale", kinds)
        self.assertIn("not_mergeable", kinds)

    def test_roles_rotate_across_ticks(self):
        s = state([pr(1)])
        roles = {nt.build_cards(s, tick)[0]["role"] for tick in range(4)}
        self.assertEqual(len(roles), 4)

    def test_healthy_state_yields_explicit_healthy_idle_with_evidence(self):
        # A covered CLEAN PR would be ready_to_merge, so a healthy state here is "no open PRs at all".
        for tick in (1, 2, 3, 4, 5):
            for card in nt.build_cards(state([]), tick):
                self.assertEqual(card["inputs"]["kind"], "healthy_idle")
                self.assertEqual(card["evidence"]["open_prs"], 0)
                self.assertIn("do not invent work", card["acceptance"])

    def test_periodic_audit_every_nth_tick_when_healthy(self):
        s = state([])
        s["repo"] = "o/r"
        s["last_merged"] = {"number": 19, "head": "h" * 40, "merge_commit": "m" * 40}
        audits = [t for t in range(0, 13) if nt.build_cards(s, t)[0]["inputs"]["kind"] == "audit"]
        self.assertEqual(audits, [0, 6, 12])
        task = nt.build_cards(s, 0)[0]["task"]
        self.assertIn("o/r#19", task)
        self.assertIn("mmmmmmmmmmmm", task)

    def test_audit_without_an_explicit_subject_is_not_issued(self):
        for tick in (0, 6):
            kinds = {c["inputs"]["kind"] for c in nt.build_cards(state([]), tick)}
            self.assertEqual(kinds, {"healthy_idle"})

    def test_highest_priority_item_is_taken_first_regardless_of_tick(self):
        s = state([pr(1, threads_unresolved=2, seat="claude"), pr(2, seat="other"), pr(3, seat="other")],
                  backlog=[{"id": "b", "task": "backlog work", "acceptance": "x"}])
        for tick in range(0, 8):
            cards = nt.build_cards(s, tick)
            owner = next(c for c in cards if c["agent"] == "claude")
            self.assertEqual(owner["inputs"]["kind"], "fix_threads", tick)
            self.assertFalse(any("backlog work" in c["task"] for c in cards), tick)

    def test_stale_overlay_on_a_ready_pr_does_not_clear_owner_blocked(self):
        old = pr(3, covered=True, updated_at="2026-09-20T00:00:00Z")
        kinds = {i["kind"] for i in nt.work_items(state([old]))}
        self.assertEqual(kinds, {"ready_to_merge"})
        self.assertTrue(all(c["owner_blocked"] for c in nt.build_cards(state([old]), 0)))

    def test_urgent_work_is_never_hidden_by_idle(self):
        for tick in range(0, 7):
            kinds = {c["inputs"]["kind"] for c in nt.build_cards(state([pr(1)]), tick)}
            self.assertNotIn("healthy_idle", kinds)

    def test_work_is_spread_across_agents_when_there_is_enough_of_it(self):
        s = state([pr(1, seat="other"), pr(2, seat="other"), pr(3, seat="other")])
        reviews = [c["task"] for c in nt.build_cards(s, 0) if "independent whole-diff review" in c["task"]]
        self.assertEqual(len(reviews), len(set(reviews)))

    def test_a_completed_card_for_the_same_head_is_not_reissued(self):
        s = state([pr(8, covered=True)])
        first = [c for c in nt.build_cards(s, 0) if c["inputs"]["kind"] == "ready_to_merge"]
        self.assertTrue(first)
        marker = first[0]["marker"]
        self.assertTrue(marker.startswith("[card:ready_to_merge:8:"))
        s["done"] = [marker[len("[card:"):-1]]
        after = nt.build_cards(s, 0)
        self.assertFalse(any(c["inputs"]["kind"] == "ready_to_merge" for c in after))

    def test_a_new_head_reissues_the_card(self):
        s = state([pr(8, covered=True)])
        s["done"] = [nt.item_key("ready_to_merge", 8, "a" * 40)]
        s["prs"][0]["head"] = "b" * 40
        self.assertTrue(any(c["inputs"]["kind"] == "ready_to_merge" for c in nt.build_cards(s, 0)))

    def test_backlog_items_are_used_after_urgent_work(self):
        backlog = [{"id": "b1", "task": "Normalise verdict wording in review_coverage.py", "acceptance": "tests pass"}]
        cards = nt.build_cards(state([], backlog), 0)
        self.assertTrue(any("Normalise verdict wording" in c["task"] for c in cards))

    def test_a_lease_held_by_an_agent_that_may_no_longer_hold_the_item_is_ignored(self):
        s = state([pr(1, threads_unresolved=2, seat="codex")])
        s["leases"] = {nt.item_key("fix_threads", 1, "a" * 40): {"agent": "mimo", "expires": 99}}
        cards = nt.build_cards(s, 0)
        owner = next(c for c in cards if c["inputs"]["kind"] == "fix_threads")
        self.assertEqual(owner["agent"], "codex")

    def test_cards_change_with_state(self):
        a = [c["task"] for c in nt.build_cards(state([pr(1)]), 0)]
        b = [c["task"] for c in nt.build_cards(state([pr(1, covered=True)]), 0)]
        self.assertNotEqual(a, b)

    def test_every_card_states_acceptance_and_a_blocked_action(self):
        for card in nt.build_cards(state([pr(1), pr(2, threads_unresolved=1)]), 2):
            self.assertTrue(card["acceptance"])
            self.assertIn("do not report 'pending'", card["if_blocked"])


if __name__ == "__main__":
    unittest.main()
