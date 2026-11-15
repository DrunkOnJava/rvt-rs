#!/usr/bin/env python3
"""Whether a ceiling's saved mesh gives Revit's `GrossArea` (#1209).

Research tool. Reads `probe_1209_ceiling_mesh_faces`'s output (each ceiling,
whether its record holds a `CutPairGStep`, and the area of its saved mesh's
horizontal faces at each height and direction) and Revit's own IFC export.
For each ceiling Revit writes a `Qto_CoveringBaseQuantities.GrossArea` for, it
takes the largest upward face and the largest downward face of the mesh and
counts, for cut and uncut ceilings apart, those within 0.1% of Revit's value.
Areas are Revit internal square feet.

Usage: ceiling_mesh_areas_vs_ifc.py <probe output> <revit-export.ifc> [--list]
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.unit

SQUARE_FOOT = 0.09290304
TOLERANCE = 1e-3


def main(argv):
    positional = [a for a in argv[1:] if not a.startswith("--")]
    listing = "--list" in argv
    if len(positional) != 2:
        sys.exit(__doc__)
    probe, theirs = positional
    ceilings = {}
    faces = collections.defaultdict(lambda: {"up": 0.0, "down": 0.0})
    for line in open(probe):
        p = line.split()
        if p and p[0] == "ceiling":
            ceilings[p[1]] = {"cut": p[3] == "true", "status": p[5], "unbounded": int(p[9]), "triangles": int(p[11])}
        elif p and p[0] == "face":
            tag, direction, area = p[1], p[2], float(p[6])
            faces[tag][direction] = max(faces[tag][direction], area)
    f = ifcopenshell.open(theirs)
    # The file's own area unit: the length unit squared is wrong where they differ (mm and m2).
    scale = ifcopenshell.util.unit.calculate_unit_scale(f, "AREAUNIT")
    revit = {}
    for covering in f.by_type("IfcCovering"):
        q = ifcopenshell.util.element.get_psets(covering, qtos_only=True).get("Qto_CoveringBaseQuantities", {})
        if covering.Tag and "GrossArea" in q:
            revit[str(covering.Tag)] = float(q["GrossArea"]) * scale / SQUARE_FOOT
    counts = collections.Counter()
    rows = []
    for tag, want in sorted(revit.items()):
        c = ceilings.get(tag)
        if c is None:
            counts["not in the probe"] += 1
            continue
        kind = "cut" if c["cut"] else "uncut"
        counts[f"{kind}"] += 1
        if not c["triangles"]:
            counts[f"{kind}, no triangles"] += 1
            continue
        up, down = faces[tag]["up"], faces[tag]["down"]
        hit_up = want > 0 and abs(up - want) <= TOLERANCE * want
        hit_down = want > 0 and abs(down - want) <= TOLERANCE * want
        counts[f"{kind}, upper face equal"] += hit_up
        counts[f"{kind}, lower face equal"] += hit_down
        counts[f"{kind}, either equal"] += hit_up or hit_down
        rows.append((tag, kind, c["status"], c["unbounded"], want, up, down, hit_up or hit_down))
    print(f"ceiling mesh areas: ceilings in the probe {len(ceilings)}, with Revit's GrossArea {len(revit)}")
    for key in sorted(counts):
        print(f"ceiling mesh areas: {key} {counts[key]}")
    if listing:
        for tag, kind, status, unbounded, want, up, down, hit in rows:
            if kind == "cut" or not hit:
                print(f"  ceiling {tag} {kind} {status} unbounded {unbounded}: Revit {want:.3f} ft2, mesh up {up:.3f}, down {down:.3f}, {'equal' if hit else 'not'}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
