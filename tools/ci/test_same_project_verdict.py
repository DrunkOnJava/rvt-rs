#!/usr/bin/env python3
"""Failure-first tests of the pure judgement in same_project_verdict.py: what must never read as a pass."""
import pathlib
import re
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

    def test_no_areas_result_fails(self):
        self.assertEqual(self.conclusion({"a": PASS}, []), "failure")

    def test_an_error_after_exit_0_fails(self):
        self.assertEqual(self.conclusion({"a": PASS + "Traceback\nexit 1\n"}, [AREAS]), "failure")

    def test_a_red_shard_with_every_result_passing_is_not_a_pass(self):
        self.assertEqual(v.judge({"a": PASS}, [AREAS], incomplete="a measure shard ended failure")[0], "neutral")

    def test_a_red_shard_with_results_missing_says_nothing(self):
        self.assertEqual(v.judge({"a": None}, [], incomplete="a measure shard ended failure")[0], "neutral")

    def test_a_result_that_fails_in_a_red_shard_run_still_fails(self):
        failed = "bodies across saves: FAIL\nexit 1\n"
        self.assertEqual(v.judge({"a": failed, "b": None}, [], incomplete="x")[0], "failure")
        self.assertEqual(v.judge({"a": PASS}, ["areas across saves: FAILED: 2025\nexit 1\n"], incomplete="x")[0], "failure")

    def test_a_failed_areas_shard_beside_a_skipped_one_fails(self):
        self.assertEqual(self.conclusion({"a": PASS}, [SKIP_AREAS, "areas across saves: FAILED: 2025\nexit 1\n"]), "failure")

    def test_a_failure_outranks_a_skip(self):
        self.assertEqual(self.conclusion({"a": None, "b": "bodies across saves: SKIPPED (x)\nexit 0\n"}, [AREAS]), "failure")

    def test_all_passing_passes(self):
        self.assertEqual(self.conclusion({"a": PASS, "b": PASS}, [SKIP_AREAS, AREAS]), "success")


class Worst(unittest.TestCase):
    def test_a_failing_shard_outranks_a_passing_one(self):
        failed = "bodies across saves: FAIL\nexit 1\n"
        self.assertEqual(v.worst([PASS, failed]), failed)
        self.assertEqual(v.worst([failed, PASS]), failed)

    def test_a_skip_outranks_a_pass(self):
        skipped = "bodies across saves: SKIPPED (x)\nexit 0\n"
        self.assertEqual(v.worst([PASS, skipped]), skipped)

    def test_no_result_stays_no_result(self):
        self.assertIsNone(v.worst([]))


class ExpectedSamples(unittest.TestCase):
    def test_one_year_means_from_it_on(self):
        self.assertEqual(len(v.expected_samples("2027")), 5)
        self.assertEqual(len(v.expected_samples("2026")), 10)

    def test_a_narrow_range_holds_only_its_years(self):
        self.assertEqual(len(v.expected_samples("2026 2026")), 5)
        self.assertEqual(v.expected_samples("2008 2023"), [])

    def test_no_range_holds_all_ten(self):
        self.assertEqual(len(v.expected_samples("")), 10)

    def test_the_check_script_expects_the_samples_the_verdict_does(self):
        script = pathlib.Path(__file__).with_name("check-bodies-across-saves.sh").read_text()
        match = re.search(r"for project in ([a-z ]+); do", script)
        self.assertIsNotNone(match)
        self.assertEqual(set(match.group(1).split()), set(v.PROJECTS))


if __name__ == "__main__":
    unittest.main()
