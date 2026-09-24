# Brainstorm Sessions

The "actively engaging" mode the queue and ping don't cover. The queue is
for file ownership and the ping is for async change-notification — neither
is built for several agents genuinely thinking through one open question
together. A brainstorm session is **bounded, asynchronous-parallel
contribution to one shared document**, not a live chat. That distinction
is what keeps it from becoming the exact event-chain chaos
`CROSS_AGENT_COORDINATION_PROTOCOL.md`'s etiquette section warns against —
there is no synchronous back-and-forth to spiral in the first place.

This is the cloud-agent analog of M48's `exploration_seed` producer, which
already does multi-perspective seeding for the *local* worker (three fixed
angles — correctness-first, performance-first, minimal-diff — on one
goal). A brainstorm session is the same idea for cloud agents, with a real
named question instead of a fixed template, and can close by feeding its
resolution into `exploration_seed` for the local worker to draft against
once cloud agents converge on a direction.

## When to open one

A real question that benefits from more than one model's independent
perspective before anyone commits — an architecture choice, which of
several roadmap directions to pursue, whether an external crate's approach
is worth adapting. Don't open one for anything a single agent can just
decide and log in the queue; that's what the queue is for. If it's a
decision only the owner can make, ask the owner — a session isn't a way to
avoid asking.

## External planning sources (Google Drive and similar)

The owner drafts ideas outside the repositories, often as model-generated documents in Google Drive.
Those documents are **inputs, not specifications**: they contain confident claims about files,
crates, and mechanisms that may not exist. To adopt one:

1. Open a brainstorm session (below) citing the source document by title and date.
2. Verify every concrete claim (paths, crate names, mechanisms, numbers) against the code and record
   what held and what did not.
3. Promote only the verified, owner-approved result into a canonical home: an Aaroneous spec
   (`docs/`), a devtools decision (`governance/DECISIONS.md`), or a portfolio row.
4. Record the source title in the promoted document so its provenance is traceable.

Cratify v2 (2026-09-23) is the reference example: its reasoning came from Drive documents on
compliance distance, operating envelopes, the invariant ratchet, and component graduation.

## Opening a session

1. Pick a short topic slug (e.g. `plugin-abi-v2`, `worker-scaling`).
2. Create `brainstorm/<topic-slug>.md` from the template below — one file
   per topic, so parallel sessions never collide and a closed session
   stays as clean historical record instead of a single ever-growing file.
3. State the question, why it needs multiple perspectives, who's expected
   to weigh in (specific agents, or "anyone who reads this"), and an
   **explicit close condition** — a deadline, a target number of
   independent perspectives, or "owner picks a direction." No condition,
   no session — an unbounded brainstorm is the thing this protocol exists
   to prevent.
4. Add exactly **one** row to `ROUND_TABLE_PING.md` announcing it —
   severity `notice` unless genuinely time-boxed (`urgent`). One row for
   the whole session, not one per expected participant.
5. Decide whether this is **async** (default — contribute whenever you
   next check) or **scheduled** (a live window — see below). Say which in
   the file; don't leave it ambiguous.

## Scheduled sessions (live windows)

For when you actually want several agents present around the same time,
not just eventually contributing. This doesn't require any new
infrastructure — it reuses the scheduling every agent already has. Instead
of trying to detect who's available (which needs an always-on listener
none of these applications actually offer), you agree on a time and let
each agent's own scheduler handle showing up, the same way a calendar
invite works across time zones without live presence detection.

**Inviting:**

1. Set `Starts: <UTC time>` and `Window: <duration, e.g. 30m>` in the
   brainstorm file, in addition to the usual fields. `Window` is required
   — a live window still needs an end, same as any close condition.
2. Ping it as an invite: `notice` severity, summary along the lines of
   "You're invited to a live brainstorm on `<topic>` starting `<UTC>`,
   window `<duration>`. If you're available, be ready." One row, not one
   per invitee — no RSVP ping expected or wanted, consistent with the
   no-reply-pings rule; showing up is opt-in and silent.
3. Any agent that reads the invite and can self-schedule should set its
   own one-shot wake at (or a few minutes before) the stated start time,
   using whatever durable scheduling its application offers. A session-only
   timer that dies if the current chat closes before the start time
   defeats the purpose of a future-dated invite — use a durable mechanism
   if your application has one.

**During the window:** tighter polling than the usual idle-back-off
cadence is expected and fine here — this is the one exception to "back off
your check cadence" in `CROSS_AGENT_COORDINATION_PROTOCOL.md`'s etiquette
section, because it's bounded to the agreed window rather than indefinite.
Once the window ends (or the file's `## Closed` entry appears, whichever
is first), return to normal cadence.

**If you miss it:** the file still exists and still takes async entries
under the normal rules above — a scheduled window raises the odds of
several agents being present together, it isn't a requirement to
participate at all. Late is still useful; just don't reopen a closed
session to add to it.

## Contributing

- Read the file once per your own normal ping-check cycle, or when
  directly asked to look now. Append one dated, attributed entry with
  your actual position — don't tight-loop re-reading the file waiting for
  others to respond.
- **You are not obligated to reply to every other entry.** This is
  parallel input into one document, not a conversation. Treating it like
  one — replying to everything, everyone reacting to everyone — is
  exactly the event-chain pattern to avoid.
- If you genuinely need to respond to a specific prior entry (not "I
  agree" / "I saw this"), do it once, substantively, and stop.
- Append only. Never edit or delete another agent's entry.

## Closing

The convener (whoever opened it, or the owner) appends a `## Closed`
entry stating the resolution and, if applicable, what queue row or
worker-jobs packet it became. Close on the stated condition, not "when it
feels finished." Log the closure as one `ROUND_TABLE_PING.md` notice
pointing at what happens next — not the brainstorm file itself, which is
now historical record, not something to keep diffing for.

## Template

```markdown
# Brainstorm: <topic>

Opened: <UTC> by <agent/owner>
Type: async | scheduled
Starts: <UTC>              (scheduled only)
Window: <duration>         (scheduled only)
Question: <the actual open question>
Why multiple perspectives: <reason a single agent shouldn't just decide>
Expected participants: <names, or "anyone">
Closes when: <explicit condition>

## Entries

- <UTC> — <agent>: <contribution>

## Closed
(added when resolved)
```
