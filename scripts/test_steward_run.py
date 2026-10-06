import json
import os
import sys
import time
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


def read_text(path):
    with open(path, encoding="utf-8") as handle:
        return handle.read()


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


class Tip(unittest.TestCase):
    def log(self):
        path = tmp_log()
        for i in range(3):
            sr.append_event(path, "x", {"i": i}, NOW)
        return path

    def cleanup(self, path):
        for p in (path, sr.tip_path(path)):
            if os.path.exists(p):
                os.unlink(p)

    def test_an_untouched_log_passes_the_tip_check(self):
        path = self.log()
        self.assertEqual(sr.verify_tip(path), (True, ""))
        self.cleanup(path)

    def test_deleting_the_last_event_is_caught_by_the_tip_sidecar(self):
        path = self.log()
        lines = read_lines(path)
        write_lines(path, lines[:-1])
        self.assertEqual(sr.verify_chain(path), (True, ""))  # the chain alone cannot see this
        ok, problem = sr.verify_tip(path)
        self.assertFalse(ok)
        self.assertIn("sidecar", problem)
        self.cleanup(path)

    def test_editing_the_last_event_is_caught_by_the_tip_sidecar(self):
        path = self.log()
        lines = read_lines(path)
        lines[-1] = lines[-1].replace('"i": 2', '"i": 9').replace('"i":2', '"i":9')
        write_lines(path, lines)
        self.assertFalse(sr.verify_tip(path)[0])
        self.cleanup(path)

    def test_an_external_anchor_catches_a_rewritten_sidecar_too(self):
        path = self.log()
        anchor = sr.current_tip(path)[1]
        lines = read_lines(path)
        write_lines(path, lines[:-1])
        count, tip = sr.current_tip(path)
        with open(sr.tip_path(path), "w", encoding="utf-8") as handle:
            json.dump({"count": count, "tip": tip}, handle)  # attacker rewrites the sidecar to match
        self.assertTrue(sr.verify_tip(path)[0])
        ok, problem = sr.verify_tip(path, expected=anchor)
        self.assertFalse(ok)
        self.assertIn("externally anchored", problem)
        self.cleanup(path)


class HealthyIdle(unittest.TestCase):
    def test_healthy_idle_and_standby_cards_do_not_count_toward_stalls(self):
        path = tmp_log()
        results = []
        for tick in range(1, 6):
            results.append(sr.run_tick(state([]), snap({1: []}), path, tick, NOW, stall_after=3))
        self.assertTrue(all(not r["escalations"] for r in results))
        events = sr.read_events(path)
        for e in [e for e in events if e["type"] == "cards"]:
            if e["data"]["tick"] % nt.AUDIT_EVERY != 0:
                self.assertEqual(e["data"]["agents"], [])
        os.unlink(path)
        os.unlink(sr.tip_path(path))


def _append_many(path, count, start_event):
    start_event.wait()
    for i in range(count):
        with sr.exclusive(path):
            sr.append_event(path, "stall", {"tick": i, "pid": os.getpid()}, NOW)


class Locking(unittest.TestCase):
    def test_overlapping_writers_cannot_fork_the_chain(self):
        import multiprocessing
        path = tmp_log()
        start = multiprocessing.Event()
        workers = [multiprocessing.Process(target=_append_many, args=(path, 20, start)) for _ in range(4)]
        for worker in workers:
            worker.start()
        start.set()
        for worker in workers:
            worker.join()
        events = sr.read_events(path)
        self.assertEqual(len(events), 80)
        self.assertEqual([e["seq"] for e in events], list(range(1, 81)))
        self.assertEqual(sr.verify_chain(path), (True, ""))
        for leftover in (path, sr.tip_path(path)):
            os.unlink(leftover)

    def test_a_held_lock_stops_a_second_run_with_a_message_that_names_the_lock(self):
        path = tmp_log()
        with sr.exclusive(path):
            with self.assertRaises(SystemExit) as caught:
                with sr.exclusive(path, wait=0.2):
                    pass
        self.assertIn(path + ".lock", str(caught.exception))

    def test_a_live_holder_is_not_displaced_by_age_or_by_a_clock_jump(self):
        path = tmp_log()
        with sr.exclusive(path):
            for jump in (601, 10_000_000):
                with self.assertRaises(SystemExit):
                    with sr.exclusive(path, wait=0.2, clock=lambda jump=jump: time.time() + jump):
                        self.fail("a second owner entered while the first still held the lock")

    def test_one_owner_leaving_does_not_release_another_owners_lock(self):
        path = tmp_log()
        first = sr.exclusive(path)
        first.__enter__()
        with self.assertRaises(SystemExit):
            with sr.exclusive(path, wait=0.2):
                pass
        first.__exit__(None, None, None)
        with sr.exclusive(path, wait=1):
            with self.assertRaises(SystemExit):
                with sr.exclusive(path, wait=0.2):
                    pass

    def test_the_lock_is_released_when_the_body_raises(self):
        path = tmp_log()
        with self.assertRaises(RuntimeError):
            with sr.exclusive(path):
                raise RuntimeError("boom")
        with sr.exclusive(path, wait=1):
            pass

    def test_a_crashed_holder_releases_the_lock_without_any_cleanup(self):
        import subprocess
        path = tmp_log()
        code = ("import sys, time; sys.path.insert(0, %r); import steward_run as sr\n"
                "ctx = sr.exclusive(%r); ctx.__enter__(); print('locked', flush=True); time.sleep(120)\n"
                % (os.path.dirname(os.path.abspath(__file__)), path))
        child = subprocess.Popen([sys.executable, "-c", code], stdout=subprocess.PIPE, text=True)
        try:
            self.assertEqual(child.stdout.readline().strip(), "locked")
            with self.assertRaises(SystemExit):
                with sr.exclusive(path, wait=0.3):
                    pass
            child.kill()
            child.wait(timeout=10)
            with sr.exclusive(path, wait=10):
                pass
        finally:
            if child.poll() is None:
                child.kill()
            child.stdout.close()

    def test_a_whole_tick_waits_for_the_lock_and_numbers_itself_after_the_holder(self):
        path = tmp_log()
        with sr.exclusive(path):
            with self.assertRaises(SystemExit):
                sr.tick_under_lock(path, state([pr(1)]), snap({1: []}), None, NOW, 3, wait=0.2)
        sr.tick_under_lock(path, state([pr(1)]), snap({1: []}), None, NOW, 3)
        self.assertEqual(sr.next_tick(sr.read_events(path)), 2)
        sr.tick_under_lock(path, state([pr(1)]), snap({1: []}), None, NOW, 3)
        self.assertEqual(sr.next_tick(sr.read_events(path)), 3)
        self.assertEqual(sr.verify_chain(path), (True, ""))
        os.unlink(path)
        os.unlink(sr.tip_path(path))


class CompletionAtANewerHead(unittest.TestCase):
    """A thread fix pushes a new head, and a card key ends in the head. The receipt for the new head must close the
    old card as done; before this it was logged as a release, so the seat that actually fixed things was credited
    with nothing in the history that orders seats by who finishes work."""

    OLD, NEW = "a" * 40, "b" * 40

    def lease_events(self, path):
        return [e for e in sr.read_events(path) if e["type"] in ("lease_done", "lease_released")]

    def run_two_ticks(self, done_at_new_head):
        path = tmp_log()
        first = state([pr(1, head=self.OLD, threads_unresolved=1, seat="claude")])
        sr.run_tick(first, snap({1: []}), path, 1, NOW, stall_after=99)
        second = state([pr(1, head=self.NEW, threads_unresolved=1, seat="claude")])
        second["done"] = [f"fix_threads:1:{self.NEW[:8]}"] if done_at_new_head else []
        sr.run_tick(second, snap({1: []}), path, 2, NOW, stall_after=99)
        return path

    def test_a_receipt_for_the_fix_at_the_new_head_closes_the_old_card_as_done(self):
        path = self.run_two_ticks(True)
        events = self.lease_events(path)
        self.assertEqual([(e["type"], e["data"]["key"], e["data"]["agent"]) for e in events],
                         [("lease_done", f"fix_threads:1:{self.OLD[:8]}", "claude")])
        self.assertTrue(events[0]["data"]["at_newer_head"])
        self.assertEqual(sr.history_from_events(sr.read_events(path))["agents"]["claude"]["done"], 1)
        os.unlink(path)

    def test_without_that_receipt_the_old_card_is_still_just_released(self):
        path = self.run_two_ticks(False)
        events = self.lease_events(path)
        self.assertEqual([e["type"] for e in events], ["lease_released"])
        self.assertEqual(sr.history_from_events(sr.read_events(path))["agents"].get("claude", {}).get("done", 0), 0)
        os.unlink(path)


class Ingest(unittest.TestCase):
    def test_seat_is_read_from_the_text(self):
        self.assertEqual(sr.seat_from_comment("REVIEW PACKET\nSeat / agent: Mimo (x)"), "mimo")
        self.assertEqual(sr.seat_from_comment("Codex independent review at abc"), "codex")
        self.assertEqual(sr.seat_from_comment("### Review\n- **Reviewer**: Gemini / Antigravity"), "gemini")
        self.assertEqual(sr.seat_from_comment("random words"), "")

    def test_a_bot_status_comment_that_mentions_a_seat_on_a_later_line_is_not_that_seats_artifact(self):
        bot = ("<!-- codex-pull-request-review-summary -->\n\n## Codex Review Summary\n\n| Review | Status |\n| --- | --- |\n"
               "| Code Review | Running |\n\n<details>\n\nReviews are triggered when you open a pull request.\n\n"
               "Codex reacts with an eyes emoji while any review is running, and comments if it has suggestions.\n\n</details>\n")
        self.assertEqual(sr.seat_from_comment(bot), "")

    def test_the_gemini_packet_header_reviewer_slash_seat_is_attributed(self):
        packet = ("### Independent Review Packet: Aarogaming/aaroneous-devtools#63\n\n"
                  "**Reviewer / Seat**: Antigravity (`gemini-3.8-flash`)\n**Head Reviewed**: `240c9e1d`\n")
        self.assertEqual(sr.seat_from_comment(packet), "gemini")

    def test_a_bot_comment_is_nobodys_artifact_and_not_counted_as_unattributed(self):
        path = tmp_log()
        bot = {"id": 77, "user": {"login": "chatgpt-codex-connector[bot]", "type": "Bot"},
               "body": "## Summary\n\nCodex reacts with an eyes emoji while any review is running.\n"}
        result = sr.run_tick(state([pr(1)]), snap({1: [bot]}), path, 1, NOW)
        self.assertEqual(result["ingest"]["per_seat"], {})
        self.assertEqual(result["ingest"]["unattributed"], 0)
        self.assertEqual(result["ingest"]["bots"], 1)
        again = sr.run_tick(state([pr(1)]), snap({1: [bot]}), path, 2, NOW)
        self.assertEqual(again["ingest"]["bots"], 0)
        os.unlink(path)

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


def three_open():
    return state([pr(1, seat="other"), pr(2, seat="other"), pr(3, seat="other")])


class Stalls(unittest.TestCase):
    def ticks(self, path, produced_by_tick, total):
        results = []
        for tick in range(1, total + 1):
            body = produced_by_tick.get(tick)
            comments = {1: [{"id": 100 + tick, "body": body}]} if body else {1: []}
            results.append(sr.run_tick(three_open(), snap(comments), path, tick, NOW, stall_after=3))
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
    def test_bot_status_comments_in_every_tick_do_not_switch_off_stall_detection(self):
        path = tmp_log()
        results = []
        for tick in range(1, 4):
            bot = {"id": 400 + tick, "user": {"login": "chatgpt-codex-connector[bot]", "type": "Bot"},
                   "body": "## Review summary\n\nCodex reacts with an eyes emoji while any review is running.\n"}
            results.append(sr.run_tick(three_open(), snap({1: [bot]}), path, tick, NOW, stall_after=3))
        self.assertFalse(results[-1]["attribution_weak"])
        self.assertEqual({e["agent"] for e in results[-1]["escalations"]}, {"claude", "mimo", "codex"})
        os.unlink(path)

    def test_a_bot_login_is_recognised_by_type_or_by_the_bot_suffix(self):
        self.assertTrue(sr.is_bot({"user": {"login": "x", "type": "Bot"}}))
        self.assertTrue(sr.is_bot({"user": {"login": "copilot-pull-request-reviewer[bot]"}}))
        self.assertFalse(sr.is_bot({"user": {"login": "Aarogaming", "type": "User"}}))
        self.assertFalse(sr.is_bot({"body": "no author recorded"}))

    def test_weak_attribution_suppresses_stall_escalation(self):
        path = tmp_log()
        results = []
        for tick in range(1, 4):
            comments = {1: [{"id": 200 + tick * 10 + n, "body": "plain note with no seat"} for n in range(3)]}
            results.append(sr.run_tick(three_open(), snap(comments), path, tick, NOW, stall_after=3))
        self.assertTrue(results[-1]["attribution_weak"])
        self.assertEqual(results[-1]["escalations"], [])
        os.unlink(path)

    def test_good_attribution_still_escalates(self):
        path = tmp_log()
        results = []
        for tick in range(1, 4):
            comments = {1: [{"id": 300 + tick, "body": "Gemini independent review"}]}
            results.append(sr.run_tick(three_open(), snap(comments), path, tick, NOW, stall_after=3))
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

    def test_a_lease_the_holder_is_no_longer_eligible_for_is_released_not_left_to_expire(self):
        path = tmp_log()
        owned = state([pr(1, threads_unresolved=2, seat="mimo")])
        run1 = sr.run_tick(owned, snap({1: []}), path, 1, NOW)
        holder = next(c["agent"] for c in run1["cards"] if c["inputs"]["kind"] == "fix_threads")
        self.assertEqual(holder, "mimo")
        # Ownership is corrected: the PR belongs to codex now, so mimo's lease is invalid.
        fixed = state([pr(1, threads_unresolved=2, seat="codex")])
        run2 = sr.run_tick(fixed, snap({1: []}), path, 2, NOW)
        owner = next(c["agent"] for c in run2["cards"] if c["inputs"]["kind"] == "fix_threads")
        self.assertEqual(owner, "codex")
        events = sr.read_events(path)
        self.assertTrue(any(e["type"] == "lease_released" and e["data"]["agent"] == "mimo" for e in events))
        run3 = sr.run_tick(fixed, snap({1: []}), path, 3, NOW)
        self.assertEqual(run3["leases"]["expired"], [])
        os.unlink(path)

    def test_the_log_with_leases_still_verifies(self):
        path = tmp_log()
        for t in range(1, 9):
            sr.run_tick(self.two_prs(), snap({1: [], 2: []}), path, t, NOW)
        self.assertEqual(sr.verify_chain(path), (True, ""))
        os.unlink(path)


class AttributionLabels(unittest.TestCase):
    def test_the_antigravity_label_is_the_gemini_seat(self):
        self.assertEqual(sr.seat_from_comment("**Seat / Agent**: Antigravity (Gemini 2.5 Pro)"), "gemini")


class Delivery(unittest.TestCase):
    def test_a_persistent_log_numbers_its_own_ticks(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = os.path.join(tmp, "events.jsonl")
            self.assertEqual(sr.next_tick(sr.read_events(log)), 1)
            sr.append_event(log, "cards", {"tick": 1, "agents": [], "tasks": []}, "2026-10-05T00:00:00Z")
            sr.append_event(log, "cards", {"tick": 2, "agents": [], "tasks": []}, "2026-10-05T00:10:00Z")
            self.assertEqual(sr.next_tick(sr.read_events(log)), 3)

    def test_each_seat_gets_only_its_own_card_and_no_pasteable_attestation(self):
        cards = [
            {"agent": "gemini", "role": "reviewer", "tick": 4, "priority": 2, "task": "Review PR #51",
             "why_now": "x", "acceptance": "an attestation", "if_blocked": "say who",
             "recipe": ["git diff --check A...B"], "lens": "skeptic", "attempt": 2},
            {"agent": "codex", "role": "auditor", "tick": 4, "priority": 1, "task": "Fix threads on PR #47",
             "why_now": "x", "acceptance": "replies", "if_blocked": "say who"},
        ]
        result = {"cards": cards, "escalations": [{"agent": "codex", "task": "no output for 3 ticks", "acceptance": "post"}]}
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "outbox")
            paths = sr.write_outbox(out, result, "2026-10-05T01:00:00Z")
            self.assertEqual(sorted(os.path.basename(p) for p in paths), ["codex.md", "gemini.md"])
            gemini = read_text(os.path.join(out, "gemini.md"))
            codex = read_text(os.path.join(out, "codex.md"))
            self.assertIn("Review PR #51", gemini)
            self.assertIn("lens=skeptic attempt=2", gemini)
            self.assertNotIn("Fix threads", gemini)
            self.assertIn("ESCALATION", codex)
            self.assertNotIn("ESCALATION", gemini)
            for text in (gemini, codex):
                self.assertNotIn("```attestation", text)
            sr.write_outbox(out, {"cards": cards[:1], "escalations": []}, "2026-10-05T01:10:00Z")
            self.assertIn("01:10:00Z", read_text(os.path.join(out, "gemini.md")))

    def test_an_item_whose_lease_expires_this_tick_rotates_to_another_seat(self):
        agents = [{"name": "mimo"}, {"name": "gemini"}]

        def review_state():
            return {"now": NOW, "agents": agents, "prs": [pr(1, seat="claude", implementers=["claude"])],
                    "backlog": [], "done": []}

        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "events.jsonl")
            first = sr.run_tick(review_state(), snap({1: []}), path, 1, NOW)
            holder = next(c["agent"] for c in first["cards"] if c["inputs"]["kind"] == "needs_review")
            expiry_tick = next(c["lease"]["expires_tick"] for c in first["cards"] if c.get("lease"))
            later = sr.run_tick(review_state(), snap({1: []}), path, expiry_tick, NOW)
            self.assertEqual(later["leases"]["expired"], ["needs_review:1:" + "a" * 8])
            reviewer = next(c["agent"] for c in later["cards"] if c["inputs"]["kind"] == "needs_review")
            self.assertNotEqual(reviewer, holder)

    def test_history_replays_handouts_expiries_and_completions(self):
        events = [
            {"type": "lease_grant", "data": {"agent": "gemini", "key": "needs_review:51:aaaaaaaa", "tick": 1, "expires": 7}},
            {"type": "lease_expired", "data": {"agent": "gemini", "key": "needs_review:51:aaaaaaaa", "tick": 7}},
            {"type": "lease_grant", "data": {"agent": "mimo", "key": "needs_review:51:aaaaaaaa", "tick": 7, "expires": 13}},
            {"type": "lease_done", "data": {"agent": "mimo", "key": "needs_review:51:aaaaaaaa", "tick": 9}},
        ]
        h = sr.history_from_events(events)
        self.assertEqual(h["item_attempts"]["needs_review:51:aaaaaaaa"], 2)
        self.assertEqual(h["expired"]["gemini|needs_review:51:aaaaaaaa"], 1)
        self.assertEqual(h["agents"]["mimo"], {"done": 1, "expired": 0})
        self.assertEqual(h["last_head"]["gemini|51"], "aaaaaaaa")


if __name__ == "__main__":
    unittest.main()
