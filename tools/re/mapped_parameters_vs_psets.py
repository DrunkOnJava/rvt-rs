#!/usr/bin/env python3
"""Which saved parameter is each value Revit's export writes on walls,
curtain walls, coverings, members, plates and spaces (RE-239, #35, W7).

Reads `probes/examples/probe_re239_mapped_parameters.rs`'s output and Revit's IFC
export. Elements are matched by Tag; a space by its Name, the room number,
or the ElementId after the last `:` of the Name of a room with no number.
For each boolean or text property Revit writes, it counts, per parameter,
how many of those elements hold it and how many hold Revit's value, and
prints the three equal most often.

Usage:

    python3 tools/re/mapped_parameters_vs_psets.py <probe.txt> <revit.ifc>
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element

ROOM_NUMBER = "-1006901:-"


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = {}
    for line in open(args[0], encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 3 or parts[0] != "par":
            continue
        values = {}
        for part in parts[3:]:
            key, _, value = part.partition("=")
            values[key] = value.replace("_", " ")
        rows[parts[1]] = (parts[2], values)
    # A space has no Tag: Revit's export names it by its number (Revit's
    # parameter -1006901), or by `<name>:<ElementId>` where it has none.
    by_number = {
        values[ROOM_NUMBER]: tag
        for tag, (cls, values) in rows.items()
        if cls == "IFCSPACE" and values.get(ROOM_NUMBER)
    }
    f = ifcopenshell.open(args[1])
    tally = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    totals = collections.Counter()
    for element in f.by_type("IfcElement") + f.by_type("IfcSpace"):
        if element.is_a("IfcSpace"):
            name = element.Name or ""
            tag = by_number.get(name) or name.rpartition(":")[2]
        else:
            tag = getattr(element, "Tag", None)
        if tag not in rows:
            continue
        cls, values = rows[tag]
        for pset, props in ifcopenshell.util.element.get_psets(element, psets_only=True).items():
            for prop, value in props.items():
                if prop == "id" or not isinstance(value, (bool, str)):
                    continue
                key = (cls, f"{pset}.{prop}")
                totals[key] += 1
                text = str(value).lower() if isinstance(value, bool) else value
                for param, held in values.items():
                    counts = tally[key][param]
                    counts["n"] += 1
                    if held == text:
                        counts["equal"] += 1
    print(f"probe: {len(rows)} elements")
    for key in sorted(totals, key=lambda k: (-totals[k], k)):
        ranked = sorted(tally[key].items(), key=lambda kv: (-kv[1]["equal"], kv[0]))[:3]
        text = "  ".join(f"{p} {c['n']}/{c['equal']}" for p, c in ranked if c["equal"])
        print(f"  {key[0]:<16} {key[1]:<60} {totals[key]:>5}  {text or '-'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
