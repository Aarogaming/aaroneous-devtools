# Registrar local API

The registrar is a separate Foundation REST/MCP control plane, not the packet utility at port 8787. Its dispatch produces unverified review artifacts outside Aaroneous. It is not a product dependency or an unattended product integrator.

## Explicit configuration

All existing registrar storage and checkout inputs remain required. Add an absolute `http_token_file` path to the caller-selected registrar TOML before using `agentic-registrar serve`. The file must be outside Aaroneous, without traversal or symbolic-link ancestors, and contain exactly 64 hexadecimal characters encoding 32 cryptographically random bytes (an optional final newline is allowed). Protect the file with user-only permissions. Never commit it, print it in logs, or put its contents in a URL. There is no default credential and no ambient credential discovery.

```toml
# Illustrative only: replace with your explicit, protected credential-file path.
http_token_file = 'D:/selected-private-runtime/registrar.token'
```

```text
foundation agentic-registrar serve --config <absolute-registrar.toml> --port 8080
```

Serve validates configuration and credentials before binding. CLI registration/dispatch/status operations do not require the optional HTTP credential field. A credential change takes effect after restarting Serve; existing sessions are not silently reconfigured. Stop the foreground Serve process to pause it. No OS service or task is installed by Serve.

## Request contract

Every endpoint, including health and read-only status, requires `X-Devtools-Token` with the exact file contents and `Host: 127.0.0.1:<selected-port>`. An Origin header, when present, must equal `http://127.0.0.1:<selected-port>`. Native clients may omit Origin. Wrong/missing authority returns 403 before route/RPC execution. No wildcard or cross-origin CORS response is sent; OPTIONS does not grant access.

The server accepts HTTP/1.1 GET/POST/OPTIONS only. POST requires `Content-Type: application/json` and one decimal Content-Length. Headers are limited to 8 KiB and bodies to 64 KiB. Duplicate headers, folded headers, Transfer-Encoding, Expect, truncated bodies and invalid UTF-8 are rejected. Requests have a five-second total acquisition deadline, responses a five-second write timeout, and at most eight active connection handlers. Excess connections close immediately. Responses close the connection; pipelining is not supported. Credentials are never echoed in errors.

REST status/tasks/logs and JSON-RPC at `/mcp` retain their existing shapes. The hardcoded legacy `/sse` endpoint is retired (404). This does not claim complete modern MCP transport compatibility.

## IDE and dashboard migration

Serve never reads APPDATA/USERPROFILE or modifies IDE settings. The legacy `setup-mcp` action now refuses with an explanation and changes no files. Configure the selected client's explicit registrar URL and token header yourself, preserving its existing settings. An explicit path-based installation adapter is future work, not an implicit bootstrap side effect.

The separate TypeScript dashboard currently has no registrar credential setup. It will report unavailable rather than bypass authentication; do not restore wildcard CORS to make it connect. A same-origin, explicitly configured integration needs its own reviewed interface lane. The running packet utility's authentication and research/review workflow are unchanged.

## Remaining boundary

This hardens HTTP admission, not generated-code correctness or CLI promotion. The existing separately invoked promotion path still needs isolated worktrees, dirty/index protection and Aaroneous's full native gate before unattended use. No HTTP promotion endpoint is provided. Do not schedule promotion or interpret a registrar task's `Verified` label as independent framework certification.

Foundation gates: focused registrar protocol/socket tests, workspace tests, formatting and strict all-target Clippy. Independent review is required before treating this API as an unattended shared-machine service.
