#!/usr/bin/env python3
"""Local signed attestations (lane C100): seat identity without GitHub accounts.

A seat signs the canonical JSON of its attestation with its own SSH key (OpenSSH ``ssh-keygen -Y``,
the same mechanism git uses for commit signing). Anyone can verify offline against the public keys in
``governance/identities.json``; no network and no GitHub identity is involved.

  attest_sig.py sign   --record REC.json --key PRIVATE_KEY      prints the two fenced blocks to paste
  attest_sig.py verify --record REC.json --sig SIG_FILE --seat SEAT --identities governance/identities.json

The signed message is ``canonical(record)``: compact JSON with sorted keys, UTF-8. The namespace is
``aaroneous-attestation`` so a signature made for another purpose cannot be replayed here.
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import markdown_fences  # noqa: E402

NAMESPACE = "aaroneous-attestation"


def canonical(record):
    return json.dumps(record, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def pairs(body):
    """Yield (raw_json_text, signature_text_or_None) for each attestation block in a comment.

    Only a block a reader sees counts (see markdown_fences): an attestation inside an example fence or an HTML comment is
    not a posted verdict. CRLF line endings, which comments written on Windows arrive with, are normalised there."""
    found = markdown_fences.blocks(body)
    for index, (label, raw, _) in enumerate(found):
        if label != "attestation":
            continue
        signature = None
        if index + 1 < len(found) and found[index + 1][0] == "attestation-sig" and found[index + 1][2]:
            signature = found[index + 1][1].strip() or None
        yield raw, signature


def allowed_signers(identities):
    """Build an OpenSSH allowed_signers file from identities data: seat -> keys."""
    lines = []
    for seat, entry in identities.get("seats", {}).items():
        keys = entry.get("keys", []) if isinstance(entry, dict) else []
        for key in keys:
            parts = key.split()
            if len(parts) >= 2:
                lines.append(f'{seat} namespaces="{NAMESPACE}" {parts[0]} {parts[1]}')
    return "\n".join(lines) + ("\n" if lines else "")


def run(args, data=None):
    return subprocess.run(args, input=data, capture_output=True, check=False)


def sign(message, key_path):
    """Sign bytes with an SSH private key; returns the armored signature text."""
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "msg")
        with open(path, "wb") as handle:
            handle.write(message)
        result = run(["ssh-keygen", "-Y", "sign", "-f", key_path, "-n", NAMESPACE, path])
        if result.returncode != 0:
            raise RuntimeError("ssh-keygen sign failed: " + result.stderr.decode("utf-8", "replace").strip())
        with open(path + ".sig", encoding="utf-8") as handle:
            return handle.read().strip()


def verify(message, signature, seat, allowed_text):
    """True only if `signature` over `message` was made by a key listed for `seat`."""
    if not signature or not allowed_text.strip():
        return False
    with tempfile.TemporaryDirectory() as tmp:
        allowed = os.path.join(tmp, "allowed")
        sigfile = os.path.join(tmp, "sig")
        with open(allowed, "w", encoding="utf-8") as handle:
            handle.write(allowed_text)
        with open(sigfile, "w", encoding="utf-8") as handle:
            handle.write(signature + "\n")
        result = run(
            ["ssh-keygen", "-Y", "verify", "-f", allowed, "-I", seat, "-n", NAMESPACE, "-s", sigfile],
            message,
        )
        return result.returncode == 0


def blocks(record, signature):
    fence = "`" * 3
    return (
        f"{fence}attestation\n{json.dumps(record, sort_keys=True, indent=1)}\n{fence}\n"
        f"{fence}attestation-sig\n{signature}\n{fence}\n"
    )


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("sign")
    s.add_argument("--record", required=True)
    s.add_argument("--key", required=True)
    v = sub.add_parser("verify")
    v.add_argument("--record", required=True)
    v.add_argument("--sig", required=True)
    v.add_argument("--seat", required=True)
    v.add_argument("--identities", required=True)
    args = parser.parse_args(argv)
    with open(args.record, encoding="utf-8") as handle:
        record = json.load(handle)
    if args.cmd == "sign":
        print(blocks(record, sign(canonical(record), args.key)))
        return 0
    with open(args.identities, encoding="utf-8") as handle:
        identities = json.load(handle)
    with open(args.sig, encoding="utf-8") as handle:
        signature = handle.read().strip()
    ok = verify(canonical(record), signature, args.seat, allowed_signers(identities))
    print("VERIFIED" if ok else "NOT VERIFIED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
