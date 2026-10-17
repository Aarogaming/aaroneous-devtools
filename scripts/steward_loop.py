#!/usr/bin/env python3
"""Run steward ticks on an interval until told to stop (lane C99). Nothing is installed.

``steward_run.py`` does one tick. Something has to call it again; until now that was a heartbeat prompt typed into a
chat. This is the caller: a foreground loop you start yourself, in a terminal or under whatever supervisor you already
use. It installs no service, scheduled task or hook, and it ends on its own.

  steward_loop.py --interval-seconds 600 --stop-file steward.stop --lock-file steward.loop.lock -- \\
      --repo OWNER/NAME --repo-dir CLONE --identities governance/identities.json --log events.jsonl --outbox outbox

Everything after ``--`` is passed to ``steward_run.py`` unchanged. The loop:
  * waits ``--interval-seconds`` between ticks, measured from the end of the previous tick, so a slow tick never
    stacks a second one behind it;
  * stops when ``--stop-file`` exists (create it to end the loop at the next check, delete it before restarting), after
    ``--max-ticks`` ticks, or after ``--max-failures`` consecutive failures;
  * backs off exponentially after a failed tick (a nonzero exit or a crash), capped at ``--max-backoff-seconds``, and
    resets on success;
  * holds ``--lock-file`` (created exclusively) so two loops never run against one log; a lock left by a crashed loop
    is reported, never silently taken over;
  * prints one line per tick: number, exit code, seconds taken, and the next wait.

The decision logic (`plan_next`) is pure; the clock, the sleep and the tick are injected, so the tests never wait.
"""

import argparse
import os
import subprocess
import sys
import time

DEFAULT_INTERVAL = 600
DEFAULT_MAX_BACKOFF = 3600
DEFAULT_MAX_FAILURES = 5


def plan_next(interval, failures, max_backoff):
    """Seconds to wait before the next tick: the interval after a success, doubled per consecutive failure after a
    failure, never more than ``max_backoff`` (and never less than the interval)."""
    if failures <= 0:
        return interval
    return min(max(interval, interval * (2 ** failures)), max(max_backoff, interval))


def should_stop(stop_file_exists, ticks_done, max_ticks, failures, max_failures):
    """The reason to stop, or None. Checked before every tick and before every sleep."""
    if stop_file_exists:
        return "stop file present"
    if max_ticks is not None and ticks_done >= max_ticks:
        return f"reached --max-ticks {max_ticks}"
    if max_failures is not None and failures >= max_failures:
        return f"{failures} consecutive failures"
    return None


def run_loop(tick, sleep, clock, stop_requested, interval, max_backoff, max_ticks, max_failures, out):
    """Drive the loop. ``tick()`` returns the exit code of one steward tick (an exception counts as a failure),
    ``sleep(seconds)`` waits, ``clock()`` is monotonic seconds, ``stop_requested()`` says whether the stop file exists.
    Returns (ticks_done, reason)."""
    ticks_done, failures = 0, 0
    while True:
        reason = should_stop(stop_requested(), ticks_done, max_ticks, failures, max_failures)
        if reason:
            return ticks_done, reason
        started = clock()
        try:
            code = tick()
        except Exception as error:  # a crashed tick is a failed tick, not the end of the loop
            code = f"crash: {error!r}"
        ticks_done += 1
        failures = 0 if code == 0 else failures + 1
        wait = plan_next(interval, failures, max_backoff)
        out(f"tick {ticks_done} exit {code} took {clock() - started:.1f}s next in {wait}s")
        reason = should_stop(stop_requested(), ticks_done, max_ticks, failures, max_failures)
        if reason:
            return ticks_done, reason
        sleep(wait)


def acquire_lock(path):
    """Create the lock exclusively. Returns None on success, or a message when another loop's lock exists."""
    try:
        handle = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
    except FileExistsError:
        return (f"lock {path} exists: another loop is running, or one crashed and left it; "
                "confirm no loop is running, then delete the file")
    with os.fdopen(handle, "w", encoding="utf-8") as writer:
        writer.write(str(os.getpid()))
    return None


def main(argv):
    if "--" not in argv:
        print("usage: steward_loop.py [options] -- <steward_run.py arguments>", file=sys.stderr)
        return 2
    split = argv.index("--")
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--interval-seconds", type=int, default=DEFAULT_INTERVAL)
    parser.add_argument("--max-backoff-seconds", type=int, default=DEFAULT_MAX_BACKOFF)
    parser.add_argument("--max-ticks", type=int, help="stop after this many ticks (default: until stopped)")
    parser.add_argument("--max-failures", type=int, default=DEFAULT_MAX_FAILURES)
    parser.add_argument("--stop-file", required=True, help="the loop ends when this file exists")
    parser.add_argument("--lock-file", required=True, help="created exclusively; two loops never share a log")
    args = parser.parse_args(argv[:split])
    steward_args = argv[split + 1:]
    if args.interval_seconds < 1:
        parser.error("--interval-seconds must be at least 1")
    message = acquire_lock(args.lock_file)
    if message:
        print(message, file=sys.stderr)
        return 3
    script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "steward_run.py")
    try:
        ticks, reason = run_loop(
            tick=lambda: subprocess.run([sys.executable, script, *steward_args]).returncode,
            sleep=time.sleep, clock=time.monotonic, stop_requested=lambda: os.path.exists(args.stop_file),
            interval=args.interval_seconds, max_backoff=args.max_backoff_seconds, max_ticks=args.max_ticks,
            max_failures=args.max_failures, out=lambda line: print(line, flush=True))
    finally:
        if os.path.exists(args.lock_file):
            os.remove(args.lock_file)
    print(f"steward loop ended after {ticks} ticks: {reason}", flush=True)
    return 0 if reason != f"{args.max_failures} consecutive failures" else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
