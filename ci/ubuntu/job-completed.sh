#!/usr/bin/env bash
# Runner hook (ACTIONS_RUNNER_HOOK_JOB_COMPLETED): leave nothing behind for the next job, or take the runner out of service.
#
#   1. Wipe the whole _work directory, including the downloaded actions and the tool cache. Both are reused between jobs
#      otherwise, so a job could plant something in them for the next pull request.
#   2. Check that it is empty, and that no process of this user survived the job except the runner itself. A background
#      process left running by one job is how it would watch the next.
#   3. If any of that fails, QUARANTINE: write _diag/QUARANTINE, stop the listener, and exit non-zero. The service refuses to
#      start while the marker exists (devtools-runner.service), so a runner that could not prove it is clean takes no more jobs
#      until an administrator inspects it and removes the marker. Cleanup failing is never treated as success.
#
# Refuses to delete anything outside a devtools runner's own _work directory. The variables below are test seams
# (ci/ubuntu/test-job-completed.sh); the defaults are the production values.
set -uo pipefail

allowed="${DEVTOOLS_ALLOWED_WORK_ROOTS:-/var/lib/devtools-ci/runner/_work:/var/lib/devtools-merge/runner/_work}"
stop_cmd="${DEVTOOLS_STOP_CMD:-pkill -TERM -u $(id -u) -f Runner.Listener}"
ps_cmd="${DEVTOOLS_PS_CMD:-ps -u $(id -u) -o pid=,args=}"

root="$(cd "${RUNNER_WORKSPACE:-/nonexistent}/.." 2>/dev/null && pwd || true)"

quarantine() {
  reason="$1"
  echo "job-completed: QUARANTINE: ${reason}" >&2
  if [[ -n "$root" && -d "$root/../_diag" ]]; then
    printf '%s\n' "$reason" > "$root/../_diag/QUARANTINE" 2>/dev/null || echo "job-completed: could not write the quarantine marker" >&2
  fi
  eval "$stop_cmd" >/dev/null 2>&1 || true
  exit 1
}

ok=0
IFS=':' read -r -a roots <<< "$allowed"
for candidate in "${roots[@]}"; do
  if [[ -n "$root" && "$root" == "$candidate" ]]; then ok=1; fi
done
[[ "$ok" -eq 1 ]] || quarantine "refusing to clean '${root}': it is not a devtools runner _work directory"

find "$root" -mindepth 1 -maxdepth 1 -exec rm -rf -- {} + 2>/dev/null
if [[ -n "$(find "$root" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null)" ]]; then
  quarantine "the workspace could not be emptied"
fi

# Only the runner's own processes may remain: the listener, the worker that is running this hook, and the shells above it.
stray="$(eval "$ps_cmd" 2>/dev/null | grep -Ev 'Runner\.Listener|Runner\.Worker|run\.sh|runsvc\.sh|job-completed\.sh|(^|[ /])(ps|grep|bash|sh)( |$)' || true)"
if [[ -n "$stray" ]]; then
  quarantine "a process survived the job: $(printf '%s' "$stray" | head -1 | cut -c1-120)"
fi
exit 0
