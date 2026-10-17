import ast
import io
from pathlib import Path
import sys
import tarfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).parent))
import publication_check as pc


class PublicationInventory(unittest.TestCase):
    def test_credentials_and_private_artifacts_are_blockers_without_value_disclosure(self):
        token = ("sk-" + "lm-" + "a" * 20).encode()
        hits = pc.inspect_bytes("config/worker.toml", b"key=" + token)
        self.assertEqual({h["kind"] for h in hits}, {"private-artifact", "credential"})
        self.assertNotIn(token.decode(), str(hits))
        self.assertTrue(pc.inspect_bytes("seats/private-key.pem", b"key"))
        self.assertEqual(pc.inspect_bytes(".env.example", b"KEY=example"), [])

    def test_personal_paths_are_review_locations_not_credential_findings(self):
        hits = pc.inspect_bytes("doc.md", b"C:\\Users\\someone\\tools\n")
        self.assertEqual(hits, [{"severity": "review", "kind": "personal-path", "file": "doc.md", "line": 1}])

    def test_archive_traversal_is_reported_without_extraction(self):
        data = io.BytesIO()
        with tarfile.open(fileobj=data, mode="w") as archive:
            member = tarfile.TarInfo("../escape")
            member.size = 1
            archive.addfile(member, io.BytesIO(b"x"))
        hits = pc.inspect_archive("source.tar", data.getvalue())
        self.assertEqual(hits[0]["kind"], "unsafe-archive-member")

    def test_archive_contents_are_scanned(self):
        data = io.BytesIO()
        token = ("sk-" + "lm-" + "b" * 20).encode()
        with tarfile.open(fileobj=data, mode="w") as archive:
            member = tarfile.TarInfo("script.txt")
            member.size = len(token)
            archive.addfile(member, io.BytesIO(token))
        hits = pc.inspect_archive("source.tar", data.getvalue())
        self.assertEqual(hits[0]["file"], "source.tar!script.txt")
        self.assertEqual(hits[0]["kind"], "credential")

    def test_private_runtime_files_inside_archive_are_blocked(self):
        data = io.BytesIO()
        with tarfile.open(fileobj=data, mode="w") as archive:
            member = tarfile.TarInfo("config/worker.toml")
            member.size = 1
            archive.addfile(member, io.BytesIO(b"x"))
        hits = pc.inspect_archive("source.tar", data.getvalue())
        self.assertEqual(hits[0]["kind"], "private-artifact")


class HistoricalAutopilot(unittest.TestCase):
    def test_command_prompt_is_literal_and_shell_strings_are_rejected(self):
        source = Path(__file__).resolve().parents[1] / "migrations/aaroneous-2026-09-16/second-tranche/tools/autopilot.py"
        tree = ast.parse(source.read_text(encoding="utf-8"))
        function = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == "run_cmd")
        process = mock.Mock()
        process.run.return_value.returncode = 0
        process.run.return_value.stdout = ""
        process.run.return_value.stderr = ""
        namespace = {"subprocess": process}
        exec(compile(ast.Module(body=[function], type_ignores=[]), str(source), "exec"), namespace)
        prompt = 'task" & echo INJECTED & "\n$(unexpected)'
        argv = ["python", "tools/cloud_agent.py", prompt]
        namespace["run_cmd"](argv)
        self.assertEqual(process.run.call_args.args[0], argv)
        self.assertIs(process.run.call_args.kwargs["shell"], False)
        with self.assertRaises(TypeError):
            namespace["run_cmd"]("python tools/cloud_agent.py")


if __name__ == "__main__":
    unittest.main()
