#!/usr/bin/env python3
"""Inspect an explicit Git revision for private artifacts and disclosure locations.

This is a publication inventory, not a replacement for Gitleaks. Reports never
include matched credential or personal-path values. No archive is extracted.
"""
import argparse
import io
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import tarfile

MAX_FILE_BYTES = 20 * 1024 * 1024
PRIVATE_ROOTS = {"tmp", "logs", "artifacts", "reports", "config", "queue",
                 ".antigravity", ".gemini", "worker-jobs-archive"}
CREDENTIAL = re.compile(rb"\bsk-lm-[A-Za-z0-9_:.-]{16,}\b")
PERSONAL_PATH = re.compile(rb"(?i)(?:[a-z]:[\\/]+Users[\\/]+(?!example\b|user\b|<)[^\\/\s`'\"]+|/home/(?!example\b|user\b)[^/\s`'\"]+)")


def inspect_bytes(name, data):
    findings = []
    path = PurePosixPath(name)
    if path.parts and (path.parts[0] in PRIVATE_ROOTS or path.suffix.lower() in {".pem", ".key"}
                       or path.name == ".env" or path.name.startswith(".env.") and path.name != ".env.example"):
        findings.append({"severity": "blocker", "kind": "private-artifact", "file": name})
    for number, line in enumerate(data.splitlines(), 1):
        for pattern, severity, kind in [(CREDENTIAL, "blocker", "credential"),
                                        (PERSONAL_PATH, "review", "personal-path")]:
            if pattern.search(line):
                findings.append({"severity": severity, "kind": kind, "file": name, "line": number})
    return findings


def inspect_archive(name, data):
    findings = []
    total = 0
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:*") as archive:
        for index, member in enumerate(archive):
            total += member.size
            if index >= 4096 or total > MAX_FILE_BYTES:
                raise ValueError(f"archive inspection limit exceeded: {name}")
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts or member.issym() or member.islnk():
                findings.append({"severity": "blocker", "kind": "unsafe-archive-member", "file": name})
                continue
            if member.isfile():
                stream = archive.extractfile(member)
                if stream is None:
                    raise ValueError(f"unreadable archive member: {name}")
                for finding in inspect_bytes(member.name, stream.read(MAX_FILE_BYTES + 1)):
                    finding["file"] = f"{name}!{member.name}"
                    findings.append(finding)
    return findings


def git(repo, *args):
    return subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True).stdout


def inventory(repo, revision):
    if not repo.is_absolute() or not repo.is_dir():
        raise ValueError("repo must be an explicit existing absolute directory")
    # Resolve to an immutable object first; never pass arbitrary revision text
    # into later Git commands as an option or revision expression.
    head = git(repo, "rev-parse", "--verify", "--end-of-options", revision + "^{commit}").decode().strip()
    entries = git(repo, "ls-tree", "-r", "-z", head).split(b"\0")
    findings = []
    files = 0
    archives = 0
    for entry in filter(None, entries):
        metadata, name = entry.split(b"\t", 1)
        mode, kind, oid = metadata.split()
        name = name.decode("utf-8", "strict")
        if kind != b"blob" or mode == b"120000":
            findings.append({"severity": "blocker", "kind": "non-regular-git-entry", "file": name})
            continue
        if int(git(repo, "cat-file", "-s", oid.decode())) > MAX_FILE_BYTES:
            raise ValueError(f"file inspection limit exceeded: {name}")
        data = git(repo, "cat-file", "blob", oid.decode())
        files += 1
        findings.extend(inspect_bytes(name, data))
        if name.endswith((".tar", ".tar.gz", ".tgz")):
            archives += 1
            findings.extend(inspect_archive(name, data))
    return {"schema_version": 1, "revision": head, "files_inspected": files,
            "archives_inspected": archives, "findings": findings,
            "blockers": sum(f["severity"] == "blocker" for f in findings),
            "privacy_review_locations": sum(f["severity"] == "review" for f in findings)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not args.output.is_absolute() or not args.output.parent.is_dir():
        parser.error("output must have an explicit existing absolute parent")
    report = inventory(args.repo, args.revision)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"{report['files_inspected']} files, {report['archives_inspected']} archives; "
          f"{report['blockers']} blockers, {report['privacy_review_locations']} privacy review locations")
    return int(report["blockers"] > 0)


if __name__ == "__main__":
    raise SystemExit(main())
