# Persistent DevTools operations

DevTools automates development of the Aaroneous Rust Component Framework. The maintainer continues bounded DevTools implementation, hardening and design work from the existing Codex task; Aaroneous keeps its own source ownership and native verification gates.

## Maintainer schedule

The app automation `aaroneous-rolling-worklist` was repurposed, rather than duplicated, as **DevTools persistent maintainer** on 2026-09-17. It is active hourly at minute 30 and targets task `01a0ace6-af23-7860-a77e-8447f40ed428`. In addition, a daily 09:00 local maintainer review cadence is recorded for manual/scheduled maintainer inspection. This cadence is distinct from passive utility polling and the default-off C21 in-process persistence (`foundation multithreader-serve`).

Each run inspects Git changes, CI, ownership, review debt and available useful work. It gives priority to reproduced failures, registrar API authentication and the C15 dashboard corrections. Full capacity calls for review and evidence collection. Claims, verification, scoped commits and handoffs follow the repository agent guide. Reports are quiet when nothing actionable changes; progress, failures and required owner decisions may notify the owner.

Pause or resume **DevTools persistent maintainer** in the app's Automations controls. The schedule uses the current task context and account allowance; it does not install an always-running operating-system daemon or guarantee execution while this host/app is unavailable. Usage limits and missing tools cause bounded deferral to a later run, rather than repeated attempts.

## Git triggers

The existing **Foundation CI** runs on main pushes, pull requests and manual dispatch. It checks Rust formatting, tests, strict lint and declared stack validation on Linux and Windows.

**Dashboard CI** adds Linux/Windows locked dependency installation and the TypeScript/Vite production build. It runs on main pushes or pull requests affecting `gui/**` or its workflow, and supports manual dispatch. Superseded dashboard runs are cancelled and each job has a ten-minute limit. Repository contents are read-only to the workflow; it executes no product writes or inference.

The hourly maintainer fetches and compares repository/CI evidence to its previous run. GitHub checks are immediate Git-triggered validation; agent review follows on the hourly schedule. There is no Git webhook that directly launches this Codex task and no post-commit hook that recursively invokes an agent.

Pause GitHub checks by disabling their workflows in Actions; resume them there and use manual dispatch to confirm operation. Removing the maintainer schedule does not remove GitHub checks.

## Existing native workers

The local support worker and candidate-review services have separate documented configuration in [LOCAL_WORKER_SERVICE.md](LOCAL_WORKER_SERVICE.md) and [MULTITHREADER_SERVICE.md](MULTITHREADER_SERVICE.md). This setup does not retarget or recreate those services. An elevated Windows scheduled-task inventory on 2026-09-17 returned no task names containing Aaroneous or Devtools — resolved (2026-09-17, owner): the `Aaroneous-Devtools-Multithreader` task was deliberately removed, not lost. The repository "wasn't ready for the schedule yet, there were still too many bugs for it to run all day every day" unattended. `MULTITHREADER_SERVICE.md` now documents `foundation multithreader-serve`, an in-process persistent mode that replaces the removed OS scheduled task and defaults to off (nothing runs unless this command is explicitly started). Before enabling or replacing any service on this host, still reconcile its explicit paths and current state — this resolves the *why nothing was found* question, not a general exemption from that check.

The shared reservation still governs any Qwen work. Pending output remains unverified and requires owner review. No autonomous model loop, additional provider credential or reverse dependency from Aaroneous is introduced by this setup.
