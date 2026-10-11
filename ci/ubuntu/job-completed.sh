#!/usr/bin/env bash
# Runner hook (ACTIONS_RUNNER_HOOK_JOB_COMPLETED): wipe the job's workspace and temp files so nothing one pull request
# wrote is readable by the next job. Refuses to delete anything outside a devtools runner's own _work directory.
set -euo pipefail
root="$(cd "${RUNNER_WORKSPACE:-/nonexistent}/.." 2>/dev/null && pwd || true)"
case "$root" in
  /var/lib/devtools-ci/runner/_work|/var/lib/devtools-merge/runner/_work) ;;
  *) echo "job-completed: refusing to clean '${root}' (not a devtools runner _work directory)" >&2; exit 0 ;;
esac
find "$root" -mindepth 1 -maxdepth 1 ! -name '_tool' ! -name '_actions' -exec rm -rf -- {} + 2>/dev/null || true
