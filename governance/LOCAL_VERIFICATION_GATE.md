# Local verification job cards and shadow gate (C132)

This is the first executable adapter around C129's platform routing, suite and
packet core. It uses a local file spool and explicit inputs. Git supplies
immutable revision identity; GitHub availability is not needed for issuance,
execution, signature checking or local checklist evaluation. It does not
change the live auto-merge gate or install a service.

## Authority

The trusted base commit supplies `ci/platform-policy.toml` and
`ci/suites/<suite>.toml`. Issuance reads their exact bytes from Git and creates
one command-free card per required platform. A card binds repository name,
head, base, suite digest and platform. Changes to any field produce a new ID.
The suite handed off by the owner is aligned with the locked workspace and
all-target commands in the hosted bridge.

The node executes only the base-defined argument vectors. A runner config
provides absolute native executable paths and an explicit environment. The
process receives no inherited environment or stdin. It runs in a clean
checkout at the exact head, and the checkout is checked again afterward.
Log files and receipts live outside that checkout. Steps stop at failure;
timeout, failed start, oversized logs and missing evidence never pass.

`trusted-local-v1` is **owner-trusted local execution**, not a public-code
sandbox. A timeout stops the immediate process; it does not prove that all
descendants were stopped. Keys and the spool must belong to a separate trusted
authority before running potentially hostile code. Do not use this adapter
for public forks. A disposable sandbox with process-tree/resource/network
control is required for that profile; it is not silently inferred here.

## Spool and receipts

Each card directory holds an immutable `request.json`, monotonic
`attempt-N.json`, one exclusive `claim.json`, per-step stdout/stderr, and
`receipt-N.json`. Duplicate submission is idempotent. A second worker cannot
claim the same card. Restart retains the claim; leases never silently expire.
After stopping the old worker, an operator can recover a matching node and
attempt explicitly. Recovery records abandonment before releasing the claim.
Incomplete/corrupt files fail closed and require inspection rather than
being overwritten. Keep the spool on one local filesystem with one trusted
owner; shared network filesystems and hostile writers are not supported.

A receipt repeats the full card, outcome and actual step evidence. Validation
recomputes the verdict from the pinned suite and checks each log's SHA-256.
It records native executable and explicit-environment digests without exposing
environment values. A timeout retains the lease until operator recovery.
The gate verifies the exact receipt bytes it parsed using OpenSSH SSHSIG,
namespace `aaroneous-local-verification-v1`. It checks current allowed signers,
node/platform authorization, request ID and latest issued attempt. A running
retry suppresses an earlier pass. Removing a signer immediately invalidates
its receipts on the next evaluation. No private key is discovered or bundled.

Signing is a separate trusted-owner action. Inspect and validate a receipt
before signing it; signing arbitrary candidate-supplied JSON is not approval.
Do not give the candidate worker the signing key or trusted facts file.

## CLI

Build with `cargo build --manifest-path foundation/Cargo.toml -p steward
--bin local-verification --locked`. All path options are absolute and the
spool directory must already exist. Commands print JSON and return nonzero
on unreadable authority, malformed input or failed authentication.
For `run`, exit 0 means that a receipt was stored, not that the suite passed;
integration must consume the authenticated receipt verdict through `shadow`.

```text
local-verification issue --git <native-git> --repo <source-checkout> --repo-name Aarogaming/aaroneous-devtools --head <40-hex> --base <40-hex> --suite foundation-ci --spool <spool>
local-verification run --repo <clean-candidate-checkout> --card <request.json> --spool <spool> --node <registered-node> --config <runner.json> --trust owner-trusted-local
local-verification sign --ssh-keygen <native-ssh-keygen> --key <explicit-node-key> --receipt <receipt-N.json>
local-verification shadow --git <native-git> --repo <source-checkout> --repo-name Aarogaming/aaroneous-devtools --head <40-hex> --base <40-hex> --suite foundation-ci --spool <spool> --ssh-keygen <native-ssh-keygen> --allowed-signers <allowed-signers> --registry <registry.json> --facts <trusted-facts.json> --trust owner-trusted-local
local-verification recover --spool <spool> --card <request.json> --node <previous-node> --attempt <N> --old-worker-stopped yes
```

`runner.json` has exactly `git` (absolute executable), `tools` (map from suite
program name to absolute executable) and `environment` (string map). Supply
the toolchain path, OS-required variables and isolated output/cache directories
explicitly. Windows tool mappings must be `.exe`, not `.cmd` shell wrappers.
Do not include secrets. Example shape:

```json
{"git":"<absolute-native-git>","tools":{"cargo":"<absolute-cargo>"},"environment":{"PATH":"<explicit-toolchain-path>","CARGO_TARGET_DIR":"<isolated-target>","CARGO_BUILD_JOBS":"2"}}
```

Registry shape is `{"node-name":["windows-x86_64"]}`. Allowed-signers lines
use that exact node identity and its registered OpenSSH public key. The owner
controls both files. `trusted-facts.json` uses the existing Rust checklist
facts, including exact `head`, `base_tip`, review coverage, independent
accepting seats, evidence counts, blocking threads, rollback classification
and circuit-breaker state. Those facts still need a trusted local collector;
this adapter does not authenticate a hand-written review assertion.

## Shadow, then authority

Shadow regenerates the full required card set from base policy. It validates
signed receipts and substitutes that result for checklist box 1, preserving
all other review and policy boxes. It reports local and GitHub decisions and
their agreement; it has no merge side effect. A missing Linux node is pending,
not a platform exemption. A Windows-only run cannot pass a Linux requirement.

Before switching live integration to local authority:

1. Run registered Linux and Windows workers and collect receipts for real
   candidates, including errors, retries and moved heads/bases.
2. Finish sandbox profiles, cancellation/process-tree cleanup, authenticated
   transport and local signed review/facts collection. Add toolchain policy
   and durable telemetry/retention rather than relying on operator-written facts.
3. Compare shadow decisions with the existing gate. Classify mismatches;
   account-billing differences may be expected but must remain explicit.
4. Make an owner-reviewed gate change that consumes local acceptance receipts,
   rechecks candidate/base at integration, and mirrors results to GitHub.
   Missing GitHub reporting must not erase local evidence or manufacture a pass.

## Verification

Native tests declare `DEVTOOLS_TEST_GIT` and `DEVTOOLS_TEST_SSH_KEYGEN` as
absolute executable inputs. Hosted Foundation workflows supply them explicitly;
local callers do the same. Tests create fresh isolated repositories and
throwaway SSH keys; they never use workstation signing credentials.
Coverage includes real command pass/fail, actual timeout, exclusive claims,
restart/recovery and attempt fencing, dirty checkout, stale request/digest,
log and signature tampering, key revocation and preservation of review gates.
Foundation formatting, workspace/all-target tests and strict Clippy are the
required candidate gates. Remote checks remain billing-blocked.

Candidate verification on 2026-10-11 (America/New_York): workspace/all-target
tests passed (302 tests, one pre-existing ignored differential and one ignored
subprocess helper that the timeout test explicitly runs). After adding the CLI
end-to-end test, all eight focused adapter tests passed. Formatting and strict
workspace/all-target Clippy passed. New-commit Gitleaks scanning found no leaks.
Evidence is retained in `tmp/public-readiness-evidence/local-gate-*.log`;
the queue cites the immutable implementation revision. These are Windows-host
results, not a claim of Linux node execution or independent acceptance.

Rollback removes this adapter/CLI and reverts manifest alignment and test-tool
inputs. Preserve the spool and evidence for review; no live authority has
been switched by this candidate.
