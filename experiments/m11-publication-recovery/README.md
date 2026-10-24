# M11 publication recovery experiment

Contribution-ID: C107/M11-recovery-v1.
Signed-off-by: Codex (GPT-6), session 01a0f30a-4d6b-7703-9859-662e36299ca4.
UTC: 2026-10-06. Role: author. Independent review: pending.

When the discussion board creates a published record but fails to remove its
pending name, the original adapter returns failure, retains both names and
leaves its counters at zero. Blindly resubmitting can duplicate a contribution.
This isolated prototype distinguishes that state from an unpublished or
conflicting record and preserves recovery evidence before clearing a matched
pending name. It is not installed or integrated into any service.

## Files and authority

- [board.py](board.py): captured runtime adapter plus the staged reporting and
  counter repair. Session/health expose pending classification while intake is
  held; publication errors return a structured report with retry_allowed=false.
- [recovery.py](recovery.py): read-only classification and explicit offline
  matching-record reconciliation under the existing Windows host byte lock.
- [test_recovery.py](test_recovery.py): isolated fixtures and fault checks.
- [contribution.json](contribution.json), [verification.json](verification.json):
  author attribution, pinned source hashes and exact verification commands.

The runtime base SHA256 is
`7fc1bc24a2c46fd722f2fdecdca1f6aea80d2f1dc655584faaf0c6dcbb1736be`.
It is an explicitly captured input, not an import or dependency on a machine's
live board. The package removes the original hardcoded support-directory path.
No native Rust policy, product code, installed loop, credentials or schedule
is changed. Rust remains the intended owner of maintained recovery policy;
this experiment supplies reviewable behavior and fixtures for that work.
C101/#59 and C102/#60 remain separate design proposals, not dependencies or
acceptance of this implementation.

## Verification

From an explicit checkout, run:

```powershell
python -B experiments/m11-publication-recovery/test_recovery.py
```

On Windows this runs 11 tests: before-publication hold/no replay; matching
publication/counters/retained archive; conflicting bytes; malformed and
duplicate JSON; stale plans; active host lease and running-control rejection;
archive flush failure; conflicting archive preservation; duplicate pending
identity; lifecycle/history/capacity; and structured HTTP-handler failure.
On other platforms seven lease-dependent tests are explicitly skipped; those
runs do not establish Windows recovery coverage. No real HTTP or browser
transport, power-loss durability or live rollout is claimed.

Fixtures are retained beside the test module as `fixture-*` directories and
must not be committed. The committed verification receipt lists source hashes
and the retained log location; reviewers should rerun against their exact head.
The earlier staged run had one faulty conflict fixture: editing the pending
hard link modified both names. The corrected test replaces the pending name
before writing conflicting bytes. That earlier failure remains in the staged
evidence; it is not represented as a successful run.

## Explicit operations (review before use)

Read-only classification takes an explicit absolute board root:

```powershell
python -B experiments/m11-publication-recovery/recovery.py --root <absolute-board-root>
```

The output contains a plan_sha256. Reconciliation additionally requires an
explicitly stopped/drained managed host, its existing board-host.lock, and
the same plan digest. It does not stop a process or reopen intake:

```powershell
python -B experiments/m11-publication-recovery/recovery.py --root <absolute-board-root> --reconcile-matching <plan-sha256>
```

Only an entirely published-matching plan can proceed. The tool copies pending
bytes to recovery-evidence with exclusive creation, flushes and verifies them
against the published record, then removes the redundant pending name.
Published records are never replaced or deleted. Unpublished, malformed,
conflicting, linked or duplicate-identity evidence stays held for an owner
decision. An incomplete multi-record cleanup requires reinspection and an
explicit new plan digest; it never automatically retries or replays a record.

The staged board CLI requires both --root and --support-directory. No host,
browser assets or default support directory are installed by this package.

## Compatibility, limits and promotion

Board response contract becomes 4 (session, health, successful receipt,
structured publication failure and runtime metadata). Payload fields,
host/origin/token checks, ID/hash semantics, pagination and record quotas
remain as in the captured adapter; server configuration stays schema2 and
native worker jobs stay schema1. Version-pinned clients need review.

The recovery operation assumes trusted local files and a stopped managed host
without an unmanaged writer. This is not malicious-writer containment,
authenticated seat identity, a total-root/log budget, a client idempotency
protocol or a power-loss recovery guarantee. Existing invalid published
records still fail startup. File bytes are flushed; directory metadata
durability is not established. The borrowed adapter remains experimental.

Independent review must cover classification, stopped-host lease, archive
ordering, published-byte preservation, counters and response compatibility.
Any later rollout needs exact source/consumer binding, retained rollback
sources and a check of historical entry hashes. This PR supplies no rollout
or merge authority and makes no whole-M11 acceptance claim.
