#!/usr/bin/env bash
# Install one self-hosted GitHub Actions runner for aaroneous-devtools on the Ubuntu machine (lane C128).
#
#   sudo ci/ubuntu/setup-runner.sh --repo Aarogaming/aaroneous-devtools --kind ci    --token <registration token>
#   sudo ci/ubuntu/setup-runner.sh --repo Aarogaming/aaroneous-devtools --kind merge --token <registration token>
#
# Two kinds, two operating-system users, on purpose:
#   ci     runs the code a pull request brings (cargo test, npm build, the evidence check). Label: devtools-ci.
#   merge  runs only the merge gate from main's own copy of the scripts, holding the job token that can merge.
#          Label: devtools-merge. It never executes pull-request code, and PR code never runs as its user.
# A process can read another process's environment when both belong to one user, so a job that runs PR code must not
# share a user with the job that holds the merge-capable token.
#
# The registration token is one hour old at most and comes from a repository admin:
#   gh api -X POST repos/Aarogaming/aaroneous-devtools/actions/runners/registration-token --jq .token
# Nothing here stores it; config.sh exchanges it for the runner's own credential.
set -euo pipefail

RUNNER_VERSION="2.338.0"
RUNNER_SHA256="af4b794c1bc41d73d40535e3fe092a39f9679cd8d965954c2aca25a05ca41d32"   # from the v2.338.0 release notes
RUNNER_URL="https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz"

repo="" kind="" token="" name=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)  repo="${2:-}"; shift 2 ;;
    --kind)  kind="${2:-}"; shift 2 ;;
    --token) token="${2:-}"; shift 2 ;;
    --name)  name="${2:-}"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

[[ "$(id -u)" -eq 0 ]] || { echo "run as root (sudo): it creates a user and a service" >&2; exit 2; }
[[ "$repo" =~ ^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$ ]] || { echo "--repo must be OWNER/NAME" >&2; exit 2; }
[[ "$kind" == "ci" || "$kind" == "merge" ]] || { echo "--kind must be ci or merge" >&2; exit 2; }
[[ -n "$token" ]] || { echo "--token (a registration token) is required" >&2; exit 2; }
[[ "$repo" == "Aarogaming/aaroneous-devtools" ]] || { echo "refusing: this runner is for the devtools repository only (never the product repo)" >&2; exit 2; }
name="${name:-devtools-ubuntu-${kind}}"

user="devtools-${kind}"
home="/var/lib/${user}"
dir="${home}/runner"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "== packages"
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq curl ca-certificates git python3 build-essential pkg-config libssl-dev gh >/dev/null

echo "== user ${user} (no login, no sudo, home ${home})"
if ! id "$user" >/dev/null 2>&1; then
  useradd --system --create-home --home-dir "$home" --shell /usr/sbin/nologin "$user"
fi
chmod 0700 "$home"
install -d -o "$user" -g "$user" -m 0700 "$dir"

echo "== runner ${RUNNER_VERSION}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl --proto '=https' --tlsv1.2 -fsSL "$RUNNER_URL" -o "$tmp/runner.tgz"
echo "${RUNNER_SHA256}  $tmp/runner.tgz" | sha256sum --check --strict
if [[ ! -x "$dir/run.sh" ]]; then
  tar -xzf "$tmp/runner.tgz" -C "$dir"
  chown -R "$user:$user" "$dir"
fi

echo "== job-completed hook (wipes the workspace after every job)"
install -d -m 0755 /opt/devtools-ci
install -m 0755 "$here/job-completed.sh" /opt/devtools-ci/job-completed.sh
printf 'ACTIONS_RUNNER_HOOK_JOB_COMPLETED=/opt/devtools-ci/job-completed.sh\n' > "$dir/.env"
chown "$user:$user" "$dir/.env"

echo "== register ${name} (labels: devtools-ubuntu, devtools-${kind})"
if [[ ! -f "$dir/.runner" ]]; then
  sudo -u "$user" -H bash -c "cd '$dir' && ./config.sh --unattended --replace --url 'https://github.com/${repo}' \
    --token '${token}' --name '${name}' --labels 'devtools-ubuntu,devtools-${kind}' --work _work"
fi

if [[ "$kind" == "ci" ]]; then
  echo "== Rust toolchain for ${user} (same components the workflow asks for)"
  if [[ ! -x "$home/.cargo/bin/cargo" ]]; then
    sudo -u "$user" -H bash -c "curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal -c rustfmt,clippy"
  fi
fi

echo "== service"
sed "s/%KIND%/${kind}/g" "$here/devtools-runner.service" > "/etc/systemd/system/devtools-runner-${kind}.service"
systemctl daemon-reload
systemctl enable --now "devtools-runner-${kind}.service"

echo "== isolation check: ${user} must not be able to read any other account's files"
bad=0
for path in /home/*/.steward /home/*/.ssh /root /etc/shadow; do
  [[ -e "$path" ]] || continue
  if sudo -u "$user" test -r "$path"; then
    echo "FAIL: ${user} can read ${path}" >&2
    bad=1
  fi
done
if [[ "$bad" -ne 0 ]]; then
  echo "isolation check failed; stopping the service" >&2
  systemctl disable --now "devtools-runner-${kind}.service"
  exit 1
fi
echo "ok: ${user} is registered as '${name}' and cannot read other accounts' files"
systemctl --no-pager status "devtools-runner-${kind}.service" | head -5
