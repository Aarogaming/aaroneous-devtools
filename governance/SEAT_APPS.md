# Seat GitHub Apps: each seat posts as itself (lane C114)

Every seat otherwise posts through the owner's one token, so GitHub shows the owner login for every comment, review
and pull request. A **GitHub App per seat** posts as `aaroneous-<seat>-seat[bot]` instead. This page is a runbook any
seat can follow on its own. It complements the signed attestations in [IDENTITY_SETUP.md](IDENTITY_SETUP.md): a
signature proves which seat made a verdict whoever posted it; an App makes GitHub itself show who posted. The wider
onboarding path is in `governance/OPERATOR_GUIDE.md` section 3 (once that guide is on `main`).

Status as of 2026-10-09: verified live for the claude seat (App created, installed, token reached the repository, a
comment posted as `aaroneous-claude-seat[bot]`). The other seats have not run it yet.

## Run it yourself (about five minutes)

**You need**, on the machine where your seat works:

- Python 3.10 or newer (`python --version`) and `openssl` on the path (`openssl version`; Git for Windows includes it).
- A checkout of this repository at current `main`, with `governance/identities.json` listing your seat.
- The repository owner signed in to GitHub in a browser **on this same machine**. The owner makes two clicks; you cannot
  make them for the owner, and no password or token is ever typed into the command.

**Tell the owner first**, so the clicks are expected. A message you can paste:

> I am starting GitHub App setup for the `<seat>` seat. A browser tab will open. Please click **Create GitHub App**, and
> on the next page click **Install** and choose only `aaroneous-devtools`. The command waits 15 minutes for the first
> click and 10 for the second.

**Then run:**

```bash
python scripts/seat_app_setup.py setup --seat <seat>
```

`<seat>` is your name in `governance/identities.json` (`claude`, `codex`, `gemini`, `opencode` (the steward still calls this seat `mimo` internally), `ubuntu-worker-01`, ...). What
you will see, and what the owner sees:

| Step | Terminal | Browser |
| --- | --- | --- |
| 1 | `[seat] Opening http://127.0.0.1:PORT/ ...` | a GitHub page for the App with its permissions: **Create GitHub App** |
| 2 | `[seat] created aaroneous-<seat>-seat (id N); credentials saved in ...` | forwarded to the install page |
| 3 | `[seat] waiting for you to click Install ...` | **Install**, repository access: only `aaroneous-devtools` |
| 4 | `[seat] installation N found and saved` | |
| 5 | `[seat] done: posts as aaroneous-<seat>-seat[bot]; token reaches Aarogaming/aaroneous-devtools` | |

If you see `done`, setup is finished. `--seat all` runs every registered seat in turn (for an owner setting up several at
once). The command is safe to re-run: it resumes where it stopped, and a finished seat only re-checks.

**Prove it posts as you,** with one harmless comment on a PR of your own, then read back the author:

```bash
export GH_CONFIG_DIR="$HOME/.steward/seats/<seat>/gh"      # a gh config that never holds the owner's login
export GH_TOKEN="$(python scripts/seat_app_setup.py token --seat <seat>)"      # lasts one hour
gh api repos/OWNER/REPO/issues/N/comments -f body='<seat> seat identity check' --jq '.user.login'
```

It must print `aaroneous-<seat>-seat[bot]`. If it prints the owner login, `gh` is using the owner's stored login: check
that `GH_CONFIG_DIR` and `GH_TOKEN` are set in the same shell.

**Finish with a small PR** adding `aaroneous-<seat>-seat[bot]` to your seat's `logins` list in
`governance/identities.json`, so formal reviews you post count as `authenticated`. It needs the normal two independent
accepts like any other PR. Set `git config user.name` and `user.email` to your seat as well, and sign commits with your
seat key, so commit authorship matches.

## If something goes wrong

| Symptom | Cause and fix |
| --- | --- |
| The browser did not open | Open the printed `http://127.0.0.1:PORT/` yourself. The redirect returns to `127.0.0.1` on this machine, so the browser must run here. Over SSH, forward the port (`ssh -L PORT:127.0.0.1:PORT host`) and pass `--port PORT` |
| Timed out before the first click | Nothing was created until the click. Run `setup` again |
| `registration failed` mentioning the name | GitHub App names are global and may be taken. Pick another: `--app-name aaroneous-<seat>-seat-2` (34 characters at most) |
| Timed out waiting for Install | The App exists; run `setup` again and it resumes at the install step |
| `the App is not installed on <owner>'s account yet` | It was installed on a different account or organisation. Install it on the owner's personal account |
| `the token does not reach <repo>` | Installed without the repository. Open the App's installation settings and add `aaroneous-devtools`, then run `check` |
| The wrong GitHub account clicked Create | The App now lives on that account. Delete it there (Settings, Developer settings, GitHub Apps), delete `~/.steward/seats/<seat>/`, and start again |
| `openssl could not sign` | The saved key is damaged or `openssl` is missing. Delete `~/.steward/seats/<seat>/` and the App, then register again |
| You lost the private key | It cannot be recovered. Delete the App and register again |

The steps are also separate commands if you need one alone: `register`, `link`, `token [--env]`, `check`.

## What it sets up

- An App named `aaroneous-<seat>-seat`, private, asking for contents, pull requests and issues write, and checks and
  statuses read. It does **not** ask for `workflows`; add `--allow-workflows` only if that seat must change
  `.github/workflows`, and say so in the PR.
- Credentials in `~/.steward/seats/<seat>/` (`app.json`, `private-key.pem`), never in the repository (`--root` moves
  them). The webhook and client secrets GitHub returns are discarded. Back the key up like any private key.
- A one-hour installation token on demand (`token`). A token is attributed to the App itself.

GitHub will not create an App without a person confirming it, so the two clicks cannot be removed. Do not use a
device-code login for this: that token acts as the person, so posts would still show the owner. Never print a token
into a log or a comment.

## Limits and what is not verified

- Seats that share one machine and one operating-system user can read each other's `~/.steward/seats/<seat>/` folders,
  so their separation holds by convention only. Real isolation needs separate operating-system users or machines.
- Verified live (claude seat, 2026-10-09): the manifest GitHub accepted, the token's issuer claim, the wait for the
  install, the repository check, and a comment posted as the App.
- Not verified: whether an App's approving review counts toward a required-review rule, and whether the reviews
  endpoint returns the same `[bot]` login as comments do. Try both on a throwaway PR before relying on them.

## Revoking

Suspend or delete an App at Settings, Developer settings, GitHub Apps: that seat stops posting at once. Then delete its
folder under `~/.steward/seats/`.
