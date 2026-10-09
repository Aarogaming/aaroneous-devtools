import base64
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
import unittest.mock
import urllib.error
import urllib.request

sys.path.insert(0, os.path.dirname(__file__))
import seat_app_setup as sa  # noqa: E402

HAVE_OPENSSL = shutil.which("openssl") is not None
PEM = "-----BEGIN RSA PRIVATE KEY-----\nplaceholder\n-----END RSA PRIVATE KEY-----\n"


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


class OneCommandSetup(unittest.TestCase):
    """`setup` registers, waits for the install click, links and checks, with the GitHub pages as the only manual steps."""

    def test_the_landing_page_submits_itself_and_the_callback_page_forwards_to_the_install_page(self):
        manifest = sa.build_manifest("claude", "o/r", "http://127.0.0.1:5/callback")
        self.assertIn("document.forms[0].submit()", sa.landing_page(manifest, "S", "claude", "o/r"))
        directory = tempfile.mkdtemp(prefix="seat-app-test-")
        self.addCleanup(shutil.rmtree, directory, True)
        seen = {}

        def ready(url, context):
            seen["callback"] = fetch(f"{url}callback?code=abc&state={context['state']}")
            context["done"].set()

        sa.serve_registration("claude", "o/r", directory, convert=fake_conversion, timeout=10, on_ready=ready)
        install = "https://github.com/apps/aaroneous-claude-seat/installations/new"
        self.assertIn(f"http-equiv=refresh content='1;url={install}'", seen["callback"][1])

    def waiting_fixture(self, answers):
        directory = tempfile.mkdtemp(prefix="seat-app-test-")
        self.addCleanup(shutil.rmtree, directory, True)
        sa.save_credentials(directory, {"id": 7, "slug": "aaroneous-claude-seat", "pem": PEM})
        clock = {"t": 1_000_000.0}
        calls = []

        def api(method, url, headers=None, body=None, opener=None):
            calls.append(url)
            return answers(url, len(calls))

        def sleep(seconds):
            clock["t"] += seconds
        return directory, clock, calls, api, sleep

    def test_it_polls_until_the_owner_has_installed_the_app(self):
        def answers(url, n):
            return 200, ([] if n < 3 else [{"id": 99, "account": {"login": "Aarogaming"}}])
        directory, clock, calls, api, sleep = self.waiting_fixture(answers)
        with unittest.mock.patch.object(sa, "make_jwt", return_value="jwt"):
            found = sa.wait_for_installation(directory, "Aarogaming/r", lambda: clock["t"], api, sleep, timeout=60,
                                             interval=3)
        self.assertEqual(found, 99)
        self.assertEqual(len(calls), 3)

    def test_it_gives_up_with_an_instruction_when_the_install_click_never_comes(self):
        directory, clock, calls, api, sleep = self.waiting_fixture(lambda url, n: (200, []))
        with unittest.mock.patch.object(sa, "make_jwt", return_value="jwt"):
            with self.assertRaises(RuntimeError) as caught:
                sa.wait_for_installation(directory, "Aarogaming/r", lambda: clock["t"], api, sleep, timeout=10,
                                         interval=3)
        self.assertIn("run setup again", str(caught.exception))

    def test_a_rejected_credential_stops_the_wait_at_once(self):
        directory, clock, calls, api, sleep = self.waiting_fixture(lambda url, n: (401, {"message": "bad"}))
        with unittest.mock.patch.object(sa, "make_jwt", return_value="jwt"):
            with self.assertRaises(RuntimeError) as caught:
                sa.wait_for_installation(directory, "Aarogaming/r", lambda: clock["t"], api, sleep, timeout=600)
        self.assertNotIn("not installed", str(caught.exception))
        self.assertEqual(len(calls), 1)

    def full_setup(self, installed_after, repos):
        root = tempfile.mkdtemp(prefix="seat-app-test-")
        self.addCleanup(shutil.rmtree, root, True)
        clock = {"t": 1_000_000.0}
        steps = []

        def api(method, url, headers=None, body=None, opener=None):
            if url.endswith("/app/installations"):
                n = sum(1 for s in steps if s == "poll")
                steps.append("poll")
                return 200, ([] if n < installed_after else [{"id": 55, "account": {"login": "Aarogaming"}}])
            if url.endswith("/access_tokens"):
                return 201, {"token": "ghs_tok", "expires_at": "2026-10-09T09:00:00Z"}
            if url.endswith("/installation/repositories"):
                return 200, {"repositories": [{"full_name": r} for r in repos]}
            raise AssertionError(url)

        def ready(url, context):
            fetch(f"{url}callback?code=abc&state={context['state']}")

        said = []
        with unittest.mock.patch.object(sa, "make_jwt", return_value="jwt"):
            ok, summary = sa.run_setup("claude", "Aarogaming/r", root, said.append, open_browser=False,
                                       now=lambda: clock["t"], api=api, convert=fake_conversion,
                                       sleep=lambda s: clock.__setitem__("t", clock["t"] + s), on_ready=ready, timeout=10)
        return root, ok, summary, said, steps

    def test_one_run_registers_waits_for_the_install_links_and_proves_the_token_reaches_the_repo(self):
        root, ok, summary, said, steps = self.full_setup(installed_after=2, repos=["Aarogaming/r"])
        self.assertTrue(ok, summary)
        self.assertIn("posts as aaroneous-claude-seat[bot]", summary)
        self.assertEqual(sa.load_app(sa.seat_dir("claude", root))["installation_id"], 55)
        self.assertTrue(any("waiting for you to click Install" in line for line in said))
        self.assertNotIn("ghs_tok", summary + "".join(said))

    def test_a_token_that_does_not_reach_the_repository_fails_with_the_fix(self):
        root, ok, summary, said, steps = self.full_setup(installed_after=0, repos=["Aarogaming/other"])
        self.assertFalse(ok)
        self.assertIn("install the App on that repository", summary)

    def test_a_second_run_resumes_instead_of_registering_again(self):
        root, ok, summary, said, steps = self.full_setup(installed_after=0, repos=["Aarogaming/r"])
        self.assertTrue(ok)
        calls = []

        def api(method, url, headers=None, body=None, opener=None):
            calls.append(url)
            if url.endswith("/access_tokens"):
                return 201, {"token": "t", "expires_at": ""}
            return 200, {"repositories": [{"full_name": "Aarogaming/r"}]}

        def must_not_register(code):
            raise AssertionError("registered again")

        with unittest.mock.patch.object(sa, "make_jwt", return_value="jwt"):
            ok2, summary2 = sa.run_setup("claude", "Aarogaming/r", root, lambda line: None, now=lambda: 1_000_000.0,
                                         api=api, convert=must_not_register)
        self.assertTrue(ok2, summary2)
        self.assertFalse(any(url.endswith("/app/installations") for url in calls))

    @unittest.skipUnless(HAVE_OPENSSL, "openssl not installed")
    def test_an_unusable_private_key_is_reported_not_raised(self):
        root, ok, summary, said, steps = self.full_setup(installed_after=0, repos=["Aarogaming/r"])
        self.assertTrue(ok)
        ok2, summary2 = sa.run_setup("claude", "Aarogaming/r", root, lambda line: None, now=lambda: 1_000_000.0,
                                     api=lambda *a, **k: (200, {}), convert=fake_conversion)
        self.assertFalse(ok2)
        self.assertIn("is it intact", summary2)


if __name__ == "__main__":
    unittest.main()
