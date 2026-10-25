# Trusted local integration facts (C133)

This continuation of C132 replaces the unsigned, hand-written facts input with
facts collected from local refs, authenticated candidate/review/control records,
and signed platform test receipts. It reports a local integration decision
without contacting GitHub. It does not merge, push, deploy or change the live
auto-merge gate.

## Authority boundary

`ci/integration-policy.toml` is read at the exact base commit selected by a
trusted coordinator. It declares the local integration ref, required suites,
review thresholds, principal roles, model families, platform permissions and
bare ed25519 public keys. The candidate's copy cannot lower its own threshold
or enroll its own reviewer. Keys cannot be shared between principal IDs.
Verification keys are restricted to the `verify` role; the policy refuses
combining that role with issuer, reviewer, control or implementer authority.
At least two reviewers and two model families are required; implementers and
their model families do not count as independent accepting reviewers.

The committed policy is a **bootstrap template**: `principals = []`. No real
keys or production authority are created. Collection fails closed until the
owner enrolls and reviews principals on the trusted integration branch.
Its initial required-suite list covers Foundation only. Add reviewed GUI,
security and evidence suites before treating it as a repository-wide gate.
The existing live gate remains the authority during this development phase.

All paths, native tools, repository identity and local refs are explicit
inputs. Git machine configuration and replacement objects cannot substitute
different bytes for a pinned suite or policy. Trial merge runs in a temporary
bare repository sharing only immutable objects, not the candidate checkout's
config, hooks or custom merge drivers. The temporary tree is checked to stay
inside the selected state directory before cleanup.

## Signed records

Records are UTF-8 JSON, version 1, with no unknown/missing fields. Their
exact bytes are signed using OpenSSH SSHSIG in the separate namespace
`aaroneous-local-authority-v1`. Every record binds the repository and SHA-256
of the base policy's exact text. The registry supplies the signer key and role;
the document cannot declare its own authority.

The selected authority directory contains:

- `candidate.json` and its `.json.sig`: an issuer's immutable declaration of
  exact head/base, implementer identities, revertibility and readiness. The
  issuer is accountable for attribution and classification; those are signed
  assertions, not facts inferred from spoofable author names.
- `review-<principal>-<sequence>.json` and signature: a full-head accept,
  reject or abstain decision bound to the candidate document's exact digest.
  An accept must include command argument vectors, integer exit code 0, and
  bounded log evidence whose digest matches its actual bytes. Logs are confined
  to the authority directory's `logs/` subdirectory. Signing a claimed command
  result is the reviewer's assertion; independent platform execution is still
  checked separately through C132's authenticated job receipts.
- `control-<sequence>.json` and signature: a control principal's explicit
  circuit-breaker state. Missing control is blocked. A tripped control blocks
  even if tests and independent reviews pass.

For one principal, the highest review sequence decides. A standing rejection
cannot be outvoted by other accepting seats. A higher signed accept or abstain
from that principal can retract it. Unknown signers/roles, invalid signatures,
stale revisions, altered evidence, conflicting records and missing signatures
block collection rather than becoming positive facts.

The candidate declaration cannot be rewritten after its head/base/policy
context is observed, even with a new valid signature. A correction needs a new
candidate revision/context and fresh bound reviews. This prevents changing
attribution or readiness underneath already collected review evidence.

## Durable state

An explicit, trusted state directory retains immutable observed sequence and
digest records. Removing the latest rejection or control and replaying an
older signed acceptance fails. Removing an observed reviewer's history also
fails. Keep this state outside candidate checkouts, retain it across restarts,
and give write access only to the trusted coordinator. State files are synced;
partial/corrupt records fail closed instead of being reset.

This is not protection against an attacker who can delete or replace the
trusted state store itself. Network filesystems, hostile concurrent writers,
directory fsync/power-loss certification and a signed external checkpoint are
not established here. Snapshot/backup this authority store, preserve records
and logs, and never reset it merely to make a blocked candidate pass.

The collector regenerates every required platform card from each declared
suite and the base platform policy. Node keys and platform permissions come
from the same base integration policy, not an operator-written allowed-signers
file. Every latest issued job attempt must authenticate and pass; missing
Linux or Windows evidence is pending, not a platform exemption.

## CLI

Build `local-integration` from the steward package. All path options are
absolute. Authority, state and verification-spool directories must already
exist outside the candidate checkout. The trusted coordinator selects the
base ref; the source of trust is its reviewed launch configuration, not the
candidate's own command line. The policy also enforces its integration ref.

```text
local-integration sign --ssh-keygen <native-executable> --key <explicit-principal-private-key> --document <authority-record.json>
local-integration collect --git <native-executable> --ssh-keygen <native-executable> --repo <explicit-source-checkout> --repo-name Aarogaming/aaroneous-devtools --base-ref refs/heads/main --head-ref refs/heads/<candidate> --authority <candidate-authority-directory> --state <trusted-state-directory> --spool <verification-spool>
```

Signing is not automatic acceptance. Sign only records whose identity, content,
revision and evidence the principal has reviewed. No workstation signing key
is discovered by this adapter; tests generate isolated throwaway keys.
Candidate fields are:

```json
{"v":1,"kind":"candidate","repo":"Aarogaming/aaroneous-devtools","head":"<40-hex>","base":"<40-hex>","policy_digest":"<64-hex>","principal":"<issuer>","implementers":["<registered-implementer>"],"revertible":true,"ready":true}
```

A review adds `head`, `base`, `candidate_digest`, `sequence`, `verdict` and
`evidence` to the common v/kind/repo/policy_digest/principal fields. Each evidence
item has exactly `argv`, `exit`, `log`, `log_digest`. A control adds only
`sequence` and `tripped` to those common fields. Use SHA-256 of exact UTF-8
bytes for document/log digests. Changing whitespace after signing invalidates
the signature. Keep historical signed records and their referenced logs intact.

Successful collection prints `mode: local-report-only`, authenticated facts,
the local checklist, required card IDs and the local integration decision.
An exit code 0 means collection completed, not that integration was allowed;
consume the explicit `local_would_integrate` boolean. Ref identity is checked
again before the report returns. Actual integration must recheck head/base
and serialize the ref update; this collector grants no merge side effect.
The candidate must already include the current base as an ancestor. A clean
trial merge alone is insufficient: merge the current base into an isolated
candidate branch, then issue fresh cards and reviews for that combined commit.
This prevents accepting tests of an old tree while integrating a different one.

Existing checklist fields are mapped deliberately: local signed readiness
supplies draft/readiness status, signed standing rejections supply blocking
review state, and signed control supplies the breaker. This does not claim
to have read GitHub PR drafts or threads. GitHub CI is marked `missing` with
`GitHub not consulted`; it is not relabeled as passing remote CI.

## Platform rollout

Reuse the existing Ubuntu worker host with a separate verification service
account and checkout. Do not reuse the seat's keys or give the job executor a
merge token. A trusted coordinator owns the policy, authority state and
integration side effects. Signing keys must be isolated from candidate code.
Public contributions require disposable container/VM execution with bounded
process-tree, resource and network control; C132's owner-trusted local runner
does not supply that sandbox.

SSH login and explicit deployment inputs remain owner choices. No SSH login,
service installation, system account creation or public-code execution is
performed by C133. Linux node execution, Windows rollout, authenticated
transport, GUI/security suite coverage and live authority cutover remain
separate evidence-backed steps after independent review of C132/C133.

## Verification and rollback

Tests use explicit native Git/OpenSSH paths and fresh repositories/keys. They
exercise the real CLI and signatures, positive local acceptance, signer and
family independence, command/log tampering, replay and deletion of latest
records, immutable candidate identity, ref binding, candidate policy spoofing,
real merge conflicts and source-repository merge-driver isolation.
The committed empty enrollment cannot grant authority.

Required gates: Foundation formatting, locked workspace/all-target tests,
strict Clippy, coordination evidence, whitespace and secret checks. Remote CI
remains subject to the account billing block. Revert the collector/policy
template to roll back; keep observed authority records for review. The existing
live GitHub gate is unchanged.

### Collector exit and state publication

`collect` prints the JSON decision and exits 0 only when `local_would_integrate`
is true, 2 for a collected blocked decision, and 1 for errors. Consumers must
still bind the decision to its exact candidate; a successful exit is not merge authority.

Replay state is staged, flushed and exclusively hard-linked into its immutable
final name. Unix also flushes the directory. Windows power-loss directory
durability is not certified. Interrupted staging files are ignored; existing
corrupt final records fail closed and are never overwritten. Preserve such
records and recover from verified retained state under owner review; never reset
the high-water store to make a candidate pass.
