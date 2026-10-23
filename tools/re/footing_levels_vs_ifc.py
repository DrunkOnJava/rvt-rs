#!/usr/bin/env python3
"""Which field of a Revit 2015 footing's record names the Level Revit's export
puts it on (RE-209, #879).

Takes the output of `examples/probe_re209_footing_levels_2015.rs` (every
decoded field of the file's footings, air terminals, Levels and first
structural columns) and Revit's export. Revit writes a storey's `Tag` as its
Level's ElementId, so for every footing and column in the export the scorer
looks for that ElementId among the element's fields and reports which field
holds it, ranked by how many elements agree. It also lists each air terminal
the probe saw, whether Revit's export has it, and the fields that set the
exported ones apart from the others.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/footing_levels_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import argparse
import collections
import re

import ifcopenshell


def read_probe(path):
    out = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        if not line.startswith("el "):
            continue
        _, cls, ident, rest = (line.rstrip("\n").split(" ", 3) + [""])[:4]
        fields = {}
        for part in rest.split("; "):
            name, _, value = part.partition("=")
            if name:
                fields[name] = value
        if ident.isdigit():
            out[int(ident)] = (cls, fields)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("probe")
    ap.add_argument("reference")
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args()
    probe = read_probe(args.probe)
    model = ifcopenshell.open(args.reference)

    storey_of = {}
    for rel in model.by_type("IfcRelContainedInSpatialStructure"):
        storey = rel.RelatingStructure
        if storey.is_a("IfcBuildingStorey"):
            for element in rel.RelatedElements:
                if element.Tag and str(element.Tag).isdigit():
                    storey_of[int(element.Tag)] = (storey.Name, storey.Tag, storey.Elevation)
    exported = {
        int(p.Tag): p.is_a()
        for p in model.by_type("IfcProduct")
        if getattr(p, "Tag", None) and str(p.Tag).isdigit()
    }

    print(f"probe elements: {collections.Counter(c for c, _ in probe.values())}")
    holders = collections.Counter()
    unexplained = []
    for ident, (cls, fields) in sorted(probe.items()):
        if cls not in ("StructuralFoundation", "StructuralColumn") or ident not in storey_of:
            continue
        name, level_tag, elevation = storey_of[ident]
        named = [f for f, v in fields.items() if re.search(rf"#{level_tag}\b", v)]
        for f in named:
            holders[(cls, f)] += 1
        if not named:
            unexplained.append((cls, ident, name, level_tag))
        if args.list:
            levelish = {f: v for f, v in fields.items() if re.search("level|constraint|reference|host|elev", f, re.I)}
            print(f"  {cls} {ident}: Revit storey {name!r} (Level {level_tag}, {elevation}); named by {named or 'nothing'}; {levelish}")
    print("fields naming the Level of the element's storey in Revit's export:")
    for (cls, f), n in holders.most_common(12):
        print(f"  {cls} {f}: {n}")
    print(f"exported elements whose fields name no such Level: {len(unexplained)}")
    for item in unexplained[:30]:
        print(f"  {item}")

    terminals = {i: f for i, (c, f) in probe.items() if c == "DuctTerminal"}
    print(f"air terminals: {len(terminals)}, in Revit's export: {sum(1 for i in terminals if i in exported)}")
    for ident, fields in sorted(terminals.items()):
        print(f"  {ident} {'exported' if ident in exported else 'left out'}: {fields}")


if __name__ == "__main__":
    main()
