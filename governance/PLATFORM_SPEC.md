# Platform Spec v0 (PROPOSAL)

Status: **proposal for owner approval.** Nothing here changes behaviour, policy or the ledger until
accepted and implemented in later PRs. Source: owner/steward discussion of 2026-10-03.

## 1. Goal

Move operating load from agents to a scheduler, and make order of operations, evidence and gating
mechanically enforced instead of described in documents agents re-write. Aaroneous is the stable
product library. Devtools is the beta channel and orchestrator that uses and builds Aaroneous.

## 2. Channels and direction of dependency

- Devtools depends on Aaroneous. Aaroneous never depends on devtools (AGENTS.md section 4).
- **Stable** = Aaroneous. **Beta** = capabilities proven in devtools.
- Devtools runs a **pinned** stable Aaroneous. A newer Aaroneous is a **candidate**, run in shadow
  mode through the same gates. The pin moves only when the candidate passes (automated bump PR).
- Improvements graduate into Aaroneous under the AGENTS.md graduation path (classify, workspace
  trait with contract tests, inert or shadow landing). The origin copy in devtools is then deleted so
  one implementation exists.

### Promotion checklist (all machine-checked; no per-promotion owner sign-off)

1. Whole-diff independent accept at the head being promoted (section 3).
2. Contract tests behind a workspace-defined trait.
3. A shadow or inert period with recorded results.
4. Full `cargo xtask gate` green on the Aaroneous side.
5. The promotion PR names its rollback (revert commit and pin target). A rollback drill, run once,
   proves the pin can move back and devtools still runs.

Ratchet baselines, rollback and versioning replace manual sign-off. The only human gate that remains
is loosening a crate's compliance profile (AGENTS.md section 1).

## 3. Review coverage rule

An accept counts only if bound to a head and a scope. A PR is mergeable when accepted scopes, chained
from a whole-diff accept to the current head, cover all of `base...head`.

- Delta accepts compose only when chained from a whole-diff accept.
- Flattening yields a new head: it needs a new whole-diff accept.
- When the head moves, a bot reports the uncovered range and routes it to a reviewer.
- Enforced as a required status check `review-coverage` on `main`.

### Attestation record (schema draft)

| field | meaning |
|---|---|
| `pr`, `head` | PR number and full head SHA the record binds to |
| `scope` | `whole` or `delta` with `from` SHA (delta must chain to a `whole`) |
| `reviewer`, `implementer` | seat identities (independence is checked, honour-system today) |
| `commands` | list of `{command, exit_code}` actually run |
| `not_verified` | explicit list of things not checked |
| `verdict` | `accept` / `changes_requested` / `needs_revision` |

Records are append-only events (section 4). Existing REVIEW PACKET comments map onto this record.

## 4. Operations

- **Event log**: append-only, typed events (claim, lease, review, attestation, gate result). The
  markdown ledger becomes a rendered view, not the source of truth.
- **Steward service**: one scheduler replaces per-agent heartbeats (sync, route reviews, retarget,
  rebase, nudge). Agents are workers that pull a leased task and submit a result. Leases expire.
- **Path-based gates**: docs or ledger edits run encoding/link/evidence checks only; Rust changes run
  the full gate.
- **Onboarding**: one entry command issues identity, lane lease and the rules for that lane.

## 5. Metadata and readable code

- Commit trailers (`Contribution-ID` Base36, lane, evidence hash) validated by a commit-msg hook and
  CI.
- Rustdoc on public items, doctests as worked examples, a one-line "why" per module, linted like the
  stub rule.

## 6. Compliance profiles

The profile is the declared truth. A raised flag means repair the code (default) or change the
profile. Repair if the flagged behaviour is incidental; reprofile only if it is the crate's actual
job. Loosening a profile needs owner sign-off.

**Owner decision (2026-10-03): automated loosening is accepted with a guard.** The auditor may
auto-allow a loosening only when it proves the stricter rules no longer apply (no hot-path code, no
`unsafe`, no layout-sensitive types crossing into `kernel`, no dependents that would break), and
anything unproven still goes to the owner. It must never be used to produce sloppier code: if a
capability can be done at a lower (stricter) level, it is supported there and raised back up from the
base by default. Writing a second copy of a kernel-level operation in a higher tier is a defect, not a
loosening. The check therefore also fails when the loosened crate duplicates an operation already
provided by a stricter crate.

## 7. Naming (owner confirmed 2026-10-03; amendment to AGENTS.md section 4 still to be made in its own PR, NOT applied here)

Proper nouns for products and programs (Aaroneous, devtools, Cratify, Foundry, Merlin). Generic
systems terminology for crates, modules, types, traits, IPC channels and functions; a new
brand-style identifier needs a recorded reason.

## 8. Order of work

1. This spec accepted. 2. `review-coverage` required check. 3. Ledger to event log. 4. Steward
service and leases, then retire individual heartbeats. 5. Path-based gates and commit/doc lints.

## 9. Decisions and open questions

Decided 2026-10-03:

- Event log lives in a **separate service** (not a repo file). Its API, authentication and who may
  append are still to be specified before step 3; v0 must not assume repo-file storage.
- Automated profile loosening accepted with the guard in section 6.
- Naming split confirmed (section 7).
- Reviewers for whole-diff accepts: first come first served unless someone has recorded dibs.

Open:

- Service API and trust model (identity of appenders, tamper evidence, backup and rollback of the log).
- Where the service runs (local, cloud, or both) given the mixed local and cloud operators.
