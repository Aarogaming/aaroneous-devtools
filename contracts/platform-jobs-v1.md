# Platform job service v1 (C135)

This transport extends [local verification](../governance/LOCAL_VERIFICATION_GATE.md).
Git identifies immutable code and trusted-base policy. Signed local receipts
provide test evidence. This service cannot merge, change integration policy,
enroll reviewers or turn a missing platform into a pass.

## Protocol

Requests use HTTP/1.1 with `Authorization: Bearer <per-principal-token>`.
POST bodies are JSON with an explicit `Content-Length`; chunked transfer,
duplicate headers and `Expect` are refused. The backend binds loopback only.
Workers require HTTPS with certificate verification for every remote origin;
HTTP is accepted only for literal loopback addresses and explicit ports.
The client follows no redirects, uses no ambient proxy or curl configuration,
and sends the bearer via stdin rather than command-line arguments.
JSON uploads use files in the explicitly configured private `client_scratch`
directory. They are removed after the request, preserving exact body bytes
without relying on curl's version-dependent configuration-line size limit.

| Route | Role | Request | Response |
| --- | --- | --- | --- |
| POST `/v1/jobs` | issuer | `v:1, head:<full SHA>, suite:<enabled name>` | Base-issued cards, one per required platform |
| GET `/v1/jobs` | issuer or worker | empty | Card, latest attempt, claim and receipt-import status; worker sees its platforms only |
| POST `/v1/claim` | worker | `v:1` | Full C129 request, node, attempt, resumed; or idle |
| POST `/v1/results` | worker | `v:1, request_id, attempt, files` | Validated receipt acknowledgement and verdict, or refusal |

Tokens require 32-128 URL-safe characters, one unique random token per identity.
Use at least 32 random bytes. Explicit private files supply tokens; they are
loaded at service startup. Revoking/changing a token requires restarting the
service. Current allowed-signers bytes are read on every receipt submission.
Roles cannot be combined: issuers issue; workers claim and submit only for
their configured platforms. Tokens authenticate transport, while a distinct
registered node key signs test evidence. Use the same node enrollment as the
trusted integration policy; transport enrollment cannot grant gate authority.

`files` maps exact artifact filenames to lowercase hex bytes. It contains the
unaltered `receipt-N.json`, its `receipt-N.json.sig` OpenSSH signature, and
exactly the stdout/stderr files named by the receipt's base-defined steps.
Each decoded artifact is at most 8 MiB; an encoded bundle is at most 32 MiB.
The service recomputes the full receipt verdict and log digests, verifies its
signature, and checks node, platform, current claim and attempt. Arbitrary
paths, missing/extra artifacts and altered evidence are rejected.

## Persistence and recovery

The existing C132 spool is authoritative. Uploads validate in a separate
private staging directory; logs and signature publish before the receipt,
then an immutable `received-N.json` acknowledgement publishes last. Matching
partial files are reused on retry; differing or corrupt files fail closed.
An identical completed upload is idempotent and rechecks signer revocation.
Claims survive process restarts and lost replies. There is no automatic lease
expiry, failover, retry attempt or rerun of incomplete execution. A stopped
worker may resend an already produced receipt without executing the suite.

After an execution crash, stop the old process/container and inspect both
spools before using C132's explicit `recover` on the server and worker. Keep
their attempts aligned. A timeout receipt remains non-passing and retains the
claim; completion acknowledgement prevents silently dispatching it again.
Keep accepted receipt/signature/log bytes and attempt history together. Do
not sync or prune the live spool, reset high-water state, or replace it with
an older backup while integration authority remains active.

The transport is a single coordinator on local storage, not a consensus or
multi-host filesystem system. Limit: 10,000 cards before operator archival.
Partial filesystem writes require inspection; it does not certify power-loss
durability. The HTTP backend serves one connection at a time, with 8 KiB
headers, a ten-second overall request deadline and two-second idle reads.
It is for a small private pair behind a TLS proxy, not public internet ingress.

## Worker boundary

`platform-jobs worker-once` polls, runs a pinned suite in an owner-prepared
clean checkout at the exact head, validates and signs its receipt, then sends
the bundle. The explicit head-to-checkout map is the execution allowlist.
Unknown heads are retained pending provisioning; the worker never fetches
credentials, clones arbitrary URLs or runs commands supplied in a request.
`worker-loop --interval-s 15` retains claims and stops after three consecutive
failed ticks. Stop with the process supervisor or Ctrl+C.

The native profile supports owner-trusted Windows/Linux code only. The Docker
profile uses a digest-pinned, preloaded Linux SDK image on an explicitly
selected rootless Docker daemon with cgroup v2. Jobs receive a read-only source
checkout and runner configuration, one writable job directory, bounded tmpfs,
2 CPUs, 4 GiB memory and 256 processes. They have no network, privileges,
capabilities, daemon socket, controller token or signing-key mount. The
controller signs after host-side receipt validation. A private 0700 worker
spool parent protects the writable job mount from other host users.

This is still `trusted-local-v1`, not certified hostile public-fork execution.
The candidate can influence its own test output; signing does not prove code
is honest. Containers share a kernel. Native execution is not a process-tree
sandbox. Use separate worker service accounts and a dedicated job daemon, not
the daemon hosting persistent Git/artifact services. Docker cleanup failures
require operator inspection before recovery. No production deployment or
cross-platform success is implied by the deployment recipe.

See [operation and explicit configurations](../platform/verification/README.md).
