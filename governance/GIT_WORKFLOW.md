# Git Workflow

## Branches

`main` is the integrated, releasable foundation branch. Use short-lived `feat/`, `fix/`, `docs/`,
or `chore/` branches for material changes. Keep migration history separate from active tooling.

## Pull requests

A pull request states its purpose, contract impact, verification, and any external support
requirements. Prefer one reviewer for code that executes commands or handles paths. Use GitHub
branch protection on `main` to require the Foundation CI workflow before merging.

## Hobby cadence

Keep maintenance sustainable: make small changes, merge completed work often, and use the daily
automation only for evidence changes. A weekly review should reconcile queue ownership and
Dependabot updates; a monthly review should retire stale experiments and reconsider scope.
