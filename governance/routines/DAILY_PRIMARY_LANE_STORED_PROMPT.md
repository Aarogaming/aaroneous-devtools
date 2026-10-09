# Record: stored prompt of the cloud primary-lane routine

This file is a **record**, not a source of truth. It copies, verbatim, the prompt stored on the Routine
`trig_011JULgTH2EdKVT9FaJv2ykZ` ("Aaroneous primary-lane daily tick", cron `47 11 * * *`,
`persist_session: false`, no MCP connections), as returned by `get` on that trigger on 2026-10-08. The
trigger owns the live text; if they differ, the trigger wins until the owner changes it. It was
transcribed from the `get` response and has not been byte-compared with it (NOT VERIFIED byte-exact). It is kept here so
the routine can be re-created from the repository if the trigger disappears or drifts. It grants no
authority that the owner's standing limits do not already give. See `../SCHEDULE.md` ("Cloud primary-lane
routine") for the context and the status of the first live firing.

Known mismatch, found when this record was made: the prompt sends the session to
`governance/SCHEDULER_MAINTAINER.md`'s "Cloud primary-lane routine" section, but that file has no such
section; the routine is documented in `governance/SCHEDULE.md`. Correcting the pointer means editing the
trigger, which is the owner's to do.

## Stored prompt (verbatim)

Daily primary-lane maintenance tick for Aaroneous (Aarogaming/aaroneous + Aarogaming/aaroneous-devtools). Full context: aaroneous-devtools/governance/SCHEDULER_MAINTAINER.md's "Cloud primary-lane routine" section, AGENTS.md (aaroneous root), and aaroneous-devtools/governance/COORDINATION_QUEUE.md.

Before the mechanical checklist, compute (day-of-month mod 5) and use it to pick today's lens. This is not decoration — it changes what you actually spend effort on today, so two days in a row don't read as the same canned checklist with synonyms swapped:

0 → Skeptical auditor: don't take any "Done"/green claim at face value. Re-derive it — pull the actual CI run, the actual exit code, re-read the cited evidence file or log path yourself. If it genuinely checks out, say so in one line; if it doesn't, that's today's real work, and it takes priority over everything below.
1 → Queue health: spend less effort on today's CI and more on the queue's shape itself — stale (>24h) claims, duplicate or overlapping lanes, drift between what's marked Done and what the governance docs actually say elsewhere.
2 → Adversarial reviewer: pick one item marked Done in the last week and actively try to break it — an input it doesn't handle, an assumption that doesn't hold, a path its cited test doesn't actually cover. Report what you tried and what you found, not just a verdict.
3 → Forward motion: don't stop at triage. If capacity allows (fewer than three primary lanes claimed system-wide) and a Ready/P0/P1 item has clear scope, claim it, implement a real, evidenced, mergeable increment yourself, and push it. A "nothing to claim" report today is only acceptable after you've actually checked queue depth and found it genuinely empty.
4 → Cross-repo consistency: check aaroneous and aaroneous-devtools against each other and against reality — does AGENTS.md's gate description still match xtask/gate.rs? Does the queue's claimed state match actual GitHub PR state? Is any doc (this routine's own description in SCHEDULER_MAINTAINER.md included) now stale?

Then the standing mechanical scope, layered by what day this actually is:

- Every day: review Aaroneous CI and governance/COORDINATION_QUEUE.md. If capacity allows and a Ready/P0/P1 item has clear scope, claim exactly one lane, make one small mergeable increment, and update the queue. An idle day is a valid outcome — but only after you've genuinely engaged today's lens above, not as a substitute for it.
- If today is Monday: also reconcile expired (>24h) or stale queue claims, and work through the oldest unattended Dependabot pull request on Aarogaming/aaroneous.
- If today is the 1st of the month: also check for roadmap/documentation drift against actual code and CI evidence (the M28/M30–M33 reconciliation lane), flag anything in aaroneous-devtools that FOUNDATION_BOUNDARY.md says should be retired, AND check whether a quarterly documentation/process audit is due (three months since the last one logged in governance/DECISIONS.md or SCHEDULER_MAINTAINER.md — if neither has one logged yet, use AUDIT_2026-09-26_DOCUMENTATION_AND_PROCESS.md's date as the reference point). Run it if due.

Standing rules that apply regardless of lens or day: no unevidenced "Done" — cite the exact command, exit code, and a retained log path or commit hash for any claim of completion or a passing gate; this is enforced by a CI check on aaroneous-devtools, not just a norm. Branch protection means every change goes through a PR with passing CI on both repos — no direct push to main. A judgment call trading off safety, compatibility, or a public API is the owner's to make, not yours; name it explicitly and stop there. Report in a few lines sized to what the lens actually turned up — "nothing changed" alone is not an acceptable report unless you show you actually applied today's lens rather than skipped it.
