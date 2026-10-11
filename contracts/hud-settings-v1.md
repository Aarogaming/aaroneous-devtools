# HUD settings contract, v1

Lane C120 (HUD v2 phase P1a). Implemented by the `hud_core` crate (`foundation/crates/hud_core`); design in
`governance/proposals/hud/01-settings-and-permissions.md`. This file is the format a person edits and a test can check.

## Layers

Lowest to highest precedence. Every layer is a TOML file; a missing file is the same as an empty one.

| Layer | Who writes it | Where the application looks (chosen by the caller, not by `hud_core`) |
|---|---|---|
| `defaults` | built in; the file is `foundation/crates/hud_core/src/defaults.toml` | embedded in the binary |
| `user` | the owner, for this machine and account | `~/.steward/hud/settings.toml` |
| `project` | committed with the repository, shared | `<repo>/.devtools/hud.toml` |
| `local` | uncommitted overrides | `<repo>/.devtools/hud.local.toml` |

`hud_core` reads no files and no environment variables. The adapter that embeds it reads the files and passes each
layer's text in, in the order user, project, local. The paths above are the proposed conventions, not enforced here.

Scalars and tables from a higher layer replace or merge into the lower layer. Arrays replace, except
`permissions.rules`, which **adds up** across layers (a higher layer cannot remove a lower layer's rule).

## Locks

A layer may start with `locked = ["dotted.path", ...]`. The listed paths, and everything under them, can no longer
be set by any **higher** layer; the layer that locks a path may set it itself. A higher layer that tries is
refused as a whole, with an error naming the path, the locking layer and the offending layer. Locks cannot be
removed by a higher layer, and `permissions.rules` can be locked to stop higher layers adding rules.

## Fail-safe loading

A file that does not parse, uses an unknown key, has a wrong type, is out of range, or contradicts another setting is
refused with an error naming the setting. When a **reload** is refused the previous good configuration stays active
(`Manager::reload`, `Reload { applied: false, error }`). When the **start-up** layers are refused the defaults are
used and the error is kept for display (`Manager::last_error`). A bad file therefore never loosens anything.

## Permissions

`permissions.mode` is the default for writes nobody has written a rule about:

| Mode | Unmatched read | Unmatched write |
|---|---|---|
| `read-only` (default) | allow | deny |
| `ask` | allow | ask |
| `auto` | allow | allow if the action's `risk` is `low`, else ask |

`auto` takes effect only when `permissions.auto_enabled = true` is set **in the user layer**; otherwise the effective
mode is `read-only` and a warning is reported.

Rules: `"<allow|ask|deny> <actor> <action> [scope]"`.

- actor: `*`, `human`, `seat:*`, or `seat:<name>` (lower-case letters, digits, `-`).
- action: `*`, an exact name, or a prefix ending in `*` (for example `report_*`). An exact name must be registered under
  `[actions.<name>]` (or be a floor name).
- scope: omitted or `*` (any), `self` (the acting seat is the target), or a literal compared to the scope the caller passes.

When several rules match, **deny beats ask beats allow**, whichever layer wrote them. If none matches, the action
registry decides: a read is allowed, an unknown action is denied, a write follows the mode.

Actors: `human` is the person using the application; a `seat:<name>` is a model or agent identified by the adapter
(the adapter authenticates the seat; this crate only decides). The default rules give `human` everything except the
floor. Ownership checks other than `self` belong to the adapter.

### The floor

`merge`, `push`, `review` and `attest` are refused for every actor whatever the settings or rules say
(`hud_core::FLOOR_DENIED`). They stay with the steward gate and signed attestations. An action with one of these names
cannot be registered. This is a safety invariant in code, not a setting.

## Shared layers cannot loosen the owner's machine

The project and local layers are shared or copied with a repository, so a cloned repository must not be able to loosen the
permissions on the owner's machine. Unless `permissions.accept_project_allows = true` is set **in the user layer** (it is
ignored anywhere else):

- `allow` rules from the project or local layer are ignored, with a warning that counts them. `deny` and `ask` rules still apply.
- A looser `permissions.mode` from those layers is ignored (the mode from the defaults and user layers stays), with a warning. A stricter mode is honoured.
- Those layers may not change a built-in action's `kind` or `risk`; such a file is refused as a whole. They may add new actions.

Only the permission settings are protected this way. Any other setting that should not be changed by a shared file (limits,
bind address, model endpoints) is protected by a `locked` entry in the user layer.

## Settings groups

| Table | Purpose |
|---|---|
| `permissions` | `mode`, `auto_enabled`, `accept_project_allows`, `rules` |
| `actions.<name>` | registry: `kind` (`read`/`write`), `risk` (`low`/`high`, default `high`) |
| `agent_door` | `enabled`, `token_lifetime_secs`, `rate_per_minute`, `max_body_bytes` |
| `routing` | `tiers` (software, local, cloud), `cloud_enabled`, `min_confidence` |
| `models.backends.<name>` | `kind` (`ollama`, `openai-compatible`), `endpoint`, `model`, `context_tokens`, `timeout_secs` |
| `models.roles` | role to backend name; `glue` is required while the `local` tier is on |
| `budgets` | `accounting`, `per_seat_daily_tokens` (0 = no cap), `on_exhaustion` (`queue`, `ask`, `refuse`) |
| `cache` | `enabled`, `max_entries`, `ttl_secs` |
| `events` | `long_poll_timeout_secs`, `retention`, `heartbeat_secs` |
| `operations` | `visible`, `run_without_approval` (a subset of `visible`) |
| `ui`, `ui.visualizer` | `theme`, `notification_level`; `enabled`, `max_fps`, `fallback_2d_when_model_busy` |

Ranges are checked at load (see `Config::validate`). The initial values are in `defaults.toml`.

## Effective-value view

`Loaded::effective()` returns every setting with its final value and the layer that set it, so the application can
show where a value came from. `Loaded::rules()` lists rules with their layers.

## Not covered by this version

Transport, authentication of seats, token issue and revocation, rate limiting and the audit log are later phases
(P1 agent door, P1b MCP adapter). File watching and the hot-reload trigger belong to the adapter; this crate provides
the reload step. Not verified: behaviour with the real adapters, which do not exist yet.
