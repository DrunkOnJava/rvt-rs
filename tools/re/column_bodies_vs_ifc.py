#!/usr/bin/env python3
"""Score rvt-rs column bodies drawn from a section against Revit's own IFC
export, per column (W6).

For every `IfcColumn` whose `BodySource` in the rvt-rs export is a section
(`family_type_hollow_section`, `family_type_i_section`) and that Revit's
export also holds (same `Tag`), this meshes both bodies in each file's own
site frame and compares, in feet, as `stair_bodies_vs_ifc.py` does: the
volume, how far each vertex of Revit's body lies from the nearest vertex of
rvt-rs's, and from rvt-rs's surface. Per `BodySource`, the counts; then
the first columns that differ.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/column_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import argparse
import collections
import math
import os
import sys

import ifcopenshell

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from roof_bodies_vs_ifc import VERTEX_TOLERANCE_FT, VOLUME_TOLERANCE, farthest, surface_distances  # noqa: E402
from saved_meshes_vs_ifc import body_sources  # noqa: E402
from stair_bodies_vs_ifc import bodies  # noqa: E402

SOURCES = ("family_type_hollow_section", "family_type_i_section")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ours")
    ap.add_argument("reference")
    args = ap.parse_args()
    model = ifcopenshell.open(args.ours)
    source_of = {
        e.Tag: body_sources(e)
        for e in model.by_type("IfcColumn")
        if e.Tag and body_sources(e) in SOURCES
    }
    ours = {tag: body for tag, body in bodies(args.ours, "IfcColumn").items() if tag in source_of}
    ref = bodies(args.reference, "IfcColumn")
    tally = collections.defaultdict(collections.Counter)
    differs = []
    for tag in sorted(set(ours) & set(ref), key=lambda t: (len(t), t)):
        (our_vertices, our_volume, our_triangles), (ref_vertices, ref_volume, _) = ours[tag], ref[tag]
        distance = farthest(ref_vertices, our_vertices)
        surface = max(surface_distances(ref_vertices, our_triangles)) if our_triangles else math.inf
        relative = abs(our_volume - ref_volume) / ref_volume if ref_volume > 0 else math.inf
        row = tally[source_of[tag]]
        row["both"] += 1
        row["match"] += distance <= VERTEX_TOLERANCE_FT and relative <= VOLUME_TOLERANCE
        row["on surface"] += surface <= VERTEX_TOLERANCE_FT
        if not (distance <= VERTEX_TOLERANCE_FT and relative <= VOLUME_TOLERANCE):
            differs.append((tag, source_of[tag], distance, surface, our_volume, ref_volume))
    for source in SOURCES:
        drawn = sum(1 for s in source_of.values() if s == source)
        row = tally[source]
        print(
            f"{source}: {drawn} columns; in Revit's export {row['both']}; vertices within {VERTEX_TOLERANCE_FT} ft "
            f"and volume within {VOLUME_TOLERANCE:g}: {row['match']}; every Revit vertex on rvt-rs's surface: {row['on surface']}"
        )
    for tag, source, distance, surface, our_volume, ref_volume in differs[:12]:
        print(
            f"  {tag} ({source}): farthest {distance:.3g} ft, surface {surface:.3g} ft, "
            f"volume rvt-rs {our_volume:.6f} ft3, Revit {ref_volume:.6f} ft3"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
