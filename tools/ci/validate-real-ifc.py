#!/usr/bin/env python3
"""Validate a generated real-project IFC with IfcOpenShell.

This script is intentionally stricter than a smoke parse. It cross-checks
IfcOpenShell's view of the STEP file against rvt-ifc's diagnostics so CI
failures name the IFC entity class that drifted.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import ifcopenshell


STEP_TO_IFCOPENSHELL = {
    "IFCBEAM": "IfcBeam",
    "IFCBUILDINGELEMENTPROXY": "IfcBuildingElementProxy",
    "IFCCOLUMN": "IfcColumn",
    "IFCCOVERING": "IfcCovering",
    "IFCCURTAINWALL": "IfcCurtainWall",
    "IFCDOOR": "IfcDoor",
    "IFCELECTRICAPPLIANCE": "IfcElectricAppliance",
    "IFCFLOWCONTROLLER": "IfcFlowController",
    "IFCFOOTING": "IfcFooting",
    "IFCFURNITURE": "IfcFurniture",
    "IFCGRID": "IfcGrid",
    "IFCLIGHTFIXTURE": "IfcLightFixture",
    "IFCMEMBER": "IfcMember",
    "IFCRAILING": "IfcRailing",
    "IFCRAMP": "IfcRamp",
    "IFCREINFORCINGBAR": "IfcReinforcingBar",
    "IFCROOF": "IfcRoof",
    "IFCSANITARYTERMINAL": "IfcSanitaryTerminal",
    "IFCSLAB": "IfcSlab",
    "IFCSPACE": "IfcSpace",
    "IFCSTAIR": "IfcStair",
    "IFCWALL": "IfcWall",
    "IFCWINDOW": "IfcWindow",
}


def fail(message: str) -> None:
    raise AssertionError(f"real-project IFC validation failed: {message}")


def count(model: object, ifc_class: str) -> int:
    return len(model.by_type(ifc_class))


def assert_exact(model: object, ifc_class: str, expected: int) -> None:
    got = count(model, ifc_class)
    if got != expected:
        fail(f"{ifc_class} count regressed: got {got}, expected {expected}")


def assert_at_least(model: object, ifc_class: str, minimum: int) -> None:
    got = count(model, ifc_class)
    if got < minimum:
        fail(f"{ifc_class} count regressed: got {got}, expected at least {minimum}")


def require_unsupported(diagnostics: dict, feature: str) -> None:
    unsupported = set(diagnostics.get("unsupported_features", []))
    if feature not in unsupported:
        fail(f"expected unsupported feature marker {feature!r}; got {sorted(unsupported)!r}")


def load_inputs(argv: list[str]) -> tuple[Path, dict]:
    if len(argv) != 3:
        print(
            "usage: validate-real-ifc.py <generated.ifc> <diagnostics.json>",
            file=sys.stderr,
        )
        raise SystemExit(2)

    ifc_path = Path(argv[1])
    diagnostics_path = Path(argv[2])
    if not ifc_path.is_file():
        fail(f"IFC file is missing: {ifc_path}")
    if not diagnostics_path.is_file():
        fail(f"diagnostics JSON is missing: {diagnostics_path}")
    return ifc_path, json.loads(diagnostics_path.read_text(encoding="utf-8"))


def validate(ifc_path: Path, diagnostics: dict) -> None:
    model = ifcopenshell.open(str(ifc_path))
    if model.schema != "IFC4":
        fail(f"schema regressed: got {model.schema}, expected IFC4")

    assert_exact(model, "IfcProject", 1)
    assert_exact(model, "IfcSite", 1)
    assert_exact(model, "IfcBuilding", 1)
    assert_at_least(model, "IfcBuildingStorey", 1)
    assert_exact(model, "IfcUnitAssignment", 1)
    assert_at_least(model, "IfcSIUnit", 1)

    exported = diagnostics.get("exported", {})
    by_ifc_type = exported.get("by_ifc_type", {})
    if not by_ifc_type:
        fail("diagnostics exported.by_ifc_type is empty for the real project")

    for step_type, expected in sorted(by_ifc_type.items()):
        ifc_class = STEP_TO_IFCOPENSHELL.get(step_type)
        if ifc_class is None:
            fail(f"no validator mapping for diagnostics entity class {step_type}")
        got = count(model, ifc_class)
        if got != expected:
            fail(f"{ifc_class} count regressed: IfcOpenShell saw {got}, diagnostics saw {expected}")

    # Current real-project coverage comes from the 2023 Einhoven ArcWall
    # decoder. This threshold catches silent walker/decoder regressions while
    # allowing future improvements to add more recovered element classes.
    assert_at_least(model, "IfcWall", 10)

    contained = model.by_type("IfcRelContainedInSpatialStructure")
    if len(contained) != 1:
        fail(f"IfcRelContainedInSpatialStructure count regressed: got {len(contained)}, expected 1")
    related_count = len(contained[0].RelatedElements)
    building_elements = int(exported.get("building_elements", 0))
    if related_count < building_elements:
        fail(
            "spatial containment regressed: "
            f"related elements {related_count}, diagnostics building_elements {building_elements}"
        )

    with_geometry = int(exported.get("building_elements_with_geometry", 0))
    shapes = count(model, "IfcProductDefinitionShape")
    if with_geometry > 0 and shapes < with_geometry:
        fail(
            "IfcProductDefinitionShape count regressed: "
            f"got {shapes}, expected at least {with_geometry}"
        )
    if with_geometry == 0:
        require_unsupported(diagnostics, "real_file_element_geometry")

    material_count = int(exported.get("material_count", 0))
    if material_count > 0:
        assert_at_least(model, "IfcMaterial", material_count)
    else:
        require_unsupported(diagnostics, "revit_materials_and_compound_assemblies")

    unit_count = int(exported.get("unit_assignment_count", 0))
    if unit_count < 1:
        fail(f"unit assignment regressed: diagnostics unit_assignment_count={unit_count}")

    print("Real-project IfcOpenShell validation passed:")
    print(f"  file: {ifc_path}")
    print(f"  schema: {model.schema}")
    print(f"  entities: {len(list(model))}")
    print(f"  exported.by_ifc_type: {by_ifc_type}")
    print(f"  spatial containment: {related_count} related element(s)")
    print(f"  geometry-backed elements: {with_geometry}")
    print(f"  material count: {material_count}")
    print(f"  unit assignments: {unit_count}")


def main(argv: list[str]) -> None:
    ifc_path, diagnostics = load_inputs(argv)
    validate(ifc_path, diagnostics)


if __name__ == "__main__":
    main(sys.argv)
