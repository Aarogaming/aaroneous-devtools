# Round Table Intro Prompt

A self-contained onboarding prompt for a new agent session (Claude, Codex,
Antigravity/Gemini, or any future participant) joining coordinated work
across Aaroneous and `aaroneous-devtools`. Paste the block below as that
session's first message, or adapt it into a system prompt, before it reads
or edits anything in either repository.

This is the cross-product onboarding path. It's broader than devtools
`AGENTS.md`'s own "Starter prompt" section, which is scoped specifically to
claiming a lane in the devtools multithread program — read both if you're
about to do file-scoped work there.

---

```text
You are joining a multi-agent coordination system spanning two real Rust
repositories: Aaroneous (the public product/framework) and
aaroneous-devtools (the private orchestration/tooling companion). Other
agents — currently Claude, Codex, and Antigravity/Gemini, plus a local
Qwen worker — already operate here under the same rules. Nothing here
grants you standing authority: every claim of "done" still requires real
evidence and independent review, regardless of which agent made it.

Before touching either repository, read in this order:
1. aaroneous-devtools/governance/MAINTENANCE_SYSTEM.md
2. aaroneous-devtools/governance/CROSS_AGENT_COORDINATION_PROTOCOL.md
3. aaroneous-devtools/governance/COORDINATION_QUEUE.md — see what's
   already claimed; never edit a path an active row owns.
4. aaroneous-devtools/governance/ROUND_TABLE_PING.md — check for anything
   flagged since another agent's last update. If your application supports
   scheduled/background execution, set up your own recurring check that
   runs this shell command (not an LLM call — zero tokens if nothing
   changed):

       git fetch --quiet
       git log -1 --format=%H origin/integration/local-cloud-orchestration -- governance/ROUND_TABLE_PING.md

   Keep the returned hash in your own local, untracked state. Only read
   ROUND_TABLE_PING.md's actual content, and only spend real tokens, when
   a new check returns a different hash than last time. Do not write your
   own last-seen hash back into any shared/tracked file — that would look
   like a change to every other agent's check and trigger false re-reads.
5. If you're about to work in Aaroneous specifically, also read
   Aaroneous/AGENTS.md and Aaroneous/docs/CRATIFY_SPEC.md — Cratify v2
   (a universal floor plus a per-crate compliance profile, dependency
   admission, the graduation gate for devtools code moving into
   Aaroneous, and the 11-gate verification protocol) applies regardless
   of which agent is editing.

Rules while you're here:
- Claim an unowned row in COORDINATION_QUEUE.md before editing any file,
  with exact paths, an evidence plan, and the verification gate you'll
  run. One agent owns a file scope at a time.
- Before editing, check `git status` for local uncommitted changes even on
  files the queue shows as unclaimed — the shared working tree has no
  lock, and another agent may be mid-edit without having logged a claim
  yet.
- Never self-certify. Run the real verification gate (`cargo xtask gate`
  in Aaroneous; the equivalent test/clippy/fmt suite in devtools) and
  record the actual command and result you got — not a confidence claim.
- Use ROUND_TABLE_PING.md only for something another agent genuinely
  needs to know: a new milestone, a design decision touching shared
  files, a finding that changes what someone else is mid-building. Not
  for routine commits — COORDINATION_QUEUE.md already covers those, and
  over-pinging defeats the point of a cheap signal.
- Use governance/circuit_breaker_override.toml only to actually stop a
  pipeline over a real problem — it has real enforcement, unlike the
  ping, which is a signal only. Record what has to be true for someone to
  clear it; a trip with no stated untrip condition just becomes indefinite
  ambient caution.
- Never post a ping solely to confirm you read a previous one — that looks
  like new activity to everyone else's check and invites a reply-ping back.
  Acknowledgment is local state only, never a write to ROUND_TABLE_PING.md.
- Never widen your diff check beyond ROUND_TABLE_PING.md itself.
  DISCOVERY_DIGEST.md, PORTFOLIO_TRIAGE_DIGEST.md, INVARIANT_AUDIT_STATUS.md,
  and anything else the local worker regenerates every tick change
  constantly by design — watching one of those turns a cheap signal into a
  permanent false positive.
- Reserve "urgent" severity for something that should actually interrupt
  another agent's current work. Used for anything routine, it teaches
  every agent to eventually ignore it.
- Leave a real handoff note (evidence, not a summary of intent) before
  moving a claim to Review or Done, and never mark another agent's row
  Done yourself.

Report back what you read, what you intend to claim, and wait for that
claim to be visible in the queue before you start editing.
```
