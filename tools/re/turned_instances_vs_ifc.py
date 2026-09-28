#!/usr/bin/env python3
"""Score family instances turned off the model's axes against Revit's IFC4.

Research tool for `reports/element-framing/RE-87-instance-transforms.md`.
It is an end-to-end check: both files are meshed by IfcOpenShell, and the
reference is the body Revit's exporter wrote for the same element.

Every element of `new.ifc` whose `BodySource` is
`partition_family_instance_turned_box` is measured along its own plan axes,
the X axis of its placement and the one square to it. Revit's body for the
same Tag, and the same element in `old.ifc` (an export without RE-87), are
measured along the same axes. For each file it reports the largest
difference from Revit's extents, per element.

Revit's bodies are moved into Revit's internal frame through the export's
site placement, which is where rvt-rs places its own.

Usage:

    python3 tools/re/turned_instances_vs_ifc.py <new.ifc> <old.ifc> <revit-export.ifc> [--list N]

Needs IfcOpenShell (tested with 0.8.5) and NumPy.
"""

import collections
import math
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement
import numpy as np

FEET = 0.3048
SOURCE = "partition_family_instance_turned_box"


def site_frame(f):
    """Shared metres (IfcOpenShell world) -> Revit internal feet."""
    site = f.by_type("IfcSite")[0]
    mat = ifcopenshell.util.placement.get_local_placement(site.ObjectPlacement)
    ox, oy, oz = mat[0][3], mat[1][3], mat[2][3]
    c, s = mat[0][0], mat[1][0]

    def points(v):
        dx, dy = v[:, 0] - ox, v[:, 1] - oy
        return np.stack(
            [(c * dx + s * dy) / FEET, (-s * dx + c * dy) / FEET, (v[:, 2] - oz) / FEET], axis=1
        )

    return points


def vertices(element, frame):
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    try:
        shape = ifcopenshell.geom.create_shape(settings, element)
    except Exception:
        return None
    verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    return frame(verts) if len(verts) else None


def extents(verts, axes):
    return [(float((verts[:, :2] @ a).min()), float((verts[:, :2] @ a).max())) for a in axes] + [
        (float(verts[:, 2].min()), float(verts[:, 2].max()))
    ]


def by_tag(f):
    out = {}
    for element in f.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or not element.Tag:
            continue
        out.setdefault(element.Tag, element)
    return out


def main(argv):
    args = argv[1:]
    listed = 10
    if "--list" in args:
        i = args.index("--list")
        listed = int(args[i + 1])
        del args[i : i + 2]
    if len(args) != 3:
        raise SystemExit(__doc__)
    new, old, revit = (ifcopenshell.open(p) for p in args)
    frame = site_frame(revit)
    ours = lambda v: v / FEET  # noqa: E731 - rvt-rs writes internal metres
    new_tags, old_tags, revit_tags = by_tag(new), by_tag(old), by_tag(revit)
    rows = []
    for tag, element in new_tags.items():
        psets = ifcopenshell.util.element.get_psets(element)
        if psets.get("RvtElementRecordGeometry", {}).get("BodySource") != SOURCE:
            continue
        mat = ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement)
        x = np.array([mat[0][0], mat[1][0]])
        x /= np.linalg.norm(x)
        axes = [x, np.array([-x[1], x[0]])]
        reference = revit_tags.get(tag)
        verts = [vertices(e, f) if e is not None else None for e, f in (
            (reference, frame), (element, ours), (old_tags.get(tag), ours))]
        if any(v is None for v in verts):
            rows.append((tag, element.is_a(), None, None))
            continue
        want = extents(verts[0], axes)
        diffs = []
        for got in (extents(verts[1], axes), extents(verts[2], axes)):
            diffs.append(max(abs(a - b) for pair_a, pair_b in zip(want, got) for a, b in zip(pair_a, pair_b)))
        rows.append((tag, element.is_a(), diffs[0], diffs[1]))

    compared = [r for r in rows if r[2] is not None]
    print(f"turned family instances in new.ifc: {len(rows)}; with Revit's body and old.ifc's: {len(compared)}")
    for label, index in (("new", 2), ("old", 3)):
        counts = " ".join(
            f"{limit} ft: {sum(1 for r in compared if r[index] <= limit)}" for limit in (0.01, 0.1, 0.5, 1.0)
        )
        print(f"  {label}: extents along the element's own axes within {counts}; median {np.median([r[index] for r in compared]) if compared else 0:.3f} ft")
    better = sum(1 for r in compared if r[2] < r[3] - 1e-6)
    worse = sum(1 for r in compared if r[2] > r[3] + 1e-6)
    print(f"  per element against old: {better} better, {worse} worse, {len(compared) - better - worse} unchanged")
    kinds = collections.Counter(r[1] for r in compared)
    print(f"  classes: {dict(kinds)}")
    for tag, kind, a, b in sorted(compared, key=lambda r: r[2] - r[3], reverse=True)[:listed]:
        print(f"  {tag} {kind}: new {a:.3f} ft, old {b:.3f} ft")


if __name__ == "__main__":
    main(sys.argv)
