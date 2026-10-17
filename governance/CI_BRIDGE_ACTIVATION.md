# Hosted CI bridge activation (C131)

Owner requested integration of Claude's CI bridge on 2026-10-11 (America/New_York).
PR #196 was merged at `c8d2f83`; this follow-up activates its four hosted
workflow templates. The account billing/spending-limit exception carries
forward from the owner's explicit direction: blocked hosted jobs are not
reported as passing checks.

Foundation pull requests run on Linux, with Rust checks omitted only when
the diff contains no Foundation, stack, CI, root Cargo/toolchain or root
Cargo-configuration changes. An unreadable diff builds. Main pushes always
build; Windows Foundation checks run on main and manual dispatch. The GUI
build runs on Linux. Windows regressions can consequently be caught after
merge rather than before it; this is a deliberate bridge tradeoff.

Auto-merge evaluates hourly and on completed check suites or PR updates,
using `pull_request_target` with the default branch checkout. It does not
execute PR code. Its full-head merge guard and existing evidence policy
remain in force. All actions in the activated templates are pinned to
verified commit references. Foundation checks use the locked workspace,
all targets, formatting and strict Clippy.

The separate security and coordination-evidence workflows remain active.
The billing block still prevents hosted execution; this bridge reduces
usage after jobs resume and does not manufacture green check runs.

## Self-hosted deployment remains unverified

No Ubuntu runner was installed and no self-hosted workflow was activated.
Before deploying the scripts from PR #196, address these review findings:

- `setup-runner.sh` interpolates the registration token and unvalidated
  runner name into a shell command. Pass these as separate arguments.
- The service starts before the isolation check completes. Check isolation
  before accepting jobs.
- `KillMode=process` and the cleanup hook do not establish that child
  processes or writable caches are reset between jobs. Use disposable
  execution environments for public contributions.
- Workspace cleanup retains `_actions` and `_tool`; the existing prose
  claim that every job leaves nothing behind is too strong.

Keep public fork code on hosted or disposable isolated runners. Separate
OS users alone do not make a persistent workstation safe for public PRs.
Smoke-test Ubuntu registration, isolation, cleanup and tool availability
before any deployment. The current task authorizes hosted activation only.

## Verification and rollback

Local validation parses all four activated workflows, checks the trusted
merge checkout and permissions, and executes the Foundation path classifier
against docs, Rust, stack, CI, mixed, toolchain and unreadable-diff cases.
Shell syntax and coordination/whitespace checks are retained with C131.
Remote CI remains blocked by the account issue and must be rerun later.
Revert the activation commit to restore the prior hosted workflows.
