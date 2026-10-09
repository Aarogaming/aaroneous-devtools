# Seat GitHub Apps: each seat posts as itself (lane C112)

Today every seat posts through the owner's one token, so GitHub shows the owner login for every comment, review and
pull request. A **GitHub App per seat** posts as `<app>[bot]` instead. This page says how to register one, how a
seat uses it, and what is still unproven. It complements, and does not replace, the signed attestations in
[IDENTITY_SETUP.md](IDENTITY_SETUP.md): a signature proves which seat made a verdict whoever posted it; an App makes
GitHub itself show who posted.

Status as of 2026-10-09: the helper exists and is unit-tested; no seat App has been registered yet.

## What needs a person

GitHub will not create an App without a person confirming it. The helper therefore uses GitHub's manifest flow, and
the owner makes **two clicks per seat**: **Create GitHub App** (on github.com, after the helper opens its page), then
**Install** (choose only the repository). Everything else is done by the helper. Agents cannot create an App silently,
and they must not use a device-code login for this: that token acts as the person, so posts would still show the
owner.

## Register a seat (once per seat, on the machine where that seat works)

```bash
python scripts/seat_app_setup.py register --seat claude      # opens a local page; owner clicks Create GitHub App
python scripts/seat_app_setup.py link --seat claude          # after the owner installed it: saves the installation id
python scripts/seat_app_setup.py check --seat claude         # proves the token reaches the repository, as which App
```

- `register` serves one page on `127.0.0.1`, waits up to 15 minutes, and exchanges GitHub's one-time code (valid one
  hour) for the App id and private key. The webhook and client secrets in GitHub's reply are discarded.
- Seats: `claude`, `codex`, `gemini`, `mimo`. The App is named `aaroneous-<seat>-seat`, private, and asks for
  contents, pull requests and issues write, and checks and statuses read. It does **not** ask for `workflows`; add
  `--allow-workflows` only if that seat must change `.github/workflows`, and say so in the PR.
- Credentials are kept in `~/.steward/seats/<seat>/` (`app.json`, `private-key.pem`), never in the repository. Pass
  `--root` to keep them elsewhere. Back the key up like any private key; losing it means registering again.

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
- The helper has been run only up to its landing page; no real App has been created with it.

## Revoking

An App can be suspended or deleted at Settings, Developer settings, GitHub Apps, which stops that seat posting at
once. Delete the seat's folder under `~/.steward/seats/` after.
