#!/usr/bin/env python3
"""Score rvt-rs duct bodies against Revit's own IFC export, per duct (#96).

Research tool for `reports/element-framing/RE-134-curve-fields-at-the-anchor.md`.
Revit writes a duct as an extruded rectangle: for a duct along a principal axis
it is the duct's own box, the profile turned so its two sides are whichever way
round the export chose. This compares the world-space box of each duct's
geometry in the two exports, in feet, for every duct both hold (same `Tag`, the
ElementId). Revit's IFC2X3 export writes ducts as `IfcFlowSegment` like pipes,
which are named "Pipe Types:<type>:<ElementId>" and left out here.

The box is what a body along a principal axis cannot fake, and what a turned or
sloped duct would get wrong if it were drawn as its record's box.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/duct_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc> [--list]
"""

import argparse

import ifcopenshell
import ifcopenshell.geom

FT = 0.3048
TOLERANCE_FT = 0.01


def duct_boxes(path):
    """Tag -> (min, max) of the duct's world-space vertices, in feet."""
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for element in model.by_type("IfcFlowSegment"):
        name = element.Name or ""
        if not element.Tag or not element.Representation or name.startswith(("Pipe Types:", "Pipe-")):
            continue
        shape = ifcopenshell.geom.create_shape(settings, element)
        verts = shape.geometry.verts
        points = [(verts[i] / FT, verts[i + 1] / FT, verts[i + 2] / FT) for i in range(0, len(verts), 3)]
        if not points:
            continue
        low = tuple(min(p[k] for p in points) for k in range(3))
        high = tuple(max(p[k] for p in points) for k in range(3))
        out[element.Tag] = (low, high)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ours")
    ap.add_argument("reference")
    ap.add_argument("--list", action="store_true", help="print every duct that differs")
    args = ap.parse_args()
    ours = duct_boxes(args.ours)
    ref = duct_boxes(args.reference)
    both = sorted(set(ours) & set(ref), key=lambda t: (len(t), t))
    print(f"ducts in Revit's export with a body: {len(ref)}; rvt-rs writes one for {len(both)} of them")
    if not both:
        return
    ok = 0
    worst = 0.0
    misses = []
    for tag in both:
        a, b = ours[tag], ref[tag]
        error = max(abs(a[i][k] - b[i][k]) for i in (0, 1) for k in range(3))
        worst = max(worst, error)
        if error <= TOLERANCE_FT:
            ok += 1
        else:
            misses.append((tag, error))
    print(f"world box within {TOLERANCE_FT} ft of Revit's: {ok} of {len(both)} (worst {worst:.5f} ft)")
    if args.list:
        for tag, error in misses:
            print(f"  {tag}: box differs by {error:.5f} ft")


if __name__ == "__main__":
    main()
