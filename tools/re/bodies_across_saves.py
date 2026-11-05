#!/usr/bin/env python3
"""Whether the bodies a Revit 2026 or 2027 save draws are the bodies the same
project's earlier save draws (RE-341).

No Revit export of a 2026 or 2027 beam, landing, stair run or sketch outline
is available to score them against, so each is held to the same Autodesk
sample project saved by Revit 2024 or 2025, whose export is measured against
Revit's own. Both saves are exported by the build under test, so a difference
is the reader's, or the project's own edit between saves. What this cannot
catch: an error the earlier save shares with this one.

The candidate's elements checked are those whose body the widened release
lists change:

  beams and landings   `BodySource` is partition_beam_axis,
                       partition_beam_box_along_axis, native_landing_tread or
                       partition_landing_slab; held to the 2024 save
  stair runs           `BodySource` is partition_stair_run_sketch; held to the
                       2025 save, which reads runs from their native records as
                       these releases do (the 2024 save's byte-anchor reader
                       finds 5 of rac_advanced's 10 runs and 1 of rac_basic's 2)
  sketch outlines      a floor, ceiling, slab or roof (IfcSlab, IfcCovering,
                       IfcRoof) whose plan outline was recovered
                       (`ProfileResolved`); held to the 2024 save

Each is paired by `Tag` (the ElementId, which a project keeps across saves)
with the earlier save's element, and fails on any difference in: its IFC
class, GlobalId, storey, BodySource, representation types and solid kinds,
placement (0.001 ft), body volume (1e-6 relative) and vertices (each within
0.001 ft of one of the other's, both ways). It also fails on an element the
earlier save has no counterpart for, one the earlier save draws otherwise, one
of the earlier save's that this save lacks altogether, a missing file, fewer
elements than the sample is known to hold (MINIMUMS), a required element not
compared (REQUIRED), and a check that compared nothing. An element the earlier
save draws and this one leaves as a record box is counted, not scored: a body
left out is fail closed.

Usage:

    python3 tools/re/bodies_across_saves.py <candidate>/model.ifc \\
        2024=<earlier 2024>/model.ifc 2025=<earlier 2025>/model.ifc

The candidate's directory is named `sample-<year>-<project>`, as Measure
names it. Exits 1 on any failure. The last line is `bodies across saves:
PASS` or `FAIL`.
"""

import collections
import os
import re
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

FEET = 0.3048
TOLERANCE = 0.001 * FEET
ANGLE_TOLERANCE = 1e-6
VOLUME_TOLERANCE = 1e-6
SHOW = 40

BEAMS_AND_LANDINGS = "beams and landings"
STAIR_RUNS = "stair runs"
SKETCH_OUTLINES = "sketch outlines"

SOURCE_GROUP = {
    "partition_beam_axis": BEAMS_AND_LANDINGS,
    "partition_beam_box_along_axis": BEAMS_AND_LANDINGS,
    "native_landing_tread": BEAMS_AND_LANDINGS,
    "partition_landing_slab": BEAMS_AND_LANDINGS,
    "partition_stair_run_sketch": STAIR_RUNS,
}
OUTLINE_CLASSES = {"IfcSlab", "IfcCovering", "IfcRoof"}
REFERENCE_YEAR = {BEAMS_AND_LANDINGS: 2024, STAIR_RUNS: 2025, SKETCH_OUTLINES: 2024}

# Bodies the sample is known to draw by BodySource (Measure run 41988011289,
# head 1d5bbf03, exported diagnostics), and outlines by group.
MINIMUMS = {
    "racadvancedsampleproject": {
        BEAMS_AND_LANDINGS: {
            "partition_beam_axis": 29,
            "partition_beam_box_along_axis": 8,
            "native_landing_tread": 5,
        },
        STAIR_RUNS: {"partition_stair_run_sketch": 10},
    },
    "rstbasicsampleproject": {
        BEAMS_AND_LANDINGS: {"partition_beam_axis": 345, "partition_beam_box_along_axis": 9},
        STAIR_RUNS: {"partition_stair_run_sketch": 1},
    },
    "racbasicsampleproject": {STAIR_RUNS: {"partition_stair_run_sketch": 2}},
}
# Outlines that must be among those compared: rac_advanced's ceiling and floor
# the sketch lines draw as the 2024 save's.
REQUIRED = {"racadvancedsampleproject": {SKETCH_OUTLINES: {"147534", "139803"}}}


def properties(element):
    """(BodySource, ProfileResolved) from the element's property sets."""
    source, resolved = None, False
    for rel in element.IsDefinedBy:
        if not rel.is_a("IfcRelDefinesByProperties"):
            continue
        for prop in getattr(rel.RelatingPropertyDefinition, "HasProperties", None) or []:
            if prop.NominalValue is None:
                continue
            if prop.Name == "BodySource":
                source = prop.NominalValue.wrappedValue
            elif prop.Name == "ProfileResolved":
                resolved = bool(prop.NominalValue.wrappedValue)
    return source, resolved


def group_of(element, source, resolved):
    if source in SOURCE_GROUP:
        return SOURCE_GROUP[source]
    if element.is_a() in OUTLINE_CLASSES and resolved:
        return SKETCH_OUTLINES
    return None


def products(path):
    """Tag -> [(element, BodySource, group)] for every product with a body."""
    model = ifcopenshell.open(path)
    out = collections.defaultdict(list)
    for element in model.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or not element.Representation:
            continue
        tag = getattr(element, "Tag", None)
        if not tag:
            continue
        source, resolved = properties(element)
        out[tag].append((element, source, group_of(element, source, resolved)))
    for tag in out:
        out[tag].sort(key=lambda item: item[0].GlobalId or "")
    return model, out


def storey(element):
    seen = 0
    while element is not None and seen < 8:
        container = ifcopenshell.util.element.get_container(element)
        if container is not None:
            return container.Name
        element = ifcopenshell.util.element.get_aggregate(element)
        seen += 1
    return None


def signature(element):
    return sorted(
        (rep.RepresentationIdentifier, rep.RepresentationType, tuple(sorted(i.is_a() for i in rep.Items)))
        for rep in element.Representation.Representations
    )


def placement(element, scale):
    if element.ObjectPlacement is None:
        return None
    matrix = np.array(ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement), dtype=float)
    matrix[:3, 3] *= scale
    return matrix


def mesh(settings, element):
    """(volume, vertices) in metres, or the reason it did not mesh."""
    try:
        shape = ifcopenshell.geom.create_shape(settings, element)
    except RuntimeError as error:
        return (str(error).strip().splitlines() or [""])[0] or type(error).__name__
    verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
    if len(faces) == 0:
        return "no faces"
    origin = verts[0]
    a, b, c = (verts[faces[:, i]] - origin for i in range(3))
    volume = abs(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0)
    return volume, np.unique(np.round(verts, 7), axis=0)


def reach(a, b):
    """How far the vertex of a farthest from every vertex of b is from the
    nearest one, in metres."""
    worst = 0.0
    step = max(1, 2_000_000 // max(len(b), 1))
    for i in range(0, len(a), step):
        distance = np.linalg.norm(a[i : i + step, None, :] - b[None, :, :], axis=2).min(axis=1)
        worst = max(worst, float(distance.max()))
    return worst


def differences(settings, scale, left, right):
    """What differs between a candidate product and its earlier save's, as
    [(field, candidate's, earlier's)]."""
    (a, a_source, _), (b, b_source, _) = left, right
    found = []
    for field, x, y in (
        ("class", a.is_a(), b.is_a()),
        ("GlobalId", a.GlobalId, b.GlobalId),
        ("storey", storey(a), storey(b)),
        ("BodySource", a_source, b_source),
        ("representation", signature(a), signature(b)),
    ):
        if x != y:
            found.append((field, x, y))
    pa, pb = placement(a, scale[0]), placement(b, scale[1])
    if (pa is None) != (pb is None):
        found.append(("placement", pa is not None, pb is not None))
    elif pa is not None:
        moved = float(np.abs(pa[:3, 3] - pb[:3, 3]).max())
        turned = float(np.abs(pa[:3, :3] - pb[:3, :3]).max())
        if moved > TOLERANCE or turned > ANGLE_TOLERANCE:
            found.append(("placement", f"{moved / FEET:.4f} ft apart, axes differ by {turned:.2g}", ""))
    ma, mb = mesh(settings, a), mesh(settings, b)
    if isinstance(ma, str) or isinstance(mb, str):
        found.append(("mesh", ma if isinstance(ma, str) else "meshes", mb if isinstance(mb, str) else "meshes"))
        return found
    if abs(ma[0] - mb[0]) > VOLUME_TOLERANCE * max(mb[0], 1e-12):
        found.append(("volume", f"{ma[0]:.9g} m3", f"{mb[0]:.9g} m3"))
    away = max(reach(ma[1], mb[1]), reach(mb[1], ma[1]))
    if away > TOLERANCE:
        found.append(("vertices", f"the farthest is {away / FEET:.4f} ft from the earlier save's", ""))
    return found


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        print(__doc__)
        return 2
    candidate_path = args[0]
    earlier_paths = {}
    for arg in args[1:]:
        year, _, path = arg.partition("=")
        earlier_paths[int(year)] = path
    parent = os.path.basename(os.path.dirname(os.path.abspath(candidate_path)))
    named = re.fullmatch(r"sample-(\d{4})-(.+)", parent)
    project = named.group(2) if named else None
    failures = []

    def fail(text):
        failures.append(text)

    print(f"bodies across saves: {parent}, held to the {', '.join(str(y) for y in sorted(earlier_paths))} saves")
    if not os.path.isfile(candidate_path):
        fail(f"this save's export is missing: {candidate_path}")
        report_and_exit(failures)
    model, candidate = products(candidate_path)
    scale_candidate = ifcopenshell.util.unit.calculate_unit_scale(model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    earlier = {}
    for year, path in sorted(earlier_paths.items()):
        if not os.path.isfile(path):
            fail(f"the {year} save's export is missing: {path}")
            continue
        earlier[year] = products(path)
    total = 0
    minimums = MINIMUMS.get(project, {})
    required = REQUIRED.get(project, {})
    for group in (BEAMS_AND_LANDINGS, STAIR_RUNS, SKETCH_OUTLINES):
        year = REFERENCE_YEAR[group]
        chosen = {
            tag: [item for item in items if item[2] == group]
            for tag, items in candidate.items()
            if any(item[2] == group for item in items)
        }
        by_source = collections.Counter(item[1] for items in chosen.values() for item in items)
        for source, least in minimums.get(group, {}).items():
            if by_source.get(source, 0) < least:
                fail(f"{group}: {by_source.get(source, 0)} drawn as {source}, fewer than the {least} it holds")
        if year not in earlier:
            fail(f"{group}: the {year} save is missing, nothing to hold {len(chosen)} tags to")
            print(f"{group}: not compared, the {year} save is missing")
            continue
        earlier_model, reference = earlier[year]
        scale_earlier = ifcopenshell.util.unit.calculate_unit_scale(earlier_model)
        compared = collections.Counter()
        compared_tags = set()
        for tag in sorted(chosen, key=lambda t: (0, int(t)) if t.isdigit() else (1, t)):
            counterparts = reference.get(tag)
            if not counterparts:
                fail(f"{group}: {tag} is not in the {year} save")
                continue
            selected = [item for item in counterparts if item[2] == group]
            if len(selected) != len(chosen[tag]):
                drawn = sorted({str(item[1]) for item in counterparts})
                fail(f"{group}: {tag} is {len(chosen[tag])} product(s) here, {len(selected)} drawn so in the {year} save ({', '.join(drawn)})")
                continue
            ok = True
            for left, right in zip(chosen[tag], selected):
                for field, mine, theirs in differences(settings, (scale_candidate, scale_earlier), left, right):
                    ok = False
                    detail = f"{mine} here, {theirs} in the {year} save" if theirs != "" else mine
                    fail(f"{group}: {tag} {left[0].is_a()} differs in {field}: {detail}")
                compared[left[0].is_a()] += 1
            if ok:
                compared_tags.add(tag)
        total += sum(compared.values())
        left_out = collections.Counter()
        for tag, items in reference.items():
            for item in items:
                if item[2] != group or tag in chosen:
                    continue
                if tag not in candidate:
                    fail(f"{group}: {tag} ({item[0].is_a()}) is in the {year} save and not in this one")
                else:
                    left_out[item[0].is_a()] += 1
        for tag in sorted(required.get(group, ())):
            if tag not in compared_tags:
                fail(f"{group}: {tag} was not compared equal, and is required")
        print(
            f"{group}: compared {sum(compared.values())} against the {year} save ({', '.join(f'{c} {n}' for c, n in sorted(compared.items())) or 'none'}); "
            f"left as boxes here, drawn in the {year} save, not scored: {sum(left_out.values())}"
            + (f" ({', '.join(f'{c} {n}' for c, n in sorted(left_out.items()))})" if left_out else "")
        )
    if total == 0:
        fail("nothing was compared")
    report_and_exit(failures)


def report_and_exit(failures):
    for line in failures[:SHOW]:
        print(f"  {line}")
    if len(failures) > SHOW:
        print(f"  ... and {len(failures) - SHOW} more")
    if failures:
        print(f"bodies across saves: FAIL ({len(failures)} differences)")
        sys.exit(1)
    print("bodies across saves: PASS")
    sys.exit(0)


if __name__ == "__main__":
    sys.exit(main())
