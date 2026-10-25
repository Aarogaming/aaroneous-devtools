# Pair rollout evidence and next steps

The first actual Windows/Ubuntu HTTPS smoke completed on 2026-10-13 under
`C136/codex-platform-pair-smoke-v1`. It used the tested build candidates from
PR #206, not production gate enrollment. The service contribution retains its
original full identity `C135/codex-platform-job-service-v1`; the separate C135
allocation collision with PR #205 remains an explicit coordination issue.

## What ran

A private, temporary coordinator ran on Ubuntu loopback behind a restricted
Caddy container bound to the explicitly selected tailnet interface. Windows
submitted one controlled `pair-smoke` suite over certificate-verified HTTPS.
The suite contained only `git --version`, with one card for `linux-x86_64`
and one for `windows-x86_64`, both bound to fixture commit
`274b431788c1a9e9d4a15e082c44d2b7689e83b5` and manifest digest
`e79f636377f258967a3250d382d7bc9669c13d83ae37fbba02e7e4aba5e70dd3`.

An Ubuntu polling process and a Windows polling process independently claimed,
executed, signed and uploaded their own platform result. The Ubuntu native
coordinator/poller ran in separate OS sessions; SSH was used for provisioning
and evidence retrieval, while job dispatch and receipt uploads used HTTPS.
Both receipts passed the existing spool validator on Ubuntu and were verified
again on Windows against the retained signatures, public signer registry,
pinned fixture suite and exact log bytes.

Restarting the coordinator and resubmitting the same suite retained exactly
two completed cards, each at attempt 1. Wrong tokens, a wrong CA certificate
and a worker trying to issue a job were refused. Both shadow reports had zero
verification problems and `local_would_integrate: false`; missing independent
review and other integration facts were not supplied as approvals.

## Windows private-CA compatibility

The installed Schannel clients rejected Caddy's private CA because its
certificate revocation status was unavailable. The smoke selected an explicit
portable curl 8.22.0_3 x64 build from the
[official curl Windows distribution](https://curl.se/windows/), verified against
its published archive SHA-256:
`f38b0ac2e3a280fd414b607c874a21e6dd4d1ed0e2ae622afbf0020917446561`.
This build uses LibreSSL and successfully verified the explicit private CA
and endpoint. The wrong-CA test still failed. Neither `--insecure`, disabled
certificate verification, nor a global Windows trust-store change was used.

Choose an approved client compatible with the deployment's private PKI and
revocation policy; do not assume every Windows curl executable has the same
TLS backend. Schannel's CRL behavior and custom CA handling are documented in
[curl's certificate guidance](https://curl.se/docs/sslcerts.html). Keep the
client version, license and binary/archive hashes in the operational receipt.
The portable client remains an explicitly provisioned operational tool, not
a Cargo or Aaroneous product dependency.

## Retained evidence and cleanup

The owner-retained `tmp/platform-pair-smoke-20261013/verification.json` records
the source executable hashes, actual platform receipts, commands, negative
checks, public CA fingerprint and cleanup. `remote-evidence.tar.gz` contains
only selected nonsecret spool artifacts, signatures, logs and public signer
material; every payload was checked against its retained SHA-256 manifest.
Fixture signing keys and bearer tokens are excluded from this archive.

The temporary Windows poller, bootstrap TLS transfer, Ubuntu poller,
coordinator and named ingress container were stopped. Fixture bearer/signing
keys and disposable TLS private keys were removed. No permanent service,
firewall rule, global certificate trust, Tailscale daemon, real principal
enrollment or live integration gate was changed. Public evidence and fixture
repository objects remain available for review.

## Remaining rollout gates

This smoke verifies cross-machine service operation with harmless owner-trusted
native jobs. It does not certify Foundation on the actual Ubuntu machine,
the dedicated rootless Docker executor, public-fork isolation, LAN/DNS failover,
backup/restore or production service supervision. The prior full Foundation
Linux gate ran in Docker Desktop, and remains separately attributed.

Next qualify the dedicated rootless job daemon and approved SDK image, run
Foundation at the exact intended code/base revisions on both real workers,
reconcile contribution IDs and independent reviews, and enroll real node keys
through the trusted policy. Establish bounded supervision and retained-state
recovery before installing persistent services. Keep the collector in shadow
mode until these evidence and authority gates are satisfied.
