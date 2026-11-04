#!/usr/bin/env python3
"""Score rvt-rs bodies drawn from a family's own extrusions
(`rvt-ifc --family-extrusions`, RE-327) against the body Revit's own IFC
export writes for the same element.

This is `mesh_bodies_vs_ifc.py` on the elements whose `BodySource` is
`family_extrusions` instead of `saved_graphics_mesh`: per class and Revit
type, how many have Revit's volume within 1% and 0.01%, every vertex of each
body within 0.01 ft and 0.001 ft of the other's surface, and "ref on ours":
every tested vertex of Revit's body within 0.001 ft of rvt-rs's surface.
Bodies only exist in an export made with `--family-extrusions`; without it
this prints none.

The bar is `mesh_bodies_vs_ifc`'s (#1365, RE-331 section 8): a body that
both files write as true curves, which here is a family extrusion with an arc
against Revit's body written as a circle profile, an outline with an arc and the
like (true_curves.py), is judged in the fine tessellation, and every other body in
IfcOpenShell's default one.

After those rows it prints, for the scored bodies with an arc in a profile
(#1346, RE-327 section 11), a census by the form Revit's export gives the
body and by the largest radius of its arcs (up to 0.12 ft, where IfcOpenShell
cuts any form of an arc within 0.00088 ft of it, or over), with how many of
each hold "ref on ours", both directions within 0.001 ft and within 1e-5 ft
(the tessellations coincide), and the volume within 1% and 0.01%. A row for the
bodies decided by the fine tessellation says so at its end, and an "all" row, which
mixes the two, says how many of its bodies were; the other rows, and the bodies in the
list of misses without `(fine tessellation)`, are in the default one. Then the scored bodies with an arc that miss "ref on ours",
one line each.

With RVT_SCORER_FINE_MESH=1 (#1355, RE-331) `mesh_bodies_vs_ifc` also prints its
`fine` rows for every body (both files meshed at one equal, fine deflection), and
this prints the same census for the fine scores, each line led by `fine`.

Needs IfcOpenShell and numpy.

Usage:

    python3 tools/re/family_extrusion_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import collections
import math
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mesh_bodies_vs_ifc  # noqa: E402
from true_curves import body_items  # noqa: E402

FEET = 0.3048
# The 0.12 ft that the rule of #1352 drew its line at (`ARC_MAX_RADIUS` in
# src/ifc/family_extrusions.rs until #1366 removed it). The census still splits
# the arcs there, to show whether the bodies that rule kept as meshes score
# differently from the small ones. The radius is read back from 3 points rounded
# to a micrometre, so an arc within a fraction of a percent of 0.12 ft can land in
# the other row, which changes no score.
SMALL_ARC_RADIUS = 0.12


def curve_kind(curve):
    if curve.is_a("IfcPolyline"):
        return "polyline"
    if curve.is_a("IfcIndexedPolyCurve"):
        arcs = any(segment.is_a("IfcArcIndex") for segment in curve.Segments or ())
        return "indexed curve with arcs" if arcs else "indexed curve of lines"
    if curve.is_a("IfcCompositeCurve"):
        arcs = any(not segment.ParentCurve.is_a("IfcLine") and not segment.ParentCurve.is_a("IfcPolyline") for segment in curve.Segments)
        return "composite curve with arcs" if arcs else "composite curve of lines"
    return curve.is_a()


def item_kind(item):
    """A short name for one item of a body: an extrusion by its profile, else its class."""
    if not item.is_a("IfcExtrudedAreaSolid"):
        return item.is_a()
    profile = item.SweptArea
    if profile.is_a("IfcArbitraryClosedProfileDef"):
        voids = "with voids " if profile.is_a("IfcArbitraryProfileDefWithVoids") else ""
        return f"extrusion of {voids}{curve_kind(profile.OuterCurve)}"
    name = profile.is_a().removeprefix("Ifc").removesuffix("ProfileDef")
    return "extrusion of " + re.sub(r"(?<!^)(?=[A-Z])", " ", name).lower()


def revit_form(element):
    """The distinct kinds of the items of Revit's body for the element, as one string."""
    kinds = sorted({item_kind(item) for item in body_items(element)})
    return " + ".join(kinds) or "no body item"


def circumradius(a, b, c):
    ab, bc, ca = math.dist(a, b), math.dist(b, c), math.dist(c, a)
    area2 = abs((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]))
    return ab * bc * ca / (2 * area2) if area2 > 0 else math.inf


def curves_of(profile):
    if profile.is_a("IfcArbitraryClosedProfileDef"):
        yield profile.OuterCurve
        if profile.is_a("IfcArbitraryProfileDefWithVoids"):
            yield from profile.InnerCurves


def our_arcs(element):
    """(arcs in the profiles of the body, the largest of their radii in feet)."""
    count, largest = 0, 0.0
    for item in body_items(element):
        if not item.is_a("IfcExtrudedAreaSolid"):
            continue
        for curve in curves_of(item.SweptArea):
            if not curve.is_a("IfcIndexedPolyCurve"):
                continue
            points = curve.Points.CoordList
            for segment in curve.Segments or ():
                if not segment.is_a("IfcArcIndex"):
                    continue
                a, b, c = (points[i - 1] for i in segment.wrappedValue)
                count += 1
                radius = circumradius(a, b, c)
                if math.isfinite(radius):
                    largest = max(largest, radius / FEET)
    return count, largest


def census(scored, label="", decided=frozenset()):
    ours = mesh_bodies_vs_ifc.DESCRIBED.get("ours", {})
    ref = mesh_bodies_vs_ifc.DESCRIBED.get("ref", {})
    rows = collections.defaultdict(collections.Counter)
    misses = []
    for far, tag, cls, object_type, ratio, to_ours, to_ref in scored:
        arcs, largest = ours.get(tag, (0, 0.0))
        if not arcs:
            continue
        written = "arcs up to 0.12 ft" if largest <= SMALL_ARC_RADIUS else "arcs over 0.12 ft"
        by = "decided by the fine tessellation" if tag in decided else ""
        for key in ((ref.get(tag, "not described"), written, by), ("all", written, ""), ("all", "all", "")):
            row = rows[key]
            row["n"] += 1
            row["fine"] += tag in decided
            row["ref on ours"] += to_ours <= 0.001
            row["0.001 ft"] += far <= 0.001
            row["1e-5 ft"] += far <= 1e-5
            row["vol 1%"] += abs(ratio - 1) <= 0.01
            row["vol 0.01%"] += abs(ratio - 1) <= 1e-4
        if to_ours > 0.001:
            misses.append((to_ours, tag, cls, object_type, ratio, to_ref, largest, ref.get(tag, "not described")))
    print()
    if not rows:
        print(f"{label}scored bodies with an arc in a profile: none")
        return
    total = rows[("all", "all", "")]["n"]
    print(f"{label}scored bodies with an arc in a profile: {total}")
    print(f"{label}{'n':>6} {'ref on ours':>11} {'0.001 ft':>8} {'1e-5 ft':>7} {'vol 1%':>7} {'vol .01%':>8}  Revit's form; rvt-rs's arcs")
    for key in sorted(rows, key=lambda k: (k[0] != "all", k[0], k[1] != "all", k[1], k[2])):
        row = rows[key]
        print(
            f"{label}{row['n']:>6} {row['ref on ours']:>11} {row['0.001 ft']:>8} {row['1e-5 ft']:>7} {row['vol 1%']:>7}"
            f" {row['vol 0.01%']:>8}  {key[0]}; {key[1]}"
            + (f"; {key[2]}" if key[2] else f"; {row['fine']} of them decided by the fine tessellation" if key[0] == "all" and row["fine"] else "")
        )
    misses.sort(reverse=True)
    print(f"{label}scored bodies with an arc that miss 'ref on ours': {len(misses)}")
    for to_ours, tag, cls, object_type, ratio, to_ref, largest, form in misses[:15]:
        print(
            f"  {label}{tag} {cls} '{object_type}': volume ratio {ratio:.4f}, Revit's vertices to rvt-rs's surface"
            f" {to_ours:.4g} ft, rvt-rs's to Revit's {to_ref:.4g} ft, largest arc radius {largest:.4g} ft; Revit: {form}"
            + (" (fine tessellation)" if tag in decided else "")
        )


def lenient(describe, unread):
    """`describe` that gives `unread` for a body whose structure it does not read: the census
    is beside the scores, so it never fails them."""

    def described(element):
        try:
            return describe(element)
        except (AttributeError, IndexError, KeyError, TypeError, ValueError, RuntimeError):
            return unread

    return described


def main():
    mesh_bodies_vs_ifc.SOURCE = "family_extrusions"
    mesh_bodies_vs_ifc.DESCRIBE.update(ours=lenient(our_arcs, (0, 0.0)), ref=lenient(revit_form, "not read"))
    status = mesh_bodies_vs_ifc.main()
    if status == 0:
        census(mesh_bodies_vs_ifc.RESULT.get("scored", []), decided=mesh_bodies_vs_ifc.RESULT.get("decided_fine", frozenset()))
        if "scored_fine" in mesh_bodies_vs_ifc.RESULT:
            census(mesh_bodies_vs_ifc.RESULT["scored_fine"], "fine ")
    return status


if __name__ == "__main__":
    sys.exit(main())
