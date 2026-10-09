#!/usr/bin/env python3
"""Per-seat GitHub App setup (lane C112): each seat registers its own GitHub App and posts under that App's identity.

Why: every seat posts through the owner's one token, so GitHub shows the owner login for everything. A GitHub App per
seat posts as `<app>[bot]`. GitHub will not create an App without a person confirming it, so this uses the manifest
flow: this script serves one local page, the owner clicks "Create GitHub App" on github.com, GitHub redirects back
here with a one-time code, and the script exchanges it for the App id and private key. The owner then installs the App
on the repository (second click). Stdlib plus `openssl` only; nothing is installed and nothing is posted.

  seat_app_setup.py register --seat claude          # serve the page, wait for the owner's click, save id and key
  seat_app_setup.py link     --seat claude          # after the owner installed the App: find and save its install id
  seat_app_setup.py token    --seat claude [--env]  # print a one-hour installation token (prints only the token)
  seat_app_setup.py check    --seat claude          # prove the token reaches the repository, as which App

Credentials live under `~/.steward/seats/<seat>/` (override with --root), never in the repository: `app.json` (ids
only) and `private-key.pem`. The webhook secret and client secret in GitHub's reply are discarded: no webhook is used.
Installation tokens act as the App. Do not use a GitHub device-code login for this: that token acts as the person.
"""

import argparse
import base64
import html
import json
import os
import secrets
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
import webbrowser
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import parse_qs, urlparse

API = "https://api.github.com"
SEATS = ("claude", "codex", "gemini", "mimo")
DEFAULT_REPO = "Aarogaming/aaroneous-devtools"


def seat_dir(seat, root=None):
    return os.path.join(root or os.path.join(os.path.expanduser("~"), ".steward", "seats"), seat)


def build_manifest(seat, repo, redirect_url, allow_workflows=False):
    """The App definition GitHub shows the owner. Least privilege: branches, pull requests and issue comments; the
    workflows permission only on request, because changing .github/workflows is a separate decision."""
    permissions = {"contents": "write", "pull_requests": "write", "issues": "write", "checks": "read",
                   "statuses": "read"}
    if allow_workflows:
        permissions["workflows"] = "write"
    return {
        "name": f"aaroneous-{seat}-seat",
        "url": f"https://github.com/{repo}",
        "description": f"The {seat} seat's own identity for {repo}: its pull requests, comments and reviews.",
        "redirect_url": redirect_url,
        "public": False,
        "default_permissions": permissions,
    }


def landing_page(manifest, state, seat, repo):
    rows = "".join(f"<li><code>{html.escape(k)}</code>: {html.escape(v)}</li>"
                   for k, v in manifest["default_permissions"].items())
    return (
        "<!doctype html><meta charset=utf-8><title>Register the seat App</title>"
        f"<h1>Register the {html.escape(seat)} seat's GitHub App</h1>"
        f"<p>This creates a private GitHub App named <code>{html.escape(manifest['name'])}</code> on your account, for "
        f"<code>{html.escape(repo)}</code>. Nothing is created until you click the button on github.com.</p>"
        f"<p>Permissions it asks for:</p><ul>{rows}</ul>"
        f"<form method=post action='https://github.com/settings/apps/new?state={html.escape(state)}'>"
        f"<input type=hidden name=manifest value='{html.escape(json.dumps(manifest), quote=True)}'>"
        "<button type=submit>Continue to GitHub</button></form>"
    )


def http_json(method, url, headers=None, body=None, opener=urllib.request.urlopen):
    request = urllib.request.Request(url, data=body, method=method, headers={
        "Accept": "application/vnd.github+json", "User-Agent": "aaroneous-seat-app-setup",
        "X-GitHub-Api-Version": "2022-11-28", **(headers or {})})
    try:
        with opener(request, timeout=30) as response:
            raw = response.read().decode("utf-8")
            return response.status, (json.loads(raw) if raw else {})
    except urllib.error.HTTPError as error:
        raw = error.read().decode("utf-8", "replace")
        try:
            return error.code, json.loads(raw)
        except ValueError:
            return error.code, {"message": raw[:200]}


def convert_code(code, opener=urllib.request.urlopen):
    """Exchange the one-time manifest code (valid one hour) for the App's id and private key."""
    status, data = http_json("POST", f"{API}/app-manifests/{code}/conversions", body=b"", opener=opener)
    if status not in (200, 201):
        raise RuntimeError(f"GitHub refused the manifest code (HTTP {status}): {data.get('message', '')}")
    return data


def save_credentials(directory, result):
    """Keep what is needed to act as the App and nothing else: the webhook and client secrets are dropped."""
    os.makedirs(directory, exist_ok=True)
    pem_path = os.path.join(directory, "private-key.pem")
    with open(pem_path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(result["pem"])
    if os.name != "nt":
        os.chmod(pem_path, 0o600)
    info = {"id": result["id"], "slug": result["slug"], "name": result.get("name", result["slug"]),
            "client_id": result.get("client_id", ""), "html_url": result.get("html_url", "")}
    with open(os.path.join(directory, "app.json"), "w", encoding="utf-8", newline="\n") as handle:
        json.dump(info, handle, indent=2)
        handle.write("\n")
    return info


def load_app(directory):
    with open(os.path.join(directory, "app.json"), encoding="utf-8") as handle:
        return json.load(handle)


def b64url(data):
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode("ascii")


def make_jwt(app_id, pem_path, now, openssl="openssl"):
    """A ten-minute RS256 JWT for the App, signed with `openssl dgst -sha256 -sign` (no third-party library)."""
    header = b64url(json.dumps({"alg": "RS256", "typ": "JWT"}, separators=(",", ":")).encode())
    payload = b64url(json.dumps({"iat": int(now) - 60, "exp": int(now) + 540, "iss": app_id},
                                separators=(",", ":")).encode())
    signing_input = f"{header}.{payload}".encode("ascii")
    signature = subprocess.run([openssl, "dgst", "-sha256", "-sign", pem_path], input=signing_input,
                               capture_output=True, check=True).stdout
    return f"{header}.{payload}.{b64url(signature)}"


def find_installation(app_jwt, repo, api=http_json):
    """The installation on the repository owner's account."""
    owner = repo.split("/", 1)[0].lower()
    status, data = api("GET", f"{API}/app/installations", headers={"Authorization": f"Bearer {app_jwt}"})
    if status != 200 or not isinstance(data, list):
        raise RuntimeError(f"could not list installations (HTTP {status})")
    for installation in data:
        if str(installation.get("account", {}).get("login", "")).lower() == owner:
            return installation["id"]
    raise RuntimeError(f"the App is not installed on {owner}'s account yet: open the install link first")


def mint_token(directory, now, api=http_json, openssl="openssl"):
    app = load_app(directory)
    if "installation_id" not in app:
        raise RuntimeError("no installation id saved: run `link` after installing the App")
    app_jwt = make_jwt(app["id"], os.path.join(directory, "private-key.pem"), now, openssl)
    status, data = api("POST", f"{API}/app/installations/{app['installation_id']}/access_tokens",
                       headers={"Authorization": f"Bearer {app_jwt}"}, body=b"")
    if status != 201 or "token" not in data:
        raise RuntimeError(f"GitHub refused the installation token (HTTP {status}): {data.get('message', '')}")
    return data["token"], data.get("expires_at", "")


def make_handler(context):
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):  # silence: the request line holds the one-time code
            pass

        def reply(self, status, body):
            data = body.encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self):
            parsed = urlparse(self.path)
            if parsed.path == "/":
                self.reply(200, landing_page(context["manifest"], context["state"], context["seat"], context["repo"]))
            elif parsed.path == "/callback":
                query = parse_qs(parsed.query)
                if query.get("state", [""])[0] != context["state"] or not query.get("code"):
                    self.reply(400, "State did not match or the code is missing; nothing was saved.")
                    return
                try:
                    result = context["convert"](query["code"][0])
                    context["info"] = save_credentials(context["directory"], result)
                except Exception as error:  # report, never echo secrets
                    context["error"] = str(error)
                    self.reply(500, "Registration failed: " + html.escape(str(error)))
                else:
                    slug = html.escape(context["info"]["slug"])
                    self.reply(200, "<!doctype html><meta charset=utf-8><h1>App created</h1>"
                                    f"<p>Now install it: <a href='https://github.com/apps/{slug}/installations/new'>"
                                    f"https://github.com/apps/{slug}/installations/new</a></p>"
                                    "<p>Choose only the repository, then return to the terminal.</p>")
                context["done"].set()
            else:
                self.reply(404, "not found")
    return Handler


def serve_registration(seat, repo, directory, port=0, allow_workflows=False, convert=convert_code, timeout=900,
                       on_ready=None):
    """Serve the landing page on 127.0.0.1 and wait for GitHub's redirect. Returns (info, error)."""
    context = {"seat": seat, "repo": repo, "directory": directory, "state": secrets.token_urlsafe(24),
               "convert": convert, "done": threading.Event(), "info": None, "error": None}
    server = HTTPServer(("127.0.0.1", port), make_handler(context))
    context["manifest"] = build_manifest(seat, repo, f"http://127.0.0.1:{server.server_port}/callback", allow_workflows)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        if on_ready:
            on_ready(f"http://127.0.0.1:{server.server_port}/", context)
        context["done"].wait(timeout)
    finally:
        server.shutdown()
        server.server_close()
    if not context["done"].is_set():
        context["error"] = "timed out waiting for the owner's click"
    return context["info"], context["error"]


def main(argv, now=time.time):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("command", choices=("register", "link", "token", "check"))
    parser.add_argument("--seat", required=True, choices=SEATS)
    parser.add_argument("--repo", default=DEFAULT_REPO)
    parser.add_argument("--root", help="seat credential root (default ~/.steward/seats)")
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--allow-workflows", action="store_true",
                        help="also ask for the workflows permission (changing .github/workflows)")
    parser.add_argument("--no-browser", action="store_true")
    parser.add_argument("--env", action="store_true", help="token: print `GH_TOKEN=<token>` instead of the bare token")
    args = parser.parse_args(argv)
    directory = seat_dir(args.seat, args.root)
    try:
        if args.command == "register":
            if os.path.exists(os.path.join(directory, "app.json")):
                print(f"{directory} already holds an App; delete it first to register again", file=sys.stderr)
                return 2

            def ready(url, context):
                print(f"Open {url} in the browser where you are signed in to GitHub, then click the buttons.\n"
                      f"Waiting up to 15 minutes for the redirect; the one-time code is valid for one hour.",
                      file=sys.stderr)
                if not args.no_browser:
                    webbrowser.open(url)
            info, error = serve_registration(args.seat, args.repo, directory, args.port, args.allow_workflows,
                                             on_ready=ready)
            if error:
                print(f"registration failed: {error}", file=sys.stderr)
                return 1
            print(f"Created {info['slug']} (id {info['id']}). Credentials are in {directory}.\n"
                  f"Install it: https://github.com/apps/{info['slug']}/installations/new\n"
                  f"Then run: seat_app_setup.py link --seat {args.seat}", file=sys.stderr)
            return 0
        app = load_app(directory)
        pem = os.path.join(directory, "private-key.pem")
        if args.command == "link":
            installation = find_installation(make_jwt(app["id"], pem, now()), args.repo)
            app["installation_id"] = installation
            with open(os.path.join(directory, "app.json"), "w", encoding="utf-8", newline="\n") as handle:
                json.dump(app, handle, indent=2)
                handle.write("\n")
            print(f"Linked {app['slug']} to installation {installation}.", file=sys.stderr)
            return 0
        token, expires = mint_token(directory, now())
        if args.command == "token":
            print(f"GH_TOKEN={token}" if args.env else token)
            return 0
        status, data = http_json("GET", f"{API}/installation/repositories", headers={"Authorization": f"Bearer {token}"})
        names = [r.get("full_name") for r in data.get("repositories", [])] if status == 200 else []
        print(f"app: {app['slug']}  token expires: {expires}  reaches {args.repo}: {args.repo in names}")
        return 0 if args.repo in names else 1
    except (RuntimeError, FileNotFoundError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
