#!/usr/bin/env python3
"""Failure-first tests of the pure judgement in measured_numbers.py.

Each case is a way the check could pass a number it must refuse (a count read
as an identifier, a model read out of a link or a longer name, a flag that
covers the wrong model, a marker that excuses nothing) or refuse one it must
pass (a year, a run id, a thousands comma split in two). The check on the real
pages, and its refusals on copies of them, are `measured_numbers.py` and
`measured_numbers.py --refusals`."""
import unittest

import measured_numbers as m

INDEX = """# Reference models

## Model index

| Model | Names the pages use | Who can fetch it | How |
|---|---|---|---|
| magnetar Core Interior | Core Interior | redistributable | fetch script |
| the Snowdon samples | Snowdon Towers samples | fetched by Measure | -f samples=true |
| Snowdon's exports | Snowdon | local only | no export for a run |
| Ribeiro projects | Projeto1, `Exemplo_data` | local only | no download |
| client files | client models | not shared | their reporter's |

## The files
"""


def models():
    found, problems = m.read_index(INDEX)
    assert problems == [], problems
    return found


def page(text):
    found, _ = m.check_page("p.md", text, models())
    return [(line, message) for _, line, message in found]


class Numbers(unittest.TestCase):
    def test_what_spells_something_else_is_not_a_number(self):
        for text in (
            "RE-345 and RE-328.2 and RE-12-3",
            "see #1192 and (#1075)",
            "RE-327 §9 and §11, section §8.2",
            "v0.15.0, 0.12.0 and v2",
            "on 2026-11-05",
            "Measure run 42091457764 at 830a4d94",
            "Revit 2024, saves of 2015 to 2020, and 2008's envelope",
            "the 32-bit layout and 12-byte records",
            "Rust 1.87, MSRV 1.88, Apache License 2.0, glTF 2.0, Revit 26.2",
            "outside the 1.0 range",
            "`--phase 86961`, `ALD26 Before`, `tools/re/areas_across_saves.py` and `m_area 1e-6`",
            "[the files](docs/12-files.md) and https://example.org/4321",
            "IFC4, RE1, W7, 2D, 3D, the 2nd and B55",
        ):
            self.assertEqual(m.numbers(text), [], text)

    def test_counts_ratios_and_tolerances_are_numbers(self):
        self.assertEqual(m.numbers("1,030 of Snowdon's 1,078 walls"), ["1,030", "1,078"])
        self.assertEqual(m.numbers("within 0.001 ft, 0.45% under it"), ["0.001", "0.45"])
        self.assertEqual(m.numbers("100.4% to 104.7%"), ["100.4", "104.7"])
        self.assertEqual(m.numbers("to 1e-6 and 5e-7 ft"), ["1e-6", "5e-7"])
        self.assertEqual(m.numbers("1,234,567 records"), ["1,234,567"])
        self.assertEqual(m.numbers("[12 files](docs/12-files.md)"), ["12"])
        self.assertEqual(m.numbers("10GB, 0.01ft and 403MiB"), ["10", "0.01", "403"])

    def test_a_code_span_that_is_only_a_number_is_still_a_number(self):
        self.assertEqual(m.numbers("`4,321 walls` and `0.001 ft` and `4 of 5`"), ["4,321", "0.001", "4", "5"])
        self.assertEqual(m.numbers("`4,321 walls.rs` and `x 5`"), [])

    def test_a_count_that_looks_like_a_year_or_a_hash_is_still_a_count(self):
        self.assertEqual(m.numbers("2,024 walls"), ["2,024"])
        self.assertEqual(m.numbers("1030 walls"), ["1030"])
        self.assertEqual(m.numbers("1,234,567,890 bytes"), ["1,234,567,890"])
        self.assertEqual(m.numbers("34 of 34 models"), ["34", "34"])
        self.assertEqual(m.numbers("a hash of 99999999 digits"), [])
        self.assertEqual(m.numbers("deadbeef is a hash and 1234567 is a count"), ["1234567"])


class Segments(unittest.TestCase):
    def test_a_semicolon_or_a_sentence_ends_a_clause_and_a_decimal_does_not(self):
        parts = [c for c, _ in m.clauses("A 1 of 2; B 3 of 4. C 5.1 ft, 0.45% to 0.68% under it, e.g. Core Interior 6 of 7")]
        self.assertEqual(parts, ["A 1 of 2", "B 3 of 4", "C 5.1 ft, 0.45% to 0.68% under it, e.g. Core Interior 6 of 7"])

    def test_a_marker_may_hold_a_semicolon_and_a_full_stop_and_stays_in_its_clause(self):
        got = m.clauses("It has 5 files<!-- w10: a default; set. Not measured --> and more; next 6")
        self.assertEqual([c for c, _ in got], ["It has 5 files and more", "next 6"])
        self.assertEqual(got[0][1], ["a default; set. Not measured"])
        self.assertEqual(got[1][1], [])

    def test_nothing_inside_parentheses_ends_a_clause(self):
        got = [c for c, _ in m.clauses("a (1 of 2; 3 of 4. 5) b; c (d; e")]
        self.assertEqual(got, ["a (1 of 2; 3 of 4. 5) b", "c (d; e"])
        self.assertEqual([c for c, _ in m.clauses("a ) b; c")], ["a ) b", "c"])

    def test_other_comments_are_not_text(self):
        self.assertEqual(m.clauses("a <!-- 5 files --> b"), [("a   b", [])])

    def test_units_are_cells_paragraphs_and_items_with_their_first_line(self):
        text = "\n".join(
            [
                "# Heading 1",
                "| a | b |",
                "|---|:--|",
                "| 1 of 2 | 3 of 4 |",
                "",
                "```bash",
                "echo 5",
                "```",
                "<!-- a comment -->",
                "wrapped line one",
                "#1016) wrapped line two",
                "- item 7",
                "  continued 8",
                "2. numbered 9",
            ]
        )
        self.assertEqual(
            m.units(text),
            [
                (1, "Heading 1"),
                (2, "a"),
                (2, "b"),
                (4, "1 of 2"),
                (4, "3 of 4"),
                (10, "wrapped line one #1016) wrapped line two"),
                (12, "item 7 continued 8"),
                (14, "numbered 9"),
            ],
        )


class Names(unittest.TestCase):
    def names(self, clause):
        return sorted(models()[i]["model"] for i in m.named(clause, models()))

    def test_a_name_is_a_whole_word(self):
        self.assertEqual(self.names("Snowdon's walls"), ["Snowdon's exports"])
        self.assertEqual(self.names("Snowdonia and Snowdon_2 and xSnowdon"), [])
        self.assertEqual(self.names("`Exemplo_data` and Projeto1"), ["Ribeiro projects"])
        self.assertEqual(self.names("Core Interior"), ["magnetar Core Interior"])
        self.assertEqual(self.names("Core Interiors"), [])

    def test_a_longer_name_beats_the_name_inside_it(self):
        self.assertEqual(self.names("the Snowdon Towers samples"), ["the Snowdon samples"])
        self.assertEqual(self.names("Snowdon Towers samples and Snowdon"), ["Snowdon's exports", "the Snowdon samples"])

    def test_a_name_in_a_link_target_or_url_is_not_named(self):
        self.assertEqual(self.names("[the page](Snowdon.md) and <https://x.org/Snowdon>"), [])


class Index(unittest.TestCase):
    def problems(self, text):
        return m.read_index(text)[1]

    def test_the_valid_index_has_no_problem_and_every_model(self):
        self.assertEqual([x["model"] for x in models()], ["magnetar Core Interior", "the Snowdon samples", "Snowdon's exports", "Ribeiro projects", "client files"])

    def test_a_missing_section_or_columns_is_named(self):
        self.assertEqual(self.problems("no index"), [f'{m.INDEX} has no "## Model index" section'])
        self.assertIn("columns are not", self.problems("## Model index\n\n| a | b |\n|---|---|\n")[0])

    def test_a_fetchability_the_vocabulary_lacks_is_named(self):
        bad = INDEX.replace("| local only | no export", "| available | no export")
        self.assertIn("'available'", self.problems(bad)[0])

    def test_a_row_without_names_or_how_or_cells_is_named(self):
        self.assertIn("names no name", self.problems(INDEX.replace("| Core Interior |", "|  |"))[0])
        self.assertIn("does not say how", self.problems(INDEX.replace("| fetch script |", "|  |"))[0])
        self.assertIn("has 3 cells", self.problems(INDEX.replace("| fetch script |", "|"))[0])

    def test_one_name_for_two_models_is_named(self):
        self.assertIn("'Snowdon'", self.problems(INDEX.replace("Projeto1,", "Snowdon,"))[0])


class Pages(unittest.TestCase):
    def test_a_number_with_no_model_is_refused_on_its_line(self):
        got = page("intro\n\nwalls: 4 of 5 exact\n")
        self.assertEqual(len(got), 1)
        self.assertEqual(got[0][0], 3)
        self.assertIn("names no model", got[0][1])

    def test_a_number_with_a_fetchable_model_passes_and_a_number_free_clause_needs_none(self):
        self.assertEqual(page("4 of 5 walls on Core Interior\n\nno model here and no digit\n"), [])
        self.assertEqual(page("Snowdon and Projeto1 are named here, with no numeral.\n"), [])

    def test_a_continuation_after_a_semicolon_must_name_its_model_again(self):
        got = page("4 of 5 walls on Core Interior; 6 of 7 of them exact\n")
        self.assertEqual(len(got), 1)
        self.assertIn("6, 7", got[0][1])

    def test_a_local_only_model_must_say_so_and_a_flag_covers_only_that_model(self):
        self.assertIn("stands beside its name", page("4 of 5 walls on Snowdon\n")[0][1])
        self.assertEqual(page("4 of 5 walls on Snowdon (local only)\n"), [])
        self.assertEqual(page("Snowdon is local only; 4 of 5 walls on Snowdon\n"), [])
        self.assertEqual(len(page("4 of 5 on Snowdon (local only); 6 of 7 on Projeto1\n")), 1)
        self.assertIn("not shared", page("8 of the client models\n")[0][1])
        self.assertEqual(page("8 of the client models (not shared)\n"), [])

    def test_the_local_only_word_is_said_of_the_name_it_stands_beside(self):
        self.assertEqual(len(page("Snowdon 7 of 8 (Projeto1 is local only)\n")), 1)
        self.assertEqual(page("Snowdon (local only) 7 of 8 and Projeto1 (local only)\n"), [])
        self.assertEqual(page("Snowdon and Projeto1 (both local only) 7 of 8\n"), [])
        self.assertEqual(page("Core Interior, Snowdon and Projeto1 (all local only) 7 of 8\n"), [])
        self.assertEqual(len(page("Core Interior and Snowdon (local only) and Projeto1 7 of 8\n")), 1)
        far = "Snowdon " + "x " * 60 + "(local only) 7 of 8"
        self.assertEqual(len(page(far + "\n")), 1)

    def test_a_flag_does_not_leave_its_cell_paragraph_or_item(self):
        text = "| 4 of 5 on Snowdon (local only) | 6 of 7 on Snowdon |\n\n- 8 of 9 on Snowdon\n"
        self.assertEqual([line for line, _ in page(text)], [1, 3])

    def test_the_wrong_word_does_not_flag(self):
        self.assertEqual(len(page("4 of 5 on Snowdon (not shared)\n")), 1)
        self.assertEqual(len(page("8 of the client models (local only)\n")), 1)

    def test_a_marker_with_a_reason_excuses_a_clause_with_no_model(self):
        self.assertEqual(page("12 bindings<!-- w10: a count another check pins --> in all\n"), [])

    def test_a_marker_without_a_reason_or_with_a_one_word_reason_is_refused(self):
        for reason in ("", "x", "short", "12345678901234"):
            got = page(f"12 bindings<!-- w10: {reason} -->\n")
            self.assertTrue(any("without a reason" in message for _, message in got), reason)

    def test_a_marker_with_nothing_to_excuse_is_refused(self):
        self.assertIn("no number", page("bindings<!-- w10: nothing numeric here at all -->\n")[0][1])
        self.assertIn("beside its model", page("4 of 5 on Core Interior<!-- w10: it names its model already -->\n")[0][1])

    def test_a_number_far_from_every_model_name_is_refused_and_one_beside_it_is_not(self):
        filler = "the wall is exact, " * 25
        self.assertEqual(page(f"On Core Interior, {filler}4 of 5.\n")[0][0], 1)
        self.assertIn("more than 300 characters", page(f"On Core Interior, {filler}4 of 5.\n")[0][1])
        self.assertIn("more than 300 characters", page(f"4 of 5, {filler}on Core Interior.\n")[0][1])
        self.assertEqual(page(f"{filler}on Core Interior, 4 of 5.\n"), [])
        self.assertEqual(page(f"On Core Interior, {filler[:200]}4 of 5.\n"), [])

    def test_a_marker_excuses_a_number_far_from_every_model_and_not_one_beside_one(self):
        filler = "the wall is exact, " * 25
        self.assertEqual(page(f"On Core Interior, {filler}12 bindings<!-- w10: a count another check pins -->.\n"), [])
        self.assertEqual(len(page(f"On Core Interior, 4 of 5 {filler}12 bindings<!-- w10: a count another check pins -->.\n")), 0)
        self.assertIn("beside its model", page("On Core Interior 4 of 5<!-- w10: it is beside its model -->.\n")[0][1])

    def test_a_refusal_says_how_to_mark_a_number_that_is_no_measurement(self):
        self.assertIn("<!-- w10: reason -->", page("macOS 14 and 4 of 5\n")[0][1])

    def test_a_heading_is_read(self):
        got = page("## 9,999 walls\n\ntext\n")
        self.assertEqual([line for line, _ in got], [1])
        self.assertEqual(page("## Core Interior 9 walls\n"), [])

    def test_a_parenthesis_joins_the_clause_so_a_semicolon_in_it_cannot_hide_a_number(self):
        self.assertEqual(page("1 of 2 on Core Interior (all its windows; 48 of 269 doors)\n"), [])
        self.assertEqual(len(page("1 of 2 on Core Interior; 48 of 269 doors\n")), 1)

    def test_a_marker_excuses_only_its_own_clause(self):
        got = page("12 bindings<!-- w10: a count another check pins -->; 13 others\n")
        self.assertEqual(len(got), 1)
        self.assertIn("13", got[0][1])

    def test_numbers_in_code_and_comments_and_fences_are_not_read(self):
        self.assertEqual(page("`probe_5 4`\n\n```\n4 of 5\n```\n\n<!-- 4 of 5 -->\n"), [])

    def test_a_measured_count_in_backticks_does_not_hide(self):
        self.assertEqual(len(page("walls: `4 of 5`\n")), 1)


if __name__ == "__main__":
    unittest.main()
