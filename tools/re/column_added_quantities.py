#!/usr/bin/env python3
"""For the columns whose base quantities one export adds to another's, check that each is what the rule says
(#1016, RE-424): the extent, the area of one end, the lateral area or the volume of the prism the column's body is
along the vertical.

For models no Revit export scores (Autodesk's samples), where a change that writes a value cannot be read against
Revit's: it reads the two files' kept exports, pairs the `IfcColumn`s by Tag, takes each that holds a quantity of
`Qto_ColumnBaseQuantities` in the second that the first lacks, reads the column's body with the scorer's rule
(`column_quantities_vs_ifc.rule`: a prism along its longest axis with two flat ends of one area, whose axis is the
vertical) and prints, per added quantity, whether it is the value the rule gives, and the first few columns where it
is not. It then groups the columns of the second export that the rule accepts and that were not added to by the
source of their body, what they hold of the four and whether they hold NetVolume, which the pass writes from a
closed mesh of flat faces: a column drawn from a saved mesh that holds NetVolume and none of the four is one the
pass should have written and did not. The rule here cannot see curved faces, so it accepts a section with fillets
or a round column, whose mesh the pass leaves alone (no NetVolume, none of the four).

Usage:

    python3 tools/re/column_added_quantities.py <before.ifc> <after.ifc>

Needs IfcOpenShell and numpy.
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import column_quantities_vs_ifc as columns  # noqa: E402
from mullion_profiles_vs_ifc import close, fmt, mesh, scales  # noqa: E402
from saved_meshes_vs_ifc import body_sources  # noqa: E402

LISTED = 8


def accepted(found, how):
    """Whether the rule gives the column quantities: a prism the beam rule accepts whose axis is the vertical."""
    return found is not None and "not vertical" not in how


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    before, after = ifcopenshell.open(sys.argv[1]), ifcopenshell.open(sys.argv[2])
    scale_before, scale_after = scales(before), scales(after)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    held = {str(c.Tag): c for c in before.by_type("IfcColumn") if c.Tag}
    tally, rest, odd, changed = collections.Counter(), collections.Counter(), [], 0
    for column in after.by_type("IfcColumn"):
        if not column.Tag:
            continue
        tag = str(column.Tag)
        now = columns.column_values(column, scale_after)
        was = columns.column_values(held[tag], scale_before) if tag in held else None
        added = [q for q in columns.FOUR if now[q] is not None and (was is None or was[q] is None)]
        found, how = columns.rule(mesh(settings, column))
        if not added:
            if accepted(found, how):
                # The rule gives this column quantities and the export did not add them: it already held them, or
                # the pass does not read its body (a record box, a hollow section), or the set is a cut column's.
                present = sum(now[q] is not None for q in columns.FOUR)
                # NetVolume is written from a closed mesh of flat faces: a column that holds it and is a saved
                # mesh the rule accepts, with nothing added, is one the pass should have written.
                volume = "NetVolume held" if now["NetVolume"] is not None else "no NetVolume"
                rest[(body_sources(column) or "no BodySource", f"holds {present} of the four", volume)] += 1
            continue
        changed += 1
        tally[("added set", " + ".join(added))] += 1
        if not accepted(found, how):
            tally[("the rule gives this column nothing", how)] += len(added)
            odd.append((tag, column.Name, now, found, how))
            continue
        wrong = [q for q in added if not close(found[q], now[q])]
        for quantity in added:
            tally[(quantity, "not the rule's value" if quantity in wrong else "the value the rule gives")] += 1
        if wrong:
            odd.append((tag, column.Name, now, found, how))
    print(f"columns with a base quantity the first export lacks: {changed}")
    for key, count in sorted(tally.items()):
        print(f"  {count:5}  {key}")
    print("columns the rule accepts that the export did not add to (rvt-rs's body source, what the column holds, NetVolume):")
    for key, count in sorted(rest.items()):
        print(f"  {count:5}  {key}")
    for tag, name, now, found, how in odd[:LISTED]:
        print(f"  not the rule's values: column {tag} {name!r} ({how})")
        print("      export " + " | ".join(f"{q} {fmt(now[q])}" for q in columns.FOUR))
        if found is not None:
            print("      rule   " + " | ".join(f"{q} {fmt(found[q])}" for q in columns.FOUR))
    return 0


if __name__ == "__main__":
    sys.exit(main())
