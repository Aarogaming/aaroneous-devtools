import base64
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
import urllib.error
import urllib.request

sys.path.insert(0, os.path.dirname(__file__))
import seat_app_setup as sa  # noqa: E402

HAVE_OPENSSL = shutil.which("openssl") is not None


def read_text(path):
    with open(path, encoding="utf-8") as handle:
        return handle.read()


def fetch(url):
    try:
        with urllib.request.urlopen(url, timeout=10) as response:
            return response.status, response.read().decode("utf-8")
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode("utf-8")


def fake_conversion(code):
    return {"id": 4242, "slug": "aaroneous-claude-seat", "name": "aaroneous-claude-seat", "client_id": "Iv1.abc",
            "html_url": "https://github.com/apps/aaroneous-claude-seat", "pem": "-----BEGIN RSA PRIVATE KEY-----\nxyz\n",
            "client_secret": "CLIENT-SECRET-VALUE", "webhook_secret": "WEBHOOK-SECRET-VALUE"}


class Manifest(unittest.TestCase):
    def test_the_manifest_names_the_seat_the_redirect_and_least_privilege(self):
        manifest = sa.build_manifest("gemini", "o/r", "http://127.0.0.1:5/callback")
        self.assertEqual(manifest["name"], "aaroneous-gemini-seat")
        self.assertEqual(manifest["redirect_url"], "http://127.0.0.1:5/callback")
        self.assertFalse(manifest["public"])
        self.assertEqual(manifest["default_permissions"]["pull_requests"], "write")
        self.assertNotIn("workflows", manifest["default_permissions"])
        self.assertNotIn("administration", manifest["default_permissions"])

    def test_workflows_permission_only_on_request(self):
        manifest = sa.build_manifest("claude", "o/r", "http://127.0.0.1:5/callback", allow_workflows=True)
        self.assertEqual(manifest["default_permissions"]["workflows"], "write")

    def test_the_landing_page_posts_the_escaped_manifest_to_github_with_the_state(self):
        manifest = sa.build_manifest("claude", "o/r", "http://127.0.0.1:5/callback")
        page = sa.landing_page(manifest, "STATE123", "claude", "o/r")
        self.assertIn("https://github.com/settings/apps/new?state=STATE123", page)
        self.assertIn("method=post", page)
        self.assertNotIn("'name': ", page)
        self.assertIn("&quot;name&quot;: &quot;aaroneous-claude-seat&quot;", page)


class Registration(unittest.TestCase):
    def run_flow(self, make_callback, convert=fake_conversion):
        directory = tempfile.mkdtemp(prefix="seat-app-test-")
        self.addCleanup(shutil.rmtree, directory, True)
        seen = {}

        def ready(url, context):
            seen["context"] = context
            seen["landing"] = fetch(url)
            seen["callback"] = fetch(make_callback(url, context))
            context["done"].set()  # a refused request keeps the server waiting for the real redirect; stop the test

        info, error = sa.serve_registration("claude", "o/r", directory, convert=convert, timeout=10, on_ready=ready)
        return directory, info, error, seen

    def test_a_matching_state_saves_the_id_and_key_and_offers_the_install_link(self):
        directory, info, error, seen = self.run_flow(lambda url, ctx: f"{url}callback?code=abc&state={ctx['state']}")
        self.assertIsNone(error)
        self.assertEqual(seen["landing"][0], 200)
        self.assertEqual(seen["callback"][0], 200)
        self.assertIn("https://github.com/apps/aaroneous-claude-seat/installations/new", seen["callback"][1])
        saved = json.loads(read_text(os.path.join(directory, "app.json")))
        self.assertEqual((saved["id"], saved["slug"]), (4242, "aaroneous-claude-seat"))
        self.assertTrue(os.path.exists(os.path.join(directory, "private-key.pem")))

    def test_the_webhook_and_client_secrets_are_never_written_to_disk(self):
        directory, *_ = self.run_flow(lambda url, ctx: f"{url}callback?code=abc&state={ctx['state']}")
        for name in os.listdir(directory):
            text = read_text(os.path.join(directory, name))
            self.assertNotIn("CLIENT-SECRET-VALUE", text, name)
            self.assertNotIn("WEBHOOK-SECRET-VALUE", text, name)

    def test_a_wrong_state_is_refused_and_the_code_is_never_exchanged(self):
        calls = []
        directory, info, error, seen = self.run_flow(lambda url, ctx: f"{url}callback?code=abc&state=WRONG",
                                                     convert=lambda code: calls.append(code) or fake_conversion(code))
        self.assertEqual(seen["callback"][0], 400)
        self.assertEqual(calls, [])
        self.assertFalse(os.path.exists(os.path.join(directory, "app.json")))

    def test_a_missing_code_is_refused(self):
        directory, info, error, seen = self.run_flow(lambda url, ctx: f"{url}callback?state={ctx['state']}")
        self.assertEqual(seen["callback"][0], 400)

    def test_a_failed_exchange_is_reported_without_saving_anything(self):
        def refuse(code):
            raise RuntimeError("GitHub refused the manifest code (HTTP 404): Not Found")
        directory, info, error, seen = self.run_flow(lambda url, ctx: f"{url}callback?code=abc&state={ctx['state']}",
                                                     convert=refuse)
        self.assertEqual(seen["callback"][0], 500)
        self.assertIn("HTTP 404", error)
        self.assertFalse(os.path.exists(os.path.join(directory, "app.json")))


@unittest.skipUnless(HAVE_OPENSSL, "openssl not installed")
class Tokens(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.keys = tempfile.mkdtemp(prefix="seat-app-keys-")
        cls.pem = os.path.join(cls.keys, "private-key.pem")
        cls.public = os.path.join(cls.keys, "public.pem")
        subprocess.run(["openssl", "genrsa", "-out", cls.pem, "2048"], check=True, capture_output=True)
        subprocess.run(["openssl", "rsa", "-in", cls.pem, "-pubout", "-out", cls.public], check=True,
                       capture_output=True)

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.keys, True)

    def setUp(self):
        self.directory = tempfile.mkdtemp(prefix="seat-app-test-")
        self.addCleanup(shutil.rmtree, self.directory, True)

    def test_the_jwt_is_rs256_short_lived_and_verifies_against_the_public_key(self):
        token = sa.make_jwt(4242, self.pem, now=1_000_000)
        header, payload, signature = token.split(".")
        pad = lambda s: s + "=" * (-len(s) % 4)  # noqa: E731
        self.assertEqual(json.loads(base64.urlsafe_b64decode(pad(header))), {"alg": "RS256", "typ": "JWT"})
        claims = json.loads(base64.urlsafe_b64decode(pad(payload)))
        self.assertEqual(claims["iss"], 4242)
        self.assertLessEqual(claims["exp"] - claims["iat"], 600)
        sig_file = os.path.join(self.directory, "sig.bin")
        with open(sig_file, "wb") as handle:
            handle.write(base64.urlsafe_b64decode(pad(signature)))
        verified = subprocess.run(["openssl", "dgst", "-sha256", "-verify", self.public, "-signature", sig_file],
                                  input=f"{header}.{payload}".encode(), capture_output=True)
        self.assertEqual(verified.returncode, 0, verified.stdout + verified.stderr)

    def test_minting_a_token_posts_to_the_installation_with_the_app_jwt(self):
        info = sa.save_credentials(self.directory, {"id": 4242, "slug": "aaroneous-claude-seat",
                                                    "pem": read_text(self.pem)})
        app = dict(info, installation_id=777)
        with open(os.path.join(self.directory, "app.json"), "w", encoding="utf-8") as handle:
            json.dump(app, handle)
        seen = {}

        def fake_api(method, url, headers=None, body=None, opener=None):
            seen.update(method=method, url=url, auth=(headers or {}).get("Authorization", ""))
            return 201, {"token": "ghs_exampletoken", "expires_at": "2026-10-09T08:00:00Z"}

        token, expires = sa.mint_token(self.directory, now=1_000_000, api=fake_api)
        self.assertEqual((token, expires), ("ghs_exampletoken", "2026-10-09T08:00:00Z"))
        self.assertEqual(seen["method"], "POST")
        self.assertEqual(seen["url"], "https://api.github.com/app/installations/777/access_tokens")
        self.assertTrue(seen["auth"].startswith("Bearer ") and seen["auth"].count(".") == 2)

    def test_no_saved_installation_is_an_instruction_not_a_traceback(self):
        sa.save_credentials(self.directory, {"id": 1, "slug": "s", "pem": read_text(self.pem)})
        with self.assertRaises(RuntimeError) as caught:
            sa.mint_token(self.directory, now=1_000_000)
        self.assertIn("run `link`", str(caught.exception))


class Installation(unittest.TestCase):
    def test_the_installation_on_the_repository_owners_account_is_chosen(self):
        def fake_api(method, url, headers=None, body=None, opener=None):
            return 200, [{"id": 1, "account": {"login": "someone-else"}}, {"id": 2, "account": {"login": "Aarogaming"}}]
        self.assertEqual(sa.find_installation("jwt", "aarogaming/aaroneous-devtools", api=fake_api), 2)

    def test_an_app_that_is_not_installed_yet_says_so(self):
        def fake_api(method, url, headers=None, body=None, opener=None):
            return 200, []
        with self.assertRaises(RuntimeError) as caught:
            sa.find_installation("jwt", "Aarogaming/aaroneous-devtools", api=fake_api)
        self.assertIn("not installed", str(caught.exception))

    def test_a_refused_listing_is_an_error(self):
        def fake_api(method, url, headers=None, body=None, opener=None):
            return 401, {"message": "bad credentials"}
        with self.assertRaises(RuntimeError):
            sa.find_installation("jwt", "o/r", api=fake_api)


class Paths(unittest.TestCase):
    def test_seat_credentials_default_outside_any_repository(self):
        path = sa.seat_dir("claude")
        self.assertTrue(path.endswith(os.path.join(".steward", "seats", "claude")))
        self.assertEqual(sa.seat_dir("mimo", root="/x/y"), os.path.join("/x/y", "mimo"))


if __name__ == "__main__":
    unittest.main()
