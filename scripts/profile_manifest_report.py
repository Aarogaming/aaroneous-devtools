#!/usr/bin/env python3
"""Read-only report: which product crates declare a Cratify compliance profile (C89).

Compares the profile table in the product checkout's AGENTS.md section 1 with each crate's
`[package.metadata.cratify] profile` manifest key. Writes nothing and edits nothing in the
product checkout; adding the keys is a product-spec change that needs owner sign-off.

Usage: profile_manifest_report.py <product-checkout-dir>
Exits 0 always (a report, not a gate); prints markdown to stdout.
"""

import re
import sys
import tomllib
from pathlib import Path

PROFILES = ("kernel", "control", "presentation", "tooling")
ROW = re.compile(r"^\s*\|\s*`(kernel|control|presentation|tooling)`\s*\|(.*?)\|", re.IGNORECASE)
BACKTICKED = re.compile(r"`([^`]+)`")
SKIP_DIRS = {"target", ".git", "node_modules", "legacy_staging"}


def expected_profiles(agents_text: str) -> dict[str, str]:
    """Crate name or path -> profile, from the AGENTS.md profile table."""
    expected: dict[str, str] = {}
    for line in agents_text.splitlines():
        match = ROW.match(line)
        if match:
            for name in BACKTICKED.findall(match.group(2)):
                expected[name.strip()] = match.group(1).lower()
    return expected


def manifests(root: Path) -> list[Path]:
    found = []
    for path in sorted(root.rglob("Cargo.toml")):
        if any(part in SKIP_DIRS or part.startswith(".") for part in path.relative_to(root).parts):
            continue
        found.append(path)
    return found


def package_info(manifest: Path) -> tuple[str, str | None] | None:
    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError):
        return None
    package = data.get("package")
    if not isinstance(package, dict) or "name" not in package:
        return None
    declared = package.get("metadata", {}).get("cratify", {}).get("profile")
    return package["name"], declared


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: profile_manifest_report.py <product-checkout-dir>", file=sys.stderr)
        return 2
    root = Path(sys.argv[1])
    agents = root / "AGENTS.md"
    if not agents.is_file():
        print(f"no AGENTS.md under {root}", file=sys.stderr)
        return 2
    expected = expected_profiles(agents.read_text(encoding="utf-8"))
    rows = []
    for manifest in manifests(root):
        info = package_info(manifest)
        if info is None:
            continue
        name, declared = info
        rel = manifest.parent.relative_to(root).as_posix()
        want = expected.get(name) or expected.get(rel)
        rows.append((name, rel, want, declared))

    matched = {r[0] for r in rows if r[2]}
    print("# Cratify profile manifest report (read-only)\n")
    print(f"Packages found: {len(rows)}; listed in AGENTS.md table: {len(matched)}; "
          f"declaring a profile: {sum(1 for r in rows if r[3])}\n")
    print("| package | path | AGENTS.md profile | manifest profile | status |")
    print("|---|---|---|---|---|")
    for name, rel, want, declared in rows:
        if declared and want and declared != want:
            status = "MISMATCH"
        elif declared and not want:
            status = "declared, not in AGENTS.md"
        elif declared:
            status = "ok"
        elif want:
            status = "missing key"
        else:
            status = "unclassified"
        print(f"| {name} | {rel} | {want or '-'} | {declared or '-'} | {status} |")
    names = {r[0] for r in rows} | {r[1] for r in rows}
    stale = sorted(k for k in expected if k not in names)
    if stale:
        print("\nAGENTS.md names with no matching package: " + ", ".join(stale))
    print("\nProposed key for each 'missing key' row:\n")
    print("```toml\n[package.metadata.cratify]\nprofile = \"<AGENTS.md profile>\"\n```")
    return 0


if __name__ == "__main__":
    sys.exit(main())
