# Private authority, public releases (lane C135)

Status: proposal plus the first Rust slice (`foundation/crates/steward/src/release.rs`). Author: Claude seat. Grants nothing and
changes no visibility, workflow or policy. Builds on C129 (platform verification, whose evidence the release decision reads) and on
the stage plan for removing the dependency on GitHub.

## Owner direction (chat, 2026-10-12; recorded here by the Claude seat, not signed)

- Pull requests stay, as a **fallback**, for some work.
- If devtools goes public, the **local, authoritative repository stays private**.
- The public repository receives **git pushes for official releases, confirmed feature adds** and the like, and nothing else.

## The tiers

| Tier | What it is | Authority | Who writes |
|---|---|---|---|
| Local, private | a bare repository on the Ubuntu machine (replicated to the Windows machine and backed up) | **authoritative**: `main` advances only through the gate (a server-side hook running the same checks) | seats push proposal refs; the gate advances `main` |
| Private GitHub repository | the existing repository (currently private), kept as a working mirror and as the channel for cloud-hosted seats that cannot reach the LAN | none: a copy | the gate mirrors `main` out; seats open PRs here as the **fallback** route |
| Public repository | a separate repository, created for releases | none: a **derived, one-way** publication | only the release exporter |

Nothing flows into authority automatically. A pull request on the private mirror or the public repository is a *proposal*: its
signed evidence is imported into git-native objects (stage 1 of the plan) and judged by the same gate, or its patch is re-created
privately with attribution. Public `main` never diverges from what the exporter wrote, so there is no merge button to defend there.

## One thing to decide before anything is made public

`PUBLICATION.md` (2026-10-11) records that you asked for **git history to be preserved**, accepted the disclosure of workstation
paths and operational history, and confirmed the LM Studio credential in historical commit `d516e49…` was revoked and must stay so.
That decision is about making *this* repository public with its history. Under release-only publication it is not needed, and I would
not use it: **do not flip the existing repository public**. Export to a fresh public repository with new history (one commit per
release or per confirmed feature), so the revoked-credential commit, the workstation paths and the queue and attestation history
never leave the private tier by construction rather than by scan. Whether the 2026-10-11 decision still applies is yours; I have not
changed it.

## What a release is

A release candidate is a private commit that a release rule (data, reviewed like any other file) says may be published: a tagged
`vMAJOR.MINOR.PATCH` (the existing `governance/RELEASES.md` convention) or a confirmed feature add. The exporter does five things:

1. **Verify**: the candidate has passed on every required platform for this exact commit (`verification::evidence(...).ok`, C129).
2. **Plan the export**: `release::plan_export(policy, entries)` lists the tracked paths of that commit with their git mode and decides
   each one. It is default-deny: a path must be inside an included area and outside every excluded one, safe to write on any platform
   (no `..`, no drive or absolute path, no backslash or control character, no `.git` alias such as `.GIT.` or `GIT~1`, no `:` stream),
   not a symlink or submodule, and not secret-shaped (`.env`, `*.key`, `*.pem`, `id_*` private keys, `credentials`, `.ssh`,
   `.steward`, ...). The same inputs always give the same plan, in any order.
3. **Scan the export tree** (not the private tree): secret scan, advisories, `scripts/publication_check.py` (to be ported), all of which
   must run clean.
4. **Decide**: `release::decide(candidate, plan)` blocks, and lists every reason, on a malformed tag or SHA, missing or pending
   verification, publication checks that did not run clean, open blockers, an empty export, an unsafe path, or a secret-shaped file
   inside a published area (a mistake to look at, not a path to silently skip).
5. **Publish**: write the export tree as a new commit in the public repository with provenance (the private commit id and a digest of
   the plan in the message, no private content), sign the tag, push. Nothing else is ever pushed there.

Who triggers a release is also data: a release proposal ref reviewed through the same gate (two accepts from different model families),
so it needs no human in the loop and still cannot happen by one seat's say-so.

## Why release-only publication is the safer design

The public tier holds only what passed an allowlist and a scan of the tree itself, so a mistake in the private repository (a leaked key
in an old commit, a personal path in a queue row) stays private. A scanner can miss a secret; an allowlist that never includes the
file cannot. It also means the private repository's size, noise and operational detail are not anyone's business, and the
public history is curated.

## Phases, with exit evidence

1. **This PR:** the export planner and release decision with tests and this document.
2. The tracked-tree reader (git `ls-tree` with modes), the export policy file (`ci/export-policy.toml`, validated like the platform
   policy), and the scanners wired in; a dry run on the real tree printing what would be published and why each other path is withheld.
3. The exporter: build the tree, make the provenance commit and signed tag in a scratch public repository; a test that exports the
   same commit twice and gets identical trees.
4. Stage 1 of the GitHub plan (git-native evidence and the importer for PR evidence), then the authoritative bare repository and its
   gate hook, with the private GitHub repository as the mirror.
5. Create the public repository and make the first release.

## Not decided here

The public repository's name and licence (`PUBLICATION.md` says MIT, to be reviewed); which areas are included on the first release
(my default is `foundation/` and the documentation, nothing from `governance/` or `scripts/`); whether outside pull requests to the
public repository are accepted at all; how often confirmed feature adds are batched.

## Verified and not

Verified: `cargo test -p steward --lib release` (10 tests), clippy and `cargo fmt --check` (commands and results in the lane row); eleven
deliberate breakages (default-deny off, prefix matching a name prefix, the git-alias fold removed, symlinks published, sensitive
names published, verification, publication checks, open blockers, an empty export, an unsafe path, or the tag format not enforced)
each fail a test. **Not run:** nothing reads a real tree, runs a scanner, touches a repository or pushes anywhere; the export
policy, the exporter and every adapter are later phases. The stack depends on #197 (`verification::Evidence`).
