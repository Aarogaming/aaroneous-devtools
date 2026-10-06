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
        self.assertEqual(marker, "[done:ready_to_merge:8:" + "a" * 40 + "]")
        self.assertIn(marker, first[0]["acceptance"])
        s["done"] = [nt.item_key("ready_to_merge", 8, "a" * 40)]
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

    def test_a_covered_pr_stacked_on_a_non_default_base_gets_no_merge_brief(self):
        s = state([pr(9, covered=True, base="proposal/other")])
        s["default_branch"] = "main"
        kinds = {i["kind"] for i in nt.work_items(s)}
        self.assertEqual(kinds, {"stacked"})
        for card in nt.build_cards(s, 0):
            self.assertNotEqual(card["inputs"]["kind"], "ready_to_merge")
            self.assertNotIn("merge brief", card["task"])

    def test_a_covered_pr_on_the_default_base_still_gets_the_merge_brief(self):
        s = state([pr(9, covered=True, base="main")])
        s["default_branch"] = "main"
        self.assertEqual({i["kind"] for i in nt.work_items(s)}, {"ready_to_merge"})

    def test_an_uncovered_draft_gets_a_review_card_not_a_flip_ready_card(self):
        items = nt.work_items(state([pr(9, draft=True, covered=False)]))
        self.assertEqual({i["kind"] for i in items}, {"needs_review"})
        self.assertIn("draft", items[0]["detail"])

    def test_a_covered_draft_still_gets_the_flip_ready_card(self):
        self.assertEqual({i["kind"] for i in nt.work_items(state([pr(9, draft=True, covered=True)]))}, {"draft"})

    def review_recipe(self, implementers):
        s = state([pr(63, seat="", implementers=implementers, head="a" * 40, base_sha="b" * 40)])
        s["agents"] = [{"name": "gemini"}]
        card = [c for c in nt.build_cards(s, 0) if c["inputs"].get("kind") == "needs_review"][0]
        return [step for step in card["recipe"] if step.startswith("post ONE attestation block")][0]

    def test_unknown_implementers_are_not_printed_as_an_empty_list_the_tool_would_reject(self):
        step = self.review_recipe([])
        self.assertNotIn("implementers=[]", step)
        self.assertIn("NOT KNOWN", step)
        self.assertIn("git log --format='%(trailers:key=Co-Authored-By,key=Contribution-ID", step)
        self.assertIn("b" * 40 + ".." + "a" * 40, step)
        self.assertIn("instead of an accept block", step)

    def test_the_suggested_git_command_is_valid_shell(self):
        """Unquoted, the parentheses in the format are shell syntax and bash stops at "syntax error near unexpected token"."""
        import re
        import shutil
        import subprocess
        bash = shutil.which("bash")
        if not bash:
            self.skipTest("bash is not available")
        command = re.search(r"git log --format='[^']+' \S+\.\.\S+", self.review_recipe([]))
        self.assertIsNotNone(command)
        result = subprocess.run([bash, "-n", "-c", command.group(0)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_a_base_branch_name_with_shell_characters_stays_one_argument(self):
        import shlex
        for ref in ("main", "feat/it's-ok", "x;touch pwned", "a$(id)b", "q&r|s", "sp (1)"):
            s = state([pr(64, base=ref, head="a" * 40, base_sha="b" * 40)])
            s["agents"] = [{"name": "gemini"}]
            card = [c for c in nt.build_cards(s, 0) if c["inputs"].get("kind") == "needs_review"][0]
            step = [x for x in card["recipe"] if x.startswith("git merge-tree")][0]
            command = step.split("   # ")[0]
            self.assertEqual(shlex.split(command), ["git", "merge-tree", "--write-tree", "origin/" + ref, "a" * 40], ref)

    def test_known_implementers_are_printed_exactly_as_before(self):
        self.assertIn("implementers=['claude'], verdict", self.review_recipe(["claude"]))

    def test_a_co_implementer_is_never_assigned_to_review_the_pr(self):
        s = state([pr(9, seat="claude", implementers=["claude", "mimo"])])
        s["agents"] = [{"name": "mimo"}, {"name": "gemini"}, {"name": "claude"}]
        for card in nt.build_cards(s, 0):
            if card["inputs"].get("kind") == "needs_review":
                self.assertEqual(card["agent"], "gemini")

    # ---- rotation, variation and fresh evidence (owner direction 2026-10-05)

    def review_state(self, history=None, leases=None, **kw):
        s = state([pr(9, seat="claude", implementers=["claude"], head="c" * 40, base="main",
                      base_sha="b" * 40, size="+10/-2 in 1 file(s)", **kw)])
        s["agents"] = [{"name": "claude"}, {"name": "mimo"}, {"name": "gemini"}]
        if history:
            s["history"] = history
        if leases:
            s["leases"] = leases
        return s

    def review_card(self, s, tick=0):
        cards = [c for c in nt.build_cards(s, tick) if c["inputs"].get("kind") == "needs_review"]
        self.assertEqual(len(cards), 1)
        return cards[0]

    def test_the_framing_changes_every_time_the_same_item_is_handed_out(self):
        key = "needs_review:9:cccccccc"
        seen = []
        for handed in range(5):
            card = self.review_card(self.review_state(history={"item_attempts": {key: handed}}))
            seen.append((card["lens"], card["attempt"]))
        self.assertEqual([lens for lens, _ in seen], ["direct", "skeptic", "minimal", "handoff", "direct"])
        self.assertEqual([n for _, n in seen], [1, 2, 3, 4, 5])

    def test_a_held_lease_keeps_its_framing_instead_of_advancing(self):
        key = "needs_review:9:cccccccc"
        first = self.review_card(self.review_state(history={"item_attempts": {key: 0}}))
        holder = first["agent"]
        held = self.review_card(self.review_state(
            history={"item_attempts": {key: 1}}, leases={key: {"agent": holder, "expires": 6}}))
        self.assertEqual((held["agent"], held["lens"]), (holder, first["lens"]))

    def test_an_agent_that_let_the_lease_expire_is_not_handed_the_same_item_again(self):
        key = "needs_review:9:cccccccc"
        first = self.review_card(self.review_state())
        burned = first["agent"]
        again = self.review_card(self.review_state(history={"expired": {f"{burned}|{key}": 1}}))
        self.assertNotEqual(again["agent"], burned)

    def test_the_only_eligible_agent_still_gets_it_when_nobody_else_can(self):
        key = "needs_review:9:cccccccc"
        s = self.review_state(history={"expired": {f"mimo|{key}": 1}})
        s["agents"] = [{"name": "claude"}, {"name": "mimo"}]
        self.assertEqual(self.review_card(s)["agent"], "mimo")

    def test_agents_that_finish_work_are_served_first(self):
        # One free item, two eligible agents listed with the unreliable one first: the one that finishes work wins.
        s = state([pr(1, seat="claude", implementers=["claude"], head="1" * 40)])
        s["agents"] = [{"name": "mimo"}, {"name": "gemini"}]
        s["history"] = {"agents": {"gemini": {"done": 4, "expired": 0}, "mimo": {"done": 0, "expired": 3}}}
        cards = {c["agent"]: c for c in nt.build_cards(s, 0)}
        self.assertEqual(cards["gemini"]["inputs"]["kind"], "needs_review")
        self.assertEqual(cards["mimo"]["inputs"]["kind"], "standby")
        s["history"] = {}
        cards = {c["agent"]: c for c in nt.build_cards(s, 0)}
        self.assertEqual(cards["mimo"]["inputs"]["kind"], "needs_review")

    def test_the_card_carries_exact_shas_and_commands_but_never_a_pasteable_verdict(self):
        card = self.review_card(self.review_state())
        recipe = "\n".join(card["recipe"])
        self.assertIn("c" * 40, recipe)
        self.assertIn("b" * 40, recipe)
        self.assertIn("git merge-tree --write-tree origin/main", recipe)
        self.assertIn("+10/-2 in 1 file(s)", card["state"])
        self.assertNotIn("```", nt.render([card]))
        self.assertNotIn('"verdict": "accept"', nt.render([card]))

    def test_the_card_says_what_changed_since_this_agent_last_saw_the_pr(self):
        base = self.review_card(self.review_state())
        agent = base["agent"]
        moved = self.review_card(self.review_state(history={"last_head": {f"{agent}|9": "aaaaaaaa"}}))
        self.assertIn("head moved aaaaaaaa -> cccccccc", moved["new_since_last"])
        same = self.review_card(self.review_state(history={"last_head": {f"{agent}|9": "cccccccc"}}))
        self.assertIn("same head", same["new_since_last"])
        self.assertIn("first time", base["new_since_last"])

    def test_the_brief_is_forward_formatted(self):
        text = nt.render([self.review_card(self.review_state())])
        for label in ("TASK:", "STATE NOW:", "NEW SINCE YOUR LAST CARD:", "APPROACH:", "STEP 1:", "DONE WHEN:", "IF BLOCKED:"):
            self.assertIn(label, text)

    # ---- rejection feedback

    def rejected_state(self, covered=False, seat="gemini"):
        s = state([pr(9, seat="claude", implementers=["claude"], head="c" * 40, base_sha="b" * 40, covered=covered,
                      rejected_verdicts=[{"comment_id": 4242, "seat": seat, "reason": "the attestation block is not valid JSON"}])])
        s["agents"] = [{"name": "claude"}, {"name": "mimo"}, {"name": "gemini"}]
        return s

    def test_a_rejected_verdict_goes_back_to_its_author_with_the_reason(self):
        cards = {c["agent"]: c for c in nt.build_cards(self.rejected_state(), 0)}
        card = cards["gemini"]
        self.assertEqual(card["inputs"]["kind"], "repair_verdict")
        self.assertEqual(card["rejected"], [{"comment_id": 4242, "reason": "the attestation block is not valid JSON"}])
        text = nt.render([card])
        self.assertIn("REJECTED: comment 4242: the attestation block is not valid JSON", text)
        self.assertIn("c" * 40, text)
        self.assertNotIn("```", text)
        for other in ("claude", "mimo"):
            self.assertNotEqual(cards[other]["inputs"]["kind"], "repair_verdict")

    def test_nothing_is_asked_of_the_author_once_the_pr_is_covered(self):
        kinds = {c["inputs"]["kind"] for c in nt.build_cards(self.rejected_state(covered=True), 0)}
        self.assertNotIn("repair_verdict", kinds)

    def test_an_implementers_rejected_verdict_is_not_worth_repairing(self):
        s = self.rejected_state(seat="mimo")
        s["prs"][0]["implementers"] = ["claude", "mimo"]
        for card in nt.build_cards(s, 0):
            self.assertNotEqual(card["inputs"]["kind"], "repair_verdict")

    def test_an_unattributed_rejection_becomes_a_note_on_the_review_card(self):
        card = next(c for c in nt.build_cards(self.rejected_state(seat=""), 0) if c["inputs"]["kind"] == "needs_review")
        self.assertEqual(card["rejected_note"][0]["comment_id"], 4242)
        self.assertIn("if it is yours, repost it", nt.render([card]))

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
