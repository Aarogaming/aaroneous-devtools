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
    def test_pairs_extracts_a_signature_when_present_and_none_when_absent(self):
        with_sig = at.blocks(record(), "SIGDATA")
        without = at.blocks(record(), "x").split("```attestation-sig")[0]
        self.assertEqual([s for _, s in at.pairs(with_sig)], ["SIGDATA"])
        self.assertEqual([s for _, s in at.pairs(without)], [None])


if __name__ == "__main__":
    unittest.main()
