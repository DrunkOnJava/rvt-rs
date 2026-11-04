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

Usage:

    python3 tools/re/arc_tessellation_study.py [RE1-Architecture.ifc]
"""

import math
import os
import sys
import tempfile

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
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


def file_text(lines, items):
    """A file with one proxy whose body is the extruded solids `items` (entity ids)."""
    tail = [
        "#38=IFCSHAPEREPRESENTATION(#17,'Body','SweptSolid',(%s));" % ",".join(f"#{i}" for i in items),
        "#39=IFCPRODUCTDEFINITIONSHAPE($,$,(#38));",
        "#40=IFCLOCALPLACEMENT(#19,#16);",
        "#41=IFCBUILDINGELEMENTPROXY('3144CP5Ub2qhBtGD1R3i$t',#5,'x',$,$,#40,#39,'360331',$);",
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
        return file_text(self.lines, self.items)


def triangles(text, path):
    """The body of the file's one proxy as triangles (T, 3, 3) in feet, as the scorers read it."""
    with open(path, "w") as out:
        out.write(text)
    model = ifcopenshell.open(path)
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


def dispenser(revit_path, scratch):
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
    settings = ifcopenshell.geom.settings()
    ref = []
    for item in solids:
        shape = ifcopenshell.geom.create_shape(settings, item)
        vertices = np.array(shape.verts).reshape(-1, 3) / FEET
        ref.append(vertices[np.array(shape.faces).reshape(-1, 3)])
    ref = np.concatenate(ref)
    kinds = [item.SweptArea.is_a() for item in solids]
    print()
    print(f"Element 360331 in {os.path.basename(revit_path)}: {len(solids)} extrusions, profiles {kinds}")

    def redraw(circles, turn=0.0):
        builder = Builder()
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


def main():
    if len(sys.argv) > 2 or (len(sys.argv) == 2 and sys.argv[1] in ("-h", "--help")):
        print(__doc__)
        return 2
    with tempfile.TemporaryDirectory() as scratch:
        path = os.path.join(scratch, "study.ifc")
        chord_counts(path)
        worst_cases(path)
        if len(sys.argv) == 2:
            dispenser(sys.argv[1], path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
