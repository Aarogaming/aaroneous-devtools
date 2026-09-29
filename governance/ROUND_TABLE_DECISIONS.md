# Round-table decisions

This is the bounded problem board for cross-agent coordination problems that
cannot be resolved from an existing queue claim alone: lane-ID collisions,
scope overlaps, ordering disputes, gate interpretation, and authority gaps.
It is not another backlog, implementation queue, or conversation transcript.

## Rules

1. Open one entry only when the problem affects an active claim, review, or
   safe next action.
2. Cite the affected lanes, commits, PRs, and evidence. Agents may add concise
   options and evidence-backed positions; they do not decide outside their
   existing authority.
3. The owner records the decision, or names an authorized decider. The entry
   then records the implementation/queue commit that applied it.
4. Newest open entry first. Resolved entries remain as history; do not delete
   or recycle identifiers.
5. A decision board entry never grants a file claim. Follow the coordination
   queue before editing any path.

## Entry template

```markdown
## D-YYYY-NN — short problem title

- Status: Open | Decided | Applied
- Affected: lane IDs, PRs, commits, exact paths
- Question: one decision that unblocks work
- Evidence: links, commands, CI result, or source facts
- Options: bounded alternatives and consequences
- Positions: named agent seat + concise evidence-based view
- Owner decision: pending | decision text, timestamp, owner
- Applied by: pending | commit/PR and verification
```

## Decisions

## D-2026-01 — C77 lane-ID collision

- Status: Applied
- Affected: devtools PR #19 (conformance wiring), PR #20 (Foundry transition),
  `governance/COORDINATION_QUEUE.md`
- Question: which lane owns C77?
- Evidence: the PRs have disjoint file scopes but both independently used C77.
- Options: renumber code lane; renumber architecture lane; abandon one lane.
- Positions: Claude and Gemini recommended retaining C77 for code and assigning
  the Foundry documentation lane a new identifier.
- Owner decision: 2026-09-29 — C77 remains conformance wiring; the Foundry
  transition is C78.
- Applied by: pending C78 queue amendment in PR #20.
