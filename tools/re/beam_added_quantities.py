#!/usr/bin/env python3
"""For the beams whose base quantities one export adds to another's, check that each is what the rule says
(#1693, RE-423): the area of one end, the lateral area, the lateral area plus both flat ends, or the volume, of the
prism the beam's body is along its longest axis with two flat ends.

For models no Revit export scores (Autodesk's samples), where a change that writes a value cannot be read against
Revit's: it reads the two files' kept exports, pairs the `IfcBeam`s by Tag, takes each that holds a quantity of
`Qto_BeamBaseQuantities` in the second and none of it in the first, finds the longest axis its body is a prism
along with two flat ends (`beam_quantities_vs_ifc.prism_axes`), measures the body along it
(`mullion_profiles_vs_ifc.analyse`), and prints, per added quantity, whether it is that sum, and the first few beams
where one is not.

Usage:

    python3 tools/re/beam_added_quantities.py <before.ifc> <after.ifc>

Needs IfcOpenShell and numpy.
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import beam_quantities_vs_ifc as beams  # noqa: E402
from mullion_profiles_vs_ifc import analyse, close, fmt, mesh, scales  # noqa: E402

ADDED = ("CrossSectionArea", "OuterSurfaceArea", "GrossSurfaceArea", "NetSurfaceArea", "GrossVolume", "NetVolume")
LISTED = 8


def sums(m):
    """What the rule writes for each quantity of a prism with two flat ends."""
    return {
        "CrossSectionArea": m["flat"] / 2,
        "OuterSurfaceArea": m["lateral"],
        "GrossSurfaceArea": m["lateral"] + m["flat"],
        "NetSurfaceArea": m["lateral"] + m["flat"],
        "GrossVolume": m["volume"],
        "NetVolume": m["volume"],
    }


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    before, after = ifcopenshell.open(sys.argv[1]), ifcopenshell.open(sys.argv[2])
    scale_before, scale_after = scales(before), scales(after)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    held = {str(b.Tag): b for b in before.by_type("IfcBeam") if b.Tag}
    tally, odd, changed = collections.Counter(), [], 0
    for beam in after.by_type("IfcBeam"):
        if not beam.Tag:
            continue
        tag = str(beam.Tag)
        now = beams.beam_values(beam, scale_after)
        was = beams.beam_values(held[tag], scale_before) if tag in held else None
        added = [q for q in ADDED if now[q] is not None and (was is None or was[q] is None)]
        if not added:
            continue
        changed += 1
        triangles = mesh(settings, beam)
        axes = [a for a in beams.prism_axes(triangles) if a[2] == ("flat", "flat")] if triangles is not None else []
        if not axes:
            tally[("no axis with two flat ends", len(added))] += 1
            odd.append((tag, beam.Name, now, None))
            continue
        body = analyse(triangles, axes[0][0])
        want = sums(body)
        wrong = [q for q in added if not close(want[q], now[q])]
        for quantity in added:
            tally[(quantity, "not that sum" if quantity in wrong else "the sum the rule gives")] += 1
        if wrong:
            odd.append((tag, beam.Name, now, body))
    print(f"beams with a base quantity the first export lacks: {changed}")
    for key, count in sorted(tally.items()):
        print(f"  {count:5}  {key}")
    for tag, name, now, body in odd[:LISTED]:
        shown = " | ".join(f"{q} {fmt(now[q])}" for q in ADDED)
        print(f"  not the sums: beam {tag} {name!r} {shown}")
        if body:
            print(
                f"      body: ends {'+'.join(body['ends'])} extent {body['extent']:.6g} lateral {body['lateral']:.6g} "
                f"flat {body['flat']:.6g} volume {body['volume']:.6g}"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
