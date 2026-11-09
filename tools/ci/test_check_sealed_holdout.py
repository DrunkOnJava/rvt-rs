#!/usr/bin/env python3
"""Failure-first tests of check_sealed_holdout.py's pure judgement: every way a sealed pair could be read before the final 1.0 check."""
import unittest

import check_sealed_holdout as c

RVT_SHA = "a" * 64
IFC_SHA = "b" * 64
RVT_URL = "https://github.com/Owner/Pairs/releases/download/v1/house-2026.rvt"
IFC_URL = "https://raw.githubusercontent.com/Owner/Pairs/" + "0123456789abcdef" * 2 + "01234567/House-2026.ifc"
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

    def test_a_row_with_a_twelfth_field_fails(self):
        self.assertTrue(judge(HEADER + row() + "\textra\n"))

    def test_a_name_outside_the_pattern_fails(self):
        for name in ("house 2026", "house/2026", "house;rm"):
            self.assertTrue(judge(row(name=name)), name)

    def test_a_release_outside_1_0_fails(self):
        for release in ("2023", "2028", "26"):
            self.assertTrue(judge(row(release=release)), release)

    def test_a_bad_hash_fails(self):
        for sha in ("A" * 64, "a" * 63, "g" * 64):
            self.assertTrue(judge(row(rvt_sha256=sha)), sha)

    def test_a_link_that_can_change_fails(self):
        for url in (
            "https://raw.githubusercontent.com/Owner/Pairs/main/House-2026.ifc",
            "https://github.com/Owner/Pairs/blob/main/House-2026.ifc",
            "https://drive.google.com/file/d/abc/view",
            "https://www.dropbox.com/s/abc/house.ifc?dl=1",
            "https://example.org/pairs/house-2026.ifc",
            # The newest release's asset, which moves with every release.
            "https://github.com/Owner/Pairs/releases/latest/download/house-2026.ifc",
            # A blob page at a commit serves HTML, not the file.
            "https://github.com/Owner/Pairs/blob/" + "0123456789abcdef" * 2 + "01234567/House-2026.ifc",
            # A short hash can come to name another commit.
            "https://raw.githubusercontent.com/Owner/Pairs/0123456/House-2026.ifc",
            # A Zenodo concept record or DOI resolves to the newest version.
            "https://zenodo.org/doi/10.5281/zenodo.1234567",
            "https://zenodo.org/records/1234567",
        ):
            self.assertTrue(judge(row(ifc_url=url)), url)

    def test_each_pinned_link_form_passes(self):
        commit = "fedcba9876543210" * 2 + "fedcba98"
        for url in (
            f"https://github.com/Owner/Pairs/raw/{commit}/house.rvt",
            f"https://media.githubusercontent.com/media/Owner/Pairs/{commit}/house.rvt",
            "https://zenodo.org/records/1234567/files/house.rvt?download=1",
        ):
            self.assertEqual(judge(row(rvt_url=url)), [], url)

    def test_a_plain_http_url_fails(self):
        self.assertTrue(judge(row(ifc_url="http://example.org/x.ifc")))

    def test_a_bad_byte_count_fails(self):
        for size in ("0", "-5", "12kB", ""):
            self.assertTrue(judge(row(rvt_bytes=size)), size)

    def test_each_accepted_licence_passes(self):
        for licence in ("Apache-2.0", "MIT", "CC-BY-4.0", "CC0-1.0"):
            self.assertEqual(judge(row(licence=licence)), [], licence)

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

    def test_a_mixed_case_url_in_another_case_fails(self):
        self.assertTrue(judge(self.REG, {"research/holdout-models.tsv": IFC_URL.lower()}))

    def test_a_data_or_text_file_naming_it_fails(self):
        for path in ("research/holdout-models.tsv", "docs/schemas/x.json", "tools/ci/test-targets.txt"):
            self.assertTrue(judge(self.REG, {path: "sealed-holdout-models.tsv"}), path)

    def test_a_sealed_hash_in_any_case_fails(self):
        self.assertTrue(judge(self.REG, {"research/holdout-models.tsv": RVT_SHA.upper()}))

    def test_the_seal_script_and_docs_may_name_it(self):
        files = {
            "tools/ci/seal-holdout-pair.sh": "REGISTRY=research/sealed-holdout-models.tsv\nBACKUP=DrunkOnJava/rvt-rs-sealed\n",
            "docs/maintaining.md": "`research/sealed-holdout-models.tsv` lists them, `DrunkOnJava/rvt-rs-sealed` keeps a copy\n",
            "research/sealed-holdout-models.tsv": self.REG,
        }
        self.assertEqual(judge(self.REG, files), [])

    def test_a_workflow_or_script_naming_the_copies_fails(self):
        for path, text in (
            (".github/workflows/measure.yml", "gh release download sealed -R DrunkOnJava/rvt-rs-sealed\n"),
            ("tools/ci/fetch-holdout-models.sh", "repo=drunkonjava/RVT-RS-SEALED\n"),
            ("tools/re/x.py", "https://github.com/DrunkOnJava/rvt-rs-sealed/releases/download/sealed/x\n"),
        ):
            self.assertTrue(judge(self.REG, {path: text}), path)

    def test_a_doc_may_not_hold_a_sealed_url(self):
        self.assertTrue(judge(self.REG, {"docs/maintaining.md": f"see {RVT_URL}"}))


OLE = bytes.fromhex("d0cf11e0a1b11ae1") + b"\0" * 24
STEP = (
    b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [ReferenceView]'),'2;1');\n"
    b"FILE_NAME('house.ifc','2026-11-09T10:00:00',(''),(''),'ODA SDAI 24.12','Autodesk Revit 2026 (ENU)','');\n"
    b"FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n"
)


class Pair(unittest.TestCase):
    def test_a_project_and_its_own_ifc4_export_pass(self):
        self.assertEqual(c.pair_problems(OLE, STEP, "2026"), [])

    def test_an_ifc4x3_export_passes(self):
        self.assertEqual(c.pair_problems(OLE, STEP.replace(b"'IFC4'", b"'IFC4X3_ADD2'"), "2026"), [])

    def test_a_share_page_or_zip_as_the_project_fails(self):
        for head in (b"<!DOCTYPE html><html>", b"PK\x03\x04" + b"\0" * 20, b""):
            self.assertTrue(c.pair_problems(head, STEP, "2026"), head[:8])

    def test_an_ifc_that_is_not_step_fails(self):
        for head in (b"<!DOCTYPE html>", b"PK\x03\x04", b"\xef\xbb\xbfISO-10303-21;"):
            self.assertTrue(c.pair_problems(OLE, head, "2026"), head[:8])

    def test_an_ifc2x3_view_fails(self):
        self.assertTrue(c.pair_problems(OLE, STEP.replace(b"'IFC4'", b"'IFC2X3'"), "2026"))

    def test_another_releases_export_fails(self):
        self.assertTrue(c.pair_problems(OLE, STEP, "2025"))
        self.assertTrue(c.pair_problems(OLE, STEP.replace(b"Autodesk Revit 2026", b"Autodesk Revit 2027"), "2026"))

    def test_an_export_that_names_no_revit_fails(self):
        self.assertTrue(c.pair_problems(OLE, STEP.replace(b"Autodesk Revit 2026 (ENU)", b"ArchiCAD 27"), "2026"))


if __name__ == "__main__":
    unittest.main()
