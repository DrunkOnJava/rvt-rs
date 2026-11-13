#!/usr/bin/env python3
"""How long Revit's beam is beside the record box and the column-cut bounds (#1839, RE-122).

Reads `probes/examples/probe_1839_beam_column_ends.rs`'s output and Revit's IFC export. Each
beam of the probe is paired by `Tag` with Revit's element, which is meshed in its own placement's
frame (as `box_bodies_vs_ifc.py` does); its length is the longest side of that mesh's box. It is
set beside

    record   the record box's length along its longer plan axis (the line's ends),
    trimmed  the bounds RE-122's rule gives when each end that exactly one named column holds is
             cut back to the column's near face (`beam_column_trims`), where it gives any.

and the beams are counted by how Revit's length stands to the record's (the same within 1 mm,
shorter, longer), by whether the rule gives bounds, and by how many named columns hold each end.
For a beam Revit makes shorter than its record box and that the rule cuts: exact (the trimmed
length is Revit's within 1 mm), cut too far, or cut too little. For a beam the rule cuts and that
Revit leaves at the record: cut where Revit does not. The beams Revit shortens that the rule gives
no bounds are listed by what they name. Beams whose box is square in plan (no longer axis) are
counted apart.

The same is asked of each kind of element a beam's list can name (`column`, `wall`, `beam`), whose
bounds the probe prints as `<kind>_low` and `<kind>_high`: the near face of the one element of that
kind that holds the end, where it lies more than 1 mm inside the box. `section` is the columns again,
held and cut not by their record box but by the section the export draws for them, along the
beam's centre line (the probe's `section_*`). The beams Revit makes shorter
are counted by whether they name an element of the kind, whether one holds an end, whether the
kind gives them bounds and, where it does, whether the cut length is Revit's within 1 mm. The beams
Revit leaves at the record or makes longer are counted by whether the kind cuts them, which it must
not.

What stops the beams Revit shortens is asked of Revit's own export as well: each beam's mesh is
taken in the world frame, its two end caps are found as the ends of its principal axis, and the
centre of each cap is looked up among the faces of Revit's columns, beams and members, walls and
slabs. An end lies on an element where it is within 1 mm of the plane of one of its triangles and
inside that triangle (the element's true surface, not its record box). "Inside" has a slack of a
thousandth of the triangle's height past any edge, which is 1 cm on a triangle 10 m high, so a
large wall or slab can count an end that is a little past its edge: the count says where an end is
near a face, not that the two touch. The ends of the beams Revit makes shorter, of those it leaves
at the record length and of those it makes longer are counted by the kind of element they lie on
(column first, then beam, wall, slab), or on nothing.

Every summary line starts `beam ends:` and holds counts beside fixed words; `--list` adds one line
per beam, which names an ElementId.

Usage:

    python3 tools/re/beam_ends_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Needs IfcOpenShell.
"""

import collections
import math
import multiprocessing
import statistics
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy

TOLERANCE_FEET = 0.0033  # 1 mm
METRES_FEET = 1 / 0.3048
KINDS = ("column", "wall", "beam", "section")
# The elements whose faces a beam's end may lie on, by the IFC class that first matches, and the
# kind each is counted as, in the order a lying-on is reported when an end lies on several.
FACES = (
    ("IfcColumn", "column"),
    ("IfcBeam", "beam"),
    ("IfcMember", "beam"),
    ("IfcWall", "wall"),
    ("IfcSlab", "slab"),
    ("IfcFooting", "slab"),
    ("IfcPlate", "slab"),
)
FACE_KINDS = ("column", "beam", "wall", "slab")
TOLERANCE_UNITS = [0.0]  # 1 mm in the file's mesh unit, set once the unit is known


def faced_length(fields, kind, low, high):
    """The beam's length with each end that exactly one element of `kind` holds cut back to that
    element's near face, or None where the kind gives it no bounds."""
    cut_low, cut_high = fields.get(f"{kind}_low", "-"), fields.get(f"{kind}_high", "-")
    if cut_low == "-" and cut_high == "-":
        return None
    start = low if cut_low == "-" else float(cut_low)
    end = high if cut_high == "-" else float(cut_high)
    return end - start if end > start else None


def cut_by(fields, kind):
    """One `--list` cell: the length `kind` cuts the beam to, and how many it names and holds."""
    length = faced_length(fields, kind, float(fields["low"]), float(fields["high"]))
    shown = "-" if length is None else f"{length:.3f}"
    named = fields.get(f"{kind}_named", "0")
    held = fields.get(f"{kind}_held", "0/0")
    return f"{kind} {shown} (names {named} holds {held})"


def verdict_of(length, theirs):
    if abs(length - theirs) <= TOLERANCE_FEET:
        return "exact"
    return "cut too far" if length < theirs else "cut too little"


def read_probe(path):
    beams = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        if not line.startswith("beam "):
            continue
        fields = dict(part.split("=", 1) for part in line.split()[1:] if "=" in part)
        beams[fields["tag"]] = fields
    return beams


def lengths(model, tags):
    """Tag -> the longest side of the box of Revit's body, in the file's mesh unit, in its own frame."""
    elements = [
        e
        for e in model.by_type("IfcElement")
        if e.Tag in tags and not e.is_a("IfcOpeningElement") and e.Representation
    ]
    out = {}
    if not elements:
        return out
    iterator = ifcopenshell.geom.iterator(
        ifcopenshell.geom.settings(), model, multiprocessing.cpu_count(), include=elements
    )
    if not iterator.initialize():
        return out
    while True:
        shape = iterator.get()
        verts = shape.geometry.verts
        points = [(verts[i], verts[i + 1], verts[i + 2]) for i in range(0, len(verts), 3)]
        if points:
            extent = max(max(p[k] for p in points) - min(p[k] for p in points) for k in range(3))
            tag = model.by_id(shape.id).Tag
            out[tag] = max(out.get(tag, 0.0), extent)
        if not iterator.next():
            break
    return out


def face_kind(element):
    for ifc_class, kind in FACES:
        if element.is_a(ifc_class):
            return kind
    return None


def surfaces(model):
    """Element id -> (kind, Tag, triangle corners (n, 3, 3), their unit normals (n, 3), the box
    (low, high)) for every column, beam, member, wall and slab with a body, in the world frame and
    the file's mesh unit. A triangle with no area is left out."""
    elements = [
        e
        for e in model.by_type("IfcElement")
        if e.Representation and not e.is_a("IfcOpeningElement") and face_kind(e)
    ]
    out = {}
    if not elements:
        return out
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    iterator = ifcopenshell.geom.iterator(settings, model, multiprocessing.cpu_count(), include=elements)
    if not iterator.initialize():
        return out
    while True:
        shape = iterator.get()
        verts = numpy.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        faces = numpy.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
        if len(verts) and len(faces):
            corners = verts[faces]
            normals = numpy.cross(corners[:, 1] - corners[:, 0], corners[:, 2] - corners[:, 0])
            lengths_ = numpy.linalg.norm(normals, axis=1)
            keep = lengths_ > 1e-12
            if keep.any():
                element = model.by_id(shape.id)
                out[shape.id] = (
                    face_kind(element),
                    element.Tag,
                    corners[keep],
                    normals[keep] / lengths_[keep][:, None],
                    (verts.min(axis=0), verts.max(axis=0)),
                )
        if not iterator.next():
            break
    return out


def cap_centres(corners):
    """The centres of the two end caps of a beam's mesh: the vertices within the 1 mm tolerance of
    each end of its principal axis (its longest direction), averaged. None for a mesh with no length."""
    points = corners.reshape(-1, 3)
    centre = points.mean(axis=0)
    _, _, axes = numpy.linalg.svd(points - centre, full_matrices=False)
    along = (points - centre) @ axes[0]
    low, high = along.min(), along.max()
    if high - low <= 0:
        return None
    return tuple(
        points[mask].mean(axis=0)
        for mask in (along <= low + 1e-9 + TOLERANCE_UNITS[0], along >= high - 1e-9 - TOLERANCE_UNITS[0])
    )



def lies_on(point, corners, normals):
    """Whether `point` is within 1 mm of the plane of a triangle of the mesh and inside that triangle
    (its barycentric coordinates are not below -0.001 along any edge): it lies on a face."""
    tol = TOLERANCE_UNITS[0]
    a = corners[:, 0]
    distance = ((point - a) * normals).sum(axis=1)
    near = numpy.abs(distance) <= tol
    if not near.any():
        return False
    a, b, c = a[near], corners[near, 1], corners[near, 2]
    foot = point - distance[near][:, None] * normals[near]
    v0, v1, v2 = b - a, c - a, foot - a
    d00, d01, d11 = (v0 * v0).sum(1), (v0 * v1).sum(1), (v1 * v1).sum(1)
    d20, d21 = (v2 * v0).sum(1), (v2 * v1).sum(1)
    denominator = d00 * d11 - d01 * d01
    ok = numpy.abs(denominator) > 1e-18
    u = numpy.where(ok, (d11 * d20 - d01 * d21) / numpy.where(ok, denominator, 1), -1)
    v = numpy.where(ok, (d00 * d21 - d01 * d20) / numpy.where(ok, denominator, 1), -1)
    w = 1 - u - v
    return bool(((u >= -0.001) & (v >= -0.001) & (w >= -0.001)).any())


def print_ends(model, stands_by_tag, factor):
    """Count the ends of the beams, by how Revit's length stands to the record box's and by the kind
    of element each end lies on."""
    TOLERANCE_UNITS[0] = TOLERANCE_FEET / factor
    meshes = surfaces(model)
    by_tag = {tag: ident for ident, (_, tag, *_rest) in meshes.items()}
    ids = list(meshes)
    lows = numpy.array([meshes[i][4][0] for i in ids]) - TOLERANCE_UNITS[0]
    highs = numpy.array([meshes[i][4][1] for i in ids]) + TOLERANCE_UNITS[0]
    ends = collections.Counter()
    for tag, stands in stands_by_tag.items():
        beam = by_tag.get(tag)
        if beam is None:
            continue
        caps = cap_centres(meshes[beam][2])
        if caps is None:
            continue
        for point in caps:
            candidates = numpy.nonzero(((point >= lows) & (point <= highs)).all(axis=1))[0]
            lying = {
                meshes[ids[k]][0]
                for k in candidates
                if ids[k] != beam and lies_on(point, meshes[ids[k]][2], meshes[ids[k]][3])
            }
            kind = next((kind for kind in FACE_KINDS if kind in lying), "nothing")
            ends[(stands, kind)] += 1
    print("beam ends: what the ends of Revit's beams lie on in Revit's export (a face of the kind within 1 mm, the beam's own excluded):")
    for stands in ("shorter", "same", "longer"):
        total = sum(n for (s, _), n in ends.items() if s == stands)
        if not total:
            continue
        cells = ", ".join(f"{kind} {ends[(stands, kind)]}" for kind in (*FACE_KINDS, "nothing"))
        print(f"beam ends:   Revit {stands:<8} {total} ends: {cells}")


def feet_per_unit(raw, records):
    """The factor from the mesh unit to feet: the one of metres, millimetres and feet under which the
    median of record length over Revit's is nearest 1."""
    best = None
    for factor in (METRES_FEET, METRES_FEET / 1000, 1.0):
        ratios = [record / (length * factor) for tag, length in raw.items() if (record := records.get(tag)) and length > 0]
        if not ratios:
            continue
        distance = abs(math.log(statistics.median(ratios)))
        if best is None or distance < best[0]:
            best = (distance, factor)
    return best[1] if best else METRES_FEET


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    beams = read_probe(args[0])
    record_lengths = {
        tag: float(f["high"]) - float(f["low"]) for tag, f in beams.items() if f["axis"] != "-"
    }
    model = ifcopenshell.open(args[1])
    raw = lengths(model, set(beams))
    factor = feet_per_unit(raw, record_lengths)
    revit = {tag: length * factor for tag, length in raw.items()}
    square = sum(1 for f in beams.values() if f["axis"] == "-")
    print(f"beam ends: {len(beams)} beams in the probe, {len(revit)} with a body in Revit's export, {square} square in plan (not scored)")

    table = collections.Counter()
    cut = collections.Counter()
    uncut = collections.Counter()
    shortened = collections.Counter()  # (kind, what) -> beams Revit makes shorter
    spared = collections.Counter()  # (kind, stands) -> beams Revit leaves or lengthens that the kind cuts
    shortfalls = []
    rows = []
    stands_by_tag = {}
    for tag, f in beams.items():
        if f["axis"] == "-" or tag not in revit:
            continue
        record = record_lengths[tag]
        theirs = revit[tag]
        stands = "same" if abs(record - theirs) <= TOLERANCE_FEET else "shorter" if theirs < record else "longer"
        trimmed = None
        if f["trim_low"] != "-" and f["trim_high"] != "-":
            trimmed = float(f["trim_high"]) - float(f["trim_low"])
        gives = "rule gives bounds" if trimmed is not None else "rule gives none"
        table[(stands, gives)] += 1
        stands_by_tag[tag] = stands
        if stands == "shorter":
            shortfalls.append(record - theirs)
        if trimmed is not None:
            cut[(stands, verdict_of(trimmed, theirs))] += 1
        else:
            uncut[(stands, f"names {f['named']} column(s), holds {f['held_low']}/{f['held_high']}")] += 1
        low, high = float(f["low"]), float(f["high"])
        for kind in KINDS:
            length = faced_length(f, kind, low, high)
            if stands == "shorter":
                held_low, held_high = (int(n) for n in f.get(f"{kind}_held", "0/0").split("/"))
                shortened[(kind, "names")] += int(f.get(f"{kind}_named", 0)) > 0
                shortened[(kind, "holds an end")] += held_low + held_high > 0
                if length is not None:
                    shortened[(kind, "gives bounds")] += 1
                    shortened[(kind, verdict_of(length, theirs))] += 1
            elif length is not None:
                spared[(kind, stands)] += 1
        rows.append((tag, f, record, theirs, trimmed, stands))

    print("beam ends: Revit's length against the record box's, and whether the rule gives bounds:")
    for (stands, gives), n in sorted(table.items()):
        print(f"beam ends:   Revit {stands:<8} {gives:<18} {n}")
    print("beam ends: the beams the rule cuts, against Revit's length (Revit's length against the record box's, verdict):")
    for (stands, verdict), n in sorted(cut.items()):
        print(f"beam ends:   Revit {stands:<8} {verdict:<14} {n}")
    print("beam ends: the beams the rule gives no bounds, by what they name (columns named, held at the low/high end):")
    for (stands, named), n in sorted(uncut.items(), key=lambda item: (item[0][0], -item[1])):
        print(f"beam ends:   Revit {stands:<8} {named:<34} {n}")
    if shortfalls:
        print(f"beam ends: the {len(shortfalls)} beams Revit makes shorter than the record box, by the kind of element their list names:")
        for kind in KINDS:
            print(
                f"beam ends:   {kind:<7} names {shortened[(kind, 'names')]}, holds an end {shortened[(kind, 'holds an end')]},"
                f" gives bounds {shortened[(kind, 'gives bounds')]}"
                f" (exact {shortened[(kind, 'exact')]}, cut too far {shortened[(kind, 'cut too far')]},"
                f" cut too little {shortened[(kind, 'cut too little')]})"
            )
    print("beam ends: the beams Revit leaves at the record box's length or makes longer, cut by a kind of element (cut where Revit does not):")
    for kind in KINDS:
        print(f"beam ends:   {kind:<7} same {spared[(kind, 'same')]}, longer {spared[(kind, 'longer')]}")
    if shortfalls:
        mm = sorted(value * 304.8 for value in shortfalls)
        print(
            f"beam ends: Revit's beam is shorter than the record box by a median {statistics.median(mm):.0f} mm"
            f" (quartiles {mm[len(mm) // 4]:.0f} and {mm[(3 * len(mm)) // 4]:.0f}, longest {mm[-1]:.0f}) on {len(mm)} beams"
        )
    print_ends(model, stands_by_tag, factor)
    if "--list" in sys.argv:
        for tag, f, record, theirs, trimmed, stands in sorted(rows, key=lambda row: (len(row[0]), row[0])):
            shown = "-" if trimmed is None else f"{trimmed:.3f}"
            faced = " ".join(cut_by(f, kind) for kind in KINDS)
            print(
                f"beam {tag} record {record:.3f} Revit {theirs:.3f} trimmed {shown} ({stands}) named {f['named']}"
                f" held {f['held_low']}/{f['held_high']} cut by {faced}"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
