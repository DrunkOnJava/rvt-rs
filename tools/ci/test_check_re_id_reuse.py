#!/usr/bin/env python3
"""Failure-first tests of check_re_id_reuse.py's pure judgement: every way a new report could take an RE id that is already taken."""
import unittest

import check_re_id_reuse as c

P = "reports/element-framing/"


class Reused(unittest.TestCase):
    def test_an_id_main_has_at_another_path_fails(self):
        problems = c.reused([P + "RE-385-new.md"], [P + "RE-385-old.md", P + "RE-385-new.md"], {})
        self.assertEqual(len(problems), 1)
        self.assertIn("RE-385-old.md", problems[0])

    def test_two_new_reports_under_one_id_fail(self):
        tree = [P + "RE-9-a.md", P + "RE-9-b.md"]
        self.assertEqual(len(c.reused(tree, tree, {})), 1)

    def test_an_earlier_open_pull_request_with_the_id_fails_the_later_one(self):
        problems = c.reused([P + "RE-50-x.md"], [P + "RE-50-x.md"], {1600: [P + "RE-50-y.md"]})
        self.assertEqual(len(problems), 1)
        self.assertIn("#1600", problems[0])

    def test_a_free_id_passes(self):
        self.assertEqual(c.reused([P + "RE-397-x.md"], [P + "RE-396-p.md", P + "RE-397-x.md"], {1620: [P + "RE-385-z.md"]}), [])

    def test_the_pull_requests_own_file_in_the_merge_tree_passes(self):
        self.assertEqual(c.reused([P + "RE-12-x.md"], [P + "RE-12-x.md"], {}), [])

    def test_a_name_with_a_dot_after_the_id_is_that_id(self):
        problems = c.reused([P + "RE-12.md"], [P + "RE-12-other.md", P + "RE-12.md"], {})
        self.assertEqual(len(problems), 1)

    def test_a_longer_id_is_not_a_shorter_one(self):
        self.assertEqual(c.reused([P + "RE-3-x.md"], [P + "RE-30-y.md", P + "RE-3-x.md"], {}), [])

    def test_paths_that_are_not_reports_are_ignored(self):
        self.assertEqual(c.reused(["docs/RE-5-x.md", "reports/README.md"], ["docs/RE-5-x.md"], {}), [])

    def test_a_report_in_a_subdirectory_counts(self):
        tree = ["reports/validation/RE-77-x.md", P + "RE-77-y.md"]
        self.assertEqual(len(c.reused(["reports/validation/RE-77-x.md"], tree, {})), 1)

    def test_no_new_report_has_nothing_to_fail(self):
        self.assertEqual(c.reused([], [P + "RE-1-a.md"], {1: [P + "RE-1-b.md"]}), [])


if __name__ == "__main__":
    unittest.main()
