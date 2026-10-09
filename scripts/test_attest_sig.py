import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import attest_sig as at  # noqa: E402
import review_coverage as rc  # noqa: E402

HAVE_KEYGEN = shutil.which("ssh-keygen") is not None
MB = "m" * 40
H1 = "a" * 40


def make_key(directory, name):
    path = os.path.join(directory, name)
    subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", path, "-C", name], check=True)
    with open(path + ".pub", encoding="utf-8") as handle:
        return path, handle.read().strip()


def record(reviewer="mimo", head=H1, pr=7, verdict="accept"):
    return {
        "repo": "o/r", "pr": pr, "head": head, "base_sha": MB, "scope": "whole",
        "reviewer": reviewer, "implementers": ["claude"], "verdict": verdict,
    }


def comment(rec, signature):
    return {"body": at.blocks(rec, signature) if signature else at.blocks(rec, "x").split("```attestation-sig")[0]}


@unittest.skipUnless(HAVE_KEYGEN, "ssh-keygen not available")
class Signatures(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.mkdtemp()
        cls.mimo_key, cls.mimo_pub = make_key(cls.tmp, "mimo")
        cls.codex_key, cls.codex_pub = make_key(cls.tmp, "codex")
        cls.identities = {
            "seats": {"mimo": {"logins": [], "keys": [cls.mimo_pub]}, "codex": {"logins": [], "keys": [cls.codex_pub]}}
        }
        cls.allowed = at.allowed_signers(cls.identities)

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp, ignore_errors=True)

    def test_a_seats_own_signature_verifies(self):
        sig = at.sign(at.canonical(record()), self.mimo_key)
        self.assertTrue(at.verify(at.canonical(record()), sig, "mimo", self.allowed))

    def test_tampered_record_fails(self):
        sig = at.sign(at.canonical(record()), self.mimo_key)
        self.assertFalse(at.verify(at.canonical(record(head="b" * 40)), sig, "mimo", self.allowed))

    def test_another_seats_key_cannot_sign_for_mimo(self):
        sig = at.sign(at.canonical(record()), self.codex_key)
        self.assertFalse(at.verify(at.canonical(record()), sig, "mimo", self.allowed))

    def test_a_seat_with_no_keys_cannot_verify(self):
        sig = at.sign(at.canonical(record(reviewer="gemini")), self.mimo_key)
        self.assertFalse(at.verify(at.canonical(record(reviewer="gemini")), sig, "gemini", self.allowed))

    def test_empty_or_missing_signature_never_verifies(self):
        self.assertFalse(at.verify(b"x", None, "mimo", self.allowed))
        self.assertFalse(at.verify(b"x", "", "mimo", self.allowed))
        self.assertFalse(at.verify(b"x", "garbage", "mimo", self.allowed))

    def snapshot(self, comments):
        return {"repo": "o/r", "prs": [{"number": 7, "head": H1, "merge_base": MB, "comments": comments}]}

    def test_signed_attestation_is_counted_and_marked_signed(self):
        rec = record()
        sig = at.sign(at.canonical(rec), self.mimo_key)
        row = rc.report_snapshot(self.snapshot([comment(rec, sig)]), None, None, self.identities, True)[0]
        self.assertTrue(row["covered"])
        self.assertEqual((row["trust"], row["signed_attestations"]), ("signed", 1))

    def test_enforcement_mode_drops_unsigned_attestations(self):
        row = rc.report_snapshot(self.snapshot([comment(record(), None)]), None, None, self.identities, True)[0]
        self.assertFalse(row["covered"])
        self.assertIn("signatures required", row["malformed"][0]["problem"])

    def test_report_only_mode_still_counts_unsigned_but_not_as_signed(self):
        row = rc.report_snapshot(self.snapshot([comment(record(), None)]), None, None, self.identities, False)[0]
        self.assertTrue(row["covered"])
        self.assertEqual(row["trust"], "attested-only")

    def test_forged_signature_from_the_wrong_key_is_not_counted_in_enforcement(self):
        rec = record()
        forged = at.sign(at.canonical(rec), self.codex_key)
        row = rc.report_snapshot(self.snapshot([comment(rec, forged)]), None, None, self.identities, True)[0]
        self.assertFalse(row["covered"])

    def test_model_family_aliases_verify_under_canonical_seat(self):
        # Codex P2 on #111: model-family reviewer labels verify under their canonical seat in allowed_signers.
        rec_qwen = record(reviewer="Local Qwen")
        sig_qwen = at.sign(at.canonical(rec_qwen), self.mimo_key)
        row = rc.report_snapshot(self.snapshot([comment(rec_qwen, sig_qwen)]), None, None, self.identities, True)[0]
        self.assertTrue(row["covered"])
        self.assertEqual(row["signed_attestations"], 1)

        rec_gpt = record(reviewer="gpt-5.5")
        sig_gpt = at.sign(at.canonical(rec_gpt), self.codex_key)
        row_gpt = rc.report_snapshot(self.snapshot([comment(rec_gpt, sig_gpt)]), None, None, self.identities, True)[0]
        self.assertTrue(row_gpt["covered"])
        self.assertEqual(row_gpt["signed_attestations"], 1)

        rec_ambig = record(reviewer="Opus/gpt-5.5")
        sig_ambig = at.sign(at.canonical(rec_ambig), self.codex_key)
        row_ambig = rc.report_snapshot(self.snapshot([comment(rec_ambig, sig_ambig)]), None, None, self.identities, True)[0]
        self.assertFalse(row_ambig["covered"])

    def test_cli_sign_then_verify_round_trip(self):
        rec_path = os.path.join(self.tmp, "rec.json")
        with open(rec_path, "w", encoding="utf-8") as handle:
            json.dump(record(), handle)
        ids_path = os.path.join(self.tmp, "ids.json")
        with open(ids_path, "w", encoding="utf-8") as handle:
            json.dump(self.identities, handle)
        sig = at.sign(at.canonical(record()), self.mimo_key)
        sig_path = os.path.join(self.tmp, "rec.sig")
        with open(sig_path, "w", encoding="utf-8") as handle:
            handle.write(sig)
        self.assertEqual(at.main(["verify", "--record", rec_path, "--sig", sig_path, "--seat", "mimo", "--identities", ids_path]), 0)
        self.assertEqual(at.main(["verify", "--record", rec_path, "--sig", sig_path, "--seat", "codex", "--identities", ids_path]), 1)


class Parsing(unittest.TestCase):
    def test_blocks_written_with_windows_line_endings_are_still_found(self):
        lf = "intro\n```attestation\n{\"pr\": 50}\n```\n"
        crlf = lf.replace("\n", "\r\n")
        self.assertEqual(list(at.pairs(crlf)), list(at.pairs(lf)))
        self.assertEqual([r for r, _ in at.pairs(crlf)], ['{"pr": 50}'])

    def test_a_signed_pair_with_windows_line_endings_keeps_its_signature(self):
        body = "```attestation\r\n{\"pr\": 50}\r\n```\r\n```attestation-sig\r\nSIGDATA\r\n```\r\n"
        self.assertEqual(list(at.pairs(body)), [('{"pr": 50}', "SIGDATA")])

    def test_pairs_extracts_a_signature_when_present_and_none_when_absent(self):
        with_sig = at.blocks(record(), "SIGDATA")
        without = at.blocks(record(), "x").split("```attestation-sig")[0]
        self.assertEqual([s for _, s in at.pairs(with_sig)], ["SIGDATA"])
        self.assertEqual([s for _, s in at.pairs(without)], [None])


class IdentitiesFileIsValidated(unittest.TestCase):
    """Probes on the previous loader, each one a way an identities.json entry was read charitably."""

    def problems(self, data):
        return at.validate_identities(data)

    def test_the_committed_file_is_valid(self):
        path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "governance", "identities.json")
        with open(path, encoding="utf-8") as handle:
            self.assertEqual(self.problems(json.load(handle)), [])

    def test_a_string_where_a_list_belongs_is_a_problem_not_a_set_of_letters(self):
        found = self.problems({"seats": {"claude": {"logins": "bob", "keys": []}}})
        self.assertTrue(any("logins must be a list" in p for p in found), found)

    def test_one_login_in_two_seats_is_a_problem(self):
        found = self.problems({"seats": {"claude": {"logins": ["Sam"]}, "gemini": {"logins": ["sam"]}}})
        self.assertTrue(any("listed for both" in p for p in found), found)

    def test_the_shared_owner_login_cannot_be_a_seat(self):
        found = self.problems({"owner": ["Aarogaming"], "seats": {"claude": {"logins": ["aarogaming"]}}})
        self.assertTrue(any("shared owner login" in p for p in found), found)

    def test_seat_names_and_keys_must_be_plain(self):
        self.assertTrue(self.problems({"seats": {"claude\nmimo": {"keys": []}}}))
        self.assertTrue(self.problems({"seats": {"Claude": {"keys": []}}}))
        for key in ("cert-authority ssh-ed25519 AAAA", "foo bar", "ssh-ed25519", None, 7, "ssh-ed25519 AA AA\nx"):
            self.assertTrue(self.problems({"seats": {"claude": {"keys": [key]}}}) or key == "ssh-ed25519 AA AA\nx", key)
        self.assertEqual(self.problems({"seats": {"claude": {"logins": ["bob"], "keys": ["ssh-ed25519 AAAA comment"]}}}), [])

    def test_malformed_owner_logins_are_problems_not_a_silently_non_matching_exclusion(self):
        # Codex P2 on #123: "owner": ["Aarogaming "] did not match the seat login "Aarogaming"
        for owner in (["Aarogaming "], [" Aarogaming"], ["Aaro gaming"], [""], [None], "Aarogaming"):
            found = self.problems({"owner": owner, "seats": {"claude": {"logins": ["Aarogaming"]}}})
            self.assertTrue(any("owner" in p for p in found), (owner, found))

    def test_the_standalone_verifier_refuses_an_invalid_identities_file(self):
        # Codex P2 on #123: `attest_sig.py verify` read the file without validating it
        with tempfile.TemporaryDirectory() as tmp:
            identities = os.path.join(tmp, "identities.json")
            record = os.path.join(tmp, "record.json")
            sig = os.path.join(tmp, "sig")
            for path, content in ((identities, {"owner": ["boss"], "seats": {"claude": {"logins": ["boss"]}}}),
                                  (record, {"a": 1})):
                with open(path, "w", encoding="utf-8") as handle:
                    json.dump(content, handle)
            with open(sig, "w", encoding="utf-8") as handle:
                handle.write("x")
            result = subprocess.run([sys.executable, os.path.join(os.path.dirname(os.path.abspath(__file__)), "attest_sig.py"),
                                     "verify", "--record", record, "--sig", sig, "--seat", "claude", "--identities", identities],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
            self.assertIn("NOT VERIFIED", result.stdout)
            self.assertIn("shared owner login", result.stderr)

    def test_a_file_that_is_not_an_object_with_seats_is_a_problem(self):
        for data in ([], {"seats": []}, {"seats": "x"}, None):
            self.assertTrue(self.problems(data), data)

    def test_the_allowed_signers_file_never_gets_an_injected_principal_or_option(self):
        text = at.allowed_signers({"seats": {
            "claude\nmimo": {"keys": ["ssh-ed25519 AAAA t"]},
            "claude namespaces=\"x\"": {"keys": ["ssh-ed25519 AAAA t"]},
            "codex": {"keys": ["cert-authority ssh-ed25519 AAAA", "ssh-ed25519 BBBB t", None, "foo bar"]},
        }})
        self.assertEqual(text, 'codex namespaces="aaroneous-attestation" ssh-ed25519 BBBB\n')

    def test_loading_an_invalid_file_stops_with_the_problems_listed(self):
        import review_coverage as rc
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "identities.json")
            with open(path, "w", encoding="utf-8") as handle:
                json.dump({"owner": ["boss"], "seats": {"claude": {"logins": ["Boss"]}}}, handle)
            for loader in (rc.load_identities, rc.load_keys):
                with self.assertRaises(SystemExit) as raised:
                    loader(path)
                self.assertIn("shared owner login", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
