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
import re
import subprocess
import sys
import tempfile

NAMESPACE = "aaroneous-attestation"
PAIR = re.compile(
    r"```attestation[ \t]*\n(.*?)\n```(?:[ \t]*\n+```attestation-sig[ \t]*\n(.*?)\n```)?",
    re.DOTALL,
)


def canonical(record):
    return json.dumps(record, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def pairs(body):
    """Yield (raw_json_text, signature_text_or_None) for each attestation block in a comment.

    Comments written on Windows arrive with CRLF line endings; normalise them first, otherwise the fence
    patterns (which expect a bare newline) silently skip the block and the verdict never counts."""
    body = body.replace("\r\n", "\n").replace("\r", "\n")
    for raw, signature in PAIR.findall(body):
        yield raw, (signature or "").strip() or None


SEAT_NAME = re.compile(r"[a-z][a-z0-9_-]{0,31}")
KEY_TYPES = ("ssh-ed25519", "ssh-rsa", "ecdsa-sha2-nistp256", "ecdsa-sha2-nistp384", "ecdsa-sha2-nistp521",
             "sk-ssh-ed25519@openssh.com", "sk-ecdsa-sha2-nistp256@openssh.com")
KEY_BODY = re.compile(r"[A-Za-z0-9+/]+={0,2}")


def valid_key(key):
    """True for `<key type> <base64>` (an optional comment after it is ignored) with a known key type."""
    parts = key.split() if isinstance(key, str) else []
    return len(parts) >= 2 and parts[0] in KEY_TYPES and KEY_BODY.fullmatch(parts[1]) is not None


def plain_login(value):
    """A GitHub login as written in the file: a non-empty string with no surrounding or inner whitespace."""
    return isinstance(value, str) and bool(value) and value == value.strip() and not any(c.isspace() for c in value)


FAMILY_NAME = re.compile(r"[a-z][a-z0-9-]{0,31}")


def validate_identities(data):
    """Problems with an identities file, as sentences; an empty list means it is usable. The file decides who counts as
    an independent reviewer, so a malformed entry must stop the tool rather than be read charitably. Probes on the old
    loader: `"logins": "bob"` mapped the single letters b and o to the seat; one login in two seats went to whichever
    came last; the shared owner login could be mapped to a seat; a seat named "claude\nmimo" put a key under the
    principal `mimo` in the allowed_signers file; a key such as "cert-authority ssh-ed25519" became an option."""
    problems = []
    if not isinstance(data, dict) or not isinstance(data.get("seats"), dict):
        return ["the file must be a JSON object with a \"seats\" object"]
    owners = data.get("owner", [])
    if not isinstance(owners, list) or not all(plain_login(o) for o in owners):
        # "Aarogaming " would not match the login "Aarogaming" in the exclusion set below (Codex P2 on #123)
        problems.append('"owner" must be a list of plain GitHub logins (no spaces)')
        owners = []
    owner_set = {o.lower() for o in owners}
    seen = {}
    for seat, entry in data["seats"].items():
        if not isinstance(seat, str) or SEAT_NAME.fullmatch(seat) is None:
            problems.append(f"seat name {seat!r} must be lower-case letters, digits, '-' or '_', starting with a letter")
            continue
        logins = entry.get("logins", []) if isinstance(entry, dict) else entry
        keys = entry.get("keys", []) if isinstance(entry, dict) else []
        if not isinstance(logins, list):
            problems.append(f"{seat}: logins must be a list, not {type(logins).__name__}")
            logins = []
        if not isinstance(keys, list):
            problems.append(f"{seat}: keys must be a list, not {type(keys).__name__}")
            keys = []
        for login in logins:
            if not plain_login(login):
                problems.append(f"{seat}: login {login!r} is not a plain GitHub login")
                continue
            if login.lower() in owner_set:
                problems.append(f"{seat}: {login!r} is the shared owner login, which is deliberately not a seat")
            if login.lower() in seen and seen[login.lower()] != seat:
                problems.append(f"login {login!r} is listed for both {seen[login.lower()]} and {seat}")
            seen.setdefault(login.lower(), seat)
        for key in keys:
            if not valid_key(key):
                problems.append(f"{seat}: key {str(key)[:40]!r} is not '<known key type> <base64>'")
        profile = entry.get("profile") if isinstance(entry, dict) else None
        if profile is not None:
            if not isinstance(profile, dict):
                problems.append(f"{seat}: profile must be an object, not {type(profile).__name__}")
            else:
                for field, value in profile.items():
                    if not isinstance(value, str) or not value.strip() or len(value) > 80 or not value.isprintable():
                        problems.append(f"{seat}: profile.{field} must be a short printable string")
                family = profile.get("family")
                if isinstance(family, str) and FAMILY_NAME.fullmatch(family) is None:
                    problems.append(f"{seat}: profile.family {family!r} must be lower-case letters, digits or '-', starting with a letter (at most 32)")
    table = data.get("model_families") if isinstance(data, dict) else None
    if table is not None:
        if not isinstance(table, dict):
            problems.append("model_families must be an object of model-name stem -> family")
        else:
            for stem, family in table.items():
                if not isinstance(stem, str) or re.fullmatch(r"[a-z][a-z0-9.-]{0,31}", stem) is None:
                    problems.append(f"model_families: stem {str(stem)[:40]!r} must be lower-case letters, digits, '.' or '-'")
                if not isinstance(family, str) or FAMILY_NAME.fullmatch(family) is None:
                    problems.append(f"model_families: family for {str(stem)[:40]!r} must be a plain lower-case label")
    return problems


def allowed_signers(identities):
    """Build an OpenSSH allowed_signers file from identities data: seat -> keys. Only well-formed seats and keys are
    written (see `validate_identities`); anything else is skipped, so a seat name or key cannot add principals or
    options to the file."""
    lines = []
    for seat, entry in identities.get("seats", {}).items():
        if not isinstance(seat, str) or SEAT_NAME.fullmatch(seat) is None:
            continue
        keys = entry.get("keys", []) if isinstance(entry, dict) else []
        for key in keys if isinstance(keys, list) else []:
            if valid_key(key):
                parts = key.split()
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
    problems = validate_identities(identities)
    if problems:
        print(f"{args.identities} is not a usable identities file:\n  " + "\n  ".join(problems), file=sys.stderr)
        print("NOT VERIFIED")
        return 2
    with open(args.sig, encoding="utf-8") as handle:
        signature = handle.read().strip()
    ok = verify(canonical(record), signature, args.seat, allowed_signers(identities))
    print("VERIFIED" if ok else "NOT VERIFIED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
