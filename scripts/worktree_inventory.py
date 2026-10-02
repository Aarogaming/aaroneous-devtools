#!/usr/bin/env python3
"""Read-only inventory of git worktrees and remote branches (hygiene lane).

Prints, per worktree: path, branch, clean/dirty, and whether its branch tip is already
contained in origin/main (a removal CANDIDATE only when also clean). Removes nothing, writes
nothing, never modifies any worktree. Dirty worktrees are listed but never recommended.

Usage: worktree_inventory.py <repo-dir>
"""

import os
import subprocess
import sys


def git(repo: str, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", repo, *args], capture_output=True, text=True, check=False
    ).stdout


def parse_worktrees(porcelain: str) -> list[dict[str, str]]:
    rows = []
    for block in porcelain.strip().split("\n\n"):
        row: dict[str, str] = {}
        for line in block.split("\n"):
            key, _, value = line.partition(" ")
            row[key] = value
        if row:
            rows.append(row)
    return rows


def classify(is_clean: bool, merged_into_main: bool, is_primary: bool) -> str:
    if is_primary:
        return "primary-checkout (never a candidate)"
    if not is_clean:
        return "DIRTY (leave alone; owner must resolve)"
    if merged_into_main:
        return "candidate: clean and branch already in origin/main"
    return "keep: clean but branch not in origin/main"


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: worktree_inventory.py <repo-dir>", file=sys.stderr)
        return 2
    repo = sys.argv[1]
    rows = parse_worktrees(git(repo, "worktree", "list", "--porcelain"))
    primary = os.path.normcase(os.path.abspath(repo))
    print(f"{len(rows)} worktrees\n")
    counts: dict[str, int] = {}
    for row in rows:
        path = row.get("worktree", "")
        branch = row.get("branch", "(detached)").replace("refs/heads/", "")
        head = row.get("HEAD", "")
        clean = not git(path, "status", "--porcelain").strip()
        merged = bool(head) and subprocess.run(
            ["git", "-C", repo, "merge-base", "--is-ancestor", head, "origin/main"],
            capture_output=True,
        ).returncode == 0
        label = classify(clean, merged, os.path.normcase(os.path.abspath(path)) == primary)
        counts[label.split(":")[0].split(" ")[0]] = counts.get(label.split(":")[0].split(" ")[0], 0) + 1
        print(f"{os.path.basename(path)}\t{branch}\t{label}")
    print("\nsummary:", counts)
    return 0


if __name__ == "__main__":
    sys.exit(main())
