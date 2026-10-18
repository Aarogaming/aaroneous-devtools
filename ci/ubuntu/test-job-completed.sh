#!/usr/bin/env bash
# Tests for ci/ubuntu/job-completed.sh, run on any machine with bash and find (no runner, no root, no GitHub needed):
#   bash ci/ubuntu/test-job-completed.sh
# Each case builds a throwaway runner layout under a temp directory and points the hook's test seams at it.
set -u
HOOK="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/job-completed.sh"
fails=0

check() { # name, condition-result (0 = pass)
  if [[ "$2" -eq 0 ]]; then echo "PASS  $1"; else echo "FAIL  $1"; fails=$((fails + 1)); fi
}

layout() { # makes $T/runner/{_work/repo,_work/_tool,_work/_actions,_diag} and sets T, WORK, DIAG
  T="$(mktemp -d)"
  WORK="$T/runner/_work"
  DIAG="$T/runner/_diag"
  mkdir -p "$WORK/repo/sub" "$WORK/_tool/node" "$WORK/_actions/checkout" "$WORK/_temp" "$DIAG"
  echo secret > "$WORK/repo/sub/file"
  echo planted > "$WORK/_tool/node/bin"
  echo planted > "$WORK/_actions/checkout/index.js"
  echo sentinel > "$WORK/_temp/sentinel"
}

only_runner="123 /var/lib/devtools-ci/runner/bin/Runner.Listener run"

run_hook() { # extra env assignments are passed through "$@"
  env RUNNER_WORKSPACE="$WORK/repo" DEVTOOLS_ALLOWED_WORK_ROOTS="$WORK" "$@" bash "$HOOK" >/dev/null 2>"$T/stderr"
}

# 1. a clean run empties _work completely, including the tool cache and the downloaded actions, and does not quarantine
layout
run_hook DEVTOOLS_STOP_CMD="touch $T/stopped" DEVTOOLS_PS_CMD="echo '$only_runner'"
rc=$?
check "a clean run exits 0" $rc
check "the workspace, tool cache, actions and temp are all gone" "$([[ -z "$(find "$WORK" -mindepth 1 -print -quit)" ]] && echo 0 || echo 1)"
check "no quarantine marker" "$([[ ! -e "$DIAG/QUARANTINE" ]] && echo 0 || echo 1)"
check "the listener was not stopped" "$([[ ! -e "$T/stopped" ]] && echo 0 || echo 1)"
rm -rf "$T"

# 2. two jobs in a row: what job A left is not there for job B
layout
run_hook DEVTOOLS_STOP_CMD=true DEVTOOLS_PS_CMD="echo '$only_runner'"
mkdir -p "$WORK/repo"; echo "job B" > "$WORK/repo/b"
check "job B starts with an empty _work after job A's cleanup" "$([[ ! -e "$WORK/_temp/sentinel" && ! -e "$WORK/_tool/node/bin" ]] && echo 0 || echo 1)"
rm -rf "$T"

# 3. a deletion that fails is a quarantine, never a success
layout
mkdir -p "$T/shim"
printf '#!/usr/bin/env bash\nexit 1\n' > "$T/shim/rm"; chmod +x "$T/shim/rm"
run_hook PATH="$T/shim:$PATH" DEVTOOLS_STOP_CMD="touch $T/stopped" DEVTOOLS_PS_CMD="echo '$only_runner'"
rc=$?
check "a failed deletion exits non-zero" "$([[ $rc -ne 0 ]] && echo 0 || echo 1)"
check "a failed deletion writes the quarantine marker" "$([[ -s "$DIAG/QUARANTINE" ]] && echo 0 || echo 1)"
check "a failed deletion stops the listener" "$([[ -e "$T/stopped" ]] && echo 0 || echo 1)"
rm -rf "$T"

# 4. a process left behind by the job is a quarantine
layout
run_hook DEVTOOLS_STOP_CMD="touch $T/stopped" DEVTOOLS_PS_CMD="printf '%s\n' '$only_runner' '999 sleep 3600'"
rc=$?
check "a surviving process exits non-zero" "$([[ $rc -ne 0 ]] && echo 0 || echo 1)"
check "a surviving process writes the quarantine marker naming it" "$(grep -q 'sleep 3600' "$DIAG/QUARANTINE" 2>/dev/null && echo 0 || echo 1)"
check "a surviving process stops the listener" "$([[ -e "$T/stopped" ]] && echo 0 || echo 1)"
rm -rf "$T"

# 5. a workspace outside the allowed roots is refused and left untouched
layout
OTHER="$(mktemp -d)"; mkdir -p "$OTHER/work/repo"; echo canary > "$OTHER/work/canary"
env RUNNER_WORKSPACE="$OTHER/work/repo" DEVTOOLS_ALLOWED_WORK_ROOTS="$WORK" DEVTOOLS_STOP_CMD=true DEVTOOLS_PS_CMD="echo '$only_runner'" bash "$HOOK" >/dev/null 2>&1
rc=$?
check "a foreign workspace exits non-zero" "$([[ $rc -ne 0 ]] && echo 0 || echo 1)"
check "a foreign workspace is not touched" "$([[ -s "$OTHER/work/canary" ]] && echo 0 || echo 1)"
rm -rf "$T" "$OTHER"

# 6. no workspace at all is refused
layout
env -u RUNNER_WORKSPACE DEVTOOLS_ALLOWED_WORK_ROOTS="$WORK" DEVTOOLS_STOP_CMD=true DEVTOOLS_PS_CMD="echo '$only_runner'" bash "$HOOK" >/dev/null 2>&1
rc=$?
check "a missing RUNNER_WORKSPACE exits non-zero and deletes nothing" "$([[ $rc -ne 0 && -s "$WORK/repo/sub/file" ]] && echo 0 || echo 1)"
rm -rf "$T"

echo
if [[ "$fails" -eq 0 ]]; then echo "all cases passed"; else echo "$fails case(s) FAILED"; exit 1; fi
