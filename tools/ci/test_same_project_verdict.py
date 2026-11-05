#!/usr/bin/env python3
"""Failure-first tests of the pure judgement in same_project_verdict.py: what must never read as a pass."""
import unittest

import same_project_verdict as v

PASS = "bodies across saves: PASS\nexit 0\n"
AREAS = "areas across saves: passed: 25 Areas, 25 with a body, on each of 4 saves, every field equal\nexit 0\n"
SKIP_AREAS = "areas across saves: skipped: no rac_advanced sample of 2024 in this run, so nothing was compared\n"


class Judge(unittest.TestCase):
    def conclusion(self, bodies, areas, not_run=None):
        return v.judge(bodies, areas, not_run)[0]

    def test_a_run_that_could_not_check_is_not_a_pass(self):
        self.assertEqual(self.conclusion({"a": PASS}, [AREAS], "samples=true was not set"), "neutral")

    def test_no_samples_expected_is_not_a_pass(self):
        self.assertEqual(self.conclusion({}, [AREAS]), "neutral")

    def test_a_missing_bodies_result_fails(self):
        self.assertEqual(self.conclusion({"a": PASS, "b": None}, [AREAS]), "failure")

    def test_a_pass_line_with_a_failed_exit_fails(self):
        self.assertEqual(self.conclusion({"a": "bodies across saves: PASS\nexit 1\n"}, [AREAS]), "failure")

    def test_partial_and_skipped_bodies_are_not_a_pass(self):
        for word in ("PARTIAL (not compared: 2025)", "SKIPPED (no save)"):
            self.assertEqual(self.conclusion({"a": f"bodies across saves: {word}\nexit 0\n"}, [AREAS]), "neutral")

    def test_areas_only_skipped_is_not_a_pass(self):
        self.assertEqual(self.conclusion({"a": PASS}, [SKIP_AREAS]), "neutral")

    def test_no_areas_result_is_not_a_pass(self):
        self.assertEqual(self.conclusion({"a": PASS}, []), "neutral")

    def test_a_failed_areas_shard_beside_a_skipped_one_fails(self):
        self.assertEqual(self.conclusion({"a": PASS}, [SKIP_AREAS, "areas across saves: FAILED: 2025\nexit 1\n"]), "failure")

    def test_a_failure_outranks_a_skip(self):
        self.assertEqual(self.conclusion({"a": None, "b": "bodies across saves: SKIPPED (x)\nexit 0\n"}, [AREAS]), "failure")

    def test_all_passing_passes(self):
        self.assertEqual(self.conclusion({"a": PASS, "b": PASS}, [SKIP_AREAS, AREAS]), "success")


class ExpectedSamples(unittest.TestCase):
    def test_a_narrow_range_holds_only_its_years(self):
        self.assertEqual(len(v.expected_samples("2026 2026")), 3)
        self.assertEqual(v.expected_samples("2008 2023"), [])

    def test_no_range_holds_all_six(self):
        self.assertEqual(len(v.expected_samples("")), 6)


if __name__ == "__main__":
    unittest.main()
