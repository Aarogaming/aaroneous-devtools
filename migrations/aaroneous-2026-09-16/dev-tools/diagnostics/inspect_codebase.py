#!/usr/bin/env python3
"""
inspect_codebase.py
Audits first-party Cargo.toml files, dependencies, workspace memberships, and detects unlinked crates in Aaroneous.
"""

import os
import sys

def main():
    workspace_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
    print("=" * 60)
    print("  Aaroneous First-Party Workspace Auditor")
    print(f"  Workspace Root: {workspace_root}")
    print("=" * 60)

    # 1. Scan for first-party Cargo.toml files (ignoring fabrication data & history docs)
    cargo_files = []
    ignored_dirs = {
        "target", "node_modules", ".git", "cache", ".cargo", ".zed", 
        ".opencode", "dist", "fabrication", "history", "wasm_deprecated"
    }
    
    for root, dirs, files in os.walk(workspace_root):
        dirs[:] = [d for d in dirs if d not in ignored_dirs]
        if "Cargo.toml" in files:
            rel_path = os.path.relpath(os.path.join(root, "Cargo.toml"), workspace_root)
            cargo_files.append(rel_path)

    print(f"\n[+] Discovered {len(cargo_files)} First-Party Cargo.toml manifests:")
    for cf in sorted(cargo_files):
        print(f"    - {cf}")

    # 2. Check root Cargo.toml workspace members
    root_cargo_path = os.path.join(workspace_root, "Cargo.toml")
    if os.path.exists(root_cargo_path):
        with open(root_cargo_path, "r", encoding="utf-8") as f:
            lines = f.readlines()
        
        in_members = False
        declared_members = []
        for line in lines:
            stripped = line.strip()
            if "members = [" in stripped:
                in_members = True
                continue
            if in_members:
                if "]" in stripped:
                    in_members = False
                    break
                member = stripped.strip('", ').replace('/', '\\')
                if member:
                    declared_members.append(member)

        print(f"\n[+] Root Cargo.toml declares {len(declared_members)} workspace members:")
        for m in declared_members:
            print(f"    - {m}")

        # Find crates NOT in workspace members
        missing_from_workspace = []
        for cf in cargo_files:
            if cf == "Cargo.toml" or "MaelstromUI" in cf:
                continue
            crate_dir = os.path.dirname(cf)
            if crate_dir not in declared_members and crate_dir.replace('\\', '/') not in [d.replace('\\', '/') for d in declared_members]:
                missing_from_workspace.append(crate_dir)

        if missing_from_workspace:
            print(f"\n[!] WARNING: {len(missing_from_workspace)} first-party crates are NOT listed in root Cargo.toml workspace members:")
            for mc in sorted(missing_from_workspace):
                print(f"    [MISSING MEMBER] {mc}")
        else:
            print("\n[OK] All first-party crates are properly registered in workspace members.")

    print("\n" + "=" * 60)
    print("  Audit Complete.")
    print("=" * 60)

if __name__ == "__main__":
    main()
