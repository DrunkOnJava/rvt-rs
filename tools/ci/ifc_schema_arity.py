#!/usr/bin/env python3
"""Schema-conformance gate for every instance in an emitted IFC file.

IfcOpenShell's ``open()`` is tolerant: it parses a record that lists
fewer attributes than its entity type declares, and only a consumer
that *reads* the missing attribute finds out, with

    RuntimeError: Index 8 is out of range for variant of size 8

That is how rvt-rs shipped ``IFCCOLUMN`` with eight attributes against
a declared nine for as long as it did (#214). This module closes that
hole by re-deriving the declared attribute count from the IFC4 EXPRESS
schema and comparing it against the count actually written in the STEP
text, for every instance in the file.

Four checks, all fail-closed:

1. **Arity** — the number of top-level attributes written must equal
   ``declaration_by_name(<entity>).attribute_count()``.
2. **PredefinedType** — for entity types where every ``category_map``
   row supplies a value, the attribute must not be null. Types whose
   Revit classes legitimately have no mapped value (``IfcSpace`` via
   ``Area`` / ``Space``, ``IfcBuildingElementProxy`` via
   ``GenericModel``) are not in that set: writing ``$`` there is the
   honest answer, and forcing a value would be an invention.
3. **Placement composition** — a swept solid's ``Position`` must never
   be the same instance as its product's
   ``ObjectPlacement.RelativePlacement``, and for a handful of pinned
   elements the explicit composition ``ObjectPlacement × Position ×
   profile-point`` must land on a pinned coordinate. That is the exact
   shape of #232, where the writer emitted the element's own
   ``IfcAxis2Placement3D`` in both slots and every conforming consumer
   applied the element translation twice.
4. **PredefinedType agreement** (``--witness-agreement``) — on the
   recorded artifact the written value must equal the value Revit's own
   exporter writes for that entity type on
   ``2024_Core_Interior_slim.ifc`` (#220). Off by default, because a
   synthetic fixture is not the recorded artifact and has no witness to
   agree with.

Usage::

    python tools/ci/ifc_schema_arity.py [--witness-agreement] FILE.ifc [FILE.ifc ...]
"""

from __future__ import annotations

import re
import sys
from collections import Counter
from pathlib import Path

import ifcopenshell
import ifcopenshell.ifcopenshell_wrapper as wrapper
import ifcopenshell.util.placement


# Entity types whose every `src/ifc/category_map.rs` row carries a
# `predefined_type`, so a null in the emitted file means the writer
# dropped a value it had rather than reporting one it lacked.
#
# `IfcDoor` / `IfcWindow` joined the set in #220, when the Door and
# Window rows stopped emitting `$` and started writing the `.DOOR.` /
# `.WINDOW.` the authoring witness writes.
PREDEFINED_TYPE_REQUIRED = {
    "IfcBeam",
    "IfcColumn",
    "IfcCovering",
    "IfcDoor",
    "IfcMember",
    "IfcSlab",
    "IfcWall",
    "IfcWindow",
}


# #220: the `PredefinedType` values Revit's own exporter writes on the
# recorded artifact, `IFC Exports/2024_Core_Interior_slim.ifc` (sha256
# bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d),
# measured with IfcOpenShell over all 950 typed products:
#
#     IfcColumn  COLUMN      256/256
#     IfcSlab    FLOOR        78/80,  ROOF 2/80
#     IfcWall    NOTDEFINED  360/360
#     IfcSpace   SPACE       116/116
#     IfcDoor    DOOR        132/132
#     IfcWindow  WINDOW        6/6
#
# `IfcSlab` carries both values because Revit splits 78 floors from 2
# roofs per element; rvt-rs cannot make that call yet and writes
# `.FLOOR.` for all 80, which is inside the witness's value set. The
# check is per instance and only runs under `--witness-agreement`.
WITNESS_PREDEFINED_TYPES = {
    "IfcColumn": {"COLUMN"},
    "IfcSlab": {"FLOOR", "ROOF"},
    "IfcWall": {"NOTDEFINED"},
    "IfcSpace": {"SPACE"},
    "IfcDoor": {"DOOR"},
    "IfcWindow": {"WINDOW"},
}


# #232: pinned world points, in the file's own length unit, for
# `ObjectPlacement × Position × first-profile-point`. Keyed by
# `(entity type, Tag)` so a file that does not hold the element skips
# the probe instead of failing; a file that does hold it must place it
# exactly. Recomputing these by hand is the point — with the element's
# own axis back in the `Position` slot every one of them moves by the
# element translation.
#
# `synthetic-project` / `synthetic-structural` are the committed
# fixtures; `20375` / `20345` are a column and a floor slab of the
# recorded artifact, whose fixed values were read back out of the
# export at 1e-9 agreement with the composition below.
PLACEMENT_PROBES = {
    # tests/fixtures/synthetic-project.ifc
    ("IfcWall", "W-N-001"): (-3.048, 2.9464, 0.0),
    ("IfcSlab", "SLAB-001"): (-3.048, -1.524, -0.3048),
    # tests/fixtures/synthetic-structural.ifc
    ("IfcColumn", "C-W12x26-1"): (-0.082423, -0.15494, 0.0),
    # The recorded artifact: a 24" x 24" column on Level 1 and an
    # arch topping slab, both of the 2024 Core Interior project.
    ("IfcColumn", "20375"): (7.0104, 33.2232, 23.1648),
    ("IfcSlab", "20345"): (50.9016, 7.62, 23.114),
}

PROBE_TOLERANCE = 1e-6

_INSTANCE = re.compile(r"^#(\d+)\s*=\s*([A-Za-z0-9_]+)\s*\((.*)\)\s*;\s*$")


class ArityError(AssertionError):
    """Raised when an emitted instance disagrees with the schema."""


def step_argument_count(args: str) -> int:
    """Count top-level attributes, honouring quotes and aggregates."""
    if not args.strip():
        return 0
    count = 1
    depth = 0
    in_string = False
    index = 0
    while index < len(args):
        char = args[index]
        if in_string:
            if char == "'":
                if index + 1 < len(args) and args[index + 1] == "'":
                    index += 2
                    continue
                in_string = False
        elif char == "'":
            in_string = True
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        elif char == "," and depth == 0:
            count += 1
        index += 1
    return count


def check_arity(ifc_path: Path, schema_name: str) -> Counter:
    """Compare every written instance against its schema declaration."""
    schema = wrapper.schema_by_name(schema_name)
    seen: Counter = Counter()
    failures: list[str] = []
    with ifc_path.open("r", encoding="utf-8", errors="replace") as handle:
        for lineno, line in enumerate(handle, start=1):
            match = _INSTANCE.match(line.strip())
            if match is None:
                continue
            _, name, args = match.groups()
            try:
                declared = schema.declaration_by_name(name).as_entity().attribute_count()
            except Exception:  # noqa: BLE001 - schema lookup is the check
                failures.append(f"{ifc_path}:{lineno}: {name} is not in {schema_name}")
                continue
            written = step_argument_count(args)
            seen[name] += 1
            if written != declared:
                failures.append(
                    f"{ifc_path}:{lineno}: {name} wrote {written} attribute(s), "
                    f"{schema_name} declares {declared} — every attribute a type "
                    f"declares must occupy its slot, `$` when unknown"
                )
    if failures:
        raise ArityError(
            "IFC schema arity check failed:\n  " + "\n  ".join(failures[:40])
        )
    return seen


def check_predefined_types(ifc_path: Path) -> Counter:
    """Assert mapped `PredefinedType` values survived to the file."""
    model = ifcopenshell.open(str(ifc_path))
    populated: Counter = Counter()
    failures: list[str] = []
    for ifc_class in sorted(PREDEFINED_TYPE_REQUIRED):
        try:
            instances = model.by_type(ifc_class, include_subtypes=False)
        except Exception:  # noqa: BLE001 - class absent from this schema
            continue
        for instance in instances:
            try:
                value = instance.PredefinedType
            except Exception as exc:  # noqa: BLE001 - short attribute list
                failures.append(f"{ifc_path}: #{instance.id()} {ifc_class}: {exc}")
                continue
            if value is None:
                failures.append(
                    f"{ifc_path}: #{instance.id()} {ifc_class} has a null "
                    f"PredefinedType, but every category_map row for "
                    f"{ifc_class} supplies one"
                )
            else:
                populated[ifc_class] += 1
    if failures:
        raise ArityError(
            "IFC PredefinedType check failed:\n  " + "\n  ".join(failures[:40])
        )
    return populated


def check_witness_predefined_types(ifc_path: Path) -> Counter:
    """Assert each written `PredefinedType` is the witness's own value.

    Only meaningful on the recorded artifact — a synthetic fixture has
    no authoring witness to agree with — so callers opt in.
    """
    model = ifcopenshell.open(str(ifc_path))
    agreed: Counter = Counter()
    failures: list[str] = []
    for ifc_class, expected in sorted(WITNESS_PREDEFINED_TYPES.items()):
        try:
            instances = model.by_type(ifc_class, include_subtypes=False)
        except Exception:  # noqa: BLE001 - class absent from this schema
            continue
        for instance in instances:
            try:
                value = instance.PredefinedType
            except Exception as exc:  # noqa: BLE001 - short attribute list
                failures.append(f"{ifc_path}: #{instance.id()} {ifc_class}: {exc}")
                continue
            if value is None:
                continue  # governed by check_predefined_types
            if value not in expected:
                failures.append(
                    f"{ifc_path}: #{instance.id()} {ifc_class} wrote "
                    f"PredefinedType .{value}., but Revit's own export of the "
                    f"recorded artifact writes {sorted(expected)} for that type"
                )
            else:
                agreed[f"{ifc_class}.{value}"] += 1
    if failures:
        raise ArityError(
            "IFC PredefinedType witness-agreement check failed:\n  "
            + "\n  ".join(failures[:40])
        )
    return agreed


def _profile_probe_point(profile) -> tuple[float, float] | None:
    """One deterministic point of a profile, in the profile's own frame."""
    if profile is None:
        return None
    if profile.is_a("IfcParameterizedProfileDef"):
        if profile.is_a("IfcRectangleProfileDef"):
            local = (-profile.XDim / 2.0, -profile.YDim / 2.0)
        elif profile.is_a("IfcCircleProfileDef"):
            local = (-profile.Radius, 0.0)
        elif profile.is_a("IfcIShapeProfileDef"):
            local = (-profile.OverallWidth / 2.0, -profile.OverallDepth / 2.0)
        else:
            return None
        position = profile.Position
        if position is None:
            return local
        ox, oy = position.Location.Coordinates[:2]
        if position.RefDirection is None:
            rx, ry = 1.0, 0.0
        else:
            rx, ry = position.RefDirection.DirectionRatios[:2]
        return (
            ox + local[0] * rx - local[1] * ry,
            oy + local[0] * ry + local[1] * rx,
        )
    curve = getattr(profile, "OuterCurve", None)
    if curve is not None and curve.is_a("IfcPolyline"):
        return tuple(curve.Points[0].Coordinates[:2])
    return None


def _swept_solids(product):
    """Yield the swept-area solids directly in a product's Body items."""
    representation = getattr(product, "Representation", None)
    if representation is None:
        return
    for shape in representation.Representations:
        for item in shape.Items:
            if item.is_a("IfcSweptAreaSolid"):
                yield shape, item


def check_placement_composition(ifc_path: Path) -> tuple[int, int]:
    """Assert `ObjectPlacement × Position` is not a doubled translation.

    Two parts. The structural invariant runs on every product in the
    file: a swept solid's `Position` may not be the very instance the
    product's `IfcLocalPlacement` already uses as its
    `RelativePlacement`, because a conforming consumer composes the two
    and would apply the same translation twice (#232). The pinned part
    then composes the two matrices for real and checks that a known
    profile point lands where it is supposed to.
    """
    model = ifcopenshell.open(str(ifc_path))
    failures: list[str] = []
    inspected = 0
    probed = 0
    for product in model.by_type("IfcProduct"):
        placement = getattr(product, "ObjectPlacement", None)
        if placement is None or not placement.is_a("IfcLocalPlacement"):
            continue
        relative = placement.RelativePlacement
        for _shape, solid in _swept_solids(product):
            inspected += 1
            position = solid.Position
            if relative is not None and position is not None:
                if position.id() == relative.id():
                    failures.append(
                        f"{ifc_path}: #{product.id()} {product.is_a()} reuses "
                        f"#{position.id()} as both its ObjectPlacement's "
                        f"RelativePlacement and its {solid.is_a()}.Position — a "
                        f"consumer composing the two applies the element "
                        f"translation twice (#232)"
                    )
                    continue
            key = (product.is_a(), getattr(product, "Tag", None))
            expected = PLACEMENT_PROBES.get(key)
            if expected is None:
                continue
            local = _profile_probe_point(solid.SweptArea)
            if local is None:
                continue
            world = ifcopenshell.util.placement.get_local_placement(placement)
            world = world @ ifcopenshell.util.placement.get_axis2placement(position)
            point = world @ [local[0], local[1], 0.0, 1.0]
            probed += 1
            delta = max(abs(point[i] - expected[i]) for i in range(3))
            if delta > PROBE_TOLERANCE:
                failures.append(
                    f"{ifc_path}: #{product.id()} {product.is_a()} Tag "
                    f"{key[1]}: ObjectPlacement x Position x profile-point "
                    f"= ({point[0]:.6f}, {point[1]:.6f}, {point[2]:.6f}), "
                    f"pinned ({expected[0]:.6f}, {expected[1]:.6f}, "
                    f"{expected[2]:.6f}), delta {delta:.6g} > "
                    f"{PROBE_TOLERANCE:g}"
                )
    if failures:
        raise ArityError(
            "IFC placement-composition check failed:\n  " + "\n  ".join(failures[:40])
        )
    return inspected, probed


def validate(ifc_path: Path, witness_agreement: bool = False) -> None:
    model = ifcopenshell.open(str(ifc_path))
    schema_name = model.schema
    seen = check_arity(ifc_path, schema_name)
    populated = check_predefined_types(ifc_path)
    inspected, probed = check_placement_composition(ifc_path)
    agreed = check_witness_predefined_types(ifc_path) if witness_agreement else Counter()
    print(f"IFC schema conformance passed: {ifc_path}")
    print(f"  schema: {schema_name}")
    print(f"  instances checked: {sum(seen.values())} across {len(seen)} entity type(s)")
    if populated:
        summary = ", ".join(f"{k}={v}" for k, v in sorted(populated.items()))
        print(f"  PredefinedType populated: {summary}")
    else:
        print("  PredefinedType populated: none of the required types are present")
    print(
        f"  placement composition: {inspected} swept solid(s) carry a Position "
        f"distinct from their product placement, {probed} pinned probe(s) matched"
    )
    if witness_agreement:
        summary = ", ".join(f"{k}={v}" for k, v in sorted(agreed.items()))
        print(f"  PredefinedType agrees with the witness: {summary or 'no typed product'}")


def main(argv: list[str]) -> None:
    args = argv[1:]
    witness_agreement = "--witness-agreement" in args
    files = [a for a in args if not a.startswith("--")]
    unknown = [a for a in args if a.startswith("--") and a != "--witness-agreement"]
    if unknown or not files:
        print(
            "usage: ifc_schema_arity.py [--witness-agreement] FILE.ifc [FILE.ifc ...]",
            file=sys.stderr,
        )
        raise SystemExit(2)
    for raw in files:
        path = Path(raw)
        if not path.is_file():
            raise ArityError(f"IFC file is missing: {path}")
        validate(path, witness_agreement=witness_agreement)


if __name__ == "__main__":
    main(sys.argv)
