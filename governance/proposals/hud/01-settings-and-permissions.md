---
notation: v1
purpose: Layered settings and a permission engine so every behaviour is configuration and the default is read-only
authority: propose
status: proposal, advisory, grants nothing
relations:
  - implemented-by: lane:C120
  - informs: doc:02-agent-door-and-mcp
  - informs: doc:08-roles-skills-and-trust
checked-by: none yet (see "Not verified")
---

# Settings layers and the permission engine (proposal 01)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 01 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## Configuration and permissions (R8)

Nothing below is a constant in code except the **built-in defaults**, which live in one embedded `defaults.toml`
(readable, testable, and the only place a default is written down). Everything else is read from settings.

**Layers, lowest to highest precedence:** built-in `defaults.toml`, then user settings
(`~/.steward/hud/settings.toml`), then project settings (`<repo>/.devtools/hud.toml`, committed, shared), then local
overrides (`<repo>/.devtools/hud.local.toml`, uncommitted). A setting can be marked `locked = true` at a lower layer
(for example by the owner in user settings) so a higher layer cannot loosen it. The HUD shows, for every effective
value, which layer set it, and validates the files on load: a bad file keeps the previous good configuration and
reports the error rather than falling back to something permissive. Changes are picked up without restart.

**Permission model, patterned on the permission settings of the agent tools these seats already run under** (this is
the same shape, not shared code): a *mode*, plus explicit *allow*, *ask* and *deny* rule lists, where deny beats ask
beats allow.

| Mode | Meaning |
|---|---|
| `read-only` (**the default**) | reads are allowed with no prompt; anything that changes state is refused unless a rule below allows or asks for it |
| `ask` | reads allowed; each state-changing action waits for an explicit approval in the HUD |
| `auto` | pre-approved operations run without asking; everything else still asks. Not available until the owner enables it in user settings |

Rules name an actor (a seat from `governance/identities.json`, `human`, or `*`), an action and an optional scope, for
example `allow seat:* GET card`, `allow seat:* PUT availability`, `ask seat:* POST result`, `deny * merge`. Reads of
a seat's own card and the work list are allowed by default; availability is the one write a seat may do unasked, because
it is status, not state of record, and the owner can turn it off. Results, intake, operations that run commands, and
anything that touches git or GitHub start as `ask` or `deny`. Merge, push, review and attestation are `deny` and are
not configurable from the HUD at all: they stay with the steward gate and signed attestations (principle 4).

**What is configurable, with the proposed initial value:**

| Group | Settings (examples) | Initial default |
|---|---|---|
| permissions | mode, allow/ask/deny rules, per-seat overrides, locked keys | `read-only`, rules as above |
| agent door | enabled, token lifetime, per-seat rate limit, body cap | enabled, 1 day, 60/min, 64 KiB |
| routing | tier order (software, local, cloud), per-class tier ceiling, confidence thresholds | software then local; cloud off |
| models | named backends (endpoint, model, kind, context size, timeout); which backend serves which role | the Ollama endpoint the HUD already uses |
| budgets | token budget per seat per day, per job, per tier; what happens on exhaustion (queue, ask, refuse) | accounting on, no hard cap |
| cache | result-cache on/off, size, TTL, pack retention | on, bounded |
| events | long-poll timeout, sequence retention, heartbeat | conservative |
| operations | which operations appear, which need approval | list only; none run unasked |
| ui | theme, panels shown, notification level | quiet |

Because behaviour is data, the same HUD can be run stricter or looser per machine, and a seat that misbehaves can be
narrowed by one line in settings. The settings schema is a versioned contract with tests (every default parses; deny
overrides allow; a locked key cannot be overridden; an invalid file does not weaken the active configuration).
## Everything else is configuration too

Levels and thresholds, which skills exist, how many outcomes promote, decay, the minimum level per risk class, which events trigger a taste checkpoint, role permissions, quota sizes and profile consent defaults all live in these settings layers, so the owner can retune them later without a release. Requirement R8 in the README is the rule: a default is data, not code.

## Phases for this feature

| Step | Deliverable | Status |
|---|---|---|
| P1a | `hud_core` crate: embedded defaults, layers, locks, permission engine, effective-value view, fail-safe reload | in review as lane C120 |
| follow-up | `role:`, `worker:` and `user:` actor patterns; settings groups for network, profiles, workers and traces | after the proposals that need them are accepted |

## Decisions for this feature

1. **Default permission mode** is `read-only`, modelled on the permission modes of the agent tools in use. DECIDED by the owner, 2026-10-10.
2. The availability write is allowed by default and switchable off in settings (default accepted).
3. Proposed file locations: `~/.steward/hud/settings.toml` (user), `<repo>/.devtools/hud.toml` (project), `<repo>/.devtools/hud.local.toml` (local). They are conventions for the adapter, not enforced by the crate.

## Review focus

- Are the layers, locks and fail-safe reload the right shape, and is the precedence unambiguous?
- Does deny-beats-ask-beats-allow, with an unmatched write refused in `read-only`, match the intent "model it after your own default settings"?
- Is the floor (merge, push, review, attest) correctly placed in code and kept out of settings?
- Is the `auto` gate (user layer only) strong enough?
- The code for the first step is in lane C120 (`foundation/crates/hud_core`, contract `contracts/hud-settings-v1.md`); review that separately.

## Not verified

- That the layering and permission rules express every case the seats need; the shape is modelled on the agent tools' settings from memory of how they behave, not from their source.
- That a rule engine extended with roles stays simple; to be checked when roles are designed.
- Behaviour with real adapters, which do not exist yet.
