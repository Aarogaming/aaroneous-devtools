# Gemini M63 Verify/Rebuild Prompt

Follow-up to `GEMINI_LOCAL_WORKER_RESTART_PROMPT.md`. The owner reported
the local worker rebuilt and relaunched (confirmed: PID changed from
123532 to 133684), but real evidence suggests the running binary doesn't
actually contain M63's backlog throttle yet. Asking Gemini to verify and,
if needed, rebuild again -- not executed here, same reasoning as the
original restart prompt: this is Gemini's live process to manage.

---

```text
Thanks for the rebuild/restart -- confirmed the PID changed (123532 ->
133684), so a restart genuinely happened. But there's a real discrepancy
worth checking before assuming it picked up everything: worker-jobs/
frontier-comfy-table.toml was written at 13:57 EDT, after the restart,
and it's an output of the frontier-discover expansion producer. M63
(merged into integration/local-cloud-orchestration at commit 9c259c5,
HEAD is currently 9ccdeef) added a backlog throttle that should have
skipped frontier-discover entirely at that point -- pending worker-jobs
was 366 against a throttle threshold of 192 (max_pending * 3). Its
existence suggests the binary you rebuilt from doesn't contain that code,
most likely because the rebuild started before M63 finished merging into
this checkout and only finished compiling afterward -- a timing issue,
not necessarily anything you did wrong.

Also worth noting: no new completed job has landed in archive/ since
13:10 EDT (as of this check, ~50 minutes with no throughput), separate
from the throttle question -- worth confirming the loop is actually
ticking and dispatching, not just running.

Please:

1. Confirm what commit your currently-running binary was actually built
   from, if you have that recorded (a build log, terminal history,
   whatever you have). If you don't know, that's fine -- just rebuild
   fresh per step 2 rather than guessing.
2. Pull/confirm this checkout is at integration/local-cloud-orchestration
   HEAD (9ccdeef or later), then rebuild fresh from that exact state.
3. Before restarting the service, run `foundation --help` and confirm
   outbox-list, outbox-show, outbox-verdict, outbox-promote, stage-draft,
   list-staged, and prune-staged are all present -- same check as the
   original restart prompt, worth repeating since last time's result was
   ambiguous.
4. Restart local-worker-serve.
5. Watch the next few ticks. With 366 pending jobs (well over the 192
   throttle threshold), you should see this exact message in its output
   on ticks that would otherwise refill expansion producers:
   "job_converter: pending worker-jobs backlog exceeds 3x max_pending --
   skipping this cycle's expansion producers (portfolio triage,
   capability composition, frontier discovery) until it drains"
   If you don't see that message within a few ticks despite the backlog
   still being well over threshold, the throttle still isn't active and
   something else is wrong -- worth flagging back rather than assuming
   it's fine.
6. Also confirm real dispatch is happening again -- a new file in
   archive/ with a timestamp after your restart, not just the process
   existing.

Report back: what commit it actually rebuilt from, whether step 3's
--help check passed, and whether you saw the throttle message and a new
archive/ entry after restarting.
```
