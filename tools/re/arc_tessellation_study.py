#!/usr/bin/env python3
"""How IfcOpenShell cuts the arcs rvt-rs writes against the circles Revit's
export writes, and what each drawing costs the "ref on ours" bar (#1346,
RE-327 section 11).

It writes small STEP files in the shape rvt-rs writes (metres, one
`IfcExtrudedAreaSolid` over an `IfcIndexedPolyCurve` of `IfcArcIndex`
segments) and meshes them with the scorers' own settings and arithmetic
(`mesh_bodies_vs_ifc`), so it needs no rvt-rs binary. Three tables:

1. the chords IfcOpenShell cuts a whole circle into, by radius, for an
   `IfcCircleProfileDef` (Revit's form) and for rvt-rs's two half arcs and four
   quarter arcs;
2. by radius, the worst "Revit's vertices to rvt-rs's surface" and "rvt-rs's
   vertices to Revit's" over Revit's circle written as an `IfcCircleProfileDef`
   at four reference directions and as half and quarter arcs at the same four
   starts, for rvt-rs's half arcs and for short arcs over 0.12 ft (arcs whose
   chords lie within 0.00025 ft of the circle, which the first head of #1346
   wrote and `family_extrusions.rs` does not: they hold "ref on ours" and give
   up the coincidence that IFC4 exports of Revit 2024 have with half arcs);
3. with a Revit export that holds RE1 Architecture's toilet-paper dispenser
   (element 360331), that element redrawn the ways rvt-rs could draw it and
   scored against Revit's own body.

Needs IfcOpenShell and numpy.

Then the same circles at the fine deflection of fine_mesh.py (#1355, RE-331
section 2), which the body scorers print beside the default one when
RVT_SCORER_FINE_MESH=1: the chords, by radius, of the two forms of a circle at
both deflections; the worst "Revit's vertices to rvt-rs's surface" and
"rvt-rs's to Revit's" of the two forms of one circle, at the same start and
over four starts, at both; and, with a Revit export, the dispenser redrawn the
ways above at both. With `--write-ours FILE` it also writes an IFC of the shape
rvt-rs writes (the dispenser's two half arcs per circle, BodySource
family_extrusions, Tag 360331) that `family_extrusion_bodies_vs_ifc.py` scores
against the Revit export, so the scorer's own fine rows are reproducible
without an rvt-rs binary.

Usage:

    python3 tools/re/arc_tessellation_study.py [RE1-Architecture.ifc [--write-ours FILE]] [--deflections]

`--deflections` adds the table that chose fine_mesh.py's values: for pairs of
linear and angular deflections, the mean time to mesh one circle body and the
worst miss between the two forms of a circle, over radii from 0.229 to 2 ft.
"""

import argparse
import math
import os
import sys
import tempfile
import time

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fine_mesh  # noqa: E402
import mesh_bodies_vs_ifc as M  # noqa: E402

FEET = 0.3048
SHORT_ARC_SAGITTA = 2.5e-4
TRUE_ARC_MAX_RADIUS = 0.12

HEAD = """ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('x.ifc','2023-11-14T22:13:20',('a'),('b'),'c','d','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPERSON($,$,'a',$,$,$,$,$);
#2=IFCORGANIZATION($,'a','b',$,$);
#3=IFCPERSONANDORGANIZATION(#1,#2,$);
#4=IFCAPPLICATION(#2,'1','a','a');
#5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);
#6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#7=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#8=IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.);
#9=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#12=IFCUNITASSIGNMENT((#6,#7,#8,#9));
#13=IFCCARTESIANPOINT((0.,0.,0.));
#14=IFCDIRECTION((0.,0.,1.));
#15=IFCDIRECTION((1.,0.,0.));
#16=IFCAXIS2PLACEMENT3D(#13,#14,#15);
#17=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,#16,$);
#18=IFCPROJECT('2DidPPITwEWgzLwN9h$v59',#5,'p',$,$,$,$,(#17),#12);
#19=IFCLOCALPLACEMENT($,#16);
#20=IFCSITE('3XwUud8$_6nea1TZT81Dk5',#5,'Default Site',$,$,#19,$,'Default Site',.ELEMENT.,$,$,$,$,$);
"""


def file_text(lines, items, body_source=None, place=None):
    """A file with one proxy whose body is the extruded solids `items` (entity ids). With `body_source`
    the proxy carries the BodySource property rvt-rs writes; with `place` (location, axis, reference
    direction, in metres) it is placed there, else at the origin."""
    placement = ["#40=IFCLOCALPLACEMENT(#19,#16);"]
    if place:
        placement = [
            "#46=IFCCARTESIANPOINT((%.9f,%.9f,%.9f));" % tuple(place[0]),
            "#47=IFCDIRECTION((%.9f,%.9f,%.9f));" % tuple(place[1]),
            "#48=IFCDIRECTION((%.9f,%.9f,%.9f));" % tuple(place[2]),
            "#49=IFCAXIS2PLACEMENT3D(#46,#47,#48);",
            "#40=IFCLOCALPLACEMENT(#19,#49);",
        ]
    source = []
    if body_source:
        source = [
            "#42=IFCPROPERTYSINGLEVALUE('BodySource',$,IFCLABEL('%s'),$);" % body_source,
            "#43=IFCPROPERTYSET('1HSJvhRXn1xfRIeN8FFDt6',#5,'Pset_RvtRs',$,(#42));",
            "#44=IFCRELDEFINESBYPROPERTIES('2HSJvhRXn1xfRIeN8FFDt7',#5,$,$,(#41),#43);",
        ]
    tail = [
        "#38=IFCSHAPEREPRESENTATION(#17,'Body','SweptSolid',(%s));" % ",".join(f"#{i}" for i in items),
        "#39=IFCPRODUCTDEFINITIONSHAPE($,$,(#38));",
        *placement,
        "#41=IFCBUILDINGELEMENTPROXY('3144CP5Ub2qhBtGD1R3i$t',#5,'x',$,$,#40,#39,'360331',$);",
        *source,
        "ENDSEC;",
        "END-ISO-10303-21;",
    ]
    return HEAD + "\n".join(lines + tail) + "\n"


def arcs_of_circle(radius, start, pieces):
    """Points and 1-based `IfcArcIndex` triples of a circle cut into `pieces` equal arcs."""
    step = 2 * math.pi / pieces
    at = lambda a: (radius * math.cos(a), radius * math.sin(a))  # noqa: E731
    points, segments, current = [at(start)], [], 0
    for k in range(pieces):
        a0 = start + step * k
        points.append(at(a0 + step / 2))
        middle = len(points) - 1
        if k == pieces - 1:
            last = 0
        else:
            points.append(at(a0 + step))
            last = len(points) - 1
        segments.append(f"IFCARCINDEX(({current + 1},{middle + 1},{last + 1}))")
        current = last
    return points, segments


def short_arc_pieces(radius_feet):
    """Arcs for a whole circle: two up to 0.12 ft, else short arcs."""
    if radius_feet <= TRUE_ARC_MAX_RADIUS:
        return 2
    return math.ceil(2 * math.pi / (2 * math.acos(1 - SHORT_ARC_SAGITTA / radius_feet)))


class Builder:
    """Entity lines for solids, with ids from 101."""

    def __init__(self):
        self.lines, self.items, self.next_id = [], [], 100
        self.body_source, self.place = None, None

    def add(self, text):
        self.next_id += 1
        self.lines.append(f"#{self.next_id}={text};")
        return self.next_id

    def curve_profile(self, points, segments, center=(0.0, 0.0)):
        coords = ",".join(f"({x + center[0]:.6f},{y + center[1]:.6f})" for x, y in points)
        lst = self.add(f"IFCCARTESIANPOINTLIST2D(({coords}))")
        curve = self.add(f"IFCINDEXEDPOLYCURVE(#{lst},({','.join(segments)}),.F.)")
        return self.add(f"IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#{curve})")

    def circle_profile(self, radius, phase=0.0, center=(0.0, 0.0)):
        origin = self.add(f"IFCCARTESIANPOINT(({center[0]:.6f},{center[1]:.6f}))")
        direction = self.add(f"IFCDIRECTION(({math.cos(phase):.9f},{math.sin(phase):.9f}))")
        place = self.add(f"IFCAXIS2PLACEMENT2D(#{origin},#{direction})")
        return self.add(f"IFCCIRCLEPROFILEDEF(.AREA.,$,#{place},{radius:.6f})")

    def rectangle_profile(self, x_dim, y_dim, center=(0.0, 0.0)):
        hx, hy = x_dim / 2, y_dim / 2
        points = [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)]
        lines = [f"IFCLINEINDEX(({a},{b}))" for a, b in ((1, 2), (2, 3), (3, 4), (4, 1))]
        return self.curve_profile(points, lines, center)

    def solid(self, profile, depth, location=(0.0, 0.0, 0.0), axis=(0.0, 0.0, 1.0), ref=(1.0, 0.0, 0.0)):
        point = self.add("IFCCARTESIANPOINT((%.6f,%.6f,%.6f))" % tuple(location))
        z = self.add("IFCDIRECTION((%.9f,%.9f,%.9f))" % tuple(axis))
        x = self.add("IFCDIRECTION((%.9f,%.9f,%.9f))" % tuple(ref))
        place = self.add(f"IFCAXIS2PLACEMENT3D(#{point},#{z},#{x})")
        up = self.add("IFCDIRECTION((0.,0.,1.))")
        self.items.append(self.add(f"IFCEXTRUDEDAREASOLID(#{profile},#{place},#{up},{depth:.6f})"))

    def text(self):
        return file_text(self.lines, self.items, self.body_source, self.place)


def triangles(text, path, fine=False):
    """The body of the file's one proxy as triangles (T, 3, 3) in feet, as the scorers read it."""
    with open(path, "w") as out:
        out.write(text)
    model = ifcopenshell.open(path)
    if fine:
        settings = fine_mesh.settings()
    else:
        settings = ifcopenshell.geom.settings()
        settings.set("use-world-coords", True)
    shape = ifcopenshell.geom.create_shape(settings, model.by_type("IfcBuildingElementProxy")[0])
    vertices = np.array(shape.geometry.verts).reshape(-1, 3) / FEET
    return vertices[np.array(shape.geometry.faces).reshape(-1, 3)]


def score(ours, ref):
    """(volume ratio, Revit's vertices to rvt-rs's surface, rvt-rs's to Revit's), feet."""
    ratio = M.volume(ours) / M.volume(ref)
    return ratio, M.surface_distance(M.sample(ref), ours), M.surface_distance(M.sample(ours), ref)


def circle_body(radius, form, phase, pieces=2):
    builder = Builder()
    if form == "circle":
        profile = builder.circle_profile(radius, phase)
    else:
        points, segments = arcs_of_circle(radius, phase, pieces)
        profile = builder.curve_profile(points, segments)
    builder.solid(profile, 0.1)
    return builder.text()


def chord_counts(scratch):
    print("Chords IfcOpenShell cuts a whole circle into (rvt-rs's shape of file, metres):")
    print("  radius mm | radius ft | IfcCircleProfileDef | 2 half arcs | 4 quarter arcs | sagitta ft: circle, half arcs")
    for mm in (6.35, 12.7, 25.4, 41.8, 50.0, 69.85, 100.0, 150.0, 300.0, 600.0):
        r = mm / 1000.0
        counts = []
        for form, pieces in (("circle", 0), ("arcs", 2), ("arcs", 4)):
            v = triangles(circle_body(r, form, 0.0, pieces), scratch).reshape(-1, 3)
            bottom = v[np.abs(v[:, 2] - v[:, 2].min()) < 1e-9]
            counts.append(len(np.unique(np.round(bottom, 6), axis=0)))
        sagitta = lambda n: r / FEET * (1 - math.cos(math.pi / n))  # noqa: E731
        print(
            f"  {mm:9.2f} | {r / FEET:9.4f} | {counts[0]:19d} | {counts[1]:11d} | {counts[2]:14d} |"
            f" {sagitta(counts[0]):.5f}, {sagitta(counts[1]):.5f}"
        )


def worst_cases(scratch):
    print()
    print("Worst case over Revit's circle as IfcCircleProfileDef, half arcs and quarter arcs, each at four starts")
    print("(0, 90, 7.3 and 180 degrees), against rvt-rs's half arcs and against short arcs over 0.12 ft:")
    print("  radius ft | half arcs: Revit's vertices to ours, ours to Revit's, volume ratio | short arcs over 0.12 ft: same, arcs written")
    for feet in (0.02, 0.05, 0.08, 0.10, 0.12, 0.125, 0.14, 0.20, 0.30, 0.50, 1.00, 2.00):
        r = feet * FEET
        oracles = []
        for phase in (0.0, math.pi / 2, math.radians(7.3), math.pi):
            for form, pieces in (("circle", 0), ("arcs", 2), ("arcs", 4)):
                oracles.append(triangles(circle_body(r, form, phase, pieces), scratch))
        pieces = short_arc_pieces(feet)
        row = []
        for written in (2, pieces):
            ours = triangles(circle_body(r, "arcs", 0.0, written), scratch)
            scores = [score(ours, ref) for ref in oracles]
            row.append(
                f"{max(s[1] for s in scores):.5f} {max(s[2] for s in scores):.5f}"
                f" {min(s[0] for s in scores):.5f}..{max(s[0] for s in scores):.5f}"
            )
        print(f"  {feet:9.3f} | {row[0]} | {row[1]} ({pieces} arcs)")


FORMS = (("circle", 0), ("arcs", 2), ("arcs", 4))
STARTS = (0.0, math.pi / 2, math.radians(7.3), math.pi)
FINE_MM = (6.35, 25.4, 41.8, 50.0, 69.85, 80.0, 100.0, 150.0, 300.0, 600.0)


def bottom_count(triangle_set):
    v = triangle_set.reshape(-1, 3)
    bottom = v[np.abs(v[:, 2] - v[:, 2].min()) < 1e-9]
    return len(np.unique(np.round(bottom, 6), axis=0))


def fine_chord_counts(scratch):
    print()
    print(f"Fine tessellation ({fine_mesh.LABEL}); chords of a whole circle, default and fine:")
    print("  radius mm | radius ft | IfcCircleProfileDef | 2 half arcs | 4 quarter arcs (default, fine each)")
    for mm in FINE_MM:
        r = mm / 1000.0
        cells = []
        for form, pieces in FORMS:
            counts = [bottom_count(triangles(circle_body(r, form, 0.0, pieces), scratch, fine)) for fine in (False, True)]
            cells.append(f"{counts[0]:5d}, {counts[1]:5d}")
        print(f"  {mm:9.2f} | {r / FEET:9.4f} | " + " | ".join(cells))


def fine_worst_cases(scratch):
    print()
    print("The two forms of one circle, rvt-rs's two half arcs against Revit's circle, default and fine (feet):")
    print("  same start: Revit's IfcCircleProfileDef, start 0; worst: Revit's circle as the three forms at four starts (0, 90, 7.3, 180 degrees)")
    print("  radius mm | radius ft | same start, default: Revit's vertices to ours, ours to Revit's, volume ratio | same start, fine | worst, default | worst, fine")
    for mm in FINE_MM:
        r = mm / 1000.0
        cells = []
        for fine in (False, True):
            halves = triangles(circle_body(r, "arcs", 0.0, 2), scratch, fine)
            same = score(halves, triangles(circle_body(r, "circle", 0.0), scratch, fine))
            scores = [
                score(halves, triangles(circle_body(r, form, phase, pieces), scratch, fine))
                for phase in STARTS
                for form, pieces in FORMS
            ]
            cells.append((same, max(x[1] for x in scores), max(x[2] for x in scores)))
        default, fine = cells
        print(
            f"  {mm:9.2f} | {r / FEET:9.4f} | {default[0][1]:.6f} {default[0][2]:.6f} {default[0][0]:.5f}"
            f" | {fine[0][1]:.6f} {fine[0][2]:.6f} {fine[0][0]:.5f}"
            f" | {default[1]:.6f} {default[2]:.6f} | {fine[1]:.6f} {fine[2]:.6f}"
        )


def deflection_choice(scratch):
    print()
    print("Deflections tried (IfcOpenShell mesher-linear-deflection in metres, mesher-angular-deflection in radians):")
    print("the mean time to mesh one circle body, and over radii of 0.229, 0.2625, 0.5, 1 and 2 ft the worst miss between")
    print("rvt-rs's two half arcs and Revit's circle as the three forms at four starts, in feet")
    print("  linear m | angular rad | mean s per body | worst Revit's vertices to ours, ours to Revit's")
    radii = (0.229, 0.2625, 0.5, 1.0, 2.0)
    for linear, angular in (
        (1e-5, 0.01), (1e-5, 0.02), (1e-5, 0.03), (1e-5, 0.05),
        (1e-5 * FEET, 0.02), (1e-5 * FEET, 0.03), (1e-5 * FEET, 0.04),
    ):  # fmt: skip
        settings = ifcopenshell.geom.settings()
        settings.set("use-world-coords", True)
        settings.set("mesher-linear-deflection", linear)
        settings.set("mesher-angular-deflection", angular)
        spent, bodies, to_ours, to_ref = 0.0, 0, 0.0, 0.0
        for feet in radii:
            r = feet * FEET

            def mesh(text):
                nonlocal spent, bodies
                with open(scratch, "w") as out:
                    out.write(text)
                model = ifcopenshell.open(scratch)
                started = time.perf_counter()
                shape = ifcopenshell.geom.create_shape(settings, model.by_type("IfcBuildingElementProxy")[0])
                spent += time.perf_counter() - started
                bodies += 1
                vertices = np.array(shape.geometry.verts).reshape(-1, 3) / FEET
                return vertices[np.array(shape.geometry.faces).reshape(-1, 3)]

            ours = mesh(circle_body(r, "arcs", 0.0, 2))
            for phase in STARTS:
                for form, pieces in FORMS:
                    scored = score(ours, mesh(circle_body(r, form, phase, pieces)))
                    to_ours, to_ref = max(to_ours, scored[1]), max(to_ref, scored[2])
        print(f"  {linear:.4g} | {angular:g} | {spent / bodies:.3f} | {to_ours:.6f} {to_ref:.6f}")


def dispenser(revit_path, scratch, ours_out=None):
    model = ifcopenshell.open(revit_path)
    element = next(e for e in model.by_type("IfcElement") if e.Tag == "360331")
    solids = []

    def walk(representation):
        for item in representation.Items:
            if item.is_a("IfcMappedItem"):
                walk(item.MappingSource.MappedRepresentation)
            else:
                solids.append(item)

    for representation in element.Representation.Representations:
        if representation.RepresentationIdentifier == "Body":
            walk(representation)
    ref_by_fine = {}
    for fine in (False, True):
        settings = fine_mesh.settings() if fine else ifcopenshell.geom.settings()
        ref = []
        for item in solids:
            shape = ifcopenshell.geom.create_shape(settings, item)
            vertices = np.array(shape.verts).reshape(-1, 3) / FEET
            ref.append(vertices[np.array(shape.faces).reshape(-1, 3)])
        ref_by_fine[fine] = np.concatenate(ref)
    ref = ref_by_fine[False]
    kinds = [item.SweptArea.is_a() for item in solids]
    print()
    print(f"Element 360331 in {os.path.basename(revit_path)}: {len(solids)} extrusions, profiles {kinds}")

    def redraw(circles, turn=0.0, tagged=False):
        builder = Builder()
        if tagged:
            matrix = ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement)
            unit = ifcopenshell.util.unit.calculate_unit_scale(model)
            builder.body_source = "family_extrusions"
            builder.place = (matrix[:3, 3] * unit, matrix[:3, 2], matrix[:3, 0])
        for item in solids:
            position = item.Position
            location = [c / 1000.0 for c in position.Location.Coordinates]
            axis = list(position.Axis.DirectionRatios if position.Axis else (0.0, 0.0, 1.0))
            ref_direction = np.array(position.RefDirection.DirectionRatios if position.RefDirection else (1.0, 0.0, 0.0))
            profile = item.SweptArea
            depth = item.Depth / 1000.0
            if profile.is_a("IfcRectangleProfileDef"):
                centre = [c / 1000.0 for c in profile.Position.Location.Coordinates]
                builder.solid(builder.rectangle_profile(profile.XDim / 1000.0, profile.YDim / 1000.0, centre), depth, location, axis, ref_direction)
                continue
            radius = profile.Radius / 1000.0
            centre = [c / 1000.0 for c in profile.Position.Location.Coordinates]
            if turn:
                unit = np.array(axis) / np.linalg.norm(axis)
                ref_direction = ref_direction * math.cos(turn) + np.cross(unit, ref_direction) * math.sin(turn)
            if circles == "circle":
                shape = builder.circle_profile(radius, 0.0, centre)
            else:
                pieces = short_arc_pieces(radius / FEET) if circles == "rule" else 2
                points, segments = arcs_of_circle(radius, 0.0, pieces)
                shape = builder.curve_profile(points, segments, centre)
            builder.solid(shape, depth, location, axis, ref_direction)
        return builder.text()

    print("  drawing | volume ratio | Revit's vertices to rvt-rs's surface | rvt-rs's to Revit's  (feet)")
    for name, circles, turn in (
        ("two half arcs per circle (the first run)", "arcs", 0.0),
        ("arcs up to 0.12 ft, short arcs over", "rule", 0.0),
        ("IfcCircleProfileDef, reference direction as Revit's", "circle", 0.0),
        ("IfcCircleProfileDef, turned 90 degrees", "circle", math.pi / 2),
        ("IfcCircleProfileDef, turned 180 degrees", "circle", math.pi),
    ):
        ratio, to_ours, to_ref = score(triangles(redraw(circles, turn), scratch), ref)
        print(f"  {name} | {ratio:.5f} | {to_ours:.6f} | {to_ref:.6f}")
    print()
    print(f"  fine tessellation ({fine_mesh.LABEL}), both bodies:")
    print("  drawing | volume ratio | Revit's vertices to rvt-rs's surface | rvt-rs's to Revit's  (feet)")
    for name, circles, turn in (
        ("two half arcs per circle", "arcs", 0.0),
        ("IfcCircleProfileDef, reference direction as Revit's", "circle", 0.0),
        ("IfcCircleProfileDef, turned 90 degrees", "circle", math.pi / 2),
        ("IfcCircleProfileDef, turned 180 degrees", "circle", math.pi),
    ):
        ratio, to_ours, to_ref = score(triangles(redraw(circles, turn), scratch, True), ref_by_fine[True])
        print(f"  {name} | {ratio:.5f} | {to_ours:.6f} | {to_ref:.6f}")
    if ours_out:
        with open(ours_out, "w") as out:
            out.write(redraw("arcs", 0.0, tagged=True))
        print()
        print(f"  wrote {ours_out}: the half arcs, BodySource family_extrusions, Tag 360331, placed as Revit's element is")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("revit", nargs="?", help="a Revit export that holds RE1 Architecture's element 360331")
    parser.add_argument("--write-ours", metavar="FILE", help="with a Revit export, write rvt-rs's shape of that element to FILE")
    parser.add_argument("--deflections", action="store_true", help="also print the deflections tried for fine_mesh.py")
    args = parser.parse_args()
    if args.write_ours and not args.revit:
        parser.error("--write-ours needs a Revit export")
    with tempfile.TemporaryDirectory() as scratch:
        path = os.path.join(scratch, "study.ifc")
        chord_counts(path)
        worst_cases(path)
        fine_chord_counts(path)
        fine_worst_cases(path)
        if args.deflections:
            deflection_choice(path)
        if args.revit:
            dispenser(args.revit, path, args.write_ours)
    return 0


if __name__ == "__main__":
    sys.exit(main())
