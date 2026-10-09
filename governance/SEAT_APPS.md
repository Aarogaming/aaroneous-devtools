# Seat GitHub Apps: each seat posts as itself (lane C112)

Today every seat posts through the owner's one token, so GitHub shows the owner login for every comment, review and
pull request. A **GitHub App per seat** posts as `<app>[bot]` instead. This page says how to register one, how a
seat uses it, and what is still unproven. It complements, and does not replace, the signed attestations in
[IDENTITY_SETUP.md](IDENTITY_SETUP.md): a signature proves which seat made a verdict whoever posted it; an App makes
GitHub itself show who posted.

Status as of 2026-10-09: the helper exists and is unit-tested (23 tests); no seat App has been registered with it yet.

## One command

```bash
python scripts/seat_app_setup.py setup --seat claude      # or --seat all for every seat in turn
```

That is the whole setup. The command opens your browser; **you click GitHub's own accept pages and nothing else**:

1. **Create GitHub App** on github.com (the helper's local page forwards you there).
2. **Install**, on the next page GitHub shows (choose only the repository).

The command carries on by itself between the clicks: it exchanges GitHub's one-time code for the App id and private key,
waits for the installation to appear, saves it, mints a one-hour token and proves that token reaches the repository.
It prints one `done` line per seat. It can be re-run at any point and resumes where the last run stopped (an already
registered seat skips registration; an already installed one skips the wait).

GitHub will not create an App without a person confirming it, so those two clicks cannot be removed. Agents must not
use a device-code login for this either: that token acts as the person, so posts would still show the owner.

What it sets up, per seat (`claude`, `codex`, `gemini`, `mimo`):

- An App named `aaroneous-<seat>-seat`, private, asking for contents, pull requests and issues write, and checks and
  statuses read. It does **not** ask for `workflows`; add `--allow-workflows` only if that seat must change
  `.github/workflows`, and say so in the PR.
- Credentials in `~/.steward/seats/<seat>/` (`app.json`, `private-key.pem`), never in the repository (`--root` moves
  them). The webhook and client secrets GitHub returns are discarded. Back the key up like any private key; losing it
  means registering again.
- The steps are also separate commands if you need one on its own: `register`, `link`, `token [--env]`, `check`.

## Who runs it, and when

Each seat runs its own: `python scripts/seat_app_setup.py setup --seat <your seat>`, once, the next time it gets a turn on
the machine where it works. The owner makes the two GitHub clicks, so ask for them in the same message that says you are
starting. `--seat all` is for an owner setting up several seats in one sitting. Re-running is safe: it resumes where it
stopped, and a finished seat only re-checks.

Limit to know: seats that share one machine and one operating-system user can read each other's
`~/.steward/seats/<seat>/` folders (the private keys), so separation between them holds by convention only. Real isolation
needs separate operating-system users or separate machines. This page's onboarding steps are summarised in
[OPERATOR_GUIDE.md](OPERATOR_GUIDE.md) section 3 once that guide is on `main`.

## Use it

```bash
export GH_CONFIG_DIR="$HOME/.steward/seats/claude/gh"                 # a gh config that never holds the owner's login
export GH_TOKEN="$(python scripts/seat_app_setup.py token --seat claude)"   # lasts one hour; mint a new one when needed
gh pr comment 123 --body-file note.md                                  # now posts as the App
git push "https://x-access-token:${GH_TOKEN}@github.com/OWNER/REPO.git" HEAD:refs/heads/my-branch
```

Set `git config user.name`/`user.email` per seat as well, and sign commits with the seat key, so commit authorship
matches. Never print the token into a log or a comment.

## After the Apps exist

1. Add each App's login (`aaroneous-<seat>-seat[bot]`) to that seat's `logins` list in `governance/identities.json`
   (a PR): formal reviews by that login then count as `authenticated`.
2. The owner may later require one approving review on `main`. GitHub does not let an identity approve its own pull
   request, so author and reviewer must then differ natively.

## Not verified (test on one seat before relying on it)

- The manifest field names and permission names were written from GitHub's documentation, not from a live
  registration; the first `register` run is the test.
- The JWT `iss` claim is the numeric App id; GitHub documents either the App id or the client id.
- Whether an App's approving review counts toward a required-review rule, and how `identities.json` should spell the
  `[bot]` login, are untested. Try them on a throwaway PR first.
- The helper has been run only up to its landing page and in tests with faked GitHub replies; no real App has been created with it, so the poll for the installation and the final repository check are untested against GitHub.

## Revoking

An App can be suspended or deleted at Settings, Developer settings, GitHub Apps, which stops that seat posting at
once. Delete the seat's folder under `~/.steward/seats/` after.
