import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import steward_loop as sl  # noqa: E402


class Fake:
    """Injected tick, sleep and clock: the tests never wait and never start a process."""

    def __init__(self, codes, stop_after=None):
        self.codes, self.sleeps, self.lines, self.now, self.calls = list(codes), [], [], 0.0, 0
        self.stop_after = stop_after

    def tick(self):
        self.calls += 1
        self.now += 2.0
        code = self.codes.pop(0) if self.codes else 0
        if isinstance(code, Exception):
            raise code
        return code

    def sleep(self, seconds):
        self.sleeps.append(seconds)

    def clock(self):
        return self.now

    def stop(self):
        return self.stop_after is not None and self.calls >= self.stop_after

    def run(self, **kwargs):
        defaults = dict(interval=10, max_backoff=100, max_ticks=None, max_failures=5)
        defaults.update(kwargs)
        return sl.run_loop(self.tick, self.sleep, self.clock, self.stop, out=self.lines.append, **defaults)


class Pure(unittest.TestCase):
    def test_the_wait_is_the_interval_until_a_tick_fails_then_doubles_and_is_capped(self):
        self.assertEqual([sl.plan_next(10, f, 100) for f in (0, 1, 2, 3, 4, 5)], [10, 20, 40, 80, 100, 100])

    def test_a_cap_below_the_interval_never_shortens_the_wait(self):
        self.assertEqual(sl.plan_next(60, 3, 30), 60)

    def test_stop_reasons(self):
        self.assertEqual(sl.should_stop(True, 0, None, 0, 5), "stop file present")
        self.assertEqual(sl.should_stop(False, 3, 3, 0, 5), "reached --max-ticks 3")
        self.assertEqual(sl.should_stop(False, 1, None, 5, 5), "5 consecutive failures")
        self.assertIsNone(sl.should_stop(False, 1, 3, 4, 5))


class Loop(unittest.TestCase):
    def test_it_runs_max_ticks_then_stops_and_sleeps_between_ticks_only(self):
        fake = Fake([0, 0, 0])
        ticks, reason = fake.run(max_ticks=3)
        self.assertEqual((ticks, reason), (3, "reached --max-ticks 3"))
        self.assertEqual(fake.sleeps, [10, 10], "no sleep after the last tick")
        self.assertTrue(fake.lines[0].startswith("tick 1 exit 0 took 2.0s next in 10s"))

    def test_a_failed_tick_backs_off_and_a_success_resets(self):
        fake = Fake([1, 1, 0, 0])
        fake.run(max_ticks=4)
        self.assertEqual(fake.sleeps, [20, 40, 10])

    def test_a_crashing_tick_counts_as_a_failure_and_the_loop_survives(self):
        fake = Fake([RuntimeError("boom"), 0])
        ticks, reason = fake.run(max_ticks=2)
        self.assertEqual(ticks, 2)
        self.assertIn("crash: RuntimeError('boom')", fake.lines[0])

    def test_consecutive_failures_end_the_loop(self):
        fake = Fake([1] * 10)
        ticks, reason = fake.run(max_failures=3)
        self.assertEqual((ticks, reason), (3, "3 consecutive failures"))

    def test_a_success_between_failures_keeps_the_loop_alive(self):
        fake = Fake([1, 1, 0, 1, 1, 0])
        ticks, reason = fake.run(max_failures=3, max_ticks=6)
        self.assertEqual((ticks, reason), (6, "reached --max-ticks 6"))

    def test_the_stop_file_is_checked_before_each_tick_and_before_each_sleep(self):
        fake = Fake([0, 0, 0], stop_after=2)
        ticks, reason = fake.run()
        self.assertEqual((ticks, reason), (2, "stop file present"))
        self.assertEqual(fake.sleeps, [10], "it did not sleep after the tick that saw the stop file")

    def test_a_stop_file_present_at_the_start_runs_no_tick(self):
        fake = Fake([0], stop_after=0)
        self.assertEqual(fake.run(), (0, "stop file present"))
        self.assertEqual(fake.calls, 0)


class Lock(unittest.TestCase):
    def test_the_lock_is_exclusive_and_a_leftover_is_reported_not_taken_over(self):
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "loop.lock")
            self.assertIsNone(sl.acquire_lock(path))
            with open(path, encoding="utf-8") as handle:
                self.assertEqual(handle.read(), str(os.getpid()))
            message = sl.acquire_lock(path)
            self.assertIn("another loop is running", message)
            self.assertTrue(os.path.exists(path), "the existing lock must be left alone")


class Cli(unittest.TestCase):
    def test_arguments_for_the_steward_must_follow_a_double_dash(self):
        self.assertEqual(sl.main(["--stop-file", "s", "--lock-file", "l"]), 2)

    def test_a_present_lock_stops_the_loop_before_any_tick(self):
        with tempfile.TemporaryDirectory() as directory:
            lock = os.path.join(directory, "loop.lock")
            with open(lock, "w", encoding="utf-8") as handle:
                handle.write("123")
            code = sl.main(["--stop-file", os.path.join(directory, "stop"), "--lock-file", lock, "--", "--repo", "o/r"])
            self.assertEqual(code, 3)

    def test_it_runs_the_real_script_path_and_removes_its_lock(self):
        # A stop file that already exists ends the loop without starting a tick; the lock is released.
        with tempfile.TemporaryDirectory() as directory:
            stop, lock = os.path.join(directory, "stop"), os.path.join(directory, "loop.lock")
            open(stop, "w", encoding="utf-8").close()
            self.assertEqual(sl.main(["--stop-file", stop, "--lock-file", lock, "--", "--repo", "o/r"]), 0)
            self.assertFalse(os.path.exists(lock))


if __name__ == "__main__":
    unittest.main()
