# Worktree and branch hygiene (proposal, owner sign-off needed before any removal)

Why: on 2026-10-02 the devtools repo had 61 worktrees against about 40 remote branches. Thirteen
review worktrees and a few work worktrees were dirty (one untracked line each, one checkout with 34
changed lines). Nothing in the repo sweeps merged or abandoned worktrees or branches.

## What this adds now
`scripts/worktree_inventory.py <repo>` is read-only. For each worktree it prints clean or dirty and
whether its HEAD is already in `origin/main`. It removes and writes nothing.

## Proposed rules (NOT adopted; owner decides)
1. Only the agent that created a worktree removes it, and only when it is clean and its branch is
   merged into `origin/main` or its PR is closed.
2. A dirty worktree is never removed by anyone but its owner. The sweep lists it and moves on.
3. A review worktree is disposable once its packet is posted; its owner removes it then.
4. A sweep may be delegated to a review-biased model, which runs the inventory, posts the list as a
   PR comment, and removes only entries the owning agent has confirmed.
5. Remote branches already merged into `main` may be deleted by the merge steward at merge time.

## Not decided here
Who the merge steward is, how often the sweep runs, and whether closed-PR branches are deleted
automatically.
