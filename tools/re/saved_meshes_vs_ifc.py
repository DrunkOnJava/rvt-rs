#!/usr/bin/env python3
"""Score rvt-rs bodies against Revit's own IFC export, per element (#255).

Research tool for `reports/element-framing/RE-78-saved-graphics-meshes.md`.
For every element of an rvt-rs IFC whose `BodySource` is one of the given
sources, it compares the world axis-aligned box of the body with the box of
the element with the same `Tag` in Revit's export, and buckets the largest
of the six corner differences (feet). `IfcOpeningElement` is skipped on
both sides: Revit tags an opening with the id of the element that cuts it.

Each file's bodies are taken in Revit's internal feet by undoing that
file's own IfcSite placement (`ifc_common.site_frame`), as the other body
scorers do, so a reference export in shared coordinates, turned to true
north (the BIM4LCA office), compares box for box. With `--rotate-deg` or
`--translate-ft X,Y,Z`, rvt-rs's bodies are instead moved into the
reference's world coordinates by that internal-to-shared transform, and the
reference's are taken as they are. Either way any residual is removed as
one translation for the whole file: `--offset-ft`, or by default the median
difference over the scored elements. The offset is in the frame the boxes
are compared in: internal feet by default, the reference's world feet with
a transform.

Only the rvt-rs elements with a wanted `BodySource`, and the reference's
elements with their Tags, are meshed.

The bar (#1365, RE-331 section 8; Griffin, #1355): an element that both files
write as true curves (true_curves.py), which a saved mesh never is, since
rvt-rs writes it as triangles, has its box taken from a mesh at the fine
deflection (fine_mesh.py) in both files; every other element keeps the default
tessellation. When any element is so decided a line says how many and the
table gains a last column, `fine`, how many of each class were; when none is,
the output is what it was before #1365.

With RVT_SCORER_FINE_MESH=1 (#1355, RE-331) it then meshes an even stride of at
most 300 of each class's paired elements again in both files at one equal, fine
deflection (fine_mesh.py) and prints, after the table above, `fine` rows for
them: the same buckets, with one for a box within 0.001 ft, beside the count
within 0.001 ft in the default tessellation for the same elements, both with
the file-wide offset of the table above. A box is
the extent of a tessellated body, so a curved body's differs by the chords'
sagitta between tessellations. The output without it is as it was.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).

Usage:

    python3 tools/re/saved_meshes_vs_ifc.py <rvt-rs.ifc> <revit.ifc> \
        [--source saved_graphics_mesh] [--rotate-deg DEG] \
        [--translate-ft X,Y,Z] [--offset-ft X,Y,Z]
"""

import argparse
import collections
import math
import os
import statistics
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fine_mesh  # noqa: E402
import true_curves  # noqa: E402
from ifc_common import mesh_parts, site_frame  # noqa: E402

FT = 0.3048


def body_sources(element):
    for rel in element.IsDefinedBy:
        if not rel.is_a("IfcRelDefinesByProperties"):
            continue
        for prop in getattr(rel.RelatingPropertyDefinition, "HasProperties", None) or []:
            if prop.Name == "BodySource" and prop.NominalValue is not None:
                return prop.NominalValue.wrappedValue
    return None


def world_boxes(path, tags=None, transform=None, wanted=None, frame=True, settings=None, curves=None, detect=None):
    """Tag -> (class, box in feet, BodySource) for elements with a body, the
    box of the union of the bodies where several carry one Tag: in
    the file's internal feet through its site placement (`frame`), else in
    its world feet, moved by `transform` when given. With `wanted`, only the
    elements with one of those BodySources are meshed. With `detect` (True for
    every element, or the Tags to test), `curves` is filled with whether each
    body is made of true curves (true_curves.py)."""
    model = ifcopenshell.open(path)
    if settings is None:
        settings = ifcopenshell.geom.settings()
        settings.set("use-world-coords", True)
    elements = [
        e
        for e in model.by_type("IfcElement")
        if not e.is_a("IfcOpeningElement")
        and e.Tag
        and e.Representation
        and (tags is None or e.Tag in tags)
    ]
    sources = {e.Tag: body_sources(e) for e in elements}
    if wanted is not None:
        elements = [e for e in elements if sources.get(e.Tag) in wanted]
    out = {}
    if not elements:
        return out
    to_internal = site_frame(model) if frame else None
    # The order the geometry iterator returned them in, one thread, which the
    # tables' ties (equal counts) follow.
    elements.sort(key=lambda e: e.id())
    shapes = mesh_parts(model, settings, elements, path)
    for element in elements:
        if element.id() not in shapes:
            continue
        verts = shapes[element.id()][0]
        if len(verts):
            if to_internal is not None:
                points = to_internal(verts)
            else:
                points = verts / FT
                if transform:
                    points = np.array([transform(p) for p in points])
            box = [float(v) for v in points.min(axis=0)] + [float(v) for v in points.max(axis=0)]
            if element.Tag in out:
                # Revit writes the parts of a roof as slabs that carry the roof's own
                # Tag, and the element rvt-rs draws whole is all of them: the box of
                # the Tag is the union, not the last part's.
                before = out[element.Tag][1]
                box = [min(a, b) for a, b in zip(before[:3], box[:3])] + [max(a, b) for a, b in zip(before[3:], box[3:])]
            out[element.Tag] = (element.is_a(), box, sources.get(element.Tag))
            if curves is not None and detect and (detect is True or element.Tag in detect):
                curves[element.Tag] = curves.get(element.Tag, True) and true_curves.is_true_curve_body(element)
    return out


FINE_CAP = 300


def box_error(box, other, offset):
    return max(abs(box[k] - offset[k] - other[k]) for k in range(6))


def fine_table(args, wanted, frames, to_shared, ours, ref, offset):
    """The `fine` rows (#1355): an even stride of at most FINE_CAP paired elements of each class,
    meshed again in both files at the fine deflection."""
    paired = collections.defaultdict(list)
    for tag, (cls, _, _) in ours.items():
        if tag in ref:
            paired[cls].append(tag)
    sample = set()
    for tags in paired.values():
        tags = sorted(tags, key=lambda t: (len(t), t))
        sample.update(tags[:: max(1, -(-len(tags) // FINE_CAP))])
    settings = fine_mesh.settings()
    ours_fine = world_boxes(args.ours, tags=sample, transform=None if frames else to_shared, wanted=wanted, frame=frames, settings=settings)
    ref_fine = world_boxes(args.reference, tags=set(ours_fine), frame=frames, settings=settings)
    print()
    print(f"fine tessellation: {fine_mesh.LABEL}, both files; {len(ours_fine)} of {len(sample)} sampled elements meshed")
    rows = collections.defaultdict(lambda: [0] * 8)
    for tag, (cls, box, _) in ours_fine.items():
        for key in (cls, "total"):
            row = rows[key]
            row[0] += 1
            row[1] += tag in ours and tag in ref and box_error(ours[tag][1], ref[tag][1], offset) <= 0.001
            if tag not in ref_fine:
                row[7] += 1
                continue
            err = box_error(box, ref_fine[tag][1], offset)
            row[2] += err <= 0.001
            row[3 if err <= 0.01 else 4 if err <= 0.1 else 5 if err <= 1 else 6] += 1
    heads = ("n", "default<=.001", "<=0.001ft", "<=0.01ft", "<=0.1", "<=1", ">1", "no ref")
    widths = (6, 14, 10, 9, 7, 6, 6, 8)
    print(f"fine {'class':28}" + "".join(f"{h:>{w}}" for h, w in zip(heads, widths)))
    for cls in sorted((k for k in rows if k != "total"), key=lambda c: (-rows[c][0], c)) + ["total"]:
        print(f"fine {cls:28}" + "".join(f"{v:>{w}}" for v, w in zip(rows[cls], widths)))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ours")
    ap.add_argument("reference")
    ap.add_argument("--source", action="append", default=None)
    ap.add_argument("--rotate-deg", type=float, default=None)
    ap.add_argument("--translate-ft", default=None)
    ap.add_argument("--offset-ft", default=None)
    args = ap.parse_args()
    wanted = set(args.source or ["saved_graphics_mesh"])
    frames = args.rotate_deg is None and args.translate_ft is None
    tx, ty, tz = (float(v) for v in (args.translate_ft or "0,0,0").split(","))
    angle = math.radians(args.rotate_deg or 0.0)
    c, s = math.cos(angle), math.sin(angle)

    def to_shared(p):
        x, y, z = p
        return (x * c - y * s + tx, x * s + y * c + ty, z + tz)

    ours_curves, ref_curves = {}, {}
    ours = world_boxes(args.ours, transform=None if frames else to_shared, wanted=wanted, frame=frames, curves=ours_curves, detect=True)
    curve_ours = {t for t, flag in ours_curves.items() if flag}
    ref = world_boxes(args.reference, tags=set(ours), frame=frames, curves=ref_curves, detect=curve_ours)
    decided = {t for t in curve_ours if ref_curves.get(t)}
    ours_decided = ref_decided = {}
    if decided:
        settings = fine_mesh.settings()
        ours_decided = world_boxes(args.ours, tags=decided, transform=None if frames else to_shared, wanted=wanted, frame=frames, settings=settings)
        ref_decided = world_boxes(args.reference, tags=decided, frame=frames, settings=settings)
        decided = {t for t in decided if t in ours_decided and t in ref_decided}
    print("frames:", "each file's site placement undone" if frames else "rvt-rs moved by the given transform")
    pairs = [(ours[t][1], ref[t][1]) for t in ours if t in ref]
    if args.offset_ft:
        offset = [float(v) for v in args.offset_ft.split(",")]
    else:
        offset = [statistics.median(a[k] - b[k] for a, b in pairs) for k in range(3)] if pairs else [0.0] * 3
    offset *= 2
    print("file-wide offset removed (ft):", [round(v, 4) for v in offset[:3]])

    rows = collections.defaultdict(lambda: [0] * 7)
    for tag, (cls, box, _) in ours.items():
        row = rows[cls]
        row[0] += 1
        if tag not in ref:
            row[5] += 1
            continue
        other = ref[tag][1]
        if tag in decided:
            box, other = ours_decided[tag][1], ref_decided[tag][1]
            row[6] += 1
        err = max(abs(box[k] - offset[k] - other[k]) for k in range(6))
        row[1 if err <= 0.01 else 2 if err <= 0.1 else 3 if err <= 1 else 4] += 1
    shown = 7 if decided else 6
    widths = (6, 10, 8, 8, 7, 8, 6)[:shown]
    if decided:
        print(f"decided by the fine tessellation (both files write true curves): {len(decided)} of {len(ours)} elements")
    print(f"{'class':28}" + "".join(f"{h:>{w}}" for h, w in zip(("n", "<=0.01ft", "<=0.1", "<=1", ">1", "no ref", "fine"), widths)))
    total = [0] * 7
    for cls, row in sorted(rows.items(), key=lambda kv: -kv[1][0]):
        print(f"{cls:28}" + "".join(f"{v:>{w}}" for v, w in zip(row, widths)))
        total = [a + b for a, b in zip(total, row)]
    print(f"{'total':28}" + "".join(f"{v:>{w}}" for v, w in zip(total, widths)))
    if fine_mesh.enabled():
        fine_table(args, wanted, frames, to_shared, ours, ref, offset)


if __name__ == "__main__":
    main()
