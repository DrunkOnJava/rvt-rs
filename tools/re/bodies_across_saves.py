#!/usr/bin/env python3
"""Whether the bodies a Revit 2025, 2026 or 2027 save draws are the bodies the
same project's earlier save draws (RE-341, RE-345, RE-346).

No Revit export of a 2025, 2026 or 2027 beam, landing, stair run or sketch
outline of an Autodesk sample is available to score them against, so each is
held to the same Autodesk sample project saved by Revit 2024 (or, for stair
runs, 2025), whose export is measured against Revit's own. Both saves are
exported by the build under test, so a difference is the reader's, or the
project's own edit between saves. What this cannot catch: an error the
earlier save shares with this one. A 2025 save is held to the 2024 save only:
its stair runs are what the 2026 and 2027 saves are held to, so they are not
compared (`not applicable`), though the number it draws is still checked.
The elements whose sketch holds an arc (ARC_OWNERS) are the only check of the
arc reader on 2025 to 2027: no Revit export of a floor, ceiling or roof with an
arc exists on those releases, and the output counts how many of them are
compared equal as outlines. A shaft opening is cut on 2024 only (RE-345), so
no shaft void is compared on 2025 to 2027.

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
  named elements       the elements NAMED_TAGS lists, whatever their source,
                       held to the 2024 save; their sources and volumes in
                       every save are printed
  pipes                `BodySource` is partition_pipe_axis (RE-131, RE-346);
                       held to the 2025 save, which draws them so
  doors and windows    an IfcDoor or IfcWindow that carries
                       RvtElementRecordGeometry.PlacementAxesSource, placed as
                       Revit's export places a door or window set in a plain
                       wall (RE-347); held to the 2024 save, where its axes,
                       GlobalId and body must be the same. The number compared,
                       and of those not on the default axes, is printed; the
                       least compared per class (OPENINGS_LEAST) and the most
                       the 2024 save places by the rule that this save leaves on
                       the default axes (OPENINGS_LEFT_OUT_MOST) are set per
                       project and year, and the second can only shrink

Each is paired by `Tag` (the ElementId, which a project keeps across saves)
with the earlier save's element, and fails on any difference in: its IFC
class, GlobalId, storey, BodySource, ProfileResolved, representation types
and solid kinds,
placement (0.001 ft), body volume (1e-6 relative) and vertices (each within
0.001 ft of one of the other's, both ways). Where a Tag has several products (a
floor Revit writes as several), only the first keeps the element's own GlobalId,
so the saves must share exactly one GlobalId, on the two pieces paired by storey,
volume and centre; a pipe that is a circular extrusion in both saves is held to
the cylinder itself instead of the vertices of the polygon it meshes to: the
axis ends in either order, the radius and the length, each within 0.001 ft
(Griffin, 2026-11-05: the 2027 save writes the extrusion of 115 of rme_basic's 491
pipes from the other end or turned about its axis, and 15 of those polygons lie
0.0038 to 0.0063 ft from the 2025 save's). The count of each is printed. It also fails on an element the
earlier save has no counterpart for, one the earlier save draws otherwise, one
of the earlier save's that this save lacks altogether, a missing file, fewer
elements than the sample is known to hold (MINIMUMS, OUTLINE_LEAST,
OPENINGS_LEAST), more doors and windows left on the default axes than
OPENINGS_LEFT_OUT_MOST, a
required element not compared equal (REQUIRED), and a check that compared
nothing, and on a directory whose project is not one of MINIMUMS'. An element
the earlier save draws and this one leaves as a record box is counted, not
scored: a body left out is fail closed.

A save is also held to the 2025 save for what `readers_across_saves.py` checks
(RE-346): its door and window cuts, ports and properties, and a census of every
body. The pipes' MINIMUMS and that module's are from Measure run 42063046730
(head 3149973f, 2026 saves).

An earlier save that is not among the arguments (a run that fetched only some
years) is not compared, and the verdict is `PARTIAL (not compared: ...)`, never
`PASS`; an argument whose export is missing fails. Sketch outlines are also
compared, without scoring, with the 2025 save, so that where the 2024 and 2025
saves disagree the output says so.

Usage:

    python3 tools/re/bodies_across_saves.py <candidate>/model.ifc \\
        2024=<earlier 2024>/model.ifc 2025=<earlier 2025>/model.ifc

The candidate's directory is named `sample-<year>-<project>`, as Measure
names it. Exits 1 on any failure. The last line is `bodies across saves:
PASS`, `PARTIAL (...)` or `FAIL (...)`.
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
NAMED_ELEMENTS = "named elements"
PIPES = "pipes"
DOORS_AND_WINDOWS = "doors and windows"
OPENING_CLASSES = {"IfcDoor", "IfcWindow"}

SOURCE_GROUP = {
    "partition_beam_axis": BEAMS_AND_LANDINGS,
    "partition_beam_box_along_axis": BEAMS_AND_LANDINGS,
    "native_landing_tread": BEAMS_AND_LANDINGS,
    "partition_landing_slab": BEAMS_AND_LANDINGS,
    "partition_stair_run_sketch": STAIR_RUNS,
    "partition_pipe_axis": PIPES,
}
OUTLINE_CLASSES = {"IfcSlab", "IfcCovering", "IfcRoof"}
REFERENCE_YEAR = {
    BEAMS_AND_LANDINGS: 2024,
    STAIR_RUNS: 2025,
    SKETCH_OUTLINES: 2024,
    NAMED_ELEMENTS: 2024,
    PIPES: 2025,
    DOORS_AND_WINDOWS: 2024,
}
# The save readers_across_saves.py holds a save's door and window cuts, ports and
# properties to (RE-346).
READERS_YEAR = 2025

# Bodies the sample is known to draw by BodySource (Measure run 41988011289,
# head 1d5bbf03, exported diagnostics; the MEP samples' pipes, run 42063046730,
# head 3149973f), and outlines by group.
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
    "rmeadvancedsampleproject": {PIPES: {"partition_pipe_axis": 454}},
    "rmebasicsampleproject": {PIPES: {"partition_pipe_axis": 457}},
}
# At least this many sketch outlines compared, by candidate year (2026 and 2027:
# Measure run 42010304828, head e56d31b5; 2025: run 42056599984, head 14a03c02,
# the same code as this one).
OUTLINE_LEAST = {
    "racadvancedsampleproject": {2025: 86, 2026: 82, 2027: 82},
    "racbasicsampleproject": {2025: 13, 2026: 12, 2027: 12},
    "rstbasicsampleproject": {2025: 5, 2026: 5, 2027: 5},
    # The MEP samples, first held here (RE-346): 42 IfcCovering and 36 IfcSlab in each of
    # their 2025, 2026 and 2027 saves, scored with this script on the IFC files of run
    # 42091457764 (head 830a4d94), whose 2026 and 2027 outlines this change does not move.
    "rmeadvancedsampleproject": {2025: 78, 2026: 78, 2027: 78},
    "rmebasicsampleproject": {2025: 78, 2026: 78, 2027: 78},
}
# At least this many pipes compared as cylinders, by candidate year: every pipe the
# sample draws (the MINIMUMS above) is a circular extrusion in both saves (Measure run
# 42107917715, head f1460b32, scored with this script on the IFC files it kept).
CYLINDER_LEAST = {
    "rmeadvancedsampleproject": {2026: 454, 2027: 454},
    "rmebasicsampleproject": {2026: 457, 2027: 457},
}
# Doors and windows placed by the rule (RE-347) in each candidate save, held to
# the 2024 save, by project and candidate year. OPENINGS_LEAST: at least this many
# of each class compared; OPENINGS_LEFT_OUT_MOST: at most this many that the 2024
# save places by the rule and the candidate leaves on the default axes, which can
# only shrink: a count that drops moves the ceiling down in that change, and a floor
# up. #1456 set them from Measure run 42106711251 (head c1a2a739), where the cut pass
# ran on 2024 and 2025 only: rac_advanced's 2026 save left 28 (22 doors, 6 windows)
# and its 2027 save 42 (36, 6) on the default axes, 28 of them because the host
# rebinding (RE-245) is on `CUT_LOOP_REVIT_VERSIONS`. #1454 widens that list to 2026
# and 2027, so the 2026 save leaves none, and the 2027 save the 15 doors of 8 host walls
# with no native straight-line row (#1475): 2026 holds the 2025 save's 83 doors and 24
# windows, 2027 the 68 doors and 24 windows run 42124273032 (head ac922cd9) counted. rme_advanced
# and rme_basic hold 107 placed doors per file (the second read of #1456, 59 with
# unequal flips), held here for each year and left on the default axes by none;
# their windows are not floored until a run has counted them. rst_basic holds none.
# Run 42124273032 (head ac922cd9) counted 83 doors and 24 windows compared and none left
# out on rac_advanced 2026, 107 doors and none left out on each rme save of 2025 to 2027,
# and on rac_advanced 2027 68 doors and 24 windows compared and 15 doors left out, in the
# 8 host walls of #1475: 14 whose frame differs from the 2024 save's and one whose frame
# is the same. Only the 2027 numbers were set differently from what was predicted.
OPENINGS_LEAST = {
    "racadvancedsampleproject": {
        2025: {"IfcDoor": 83, "IfcWindow": 24},
        2026: {"IfcDoor": 83, "IfcWindow": 24},
        2027: {"IfcDoor": 68, "IfcWindow": 18},
    },
    "racbasicsampleproject": {
        2025: {"IfcDoor": 10, "IfcWindow": 7},
        2026: {"IfcDoor": 10, "IfcWindow": 7},
        2027: {"IfcDoor": 10, "IfcWindow": 7},
    },
    "rmeadvancedsampleproject": {
        2025: {"IfcDoor": 107},
        2026: {"IfcDoor": 107},
        2027: {"IfcDoor": 107},
    },
    "rmebasicsampleproject": {
        2025: {"IfcDoor": 107},
        2026: {"IfcDoor": 107},
        2027: {"IfcDoor": 107},
    },
}
OPENINGS_LEFT_OUT_MOST = {
    "racadvancedsampleproject": {2025: 0, 2026: 0, 2027: 15},
    "racbasicsampleproject": {2025: 0, 2026: 0, 2027: 0},
    "rmeadvancedsampleproject": {2025: 0, 2026: 0, 2027: 0},
    "rmebasicsampleproject": {2025: 0, 2026: 0, 2027: 0},
}
# rac_advanced's ceiling 147534, which the 2024 save draws as its record box
# and the 2026 save drew as an outline before a Sketch's curve list was read
# there (run 42010304828, base side): compared under whatever source each save
# gives it, with its source and volume printed.
NAMED_TAGS = {"racadvancedsampleproject": {"147534"}}
# The elements whose sketch holds an arc that reads inside its record box and
# not as a line (tools/re probe_re96_sketch_arcs on the 2024 and 2025 saves,
# run 42056599984): rac_advanced's 6 owners and rac_basic's 1. Printed in every
# save with their class, source, volume and vertex count, and counted where they
# are compared as sketch outlines.
ARC_OWNERS = {
    "racadvancedsampleproject": frozenset({"111931", "139803", "147534", "176909", "201262", "201312"}),
    "racbasicsampleproject": frozenset({"947498"}),
}
# Also printed in every save, with its vertex count: rac_advanced's floor 139803
# and roof 201262, which the 2024, 2025, 2026 and 2027 saves draw one way (floor
# 441.111228 m3, roof 5.33213143 m3) now that the 2025 save reads their arcs;
# before RE-345 the 2025 save drew chords (440.512245 and 6.87512864 m3).
PRINT_TAGS = {
    "racadvancedsampleproject": ARC_OWNERS["racadvancedsampleproject"] | {"139803", "201262"},
    "racbasicsampleproject": ARC_OWNERS["racbasicsampleproject"],
}
# Sketch outlines are also compared, without scoring, with this other earlier
# save, so that where the 2024 and 2025 saves disagree the lines say so.
OTHER_YEAR = {SKETCH_OUTLINES: 2025}
# Elements that must be among those compared equal, by candidate year and group:
# rac_advanced's floor the sketch lines draw as the 2024 save's and, on 2025
# (RE-345), its roof, which draws a 30 ft arc, and the ceiling above.
REQUIRED = {
    "racadvancedsampleproject": {
        2025: {SKETCH_OUTLINES: {"139803", "201262"}, NAMED_ELEMENTS: {"147534"}},
        2026: {SKETCH_OUTLINES: {"139803"}, NAMED_ELEMENTS: {"147534"}},
        2027: {SKETCH_OUTLINES: {"139803"}, NAMED_ELEMENTS: {"147534"}},
    }
}


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


def placement_source(element):
    """The `PlacementAxesSource` of a door or window (RE-347), or None."""
    for rel in element.IsDefinedBy:
        if not rel.is_a("IfcRelDefinesByProperties"):
            continue
        for prop in getattr(rel.RelatingPropertyDefinition, "HasProperties", None) or []:
            if prop.Name == "PlacementAxesSource" and prop.NominalValue is not None:
                return prop.NominalValue.wrappedValue
    return None


def group_of(element, source, resolved, named=False):
    if named:
        return NAMED_ELEMENTS
    if source in SOURCE_GROUP:
        return SOURCE_GROUP[source]
    if element.is_a() in OUTLINE_CLASSES and resolved:
        return SKETCH_OUTLINES
    if element.is_a() in OPENING_CLASSES and placement_source(element):
        return DOORS_AND_WINDOWS
    return None


def products(path, named=frozenset()):
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
        out[tag].append((element, source, group_of(element, source, resolved, tag in named)))
    # Products that share a Tag (a floor Revit writes as several, one stair for each
    # storey) are ordered by what they are, not by GlobalId: all but the first carry a
    # GlobalId derived from the export, which differs between two saves of one project
    # and would pair them crosswise (rme_advanced's floors, RE-346). Storey, then the
    # volume and the centre of the body, then the GlobalId.
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    for tag, items in out.items():
        if len(items) > 1:
            items.sort(key=lambda item: (storey(item[0]) or "", shape_key(settings, item[0]), item[0].GlobalId or ""))
    return model, out


def shape_key(settings, element):
    """The body's volume to three places and its centre to four, in metres, or
    nothing where it does not mesh."""
    shape = mesh(settings, element)
    if isinstance(shape, str):
        return (-1.0, 0.0, 0.0, 0.0)
    centre = shape[1].mean(axis=0)
    return (round(float(shape[0]), 3), *(round(float(c), 4) for c in centre))


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


def cylinder(element, scale):
    """A body that is one circular extrusion, as (the two ends of its axis, the
    radius), in metres in the world frame; None for any other body.

    The polygon a circle meshes to starts at the profile's reference direction, and
    Revit's 2027 save writes a pipe's extrusion from the other end or turned about
    its axis in 115 of rme_basic's 491 pipes (RE-346), so two saves' polygons of one
    cylinder can lie up to 0.0063 ft apart (15 of those pipes, over 0.001 ft)."""
    bodies = [
        item
        for rep in element.Representation.Representations
        if rep.RepresentationIdentifier == "Body"
        for item in rep.Items
    ]
    if len(bodies) != 1 or not bodies[0].is_a("IfcExtrudedAreaSolid"):
        return None
    solid = bodies[0]
    profile = solid.SweptArea
    if not profile.is_a("IfcCircleProfileDef") or profile.is_a("IfcCircleHollowProfileDef"):
        return None
    centre = [0.0, 0.0]
    if profile.Position is not None:
        centre = [float(c) for c in profile.Position.Location.Coordinates]
    direction = np.array(solid.ExtrudedDirection.DirectionRatios, dtype=float)
    direction /= np.linalg.norm(direction)
    first = np.array([centre[0], centre[1], 0.0, 1.0])
    last = first.copy()
    last[:3] += direction * float(solid.Depth)
    frame = np.array(ifcopenshell.util.placement.get_axis2placement(solid.Position), dtype=float)
    placed = placement(element, scale)
    ends = []
    for point in (first, last):
        local = (frame @ point)[:3] * scale
        ends.append(local if placed is None else placed[:3, :3] @ local + placed[:3, 3])
    return np.array(ends), float(profile.Radius) * scale


def cylinder_differences(a, b):
    """What differs between two cylinders, the axis ends taken in either order, as
    [(field, candidate's, earlier's)], at the same 0.001 ft as the vertices."""
    (ends_a, radius_a), (ends_b, radius_b) = a, b
    found = []
    straight = max(np.linalg.norm(ends_a[0] - ends_b[0]), np.linalg.norm(ends_a[1] - ends_b[1]))
    swapped = max(np.linalg.norm(ends_a[0] - ends_b[1]), np.linalg.norm(ends_a[1] - ends_b[0]))
    away = float(min(straight, swapped))
    if away > TOLERANCE:
        found.append(("axis ends", f"the farthest is {away / FEET:.4f} ft from the earlier save's", ""))
    if abs(radius_a - radius_b) > TOLERANCE:
        found.append(("radius", f"{radius_a / FEET:.4f} ft", f"{radius_b / FEET:.4f} ft"))
    length_a, length_b = np.linalg.norm(ends_a[1] - ends_a[0]), np.linalg.norm(ends_b[1] - ends_b[0])
    if abs(length_a - length_b) > TOLERANCE:
        found.append(("length", f"{length_a / FEET:.4f} ft", f"{length_b / FEET:.4f} ft"))
    return found


def differences(settings, scale, left, right, how=None):
    """What differs between a candidate product and its earlier save's, as
    [(field, candidate's, earlier's)]. A pipe that is a circular extrusion in both
    saves is held to the cylinder, not to the vertices of the polygon it meshes to
    (Griffin, 2026-11-05); `how` counts which of the two each body was compared by."""
    (a, a_source, a_group), (b, b_source, b_group) = left, right
    found = []
    for field, x, y in (
        ("class", a.is_a(), b.is_a()),
        ("GlobalId", a.GlobalId, b.GlobalId),
        ("storey", storey(a), storey(b)),
        ("BodySource", a_source, b_source),
        ("ProfileResolved", properties(a)[1], properties(b)[1]),
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
    cylinders = (cylinder(a, scale[0]), cylinder(b, scale[1])) if a_group == PIPES and b_group == PIPES else (None, None)
    if cylinders[0] is not None and cylinders[1] is not None:
        found += cylinder_differences(*cylinders)
        if how is not None:
            how["as cylinders"] += 1
        return found
    away = max(reach(ma[1], mb[1]), reach(mb[1], ma[1]))
    if away > TOLERANCE:
        found.append(("vertices", f"the farthest is {away / FEET:.4f} ft from the earlier save's", ""))
    if how is not None:
        how["by vertices"] += 1
    return found


def describe(settings, items):
    """What a save holds under a Tag: class, BodySource, ProfileResolved and volume."""
    if not items:
        return "no product with a body"
    parts = []
    for element, source, _ in items:
        resolved = properties(element)[1]
        shape = mesh(settings, element)
        volume = shape if isinstance(shape, str) else f"{shape[0]:.9g} m3, {len(shape[1])} vertices"
        parts.append(f"{element.is_a()} BodySource={source} ProfileResolved={resolved} volume={volume}")
    return "; ".join(parts)


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
    match = re.fullmatch(r"sample-(\d{4})-(.+)", parent)
    project = match.group(2) if match else None
    own_year = match.group(1) if match else "this"
    own_number = int(match.group(1)) if match else None
    named_tags = NAMED_TAGS.get(project, frozenset())
    failures = []
    not_compared = []

    def fail(text):
        failures.append(text)

    print(f"bodies across saves: {parent}, held to the {', '.join(str(y) for y in sorted(earlier_paths))} saves")
    if project not in MINIMUMS:
        fail(f"{parent!r} names no project this check knows ({', '.join(sorted(MINIMUMS))}), so nothing is known of what it should hold")
        report_and_exit(failures, not_compared)
    if not os.path.isfile(candidate_path):
        fail(f"this save's export is missing: {candidate_path}")
        report_and_exit(failures, not_compared)
    model, candidate = products(candidate_path, named_tags)
    scale_candidate = ifcopenshell.util.unit.calculate_unit_scale(model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    earlier = {}
    for year, path in sorted(earlier_paths.items()):
        if not os.path.isfile(path):
            fail(f"the {year} save's export is missing: {path}")
            continue
        earlier[year] = products(path, named_tags)
    total = 0
    minimums = MINIMUMS.get(project, {})
    required = REQUIRED.get(project, {}).get(own_number, {})
    outline_least = OUTLINE_LEAST.get(project, {}).get(own_number, 0)
    for tag in sorted(named_tags | PRINT_TAGS.get(project, frozenset())):
        print(f"named {tag}, {own_year} save: {describe(settings, candidate.get(tag))}")
        for year, (_, reference) in sorted(earlier.items()):
            print(f"named {tag}, {year} save: {describe(settings, reference.get(tag))}")
    for group in (BEAMS_AND_LANDINGS, STAIR_RUNS, SKETCH_OUTLINES, NAMED_ELEMENTS, PIPES, DOORS_AND_WINDOWS):
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
        if own_number is not None and year >= own_number:
            print(f"{group}: not applicable, the {year} save is the reference of the later saves and this is the {own_year} save ({len(chosen)} drawn here)")
            continue
        if group == DOORS_AND_WINDOWS:
            by_class = collections.Counter(item[0].is_a() for items in chosen.values() for item in items)
            for cls, least in OPENINGS_LEAST.get(project, {}).get(own_number, {}).items():
                if by_class.get(cls, 0) < least:
                    fail(f"{group}: {by_class.get(cls, 0)} {cls} placed by the rule, fewer than the {least} it holds in the {own_year} save")
        if year not in earlier_paths:
            not_compared.append(f"{group} ({len(chosen)} here; the {year} save is not in this run)")
            print(f"{group}: not compared, the {year} save is not in this run ({len(chosen)} drawn here)")
            continue
        if year not in earlier:
            print(f"{group}: not compared, the {year} save's export is missing")
            continue
        earlier_model, reference = earlier[year]
        scale_earlier = ifcopenshell.util.unit.calculate_unit_scale(earlier_model)
        compared = collections.Counter()
        how = collections.Counter()
        off_axes = 0
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
            if len(chosen[tag]) > 1:
                # Of the products Revit writes under one Tag only the first keeps the
                # element's own GlobalId; the others' are derived from the export and
                # differ between saves. So the saves share exactly one, and it is on
                # the two products paired by what they are: an identity swapped between
                # two pieces (or none kept) fails here.
                shared = {item[0].GlobalId for item in chosen[tag]} & {item[0].GlobalId for item in selected}
                kept = sum(1 for left, right in zip(chosen[tag], selected) if left[0].GlobalId == right[0].GlobalId)
                if len(shared) != 1 or kept != 1:
                    ok = False
                    fail(
                        f"{group}: {tag} is {len(chosen[tag])} products here; {len(shared)} GlobalId(s) shared with the {year} save, "
                        f"{kept} of them on the pieces paired by storey, volume and centre, where exactly one of each is expected"
                    )
            for left, right in zip(chosen[tag], selected):
                for field, mine, theirs in differences(settings, (scale_candidate, scale_earlier), left, right, how):
                    if field == "GlobalId" and len(chosen[tag]) > 1:
                        continue
                    ok = False
                    detail = f"{mine} here, {theirs} in the {year} save" if theirs != "" else mine
                    fail(f"{group}: {tag} {left[0].is_a()} differs in {field}: {detail}")
                compared[left[0].is_a()] += 1
                if group == DOORS_AND_WINDOWS:
                    frame = placement(left[0], scale_candidate)
                    off_axes += frame is None or not np.allclose(frame[:3, :3], np.eye(3), atol=ANGLE_TOLERANCE)
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
        if group == DOORS_AND_WINDOWS:
            most = OPENINGS_LEFT_OUT_MOST.get(project, {}).get(own_number)
            if most is not None and sum(left_out.values()) > most:
                fail(
                    f"{group}: {sum(left_out.values())} placed by the rule in the {year} save and left on the default axes "
                    f"in the {own_year} save, more than the {most} they are left"
                )
        for tag in sorted(required.get(group, ())):
            if tag not in compared_tags:
                fail(f"{group}: {tag} was not compared equal, and is required")
        if group == SKETCH_OUTLINES and sum(compared.values()) < outline_least:
            fail(f"{group}: {sum(compared.values())} compared, fewer than the {outline_least} it holds")
        if group == SKETCH_OUTLINES and project in ARC_OWNERS:
            arcs = ARC_OWNERS[project]
            drawn = sorted(arcs & compared_tags, key=int)
            print(
                f"{group}: of the {len(arcs)} elements whose sketch holds an arc, {len(drawn)} are outlines compared equal here"
                f" ({', '.join(drawn) or 'none'}); the other {len(arcs) - len(drawn)} are boxes here or not outlines"
            )
        print(
            f"{group}: compared {sum(compared.values())} against the {year} save ({', '.join(f'{c} {n}' for c, n in sorted(compared.items())) or 'none'}); "
            f"left as boxes here, drawn in the {year} save, not scored: {sum(left_out.values())}"
            + (f" ({', '.join(f'{c} {n}' for c, n in sorted(left_out.items()))})" if left_out else "")
            + (f"; of those compared, placed on axes other than the default: {off_axes}" if group == DOORS_AND_WINDOWS else "")
        )
        if group == PIPES:
            print(
                f"{group}: {how['as cylinders']} compared as cylinders (axis ends in either order, radius and length to 0.001 ft), "
                f"{how['by vertices']} by vertices"
            )
            least = CYLINDER_LEAST.get(project, {}).get(own_number, 0)
            if how["as cylinders"] < least:
                fail(f"{group}: {how['as cylinders']} compared as cylinders, fewer than the {least} it holds")
        other = OTHER_YEAR.get(group)
        if other in earlier and other != own_number and chosen:
            against_other(settings, (scale_candidate, ifcopenshell.util.unit.calculate_unit_scale(earlier[other][0])), chosen, earlier[other][1], group, other)
    if own_number is not None and own_number <= READERS_YEAR:
        print(f"cut openings, ports and properties: not applicable, the {READERS_YEAR} save is the reference of the later saves and this is the {own_year} save")
    elif READERS_YEAR in earlier:
        import readers_across_saves

        total += readers_across_saves.check(
            project,
            own_number or 0,
            candidate_path,
            model,
            earlier_paths[READERS_YEAR],
            earlier[READERS_YEAR][0],
            (scale_candidate, ifcopenshell.util.unit.calculate_unit_scale(earlier[READERS_YEAR][0])),
            settings,
            fail,
        )
    elif READERS_YEAR not in earlier_paths:
        not_compared.append(f"cut openings, ports and properties (the {READERS_YEAR} save is not in this run)")
    if total == 0:
        fail("nothing was compared")
    report_and_exit(failures, not_compared)


def against_other(settings, scale, chosen, reference, group, year):
    """Print, without scoring, how the candidate's `group` elements compare
    with the other earlier save's: equal, different, or drawn otherwise there.
    Where the two earlier saves disagree, the answer is in these lines."""
    equal, differ, otherwise = 0, [], 0
    for tag in sorted(chosen, key=lambda t: (0, int(t)) if t.isdigit() else (1, t)):
        selected = [item for item in reference.get(tag, []) if item[2] == group]
        if len(selected) != len(chosen[tag]):
            otherwise += 1
            continue
        found = []
        for left, right in zip(chosen[tag], selected):
            found += [(left[0].is_a(), *field) for field in differences(settings, scale, left, right)]
        if found:
            differ.append((tag, found))
        else:
            equal += 1
    print(f"{group} against the {year} save, not scored: {equal} equal, {len(differ)} differ, {otherwise} drawn otherwise there")
    for tag, found in differ[:10]:
        print(f"  {group} against the {year} save: {tag} " + "; ".join(f"{cls} {field}: {mine} here" + (f", {theirs} there" if theirs else "") for cls, field, mine, theirs in found))


def report_and_exit(failures, not_compared):
    for line in failures[:SHOW]:
        print(f"  {line}")
    if len(failures) > SHOW:
        print(f"  ... and {len(failures) - SHOW} more")
    if failures:
        print(f"bodies across saves: FAIL ({len(failures)} differences)")
        sys.exit(1)
    if not_compared:
        print(f"bodies across saves: PARTIAL (not compared: {'; '.join(not_compared)})")
        sys.exit(0)
    print("bodies across saves: PASS")
    sys.exit(0)


if __name__ == "__main__":
    sys.exit(main())
