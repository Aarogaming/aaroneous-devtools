#!/usr/bin/env python3
"""Tests for the C83 Foundry gap register policy checker."""

import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import check_gap_register as cgr


class GapRegisterTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.register_path = os.path.join(self.tmp.name, "FOUNDRY_GAP_REGISTER.md")
        self.queue_path = os.path.join(self.tmp.name, "COORDINATION_QUEUE.md")

    def write_queue(self, rows: list[tuple[str, str]]):
        lines = [
            "# Coordination Queue",
            "| ID | Scope | Status | Owner | Branch | Started UTC | Evidence |",
            "| --- | --- | --- | --- | --- | --- | --- |",
        ]
        for lane_id, status in rows:
            lines.append(f"| {lane_id} | Test scope for {lane_id} | {status} | unowned | - | - | None |")
        lines.append("")
        with open(self.queue_path, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))

    def write_register(self, entries: list[dict]):
        lines = ["# Foundry gap register\n"]
        for e in entries:
            lines.append(f"## {e['id']} — {e.get('title', 'Test gap title')}\n")
            if "boundary" in e:
                lines.append(f"- Boundary: {e['boundary']}")
            if "evidence" in e:
                lines.append(f"- Evidence: {e['evidence']}")
            if "risk" in e:
                lines.append(f"- Risk: {e['risk']}")
            if "disposition" in e:
                lines.append(f"- Disposition: {e['disposition']}")
            lines.append("")
        with open(self.register_path, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))

    def test_current_repository_files_pass(self):
        """Current repository files must pass with zero errors."""
        repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        actual_reg = os.path.join(repo_root, "governance", "FOUNDRY_GAP_REGISTER.md")
        actual_queue = os.path.join(repo_root, "governance", "COORDINATION_QUEUE.md")
        errors = cgr.check(register_path=actual_reg, queue_path=actual_queue)
        self.assertEqual(errors, [])

    def test_valid_entry_passes(self):
        self.write_queue([("C79", "Ready"), ("C82", "Blocked")])
        self.write_register([
            {
                "id": "G-2026-01",
                "title": "Valid gap entry",
                "boundary": "Aaroneous product capability discovery",
                "evidence": "`xtask/src/workspace_metadata.rs:217` lacks Git revision and gate selection contract",
                "risk": "Untraceable controller execution",
                "disposition": "**C79 Ready** — implement product capability manifest; C82 consumes it",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertEqual(errors, [])

    def test_reject_vague_evidence_keywords(self):
        self.write_queue([("C79", "Ready")])
        for vague in ["No evidence observed yet", "review", "command", "observed"]:
            self.write_register([
                {
                    "id": "G-2026-01",
                    "boundary": "Some boundary",
                    "evidence": vague,
                    "risk": "Some risk",
                    "disposition": "**C79 Ready**",
                }
            ])
            errors = cgr.check(self.register_path, self.queue_path)
            self.assertTrue(any("missing evidence" in e for e in errors), f"Expected rejection for '{vague}': {errors}")

    def test_reject_stale_review_disposition_when_done(self):
        self.write_queue([("C83", "Done")])
        self.write_register([
            {
                "id": "G-2026-05",
                "boundary": "Boundary",
                "evidence": "`some/file.rs:1` evidence",
                "risk": "Risk",
                "disposition": "**C83 Review** — stale review claim",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertTrue(any("unupdated entry" in e for e in errors), errors)

    def test_reject_missing_evidence(self):
        self.write_queue([("C79", "Ready")])
        # Completely missing evidence
        self.write_register([
            {
                "id": "G-2026-01",
                "boundary": "Some boundary",
                "risk": "Some risk",
                "disposition": "**C79 Ready**",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertTrue(any("missing evidence" in e for e in errors), errors)

        # Placeholder evidence
        self.write_register([
            {
                "id": "G-2026-01",
                "boundary": "Some boundary",
                "evidence": "TBD",
                "risk": "Some risk",
                "disposition": "**C79 Ready**",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertTrue(any("missing evidence" in e for e in errors), errors)

    def test_reject_missing_disposition(self):
        self.write_queue([("C79", "Ready")])
        self.write_register([
            {
                "id": "G-2026-01",
                "boundary": "Some boundary",
                "evidence": "`src/lib.rs:10` has unchecked unwraps",
                "risk": "Panic at runtime",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertTrue(any("missing disposition" in e for e in errors), errors)

    def test_reject_invalid_disposition(self):
        self.write_queue([("C79", "Ready")])
        self.write_register([
            {
                "id": "G-2026-01",
                "boundary": "Some boundary",
                "evidence": "`src/lib.rs:10` has unchecked unwraps",
                "risk": "Panic at runtime",
                "disposition": "We might fix this someday soon without any lane",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertTrue(any("invalid disposition" in e for e in errors), errors)

    def test_reject_missing_boundary_and_risk(self):
        self.write_queue([("C79", "Ready")])
        self.write_register([
            {
                "id": "G-2026-01",
                "evidence": "`src/lib.rs:10` has unchecked unwraps",
                "disposition": "**C79 Ready**",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertTrue(any("missing boundary" in e for e in errors), errors)
        self.assertTrue(any("missing risk" in e for e in errors), errors)

    def test_reject_unknown_lane_id(self):
        self.write_queue([("C79", "Ready")])
        self.write_register([
            {
                "id": "G-2026-01",
                "boundary": "Boundary A",
                "evidence": "`crates/paths/src/lib.rs:114` falls back to current_exe",
                "risk": "Ambiguous paths",
                "disposition": "**C999 Ready** — undefined lane",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertTrue(any("not found in coordination queue" in e for e in errors), errors)

    def test_reject_false_done_linkage(self):
        # Queue has C80 as Ready, but disposition falsely claims C80 Done / Accepted
        self.write_queue([("C80", "Ready")])
        self.write_register([
            {
                "id": "G-2026-02",
                "boundary": "MCP dispatch",
                "evidence": "`crates/mcp_server/src/mcp_service/service.rs:124` mints FullPrivilege",
                "risk": "Over-privileged external dispatch",
                "disposition": "**C80 Done** — completed caller-bound authorization",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertTrue(any("false Done linkage" in e for e in errors), errors)

    def test_reject_unupdated_entry_when_lane_is_done(self):
        # Queue has C89 marked Done, but disposition still says C89 Ready
        self.write_queue([("C89", "Done")])
        self.write_register([
            {
                "id": "G-2026-07",
                "boundary": "Cratify classification",
                "evidence": "`xtask/src/workspace_metadata.rs:100` lacks build sheet",
                "risk": "Unverified crate lifecycle",
                "disposition": "**C89 Ready** — declare crate profile in manifest",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertTrue(any("unupdated entry" in e for e in errors), errors)

    def test_accept_blocked_owner_decision(self):
        self.write_queue([])
        self.write_register([
            {
                "id": "G-2026-09",
                "boundary": "Licensing",
                "evidence": "`docs/rfcs/0101.md:14` requires owner decision on dual licensing",
                "risk": "Legal uncertainty",
                "disposition": "**Blocked on owner decision** regarding license model",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertEqual(errors, [])

    def test_accept_explicit_rejection(self):
        self.write_queue([])
        self.write_register([
            {
                "id": "G-2026-10",
                "boundary": "Build system",
                "evidence": "`build.rs:14` checks for optional clang toolchain",
                "risk": "Optional clang missing",
                "disposition": "**Rejected** — clang is an intentional optional acceleration; no defect",
            }
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertEqual(errors, [])

    def test_reject_duplicate_gap_id(self):
        self.write_queue([("C79", "Ready")])
        self.write_register([
            {
                "id": "G-2026-01",
                "boundary": "Boundary 1",
                "evidence": "Source citation `a.rs:1`",
                "risk": "Risk 1",
                "disposition": "**C79 Ready**",
            },
            {
                "id": "G-2026-01",
                "boundary": "Boundary 2",
                "evidence": "Source citation `b.rs:2`",
                "risk": "Risk 2",
                "disposition": "**C79 Ready**",
            },
        ])
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertTrue(any("duplicate gap identifier" in e for e in errors), errors)

    def test_reject_empty_register(self):
        self.write_queue([])
        with open(self.register_path, "w", encoding="utf-8") as f:
            f.write("# Empty register\n")
        errors = cgr.check(self.register_path, self.queue_path)
        self.assertTrue(any("no gap entries found" in e for e in errors), errors)


if __name__ == "__main__":
    unittest.main()
