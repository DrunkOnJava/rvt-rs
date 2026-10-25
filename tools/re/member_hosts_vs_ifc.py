#!/usr/bin/env python3
"""Curtain mullions' and panels' `IsExternal` and `Span` against Revit's
export (RE-226, #35, W7).

Reads `examples/probe_re226_mullion_hosts.rs`'s output and Revit's IFC
export. For every `IfcMember` and `IfcPlate` joined by Tag:

- `IsExternal`: Revit's value set against what the probe's record names:
  revit-ifc's rule (CLEANROOM.md source 5) predicts the host's own
  `IsExternal` in Revit's export where the record names a host, and false
  where it names none. Counted per record class and host class.
- `Span` (`Pset_MemberCommon`): Revit's value against the depth of the
  `IfcExtrudedAreaSolid` its own body is (revit-ifc's `SpanCalculator`
  falls back to the body's extrusion length), and, where the rvt-rs export
  is beside the probe (`model.ifc`, Measure's `keep_ifc`), against the
  longest side of the box rvt-rs draws.

Usage:

    python3 tools/re/member_hosts_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.unit

FEET = 0.3048
TOLERANCE_FEET = 1e-3


def read_probe(path):
    rows = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 6 or parts[0] != "member":
            continue
        fields = dict(p.split("=", 1) for p in parts[4:] if "=" in p)
        host = fields.get("host", "-")
        rows[parts[1]] = {
            "ifc": parts[2],
            "record": parts[3],
            "host": None if host in ("-", "-1") else host,
            "hostclass": fields.get("hostclass", "-"),
            "fields": " ".join(parts[6:]),
        }
    return rows


def is_external(element):
    for props in ifcopenshell.util.element.get_psets(element, psets_only=True).values():
        if "IsExternal" in props:
            return props["IsExternal"]
    return None


def extrusion_depths(element, scale):
    depths = []
    if not element.Representation:
        return depths
    stack = list(element.Representation.Representations)
    while stack:
        rep = stack.pop()
        for item in rep.Items:
            if item.is_a("IfcExtrudedAreaSolid"):
                depths.append(item.Depth * scale)
            elif item.is_a("IfcMappedItem"):
                stack.append(item.MappingSource.MappedRepresentation)
    return depths


def box_sides(element, scale):
    sides = []
    for depth_item in element.Representation.Representations if element.Representation else []:
        for item in depth_item.Items:
            if item.is_a("IfcExtrudedAreaSolid") and item.SweptArea.is_a("IfcRectangleProfileDef"):
                sides.append(
                    max(item.SweptArea.XDim * scale, item.SweptArea.YDim * scale, item.Depth * scale)
                )
    return sides


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = read_probe(args[0])
    revit = ifcopenshell.open(args[1])
    scale = ifcopenshell.util.unit.calculate_unit_scale(revit) / FEET
    external_by_tag = {}
    for element in revit.by_type("IfcElement"):
        if element.Tag:
            value = is_external(element)
            if value is not None:
                external_by_tag[str(element.Tag)] = value
    members = {}
    for entity in ("IfcMember", "IfcPlate"):
        for element in revit.by_type(entity):
            if element.Tag and str(element.Tag) in rows:
                psets = ifcopenshell.util.element.get_psets(element, psets_only=True)
                span = psets.get("Pset_MemberCommon", {}).get("Span")
                members[str(element.Tag)] = (
                    entity,
                    external_by_tag.get(str(element.Tag)),
                    None if span is None else span * scale,
                    extrusion_depths(element, scale),
                )
    print(f"probe: {len(rows)} members and plates; in Revit's export by Tag: {len(members)}")
    table = collections.Counter()
    agree = collections.Counter()
    for tag, (entity, revit_external, _, _) in members.items():
        row = rows[tag]
        has_host = row["host"] is not None
        key = (entity, row["record"], row["hostclass"] if has_host else "no host", revit_external)
        table[key] += 1
        if revit_external is None:
            continue
        predicted = external_by_tag.get(row["host"], False) if has_host else False
        agree[(entity, predicted == revit_external)] += 1
    print("IsExternal by record class and host (class, record, host class, Revit's value): count")
    for key, n in sorted(table.items(), key=lambda kv: -kv[1]):
        print(f"  {key[0]:<9} {key[1]:<22} {key[2]:<22} {str(key[3]):<6} {n}")
    for entity in ("IfcMember", "IfcPlate"):
        print(
            f"{entity}: host rule gives Revit's IsExternal on {agree[(entity, True)]}"
            f" of {agree[(entity, True)] + agree[(entity, False)]}"
        )
    ours_path = os.path.join(os.path.dirname(os.path.abspath(args[0])), "model.ifc")
    ours_sides = {}
    if os.path.exists(ours_path):
        ours = ifcopenshell.open(ours_path)
        our_scale = ifcopenshell.util.unit.calculate_unit_scale(ours) / FEET
        for element in ours.by_type("IfcMember"):
            if element.Tag:
                sides = box_sides(element, our_scale)
                if sides:
                    ours_sides[str(element.Tag)] = max(sides)
    spans = collections.Counter()
    for tag, (entity, _, span, depths) in members.items():
        if span is None:
            continue
        spans["with Span"] += 1
        if depths and any(abs(d - span) <= TOLERANCE_FEET for d in depths):
            spans["Span = Revit's extrusion depth"] += 1
        if tag in ours_sides:
            spans["rvt-rs box"] += 1
            spans["Span = rvt-rs box's longest side"] += abs(ours_sides[tag] - span) <= TOLERANCE_FEET
    print("Span: " + ", ".join(f"{k} {v}" for k, v in spans.items()))
    if "--list" in sys.argv:
        # Every member the host rule gets wrong, up to 300, then 50 it gets right.
        shown = collections.Counter()
        for tag, (entity, revit_external, span, depths) in sorted(members.items(), key=lambda kv: (len(kv[0]), kv[0])):
            row = rows[tag]
            predicted = external_by_tag.get(row["host"], False) if row["host"] else False
            kind = "right" if predicted == revit_external else "wrong"
            shown[kind] += 1
            if shown[kind] > (300 if kind == "wrong" else 50):
                continue
            span_text = "-" if span is None else f"{span:.4f}"
            depth_text = ",".join(f"{d:.4f}" for d in depths) or "-"
            print(
                f"row {kind} {tag} {entity} external={revit_external} host={row['host'] or '-'}"
                f" hostexternal={external_by_tag.get(row['host']) if row['host'] else '-'}"
                f" span={span_text} depths={depth_text} ours={ours_sides.get(tag, '-')} {row['fields']}"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
