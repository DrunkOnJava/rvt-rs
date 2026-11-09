#!/usr/bin/env python3
"""Failure-first tests of check_sealed_holdout.py's pure judgement: every way a sealed pair could be read before the final 1.0 check."""
import unittest

import check_sealed_holdout as c

RVT_SHA = "a" * 64
IFC_SHA = "b" * 64
RVT_URL = "https://example.org/pairs/house-2026.rvt"
IFC_URL = "https://example.org/pairs/house-2026.ifc"
HEADER = "# sealed pairs\n"


def row(**changes):
    fields = dict(
        release="2026", name="house-2026", rvt_url=RVT_URL, rvt_sha256=RVT_SHA, rvt_bytes="1234",
        ifc_url=IFC_URL, ifc_sha256=IFC_SHA, ifc_bytes="567", licence="CC-BY-4.0",
        contributor="someone", received="2026-11-09",
    )
    fields.update(changes)
    return "\t".join(fields[name] for name in c.FIELDS)


def judge(text, files=None):
    return c.judge(text, files or {})


class Rows(unittest.TestCase):
    def test_a_good_row_passes(self):
        self.assertEqual(judge(HEADER + row() + "\n"), [])

    def test_an_empty_registry_passes(self):
        self.assertEqual(judge(HEADER), [])

    def test_a_missing_registry_fails(self):
        self.assertTrue(judge(None))

    def test_a_short_row_fails(self):
        self.assertTrue(judge(HEADER + row().rsplit("\t", 1)[0] + "\n"))

    def test_a_release_outside_1_0_fails(self):
        for release in ("2023", "2028", "26"):
            self.assertTrue(judge(row(release=release)), release)

    def test_a_bad_hash_fails(self):
        for sha in ("A" * 64, "a" * 63, "g" * 64):
            self.assertTrue(judge(row(rvt_sha256=sha)), sha)

    def test_a_plain_http_url_fails(self):
        self.assertTrue(judge(row(ifc_url="http://example.org/x.ifc")))

    def test_a_bad_byte_count_fails(self):
        for size in ("0", "-5", "12kB", ""):
            self.assertTrue(judge(row(rvt_bytes=size)), size)

    def test_another_licence_fails(self):
        for licence in ("GPL-3.0", "CC BY 4.0", "none"):
            self.assertTrue(judge(row(licence=licence)), licence)

    def test_a_bad_date_or_login_fails(self):
        self.assertTrue(judge(row(received="09/11/2026")))
        self.assertTrue(judge(row(contributor="some one")))

    def test_a_name_listed_twice_fails(self):
        other = row(rvt_url=RVT_URL + "2", ifc_url=IFC_URL + "2", rvt_sha256="c" * 64, ifc_sha256="d" * 64)
        self.assertTrue(judge(row() + "\n" + other + "\n"))

    def test_a_file_listed_twice_fails(self):
        other = row(name="house-b", rvt_url=RVT_URL + "2", ifc_url=IFC_URL + "2", ifc_sha256="d" * 64)
        self.assertTrue(judge(row() + "\n" + other + "\n"))


class Files(unittest.TestCase):
    REG = HEADER + row() + "\n"

    def test_a_workflow_naming_the_registry_fails(self):
        files = {".github/workflows/measure.yml": "run: tools/ci/fetch.sh research/sealed-holdout-models.tsv\n"}
        self.assertTrue(judge(self.REG, files))

    def test_a_script_or_probe_naming_it_fails(self):
        for path in ("tools/ci/fetch-holdout-models.sh", "tools/re/x.py", "probes/examples/p.rs", "tests/x.rs"):
            self.assertTrue(judge(self.REG, {path: "sealed-holdout-models.tsv"}), path)

    def test_a_sealed_url_anywhere_fails(self):
        for path in ("research/holdout-models.tsv", "docs/reference-models.md", ".github/workflows/measure.yml"):
            self.assertTrue(judge(self.REG, {path: f"x\t{IFC_URL}\n"}), path)

    def test_a_sealed_hash_in_any_case_fails(self):
        self.assertTrue(judge(self.REG, {"research/holdout-models.tsv": RVT_SHA.upper()}))

    def test_the_seal_script_and_docs_may_name_it(self):
        files = {
            "tools/ci/seal-holdout-pair.sh": "REGISTRY=research/sealed-holdout-models.tsv\n",
            "docs/maintaining.md": "`research/sealed-holdout-models.tsv` lists them\n",
            "research/sealed-holdout-models.tsv": self.REG,
        }
        self.assertEqual(judge(self.REG, files), [])

    def test_a_doc_may_not_hold_a_sealed_url(self):
        self.assertTrue(judge(self.REG, {"docs/maintaining.md": f"see {RVT_URL}"}))


if __name__ == "__main__":
    unittest.main()
