#!/usr/bin/env python3
"""Each covering's base quantities against rvt-rs's body (#1016, W7).

For every IfcCovering both files write (by Tag) it prints Revit's
PredefinedType and type name, Revit's `Qto_CoveringBaseQuantities` `Width`
and `GrossArea`, and rvt-rs's `BodySource`, body depth and profile area, and
tallies by predefined type and type name whether `Width` is the body's depth
and `GrossArea` its profile's area (a relative 1e-5, or 1e-6 absolute in SI
units).

Usage:

    python3 tools/re/covering_qtos_list.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import os
import sys

import ifcopenshell

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qto_candidates_list import candidates, same  # noqa: E402
from qtos_vs_ifc import quantities  # noqa: E402


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    sources = {}
    ours = candidates(ifcopenshell.open(args[0]), sources)
    revit_f = ifcopenshell.open(args[1])
    theirs = collections.defaultdict(dict)
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == "Qto_CoveringBaseQuantities" and value is not None:
            theirs[key][name] = value
    tally = collections.Counter()
    rows = []
    for c in revit_f.by_type("IfcCovering"):
        key = f"tag:{c.Tag}" if c.Tag else None
        if key not in ours:
            continue
        q = theirs.get(key, {})
        b = ours[key]
        kind = (c.PredefinedType or "-", (getattr(c, "ObjectType", None) or "-").replace(" ", "_"))
        width_ok = same(q.get("Width"), b["depth"])
        area_ok = same(q.get("GrossArea"), b["profile area"])
        tally[(kind, sources.get(key, "-"), width_ok, area_ok)] += 1
        rows.append((key, kind, sources.get(key, "-"), q.get("Width"), b["depth"], q.get("GrossArea"), b["profile area"]))
    print("predefined type, type, rvt-rs BodySource, Width = depth, GrossArea = profile area: coverings")
    for (kind, source, w, a), n in sorted(tally.items(), key=lambda kv: -kv[1]):
        print(f"  {kind[0]} {kind[1]} [{source}] width={w} area={a}: {n}")
    print("every covering:")
    for key, kind, source, w, d, a, pa in rows:
        print(f"  {key} {kind[0]} {kind[1]} [{source}] Revit Width {w} depth {d:.6g}; Revit GrossArea {a} profile area {pa:.6g}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
