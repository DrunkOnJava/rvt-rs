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

    # IFC2X3 storeys carry no Tag: join them to the file's Levels by name.
    levels = {}
    for line in open(args.probe, encoding="utf-8", errors="replace"):
        if line.startswith("level "):
            _, ident, elevation, name = line.rstrip("\n").split(" ", 3)
            levels.setdefault(name, int(ident))
    print(f"levels: {sorted((n, i) for n, i in levels.items())}")
    storey_of = {}
    for rel in model.by_type("IfcRelContainedInSpatialStructure"):
        storey = rel.RelatingStructure
        if storey.is_a("IfcBuildingStorey"):
            for element in rel.RelatedElements:
                tag = getattr(element, "Tag", None)
                if tag and str(tag).isdigit():
                    storey_of[int(tag)] = (storey.Name, levels.get(storey.Name, "?"), storey.Elevation)
    print(f"storeys: {sorted({(s.Name, s.Elevation) for s in model.by_type('IfcBuildingStorey')}, key=lambda x: x[1] or 0)}")
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
        named = [f for f, v in fields.items() if level_tag != "?" and re.search(rf"#{level_tag}\b", v)]
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
    near = collections.Counter()
    for line in open(args.probe, encoding="utf-8", errors="replace"):
        if line.startswith("near "):
            _, cls, ident, level, rel, width = line.split()
            near[(cls, level, int(rel), width)] += 1
    print("Level ids near the records (class, Level, offset from the record, width): count")
    for (cls, level, rel, width), n in sorted(near.items(), key=lambda kv: -kv[1])[:20]:
        print(f"  {cls} {level} {rel:+d} {width}: {n}")
    lists = collections.defaultdict(list)
    for line in open(args.probe, encoding="utf-8", errors="replace"):
        if line.startswith("list "):
            parts = line.split(" ", 4)
            levels_named = line.rsplit("levels=", 1)[1].strip()
            lists[(parts[1], int(parts[2]))].append(levels_named)
    by_class = collections.Counter()
    for (cls, ident), named in lists.items():
        distinct = {n for n in named if n != "[]"}
        verdict = "no Level" if not distinct else "one Level" if len(distinct) == 1 and len(eval(next(iter(distinct)))) == 1 else "several"
        exported_storey = storey_of.get(ident)
        right = ""
        if verdict == "one Level" and exported_storey and exported_storey[1] != "?":
            right = " (Revit's)" if str(exported_storey[1]) in next(iter(distinct)) else " (not Revit's)"
        by_class[(cls, verdict + right)] += 1
    print("0x04b9 lists naming a Level, by element: " + ", ".join(f"{c} {v} {n}" for (c, v), n in sorted(by_class.items())))
    print("referenced ids, where they appear:")
    for line in open(args.probe, encoding="utf-8", errors="replace"):
        if line.startswith("ref "):
            print("  " + line.rstrip()[:200])


if __name__ == "__main__":
    main()
