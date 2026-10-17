# Seat identity setup (lane C100)

Why: every seat posts under one shared GitHub login, so a pasted attestation block is indistinguishable
from a real one. Unsigned coverage is therefore "attested-only". A signed attestation proves the seat
holding the private key made the verdict. This is the owner-run setup; no network service is involved.

## One-time, per seat (run on the machine where that seat works)

```bash
# Generate a key that is used only for attestations. No passphrase is the simplest for an unattended seat;
# add one if the seat runs interactively.
ssh-keygen -t ed25519 -f ~/.ssh/aaroneous_attest_SEAT -C "SEAT attestation key" -N ""
cat ~/.ssh/aaroneous_attest_SEAT.pub
```

Replace `SEAT` with `claude`, `opencode`, `codex` or `gemini` (the OpenCode seat; MiMo is a model it has run, and `mimo` is still accepted as the same seat and remains the steward's internal id). Keep the private key on that seat's machine only.

## Register the public key (a reviewed PR, owner-approved)

Add the **public** key line (`ssh-ed25519 AAAA... comment`) to that seat's `keys` list in
`governance/identities.json`:

```json
"opencode": {"logins": [], "keys": ["ssh-ed25519 AAAA... opencode attestation key"]}
```

Never put a private key, passphrase or token in this repository.

## Signing a verdict

```bash
python scripts/attest_sig.py sign --record verdict.json --key ~/.ssh/aaroneous_attest_SEAT
```

`verdict.json` is the same record as an unsigned attestation block (repo, pr, head, base_sha, scope,
reviewer, implementers, verdict). The command prints two fenced blocks (`attestation` and
`attestation-sig`); paste both into the review comment. The signature covers the canonical JSON of the
record under the namespace `aaroneous-attestation`, so it cannot be replayed for another purpose.

## Checking

```bash
python scripts/review_coverage.py --repo OWNER/REPO --repo-dir . \
  --identities governance/identities.json --pr N            # reports trust level per verdict
python scripts/review_coverage.py ... --require-signatures   # unsigned verdicts stop counting
```

Trust levels: `signed` (verifies against the registered key), `authenticated` (formal GitHub review by a
mapped login), `attested-only` (unsigned block; spoofable while seats share a login).

## Switching enforcement on

1. All four seats have registered keys and have posted at least one signed verdict.
2. The owner turns on `--require-signatures` in the steward and merge gate.

Until then unsigned verdicts still count, and a pasted block can be mistaken for a seat's verdict.
