"""Always-on Python tests for the Lane Nine decoded-model API.

Uses redistributable `corpus/tier1/` fixtures (no Autodesk samples)
so Cloud / fresh clones stay green. Corpus-dependent coverage remains
in `test_rvt.py`.
"""
from __future__ import annotations

import json
import pathlib

import pytest

import rvt  # type: ignore

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
TIER1_ARCH = (
    REPO_ROOT
    / "corpus"
    / "tier1"
    / "architectural-2024"
    / "architectural-2024.rvt"
)
SCHEMAS = REPO_ROOT / "docs" / "schemas"


@pytest.fixture
def tier1_arch():
    if not TIER1_ARCH.exists():
        pytest.skip(f"tier1 fixture missing at {TIER1_ARCH}")
    return rvt.RevitFile(str(TIER1_ARCH))


def test_mvp_typed_classes_constant():
    assert hasattr(rvt, "MVP_TYPED_CLASSES")
    assert list(rvt.MVP_TYPED_CLASSES) == [
        "Level",
        "Wall",
        "Floor",
        "Door",
        "Window",
        "Room",
        "Material",
    ]


def test_decoded_elements_shape_and_typed_key(tier1_arch):
    elements = tier1_arch.decoded_elements()
    assert isinstance(elements, list)
    for element in elements:
        assert set(element) >= {"id", "class_name", "byte_range", "fields", "typed"}
        assert set(element["byte_range"]) == {"start", "end"}
        assert isinstance(element["fields"], list)
        if element["class_name"] in rvt.MVP_TYPED_CLASSES:
            assert isinstance(element["typed"], dict)
        else:
            assert element["typed"] is None


def test_element_counts_match_decoded_list(tier1_arch):
    elements = tier1_arch.decoded_elements()
    counts = tier1_arch.element_counts()
    assert counts["total"] == len(elements)
    by_class: dict[str, int] = {}
    for element in elements:
        by_class[element["class_name"]] = by_class.get(element["class_name"], 0) + 1
    assert counts["by_class"] == by_class


def test_export_diagnostics_and_write_ifc_modes(tier1_arch):
    diagnostics = tier1_arch.export_diagnostics()
    assert diagnostics["schema_version"] == 1
    assert "confidence" in diagnostics
    assert (
        tier1_arch.element_counts()["total"]
        == diagnostics["decoded"]["production_walker_elements"]
    )

    ifc = tier1_arch.write_ifc(mode="scaffold")
    assert ifc.startswith("ISO-10303-21;\n")
    assert "IFCPROJECT" in ifc

    with pytest.raises(ValueError):
        tier1_arch.write_ifc(mode="strict")

    with pytest.raises(ValueError, match="unknown IFC export mode|expected scaffold"):
        tier1_arch.write_ifc(mode="not-a-mode")


def test_schema_files_are_valid_json():
    for name in (
        "decoded-elements.schema.json",
        "element-counts.schema.json",
        "export-diagnostics.schema.json",
    ):
        path = SCHEMAS / name
        assert path.exists(), path
        parsed = json.loads(path.read_text())
        assert "$id" in parsed
        assert parsed["$schema"].endswith("2020-12/schema")


def test_optional_jsonschema_validation(tier1_arch):
    jsonschema = pytest.importorskip("jsonschema")
    elements = tier1_arch.decoded_elements()
    counts = tier1_arch.element_counts()
    decoded_schema = json.loads((SCHEMAS / "decoded-elements.schema.json").read_text())
    counts_schema = json.loads((SCHEMAS / "element-counts.schema.json").read_text())
    jsonschema.validate(instance=elements, schema=decoded_schema)
    jsonschema.validate(instance=counts, schema=counts_schema)
