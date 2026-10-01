import unittest

from profile_manifest_report import expected_profiles


class ExpectedProfiles(unittest.TestCase):
    def test_reads_indented_table_rows_and_ignores_prose(self):
        text = (
            "  | Profile | Crates |\n"
            "  | `kernel` | `core/hypervisor` (library), `wire` | Adds |\n"
            "  | `tooling` | `xtask` | Adds |\n"
            "- **Three-Phase** (`kernel`): not a table row\n"
        )
        self.assertEqual(
            expected_profiles(text),
            {"core/hypervisor": "kernel", "wire": "kernel", "xtask": "tooling"},
        )


if __name__ == "__main__":
    unittest.main()
