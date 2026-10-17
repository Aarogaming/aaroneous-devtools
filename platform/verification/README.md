# Operating the Ubuntu/Windows verification pair

C135 adds the executable transport around C132/C133. It is initially a private,
owner-trusted operation in shadow mode. It does not install services, migrate
Git hosting, expose the machine publicly or switch integration authority.
The [wire and recovery contract](../../contracts/platform-jobs-v1.md) states
the supported trust boundary and limits.
The [pair rollout record](ROLLOUT.md) distinguishes the completed two-machine
HTTPS smoke from production deployment and remaining gate qualification.

## Roles and deployment

Ubuntu hosts one Rust coordinator, its private durable spool, the HTTPS front
door and a separately owned Linux worker controller. Windows runs a native
worker controller and a submit/status client. Workers initiate outbound HTTPS
connections. GitHub and SSH are not needed to dispatch and collect jobs once
approved immutable checkouts and images are provisioned. Checkouts are explicit
inputs; automatic source distribution is intentionally not implemented.

Build the reviewed revision on each platform:

```text
cargo build --manifest-path foundation/Cargo.toml -p steward --bin platform-jobs --bin local-verification --bin local-integration --locked
```

Copy `server.example.json` and `worker.example.json` outside the checkout and
replace every placeholder with a reviewed explicit value. Precreate source,
spool, staging, client-scratch and private directories; use separate service accounts and
0700 directories/0600 token and private-key files on Ubuntu, equivalent private
ACLs on Windows. Keep token/key/config files outside candidates and worker
spools. Supply random tokens and authorized node public keys explicitly;
these examples create neither credentials nor gate enrollment. The trusted
base must enable the suite, platform policy and test fixture environment.
Do not substitute an old head merely to make an example start.

Start the coordinator:

```text
platform-jobs serve --config /srv/aaroneous-verification/private/server.json --listen 127.0.0.1:31493
```

For Ubuntu, `compose.yaml` runs only the TLS proxy with host networking so
it can reach the loopback coordinator. It requires an explicit interface IP,
TLS hostname, approved digest-pinned Caddy image and precreated private TLS
data/config directories writable by uid/gid 10001. Inspect the resolved
Compose configuration before starting it. It starts no test worker or daemon,
mounts no Docker socket, and has automatic restart disabled for staged rollout.
Keep it on the LAN/tailnet, restrict ingress to the two machines and do not
publish router ports. Its internal CA root must be securely exported and
explicitly supplied to clients as `ca_file`. Never disable TLS verification.
The same certificate hostname can resolve to a LAN or Tailscale address;
changing an address does not grant a different identity or gate exemption.

The template uses Caddy's [internal TLS](https://caddyserver.com/docs/caddyfile/directives/tls)
and [request-size limits](https://caddyserver.com/docs/caddyfile/directives/request_body).
Image pinning, certificate trust, firewall rules, ownership and an actual
`caddy validate`/HTTPS test are deployment prerequisites, not implied successes.

Build `Dockerfile.ingress` with `CADDY_BASE` set to an approved official Caddy
digest. It removes the executable's low-port file capability; an unmodified
official image may fail to execute with `cap_drop: ALL`. Pin the resulting
derived image by digest for `VERIFICATION_CADDY_IMAGE`. Port 8443 requires no
low-port capability. Do not relax the Compose restrictions to fix this.

Issuer/status configuration has exactly:

```json
{"v":1,"curl":"C:/REPLACE_WITH_APPROVED_PRIVATE_CA_CLIENT/bin/curl.exe","endpoint":"https://aaroneoushome:8443","ca_file":"D:/aaroneous-verification/private/ca.crt","token_file":"D:/aaroneous-verification/private/issuer-token","client_environment":{"SYSTEMROOT":"C:/Windows"},"client_scratch":"D:/aaroneous-verification/client-scratch"}
```

Use `/usr/bin/curl` and `{}` for Linux client environment. Only explicit Windows
system variables are accepted; credentials, proxy settings and curl defaults
are not inherited. Client tokens travel through stdin, never command arguments.
For Caddy's private CA, select an approved compatible TLS client explicitly;
the built-in Windows Schannel client may require unavailable CRL information.
See the [tested Windows client choice](ROLLOUT.md#windows-private-ca-compatibility).
JSON bodies use temporary files under the explicit private `client_scratch`
directory, removed after the request. Keep it outside candidates and job mounts;
an interrupted controller may leave a private upload file for later inspection.

```text
platform-jobs submit --config <absolute-issuer.json> --head <full-head-SHA> --suite foundation-ci
platform-jobs status --config <absolute-issuer.json>
platform-jobs worker-once --config <absolute-worker.json>
platform-jobs worker-loop --config <absolute-worker.json> --interval-s 15
```

Prepare a clean disposable checkout per approved full head and add it to the
worker's explicit `checkouts` map. Include trusted-base objects. Never mount
the shared working checkout, host home, secrets or persistent services into a
job. Unknown heads remain claimed pending operator provisioning. Receipt
creation/import exit 0 is not a test pass: inspect the returned verdict and
evaluate the original signed receipts through `local-integration`.

## Linux container executor

Replace the worker's `execution` object with:

```json
{"mode":"docker","docker":"/usr/bin/docker","docker_config":"/srv/aaroneous-worker/empty-docker-config","daemon":"unix:///run/user/REPLACE_UID/docker.sock","image":"REPLACE_APPROVED_SDK_IMAGE@sha256:REPLACE_64_HEX_DIGEST","runner_config":"/srv/aaroneous-worker/private/container-runner.json","host_git":"/usr/bin/git","uid":10001,"gid":10001,"timeout_s":14400}
```

The daemon must actually report rootless mode and cgroup v2. Use a dedicated
job daemon/account, separate from persistent services. The explicit Docker
client config directory must be empty. Preload the approved image there;
jobs never pull. See [Docker rootless operation](https://docs.docker.com/engine/security/rootless/).
The image must contain the reviewed Linux `local-verification` binary at
`/usr/local/bin/local-verification`, native Git, suite toolchains, locked
offline dependencies, and precreated `/spool`, `/candidate`, `/work` paths.
It must support the configured nonroot uid/gid and include an account entry
for that uid, as OpenSSH requires it. The worker refuses root and requires
a private 0700 spool parent; only the job subdirectory is made writable
for the rootless mapped uid. That directory remains under the private parent.

Container runner JSON has exactly C132's `git`, `tools`, `environment`; all
paths are container paths. For example:

```json
{"git":"/usr/bin/git","tools":{"cargo":"/opt/verification-tools/cargo"},"environment":{"PATH":"/usr/local/cargo/bin:/usr/bin:/bin","HOME":"/work","CARGO_HOME":"/work/cargo","RUSTUP_HOME":"/usr/local/rustup","CARGO_TARGET_DIR":"/work/target","CARGO_BUILD_JOBS":"2","CARGO_NET_OFFLINE":"true","DEVTOOLS_TEST_GIT":"/usr/bin/git","DEVTOOLS_TEST_SSH_KEYGEN":"/usr/bin/ssh-keygen","DEVTOOLS_TEST_CURL":"/usr/bin/curl"}}
```

Provision an offline cargo registry in the image or supply a reviewed native
Cargo wrapper that copies the image cache into `/work/cargo` before invoking
Cargo. `/opt/verification-tools/cargo` is an explicitly provisioned regular
native executable, not a symlink to rustup or a shell script. C132 refuses
symlink tool paths. The supplied image must have all locked dependencies available; this
document does not claim that a stock Rust image can run offline. No shell
arguments arrive over the protocol. Signers and bearer tokens stay in the host
controller; neither enters the candidate container. A missing dependency,
failed resource limit or cleanup never counts as a platform pass.

## Gate rollout and maintenance

First run a harmless known suite through real HTTPS on each platform, inspect
the signer and retained logs, then run Foundation at one exact head. Enroll
node keys through the trusted integration policy's review process. Keep C133
shadow-only until independent review, both platform receipts, required review
families and every required suite are demonstrated. Foundation-only coverage
does not authorize a global gate cutover or public-fork worker admission.

Stop worker loops before stopping the coordinator. Back up the coordinator's
complete spool and C133 authority/high-water state, worker attempt histories,
explicit configs and CA material using separate controlled retention. Restore
must not silently roll gate history backward. Keep signing keys separately
protected. No automatic sync or artifact deletion is installed. For removal,
stop the native processes and Compose ingress, retain evidence and revoke
transport tokens/node keys as appropriate. No SSH session is a runtime
dependency of the service; SSH remains a provisioning/recovery tool.
