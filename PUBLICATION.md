# Public-readiness handoff

This document records the publication preparation requested on 2026-10-11.
It does not change repository visibility, authorize a history rewrite, or
certify independent acceptance. The public license is MIT, matching the
existing Foundation workspace's declared license; review the root LICENSE
with this candidate before publication.

## Credential and history decision

The owner confirmed on 2026-10-11 that the LM Studio credential previously
embedded in `migrations/aaroneous-2026-09-16/dev-tools/autonomous_fix_loop.ps1`
was revoked, and requested preservation of Git history. The current source
requires an explicit credential input. The old value remains in historical
commit `d516e495c1a7d3dcc905e1c415f5ac48c00de1f9` and its descendants; it must
remain revoked. No value is reproduced in this document. Revocation is an
owner-confirmed fact, not a successful credential probe by this agent.

Full-history scans will continue to report that historical exposure. Do not
disable the LM Studio rule or allowlist the old value globally: a new commit
containing it, or another credential, must still fail. The security workflow
scans the current source, bundled archives, and newly introduced PR content.
Review a full available-ref scan separately before changing visibility.

## Candidate safeguards

- Patch materialization rejects case variants of Git metadata and protected
  infrastructure paths, plus trailing-dot/space path aliases on every platform.
  Checksums provide integrity checking, not authentication of reviewers.
- Windows task registration runs a fixed PowerShell script and supplies inputs
  as JSON data. It does not forcibly overwrite an existing scheduled task.
- Auto-merge requires the full head SHA that its gate evaluated. A moved head
  causes refusal instead of merging a revision with stale review evidence.
- The archived autopilot passes argument arrays and disables shell execution.
  Historical scripts remain unsupported migration evidence, not active services.
- The GUI uses patched Vite/Tailwind tools. The small PostCSS setup now follows
  Tailwind 4; the dashboard tests, type check, and production build were exercised.
- Dependabot includes npm. A separate security workflow has read-only repository
  permissions, pinned action commits and a checksum-pinned Gitleaks executable.
- Local checkout directories, agent state, job archives and private key files
  are ignored. Ignore rules do not remove already tracked files or old history.

## Reproduce the publication checks

Use an isolated checkout and explicit paths for outputs and caches. Run:

```text
cargo fmt --manifest-path foundation/Cargo.toml --all -- --check
cargo test --manifest-path foundation/Cargo.toml --workspace --all-targets --locked
cargo clippy --manifest-path foundation/Cargo.toml --workspace --all-targets --locked -- -D warnings
python -B -m unittest discover -s scripts -p 'test_*.py'
npm ci                      # from gui/
npm run build               # from gui/
npm audit --package-lock-only --ignore-scripts  # from gui/
cargo audit --file foundation/Cargo.lock --db <explicit-advisory-directory>
gitleaks git <checkout> --log-opts="--all" --config <checkout>/.gitleaks.toml --redact --max-archive-depth 3
python scripts/publication_check.py --repo <absolute-checkout> --revision <commit> --output <absolute-report>
```

For a current-tree Gitleaks scan, export the tracked revision to a new scratch
directory and scan that directory with `gitleaks dir`, the explicit config,
redaction and archive depth 3. Never publish raw secret-scanner output.
The checked Gitleaks release is 8.30.1. A history scan may return exit 1 for
the documented revoked credential; inspect its redacted findings and do not
misreport that as a clean history scan.

The publication inventory identifies tracked private artifacts, LM Studio
credential strings and personal-path locations, and inspects bundled tar
members without extraction. It complements Gitleaks rather than replacing
it. Unknown/oversized or unreadable input fails closed. Privacy locations
are review items; they do not silently become permission to publish.

## Retained local verification

Base: `e999b877236df8cb9c33b2cac1b2742cf44ba46f`.
Isolated candidate: `D:/aaroneous-devtools/tmp/public-readiness`.
Evidence directory: `D:/aaroneous-devtools/tmp/public-readiness-evidence`.

On 2026-10-11, formatting, workspace/all-target tests (266 passed, one
pre-existing ignored differential test), strict Clippy, Python discovery
(467 tests), and GUI build (eight dashboard tests) each returned exit 0.
The npm audit returned exit 0 with zero vulnerabilities. Cargo audit 0.22.1
returned exit 0 with zero vulnerabilities against advisory database commit
`7eebec69c352c7191b1f13eb95dd510eeca5d1de`; it also reported a pre-existing
yanked `yoke-derive 0.8.3` warning. The shared Cargo lockfile is reserved by
another open lane, so this candidate does not change it or suppress that
warning. Logs: `fmt.log`, `rust-tests.log`, `clippy.log`,
`python-tests-final.log`, `gui-build.log`, `npm-audit-final.json`, and
`cargo-audit.json`. The queue closeout cites the immutable implementation
commit and final scan results. Remote Linux/Windows CI and independent
acceptance must be assessed at the final PR head.

## Before changing visibility

1. Review and integrate the candidate through the existing PR process. Keep
   other contributors' branches, runtime state and proposals intact.
2. Re-scan the exact integrated head and all hosted branches/tags/PR heads.
   Classify every finding. Preserve the documented revoked-credential exception
   as a historical fact, not a source-wide scanner exception.
3. Review the inventory's personal-path locations and the broader history.
   Current documents and archives retain workstation paths, operational host
   names and internal work records. Some documents belong to active lanes;
   this candidate does not overwrite them. Keeping full history also keeps
   those historical disclosures accessible even if current examples are cleaned.
4. Review live GitHub settings. At inspection time the repo was private,
   `main` required PRs with administrator enforcement and prohibited force
   pushes/deletions, but had no required status checks. Require the Foundation,
   dashboard and new security checks after they have run successfully. The
   vulnerability-reporting endpoint returned HTTP 404; confirm availability
   and enable private reporting when the repository is public. The default
   workflow token was read-only and could not approve PRs.
5. Review hosted Actions artifacts/logs, release assets, licenses and open
   security discussions. They are not fully covered by a local source scan.
   Public fork PRs must use hosted runners without private credentials;
   never run untrusted fork code on a private persistent workstation runner.
   The proposed Ubuntu runner workflow is in another lane and needs this
   separate public-repository threat-model review before adoption.
6. Make the repository public only after these concrete review items have
   been accepted. This preparation makes no live visibility or protection change.
