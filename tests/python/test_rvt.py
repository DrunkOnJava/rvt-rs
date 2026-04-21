"""pytest integration tests for the Python bindings.

Assumes the wheel has been installed into the environment (via
`maturin develop --features python` or `pip install .../rvt-*.whl`)
and the sample corpus is reachable. The corpus path resolution
mirrors the Rust integration-test `tests/common/mod.rs` behaviour:

- if `RVT_SAMPLES_DIR` is set in the environment, use that
- otherwise fall back to `../../samples/` relative to this file's
  parent directory (the crate root).

Tests that would require the corpus skip gracefully when it's
missing, so running `pytest` on a fresh clone without the corpus
still exits 0.
"""
from __future__ import annotations

import os
import pathlib
import pytest

import rvt  # type: ignore


SAMPLE_YEARS = [2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026]


def samples_dir() -> pathlib.Path:
    env = os.environ.get("RVT_SAMPLES_DIR")
    if env:
        return pathlib.Path(env)
    # This file is at <crate>/tests/python/test_rvt.py; ../../samples
    # relative to <crate> ends up two levels up.
    here = pathlib.Path(__file__).resolve()
    return here.parent.parent.parent.parent / "samples"


def sample_for_year(year: int) -> pathlib.Path:
    if 2016 <= year <= 2019:
        return samples_dir() / f"rac_basic_sample_family-{year}.rfa"
    if 2020 <= year <= 2026:
        return samples_dir() / f"racbasicsamplefamily-{year}.rfa"
    raise ValueError(f"unknown sample year {year}")


def corpus_available() -> bool:
    return all(sample_for_year(y).exists() for y in SAMPLE_YEARS)


# ---------------------------------------------------------------------
# Module-level
# ---------------------------------------------------------------------

def test_module_version_is_a_nonempty_string():
    assert isinstance(rvt.__version__, str)
    assert rvt.__version__ != ""


def test_module_exposes_RevitFile():
    assert hasattr(rvt, "RevitFile")


def test_module_exposes_rvt_to_ifc():
    assert hasattr(rvt, "rvt_to_ifc")
    assert callable(rvt.rvt_to_ifc)


# ---------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------

def test_nonexistent_file_raises_ioerror(tmp_path):
    missing = tmp_path / "does-not-exist.rfa"
    with pytest.raises((OSError, IOError)):
        rvt.RevitFile(str(missing))


def test_non_cfb_input_raises(tmp_path):
    not_cfb = tmp_path / "hello.rfa"
    not_cfb.write_bytes(b"this is not a CFB file")
    with pytest.raises((OSError, IOError, ValueError)):
        rvt.RevitFile(str(not_cfb))


# ---------------------------------------------------------------------
# Happy path on the 2024 sample (the reference bandwidth for our
# walker and IFC exporter).
# ---------------------------------------------------------------------

@pytest.fixture
def sample_2024():
    p = sample_for_year(2024)
    if not p.exists():
        pytest.skip(f"corpus not available at {p}")
    return rvt.RevitFile(str(p))


def test_version_reports_2024(sample_2024):
    assert sample_2024.version == 2024


def test_part_atom_title_present(sample_2024):
    assert sample_2024.part_atom_title is not None
    # The 2024 sample's PartAtom title is deterministic for this
    # fixture family.
    assert sample_2024.part_atom_title == "0610 x 0915mm"


def test_stream_names_returns_expected_set(sample_2024):
    names = sample_2024.stream_names()
    assert isinstance(names, list)
    assert len(names) == 13
    for required in [
        "BasicFileInfo",
        "Formats/Latest",
        "Global/ElemTable",
        "Global/Latest",
        "PartAtom",
    ]:
        assert required in names, f"missing required stream {required}"


def test_read_stream_returns_bytes_for_known_stream(sample_2024):
    """`read_stream` should return ``bytes`` for a stream that exists
    in the file. Content is intentionally not validated here — we
    just want proof the binding round-trips raw bytes from CFB.
    """
    data = sample_2024.read_stream("BasicFileInfo")
    assert isinstance(data, bytes)
    assert len(data) > 0, "BasicFileInfo stream should be non-empty"


def test_read_stream_accepts_leading_slash(sample_2024):
    """Both ``/Formats/Latest`` and ``Formats/Latest`` must resolve
    the same way — the reader normalises the path internally.
    """
    without_slash = sample_2024.read_stream("Formats/Latest")
    with_slash = sample_2024.read_stream("/Formats/Latest")
    assert without_slash == with_slash


def test_read_stream_raises_on_missing_name(sample_2024):
    """Unknown stream names should raise, not silently return empty
    bytes. `OSError` in Python catches pyo3's PyIOError.
    """
    with pytest.raises((OSError, IOError)):
        sample_2024.read_stream("does/not/exist")


def test_missing_required_streams_empty_on_valid_file(sample_2024):
    assert sample_2024.missing_required_streams() == []


def test_schema_summary_has_expected_counts(sample_2024):
    summary = sample_2024.schema_summary()
    assert summary["classes"] > 100
    assert summary["fields"] > 1000
    assert summary["cpp_types"] >= 0


def test_schema_json_parses_and_matches_summary(sample_2024):
    """The JSON output of schema_json() should parse into a dict whose
    counts match schema_summary(), confirming the full-schema export
    is consistent with the cheap counts-only query.
    """
    import json

    summary = sample_2024.schema_summary()
    js = sample_2024.schema_json()
    assert isinstance(js, str)
    assert len(js) > 10_000, "full schema JSON should be substantial"
    parsed = json.loads(js)
    assert isinstance(parsed, dict)
    assert "classes" in parsed
    assert "cpp_types" in parsed
    assert len(parsed["classes"]) == summary["classes"]
    assert len(parsed["cpp_types"]) == summary["cpp_types"]
    total_fields = sum(len(c["fields"]) for c in parsed["classes"])
    assert total_fields == summary["fields"]


def test_basic_file_info_json_matches_getters(sample_2024):
    """`basic_file_info_json` should round-trip to a dict whose values
    match the individual getters, confirming the JSON surface doesn't
    silently drift from the per-field accessors.
    """
    import json

    js = sample_2024.basic_file_info_json()
    assert js is not None, "2024 sample should parse BasicFileInfo"
    parsed = json.loads(js)
    assert isinstance(parsed, dict)
    assert parsed["version"] == sample_2024.version
    # original_path / build / guid are optional; if the getter returns
    # a value, the JSON should carry the same value under the same key.
    if sample_2024.original_path is not None:
        assert parsed["original_path"] == sample_2024.original_path
    if sample_2024.build is not None:
        assert parsed["build"] == sample_2024.build
    if sample_2024.guid is not None:
        assert parsed["guid"] == sample_2024.guid


def test_part_atom_json_matches_title_getter(sample_2024):
    """`part_atom_json` should round-trip to a dict whose ``title``
    field matches the ``part_atom_title`` getter.
    """
    import json

    js = sample_2024.part_atom_json()
    if js is None:
        # Some samples may not carry a PartAtom — skip rather than fail.
        pytest.skip("PartAtom stream absent in this sample")
    parsed = json.loads(js)
    assert isinstance(parsed, dict)
    assert parsed.get("title") == sample_2024.part_atom_title
    # Structural sanity: PartAtom always carries these keys even when
    # empty. (raw_xml is the lossless pass-through.)
    for key in ("taxonomies", "categories", "raw_xml"):
        assert key in parsed, f"PartAtom JSON missing key {key!r}"


def test_schema_json_contains_adocument_class(sample_2024):
    """The schema always contains a class named `ADocument` — it's
    the root document class and what the Layer 5a walker targets.
    """
    import json

    parsed = json.loads(sample_2024.schema_json())
    names = [c["name"] for c in parsed["classes"]]
    assert "ADocument" in names


def test_read_adocument_returns_dict_with_13_fields(sample_2024):
    doc = sample_2024.read_adocument()
    assert doc is not None
    assert "entry_offset" in doc
    assert "version" in doc
    assert doc["version"] == 2024
    assert len(doc["fields"]) == 13


def test_read_adocument_last_three_elementids_match_rust(sample_2024):
    # Cross-validation: the Rust walker (via rvt-doc) reports these
    # exact ElementId values for the 2024 sample. Python bindings
    # must produce identical output — that's the whole point of the
    # bindings being a thin wrapper rather than a reimplementation.
    doc = sample_2024.read_adocument()
    expected = [("m_ownerFamilyId", 27), ("m_ownerFamilyContainingGroupId", 31), ("m_devBranchInfo", 35)]
    last_three = doc["fields"][-3:]
    for field, (exp_name, exp_id) in zip(last_three, expected):
        assert field["name"] == exp_name
        assert field["kind"] == "element_id"
        assert field["tag"] == 0
        assert field["id"] == exp_id


def test_write_ifc_produces_valid_ifc4(sample_2024):
    ifc = sample_2024.write_ifc()
    assert isinstance(ifc, str)
    assert ifc.startswith("ISO-10303-21;\n")
    assert ifc.endswith("END-ISO-10303-21;\n")
    assert "FILE_SCHEMA(('IFC4'));" in ifc
    assert "IFCPROJECT" in ifc
    # Sample's project name should appear in the output.
    assert "0610 x 0915mm" in ifc


def test_write_ifc_produces_exactly_one_ifcproject(sample_2024):
    ifc = sample_2024.write_ifc()
    # Count occurrences of the entity constructor, not the type
    # name (the comment / header may also contain the string).
    assert ifc.count("IFCPROJECT(") == 1


def test_repr_contains_version(sample_2024):
    r = repr(sample_2024)
    assert "RevitFile" in r
    assert "2024" in r


# ---------------------------------------------------------------------
# ElemTable bindings (2026-04-21 — family-file 12 B implicit layout)
# ---------------------------------------------------------------------

def test_elem_table_header_fields_present(sample_2024):
    h = sample_2024.elem_table_header()
    for key in ("element_count", "record_count", "header_flag", "decompressed_bytes"):
        assert key in h, f"missing key {key}"
        assert isinstance(h[key], int)
    # Family 2024 has header_flag = 0x0011; flag == 0 on project files.
    # For this fixture (family) we expect the magic to be present.
    assert h["header_flag"] == 0x0011
    # Declared counts should be positive.
    assert h["element_count"] > 0
    assert h["record_count"] > 0


def test_elem_table_records_are_well_formed(sample_2024):
    recs = sample_2024.elem_table_records()
    assert isinstance(recs, list)
    assert len(recs) > 0, "expected at least one record on the family sample"
    # Every entry must be a dict with the three documented keys.
    for r in recs:
        for key in ("offset", "id_primary", "id_secondary"):
            assert key in r, f"record missing key {key}: {r}"
            assert isinstance(r[key], int)
    # Offsets should be strictly increasing (records live sequentially).
    offsets = [r["offset"] for r in recs]
    assert all(a < b for a, b in zip(offsets, offsets[1:])), (
        "ElemTable records not in ascending offset order"
    )


def test_declared_element_ids_are_sorted_and_unique(sample_2024):
    ids = sample_2024.declared_element_ids()
    assert isinstance(ids, list)
    assert len(ids) > 0
    # Strictly ascending (sorted + deduped).
    assert all(a < b for a, b in zip(ids, ids[1:])), "ids not sorted"


def test_rvt_to_ifc_matches_write_ifc_method(sample_2024):
    # The free function `rvt_to_ifc(path)` should be equivalent
    # (up to the timestamp in FILE_NAME, which we strip before
    # comparing).
    import re
    p = sample_for_year(2024)
    via_fn = rvt.rvt_to_ifc(str(p))
    via_method = sample_2024.write_ifc()
    norm = lambda s: re.sub(r"FILE_NAME\([^)]*\);", "FILE_NAME(...);", s)
    assert norm(via_fn) == norm(via_method)


# ---------------------------------------------------------------------
# Cross-version sanity — the walker works on all 11 releases (the
# entry-point detector has different bands per release era per the
# Rust §Q6.5-F finding).
# ---------------------------------------------------------------------

@pytest.mark.parametrize("year", SAMPLE_YEARS)
def test_every_release_opens_and_reports_version(year):
    p = sample_for_year(year)
    if not p.exists():
        pytest.skip(f"corpus not available at {p}")
    f = rvt.RevitFile(str(p))
    assert f.version == year


@pytest.mark.parametrize("year", SAMPLE_YEARS)
def test_every_release_produces_valid_ifc(year):
    p = sample_for_year(year)
    if not p.exists():
        pytest.skip(f"corpus not available at {p}")
    ifc = rvt.rvt_to_ifc(str(p))
    assert ifc.startswith("ISO-10303-21;\n")
    assert "IFC4" in ifc
    assert ifc.count("IFCPROJECT(") == 1
