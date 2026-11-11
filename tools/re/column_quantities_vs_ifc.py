#!/usr/bin/env python3
"""Per column, Revit's `Qto_ColumnBaseQuantities` against the prism of rvt-rs's own body (#1016, RE-424).

Pairs each `IfcColumn` of the two files by `Tag`. Revit's export writes `Length`, `CrossSectionArea`,
`OuterSurfaceArea` and `GrossVolume` on a column it draws as an extrusion and `CrossSectionArea` alone on one it
draws as a cut mesh (RE-291, RE-353), and `NetVolume` on both, so a rule that wrote all four on every prism would
add values Revit omits. For each pair the scorer says which set Revit wrote (the four, `CrossSectionArea` alone,
or something else), the class of Revit's body (an extrusion and its profile, or a face set), rvt-rs's body source
and what rvt-rs writes. It then reads rvt-rs's own body as the saved-mesh pass reads a beam's (`beam_quantities`,
RE-423): a prism along its longest axis with two flat ends of one area, whose volume is the end area times its
extent. For each column the rule accepts it says, per quantity, whether the value the rule gives (the end area, the
lateral area, the volume, the extent) equals Revit's, differs from it or is one Revit does not write, and what
rvt-rs writes today, so the first column of the last table is what the rule would add that is right.

A body that is a prism in planes may have been a curved one: a tessellated cylinder has many lateral planes, and
the saved-mesh pass refuses a mesh whose faces are not all flat, which this scorer cannot see. The class of
Revit's body (a circle profile) tells a round column apart.

Revit's export in IFC2X3, which has no `Qto_` sets, is not scored.

Usage:

    python3 tools/re/column_quantities_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell and numpy.
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np  # noqa: E402
from beam_quantities_vs_ifc import body_kind, prism_axes  # noqa: E402
from mullion_profiles_vs_ifc import analyse, close, facts, fmt, mesh, scales  # noqa: E402
from saved_meshes_vs_ifc import body_sources  # noqa: E402

QUANTITIES = ("Length", "CrossSectionArea", "OuterSurfaceArea", "GrossVolume", "NetVolume")
FOUR = ("Length", "CrossSectionArea", "OuterSurfaceArea", "GrossVolume")
UNIT = {"Length": 0, "GrossVolume": 2, "NetVolume": 2}  # the index into scales(); the rest are areas (1)
ENDS_EQUAL = 1e-4  # the saved-mesh pass's rule: the two ends' areas agree to this fraction
VOLUME_AGREES = 1e-4  # and the volume is the end area times the extent to this fraction
EXTENT_TIE = 1e-6  # two longest axes within this fraction of each other: no axis
VERTICAL_COSINE = 0.9999  # the saved-mesh pass's own: the axis it takes is the vertical to this cosine
LISTED = 40


def column_values(column, scale):
    """Each quantity of the column's `Qto_ColumnBaseQuantities` as metres, square or cubic metres, or None."""
    base = ifcopenshell.util.element.get_psets(column).get("Qto_ColumnBaseQuantities", {})
    out = {}
    for name in QUANTITIES:
        value = base.get(name)
        out[name] = None if value is None else float(value) * scale[UNIT.get(name, 1)]
    return out


def set_kind(values):
    """Which of the sets Revit writes on a column this is, by the quantities present."""
    if all(values[name] is not None for name in FOUR):
        return "the four"
    others = [name for name in FOUR if values[name] is not None]
    if others == ["CrossSectionArea"]:
        return "CrossSectionArea alone"
    return "other: " + (",".join(others) or "none of the four")


def rule(triangles):
    """The saved-mesh pass's beam rule on a body (RE-423): the values it would give and why, or None and why not."""
    if triangles is None:
        return None, "no body"
    every = prism_axes(triangles)
    axes = [axis for axis in every if axis[2] == ("flat", "flat")]
    if not axes:
        return None, "no prism axis with two flat ends"
    if len(axes) > 1 and abs(axes[0][1] - axes[1][1]) <= EXTENT_TIE * axes[0][1]:
        return None, "the two longest axes tie"
    axis = axes[0][0]
    # Beyond the beam rule: whether the axis it takes is the longest of all the axes the body is a prism along (a
    # cut column's longest may have unequal ends, leaving only a short flat-ended one), and whether it is vertical.
    longest = every[0][1] <= axes[0][1] * (1 + EXTENT_TIE)
    vertical = abs(float(axis[2])) >= VERTICAL_COSINE
    measures = analyse(triangles, axis)
    _, normals, area, _ = facts(triangles)
    along = normals @ axis
    up, down = float(area[along > 0.9999].sum()), float(area[along < -0.9999].sum())
    if min(up, down) <= 0 or abs(up - down) > ENDS_EQUAL * max(up, down):
        return None, "the ends differ in area"
    end = (up + down) / 2
    if abs(measures["volume"] - end * measures["extent"]) > VOLUME_AGREES * measures["volume"]:
        return None, "the volume is not the end area times the extent"
    how = "accepted, " + ("vertical" if vertical else "not vertical") + ", " + (
        "the longest prism axis" if longest else "a longer prism axis has unequal or mitred ends")
    return {
        "Length": measures["extent"],
        "CrossSectionArea": end,
        "OuterSurfaceArea": measures["lateral"],
        "GrossVolume": measures["volume"],
        "NetVolume": measures["volume"],
        "extent of the longest prism axis": every[0][1],
    }, how


def today(ours, revit, name):
    """What rvt-rs writes of a quantity now, against Revit's."""
    if revit is None:
        return "writes a value Revit omits" if ours is not None else "writes none"
    if ours is None:
        return "writes none"
    return "equal" if close(ours, revit) else "different"


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours_model, revit_model = ifcopenshell.open(sys.argv[1]), ifcopenshell.open(sys.argv[2])
    if revit_model.schema == "IFC2X3":
        print("Revit's export is IFC2X3, which has no Qto_ sets: not scored")
        return 0
    ours_scale, revit_scale = scales(ours_model), scales(revit_model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    ours = {str(c.Tag): c for c in ours_model.by_type("IfcColumn") if c.Tag}
    revit = {str(c.Tag): c for c in revit_model.by_type("IfcColumn") if c.Tag}
    both = sorted(set(ours) & set(revit), key=lambda t: (len(t), t))
    print(f"columns: rvt-rs {len(ours)}, Revit {len(revit)}, paired by Tag {len(both)}")
    if not both:
        return 0
    kinds, outcomes, states = collections.Counter(), collections.Counter(), collections.Counter()
    odd = []
    for tag in both:
        r = column_values(revit[tag], revit_scale)
        o = column_values(ours[tag], ours_scale)
        kind, body = set_kind(r), body_kind(revit[tag])
        src = body_sources(ours[tag]) or "no BodySource"
        # The saved-mesh pass reads a mesh; a record box or a hollow section is a body it does not touch.
        drawn = "drawn from a saved mesh" if src == "saved_graphics_mesh" else "another body"
        found, why = rule(mesh(settings, ours[tag]))
        kinds[(kind, body)] += 1
        outcomes[(kind, body, src, why)] += 1
        for name in QUANTITIES:
            now = today(o[name], r[name], name)
            if found is None:
                if r[name] is not None:
                    states[(name, kind, drawn, "rule gives none", now)] += 1
                continue
            if r[name] is None:
                states[(name, kind, drawn, "Revit omits it", now)] += 1
            else:
                states[(name, kind, drawn, "rule equal" if close(found[name], r[name]) else "rule different", now)] += 1
        if found is not None and len(odd) < LISTED and any(r[n] is not None and not close(found[n], r[n]) for n in QUANTITIES):
            odd.append((tag, revit[tag].Name, kind, body, src, r, found))
    print("Revit's set on each column, and the class of Revit's body (set, body):")
    for key, count in kinds.most_common():
        print(f"  {count:5}  {key}")
    print("what the rule makes of rvt-rs's own body (Revit's set, Revit's body, rvt-rs's body source, outcome):")
    for key, count in outcomes.most_common():
        print(f"  {count:5}  {key}")
    print("per quantity, the rule against Revit's and what rvt-rs writes now (quantity, Revit's set, rvt-rs's body, rule, rvt-rs now):")
    for key, count in sorted(states.items(), key=lambda kv: (QUANTITIES.index(kv[0][0]),) + kv[0][1:]):
        print(f"  {count:5}  {key}")
    for drawn in ("drawn from a saved mesh", "another body"):
        print(f"the rule would add, per quantity, on the columns {drawn}: right where rvt-rs writes none / different from Revit's / a value Revit omits:")
        for name in QUANTITIES:
            right = sum(c for (q, _, d, rule_state, now), c in states.items() if q == name and d == drawn and rule_state == "rule equal" and now == "writes none")
            wrong = sum(c for (q, _, d, rule_state, _), c in states.items() if q == name and d == drawn and rule_state == "rule different")
            omit = sum(c for (q, _, d, rule_state, _), c in states.items() if q == name and d == drawn and rule_state == "Revit omits it")
            print(f"  {name}: {right} / {wrong} / {omit}")
    print(f"the first {len(odd)} columns the rule accepts with a quantity different from Revit's:")
    for tag, name, kind, body, src, r, found in odd:
        print(f"  column {tag} {name!r} set {kind!r} body {body} src {src}")
        print("      Revit " + " | ".join(f"{q} {fmt(r[q])}" for q in QUANTITIES))
        print("      rule  " + " | ".join(f"{q} {fmt(found[q])}" for q in QUANTITIES) + f" | longest prism axis {fmt(found['extent of the longest prism axis'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
