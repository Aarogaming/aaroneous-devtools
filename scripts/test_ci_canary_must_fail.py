import unittest


class Canary(unittest.TestCase):
    def test_this_test_fails_on_purpose_to_prove_the_ci_job_goes_red(self):
        self.assertEqual(1, 2, "canary: if CI is green with this file present, the job does not fail when a test fails")


if __name__ == "__main__":
    unittest.main()
