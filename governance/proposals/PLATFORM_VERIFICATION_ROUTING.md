# Platform verification by routed packets (lane C129)

Status: proposal plus the first Rust slice (`foundation/crates/steward/src/verification.rs`). Author: Claude seat. Builds on HUD
proposal 03 (the hall, registered nodes, heartbeats, leases) and on C128 (self-hosted runners) and C127 (Python to Rust).
Grants nothing; the merge gate stays the authority.

## What the owner asked for

Verify Linux changes on the Linux machine and Windows changes on the Windows machine, each running its own devtools, with
telemetry behind it and live routing: a change that needs verification is sent to the node that can do it, a process fires on
arrival, the result comes back, and the same happens in parallel for the other platform.

## The shape

```
 PR head changes
      |
      v
  [hall]  builds one Request per required platform  ----------------------+
      | route(): pick the node for that platform                           |
      v                                                                    v
 [ubuntu node]  --- heartbeat/telemetry --->  [hall]  <--- heartbeat ---  [windows node]
      | request arrives -> run the suite named in main's manifest              |
      v                                                                        v
  signed Result  ------------------------------>  [hall]  <------------------  signed Result
                                                    |
                                          evidence(): every required platform passed?
                                                    v
                                          merge gate box 1 reads the answer
```

The hall is the process described in proposal 03 (one machine, nodes register, heartbeat, and pull; the hall never reaches
into a node). "Live" is a held connection (long poll or server-sent events) on which the hall pushes a request the moment it is
created; a short poll is the backstop. Nodes behind other networks and cloud sessions still work through the existing GitHub
route, which the hall reads.

## Packets

**Request** (hall to node). Names a commit, a platform and a suite; **never carries a command.**

| Field | Meaning |
|---|---|
| `repo`, `head`, `base_sha` | the exact commits (full SHAs) |
| `platform` | `linux-x86_64`, `windows-x86_64`, ... |
| `suite` | a name such as `foundation-ci` |
| `suite_digest` | SHA-256 of the suite manifest **as it stands at `base_sha`** |

`id` = SHA-256 over the canonical fields, so a retried or duplicated request is recognised and stays on its holder.

The node reads `ci/suites/<suite>.toml` from git at `base_sha`, recomputes the digest and refuses a mismatch. The manifest is a
list of argument vectors (no shell string), so what runs is whatever the **base branch** says, not whatever the pull request
brings. This is the same rule that keeps the merge workflow and the evidence checker out of a PR's reach.

**Result** (node to hall). Signed by the node's registered key (namespace `aaroneous-verification`, the same `ssh-keygen -Y`
scheme as attestations). It repeats `request_id`, `head`, `platform`, `suite_digest`, adds `node`, `attempt`, per-step exit codes and
log digests, the toolchain versions, and a verdict of `pass`, `fail` or `error` (the suite could not run: never a pass).
Because it is bound to the exact head and suite digest it needs no expiry.

## The decision (already implemented, pure Rust)

`verification::evidence(expected, outcomes, authorised, admitted)`:

- **`expected`** is the set of validated requests the hall issued for one candidate (one per required platform, same repository, head
  and base). A result is matched to its request by the request's **canonical id**, not by head and platform alone, so a result for a
  different repository, base, suite or digest never counts. (Codex reproduced the opposite on the first version: a verified, authorised
  pass carrying another valid request's id made the gate green. The fix is this signature, with negative tests that change the
  repository, base, suite, digest and id one at a time.)
- **`admitted`** is the hall's own durable state: for each request, the attempt that is current and the node it was assigned to. An
  outcome counts only for the admitted attempt on the assigned node. A late pass from an earlier attempt cannot turn a newer pending
  attempt green, and a request the hall has not admitted is pending whatever arrives.
- a result counts only if its id, head, platform and suite digest equal the request's, **its signature was verified**, and **its node is
  authorised for that platform**;
- two different verdicts from the assigned node for the admitted attempt are a **conflict**, not a pass; the same verdict delivered twice
  is harmless;
- `error` is never a pass; an empty, malformed, duplicated-platform or self-contradictory expectation is a misconfigured gate and is never ok.
- the returned `Evidence` carries its **subject** (repository, head, base, and per platform the suite, suite digest and request id), set by this constructor from the validated requests and never by a caller. A consumer that reduces verification to `ok` loses what it was about; the subject lets a release decision (C135) reject a green result computed for another commit (Codex's A-pass/B-candidate counterexample).

`verification::required_platforms(default, rules, changed_paths)` reads the policy as data: Linux for everything, plus Windows
when a changed path starts with a listed prefix (for example `foundation/crates/platform_bridge/`).

`verification::route(platform, roster, current_lease)`: a node holding the request's lease keeps it while online; otherwise the
online node with a free slot and the shortest queue wins, ties by name, so the same roster gives the same answer. A platform with
no registered node is `Unsupported` (retrying will not help); a platform whose nodes are all offline or full is `Defer`.

Tests: 17 unit tests. Deliberate breakages that each fail a test: unverified signatures counting, any node attesting any platform,
an empty expectation being ok, a stale head counting, the request id not compared, the attempt not compared, the assigned node not
compared, a platform expected twice, requests that disagree about repository, head or base, and an unadmitted request being judged
anyway. Two first survived because the test did not isolate the rule (the disagreement test left one request unadmitted, so it was
pending for another reason); they were rewritten so only the rule under test can make the result differ.

## Telemetry

**Heartbeat (node to hall), every few ticks:** node name, `platforms`, OS and architecture, devtools build (git SHA and the steward
contract digest, so a stale node is visible), toolchain versions, load, free disk, `slots` total and free, `queue_depth`. A node is
online while its last beat is recent **in the hall's own ticks**; no clock is compared across machines. The beat is authenticated by
the per-node token on the channel; only results are signed.

**Event log (hall):** the existing hash-chained log gains `request_issued`, `routed`, `deferred`, `started`, `step_finished`,
`result_posted`, `verified`, `rejected` (with the reason). Everything below is derived from it, nothing is stored twice:

- queue wait, run time and pass rate per platform and suite;
- defers and offline time per node;
- a card when a required platform has had no online node for longer than a setting, so a powered-off Windows machine is a visible
  state and not a silent wait;
- the HUD and `foundation status` show the same table.

## Trust and failure

| Situation | Result |
|---|---|
| Node offline | request is deferred; the gate shows the platform as pending; never skipped |
| Result with a bad or missing signature | ignored, counted in the detail ("1 unverified result(s) ignored") |
| Node signs for a platform it is not registered for | ignored |
| Result for an older head, another suite digest, another request, or an earlier attempt | ignored |
| The assigned node reports two verdicts for one attempt | conflict, blocks |
| Suite manifest changed in the PR | the node verifies against `base_sha`; the PR's change is judged at merge by the next base |
| Duplicate request | same id; `route` returns the same holder. Delivery is at-least-once: suppressing a duplicate *effect* needs durable request and attempt state with atomic admission and fencing in the hall (a later phase), because an offline holder may still be running when a replacement is assigned |
| Node cannot run the suite (disk, toolchain) | `error`, blocks, retried with a higher attempt |
| Node compromised | can attest only the platforms it is registered for, and its key is revocable through the registry; two nodes per platform give a cross-check |

Node keys are registered like seat keys (an additive registration is the existing exempt class). Request packets are untrusted data
to the node: it validates every field (the id's canonical form depends on it) and refuses the rest.

## How this relates to what exists

- **C128 (runners)** is the bridge: it gets Linux CI running on the Ubuntu machine now. Nodes replace the GitHub runner with a
  devtools process that does the same job and also does Windows, with telemetry and signed results. During the move both run and
  are compared.
- **C127 (Python to Rust)**: this is phase 2 and 3 of the plan for the gate. The routing and evidence code is Rust from the start
  and there is no Python oracle for it.
- **Proposal 03 / C112:** the hall, the roster and the heartbeat are 03's; the seat-status files remain the reviewed record.
- **The gate:** box 1 ("CI green on the exact head") becomes "`evidence().ok`" once nodes exist. Until then it still reads check runs.

## Phases

1. **This PR:** the pure core above with tests; this document.
2. Contracts as JSON with vectors (Rust-owned, the first test of C127's "flip the oracle"), the suite manifest format, the policy file.
3. Node agent (`foundation node`): register, heartbeat, hold the connection, verify the digest, run the suite in a sandbox, sign, post.
   Linux first on the Ubuntu machine; Windows node next. **Open:** a Windows sandbox (a restricted user with a job object) is weaker
   than the systemd sandbox on Linux; say so in the node's own status until fixed.
4. Hall side: the router and the event log in the guild daemon; telemetry views.
5. Shadow beside Actions and the C128 runners on real PRs; compare; then box 1 reads `evidence`.

## Not decided here

The transport for the LAN (an encrypted overlay already in use, pinned TLS, or SSH tunnels: proposal 03 lists them); whether the
hall runs on the Windows machine as 03 suggests or on the Ubuntu one; how many nodes per platform before the cross-check is on by
default.

## Verified and not

Verified: `cargo test -p steward --lib verification` (16 tests), `cargo clippy -p steward --all-targets -- -D warnings`, and
`cargo fmt --all -- --check` (commands and results are in the lane row). **Not run:** nothing here touches a network or a machine;
the node agent, the hall wiring and every transport are future phases.
