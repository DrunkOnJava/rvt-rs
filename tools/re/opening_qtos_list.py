#!/usr/bin/env python3
"""Which of Revit's opening base quantities an opening's body gives (#1016, W7).

For every IfcOpeningElement Revit's export gives
`Qto_OpeningElementBaseQuantities`, keyed by the Tag of the door or window
filling it, else by its own Tag (as `opening_boxes_vs_ifc.py` keys them), it
compares Revit's Width, Height and Depth with the extents of its own body's
extruded profile and the extrusion's depth, and with rvt-rs's opening of the
same key. Each line counts the equal ones (a relative 1e-5, or 1e-6
absolute in SI units), split by whether the opening is filled, and lists up
to 8 that are not.

Usage:

    python3 tools/re/opening_qtos_list.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.util.unit

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wall_qtos_list import points_of, solids  # noqa: E402

NAMES = ("Width", "Height", "Depth")


def extents(p, scale):
    if p.is_a("IfcRectangleProfileDef"):
        return p.XDim * scale, p.YDim * scale
    if p.is_a("IfcArbitraryClosedProfileDef"):
        pts = points_of(p.OuterCurve, scale)
        if pts:
            xs = [x for x, _ in pts]
            ys = [y for _, y in pts]
            return max(xs) - min(xs), max(ys) - min(ys)
    return None, None


def openings(f):
    """key -> {'filled': bool, 'body': (x extent, y extent, depth) or None, 'qto': {name: SI value}}."""
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    lengths = ifcopenshell.util.unit.calculate_unit_scale(f, "LENGTHUNIT")
    keys = {}
    for rel in f.by_type("IfcRelFillsElement"):
        if rel.RelatedBuildingElement.Tag:
            keys[rel.RelatingOpeningElement.id()] = (f"tag:{rel.RelatedBuildingElement.Tag}", True)
    out = {}
    for opening in f.by_type("IfcOpeningElement"):
        key, filled = keys.get(opening.id(), (f"tag:{opening.Tag}" if opening.Tag else None, False))
        if key is None:
            continue
        body = None
        if opening.Representation is not None:
            for rep in opening.Representation.Representations:
                if rep.RepresentationIdentifier != "Body":
                    continue
                found = [s for item in rep.Items for s in solids(item)]
                if len(found) == 1:
                    x, y = extents(found[0].SweptArea, scale)
                    if x is not None:
                        body = (x, y, found[0].Depth * scale)
        qto = {}
        for rel in getattr(opening, "IsDefinedBy", None) or ():
            if not rel.is_a("IfcRelDefinesByProperties"):
                continue
            d = rel.RelatingPropertyDefinition
            if d.is_a("IfcElementQuantity") and d.Name == "Qto_OpeningElementBaseQuantities":
                for q in d.Quantities or ():
                    if q.is_a("IfcQuantityLength") and q.Name in NAMES:
                        qto[q.Name] = float(q.LengthValue) * lengths
        out.setdefault(key, {"filled": filled, "body": body, "qto": qto})
    return out


def same(a, b):
    return a is not None and b is not None and abs(a - b) <= max(1e-6, 1e-5 * max(abs(a), abs(b)))


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours_f, theirs_f = ifcopenshell.open(args[0]), ifcopenshell.open(args[1])
    mine, theirs = openings(ours_f), openings(theirs_f)
    rows = {k: v for k, v in theirs.items() if v["qto"]}
    print(f"openings with Qto_OpeningElementBaseQuantities in Revit's export: {len(rows)}; rvt-rs writes an opening for {sum(k in mine for k in rows)}")
    sources = [
        ("Revit's body", lambda k: theirs[k]["body"]),
        ("rvt-rs's body", lambda k: mine.get(k, {}).get("body")),
    ]
    for label, body in sources:
        for i, name in enumerate(NAMES):
            for swapped in (False, True):
                if swapped and i == 2:
                    continue
                j = (1 - i) if swapped and i < 2 else i
                tally = collections.Counter()
                misses = []
                for k in sorted(rows):
                    q = rows[k]["qto"].get(name)
                    b = body(k)
                    if q is None or b is None:
                        continue
                    ok = same(q, b[j])
                    tally[(rows[k]["filled"], ok)] += 1
                    if not ok and len(misses) < 8:
                        misses.append(f"{k}: Revit {q:.6g}, body {b[0]:.6g} x {b[1]:.6g} x {b[2]:.6g}")
                axis = ("x", "y", "depth")[j]
                total = sum(tally.values())
                equal = sum(n for (_, ok), n in tally.items() if ok)
                split = ", ".join(f"{'filled' if f else 'unfilled'} {'equal' if ok else 'not'} x{n}" for (f, ok), n in sorted(tally.items()))
                print(f"{name} = {label} {axis}: {equal} of {total} ({split})")
                if not swapped:
                    for line in misses:
                        print(f"    {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
