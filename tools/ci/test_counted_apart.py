#!/usr/bin/env python3
"""Failure-first tests of tools/re/counted_apart.py's pure judgement: each way the registry, the scorers, the holdout
scrub and docs/counted-apart.md can come apart. The reading of a Measure's real results is checked on a real run, not here."""
import pathlib
import re
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "re"))
import counted_apart as c  # noqa: E402


def one(id="x", scorer="qtos", markers=("counted apart (RE-1): ",), pattern=r"^things counted apart \(RE-1\): (?P<n>\d+)$",
        example="things counted apart (RE-1): 3", holdout=False):
    return c.category(id, scorer, "t", "w", "d", list(markers), pattern, example, holdout)


class Scorers(unittest.TestCase):
    def test_a_new_line_that_sets_a_value_aside_and_no_category_names_fails(self):
        source = 'print(f"things counted apart (RE-1): {n}")\nprint("odd values left out: 2")\n'
        problems = c.check_scorers([one(markers=["counted apart (RE-1): "])], {"qtos": source})
        self.assertEqual(len(problems), 1)
        self.assertIn("odd values left out", problems[0])

    def test_a_marker_its_scorer_no_longer_holds_fails(self):
        problems = c.check_scorers([one()], {"qtos": 'print("something else")\n'})
        self.assertEqual(len(problems), 1)
        self.assertIn("is in no string", problems[0])

    def test_an_exact_marker_names_a_whole_literal_only(self):
        reg = [one(markers=[("exact", " counted apart")])]
        self.assertEqual(c.check_scorers(reg, {"qtos": 'x = " counted apart"\n'}), [])
        self.assertEqual(len(c.check_scorers(reg, {"qtos": 'x = " counted apart"\ny = "also counted apart here"\n'})), 1)

    def test_a_docstring_may_say_counted_apart(self):
        self.assertEqual(c.check_scorers([], {"qtos": '"""Values counted apart."""\nx = 1\n'}), [])

    def test_a_category_is_checked_against_its_own_scorer(self):
        reg = [one(scorer="psets")]
        problems = c.check_scorers(reg, {"qtos": 'print("things counted apart (RE-1): 1")\n', "psets": "x = 1\n"})
        self.assertEqual(len(problems), 2)


class Registry(unittest.TestCase):
    def test_an_example_its_pattern_does_not_read_fails(self):
        self.assertEqual(len(c.check_registry([one(example="things counted apart (RE-1) 3")])), 1)

    def test_two_categories_with_one_id_fail(self):
        self.assertEqual(len(c.check_registry([one(), one()])), 1)


class Lines(unittest.TestCase):
    def test_counts_are_summed_over_the_lines_a_pattern_reads(self):
        cat = one(pattern=r"^left out: \S+ (?P<n>\d+)$", example="left out: a.b 4")
        self.assertEqual(c.parse_lines(["left out: a.b 4", "left out: c.d 5", "other"], cat), 9)

    def test_no_line_read_is_none_not_zero(self):
        self.assertIsNone(c.parse_lines(["something else"], one()))

    def test_a_reworded_line_is_not_read(self):
        self.assertIsNone(c.parse_lines(["things set aside (RE-1): 3"], one()))


class Scrub(unittest.TestCase):
    def test_a_category_the_scrub_would_drop_fails(self):
        problems = c.check_scrub([one(holdout=True)], "grep -E 'nothing about it'")
        self.assertEqual(len(problems), 1)

    def test_a_category_the_scrub_keeps_passes_whatever_the_escaping(self):
        self.assertEqual(c.check_scrub([one(holdout=True)], r"grep -E 'counted apart \(RE-1\): [0-9]+'"), [])

    def test_a_category_the_held_out_models_do_not_keep_is_not_asked_of_the_scrub(self):
        self.assertEqual(c.check_scrub([one(holdout=False)], "nothing"), [])


class Page(unittest.TestCase):
    def page(self, static, ids):
        rows = "\n".join(f"| `{i}` | 1 |" for i in ids)
        return f"{c.STATIC[0]}\n{static}\n{c.STATIC[1]}\n{c.COUNTS[0]}\n| Category | A |\n|---|---:|\n{rows}\n{c.COUNTS[1]}\n"

    def test_a_page_of_the_registry_passes(self):
        reg = [one("a"), one("b")]
        self.assertEqual(c.page_problems(self.page(c.render_static(reg), ["a", "b"]), reg), [])

    def test_a_category_block_that_is_not_the_registrys_fails(self):
        reg = [one("a")]
        self.assertEqual(len(c.page_problems(self.page("| stale |", ["a"]), reg)), 1)

    def test_a_counts_table_missing_a_category_fails(self):
        reg = [one("a"), one("b")]
        self.assertEqual(len(c.page_problems(self.page(c.render_static(reg), ["a"]), reg)), 1)

    def test_a_page_without_its_blocks_fails_twice(self):
        self.assertEqual(len(c.page_problems("# nothing\n", [one()])), 2)


if __name__ == "__main__":
    unittest.main()
