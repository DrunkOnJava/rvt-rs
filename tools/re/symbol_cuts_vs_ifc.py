#!/usr/bin/env python3
"""Score a family symbol's cut loop against Revit's door and window sizes.

Research tool for RE-213 (#810, #844). `examples/probe_re213_symbol_cuts.rs`
prints each `FamilySymbol` whose `m_pCutLoops` is not empty, the objects of
its graph (the loop's `GLine`s and `GArc`s among them), and each
`FamilyInstance`'s ElementId-valued fields. This scorer takes each cut
loop's extents along the symbol's X and Z axes and compares them with the
`OverallWidth` and `OverallHeight` Revit's export gives each door and window,
within 0.001 ft, two ways:

- the loop of the element's type (the `Tag` of its `IfcDoorType` or
  `IfcWindowType`);
- the loop of any cut symbol the element's own `FamilyInstance` record
  names, or that the instance owns, counted per field name.

Usage:

    python3 tools/re/symbol_cuts_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import json
import math
import re
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.unit

FEET = 0.3048
TOLERANCE_FEET = 1e-3
NO_TOKEN = 4294967295


def curve_points(cls, f):
    if cls == "GLine":
        o, d, (a, b) = f["m_origin"], f["m_dirVec"], f["m_endParams"]
        return [[o[i] + d[i] * t for i in range(3)] for t in (a, b)]
    if cls == "GArc":
        c, r, x, y, (a, b) = f["m_center"], f["m_radius"], f["m_xVec"], f["m_yVec"], f["m_endParams"]
        steps = 64
        return [
            [c[i] + r * (math.cos(a + (b - a) * k / steps) * x[i] + math.sin(a + (b - a) * k / steps) * y[i])
             for i in range(3)]
            for k in range(steps + 1)
        ]
    return None


def read_probe(path):
    """Symbol -> (owner, loop extents (x, y, z) or None, curve classes), and
    instance -> (owner, {field: [ElementIds]})."""
    cuts, objects, instances = {}, collections.defaultdict(list), {}
    for line in open(path):
        if line.startswith("cut "):
            _, symbol, _family, owner, _fields = line.split(" ", 4)
            cuts[symbol] = int(owner)
        elif line.startswith("object "):
            _, symbol, cls, _tag, _start, token, fields = line.split(" ", 6)
            objects[symbol].append((cls, int(token), json.loads(fields)))
        elif line.startswith("instance "):
            _, instance, owner, fields = line.split(" ", 3)
            ids = {
                name: [int(v) for v in re.findall(r'"m_id64":\s*(-?\d+)', json.dumps(value))]
                for name, value in json.loads(fields).items()
            }
            instances[instance] = (int(owner), ids)
    symbols = {}
    for symbol, owner in cuts.items():
        by_token = {token: (cls, f) for cls, token, f in objects[symbol] if token != NO_TOKEN}
        loops = [f for cls, _, f in objects[symbol] if cls == "CurveLoop"]
        extents, classes = None, []
        if len(loops) == 1:
            points = []
            for curve in loops[0].get("m_curves", []):
                cls, f = by_token.get(curve.get("pointer_token"), (None, None))
                classes.append(cls)
                p = curve_points(cls, f) if f else None
                if p is None:
                    points = None
                    break
                points += p
            if points:
                extents = tuple(max(p[i] for p in points) - min(p[i] for p in points) for i in range(3))
        symbols[symbol] = (owner, extents, classes)
    return symbols, instances


def revit_sizes(path):
    """Tag -> (class, type Tag, OverallWidth ft, OverallHeight ft)."""
    f = ifcopenshell.open(path)
    metres = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for entity in ("IfcDoor", "IfcWindow"):
        for element in f.by_type(entity):
            if not element.Tag or element.OverallWidth is None or element.OverallHeight is None:
                continue
            kind = ifcopenshell.util.element.get_type(element)
            out[str(element.Tag)] = (
                entity,
                str(kind.Tag) if kind is not None and kind.Tag else None,
                element.OverallWidth * metres / FEET,
                element.OverallHeight * metres / FEET,
            )
    return out


def equal(extents, width, height):
    return (
        extents is not None
        and abs(extents[0] - width) <= TOLERANCE_FEET
        and abs(extents[2] - height) <= TOLERANCE_FEET
    )


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    symbols, instances = read_probe(args[0])
    revit = revit_sizes(args[1])
    owned = collections.defaultdict(list)
    for symbol, (owner, _, _) in symbols.items():
        owned[str(owner)].append(symbol)
    print(f"probe: {len(symbols)} symbols with a cut loop, "
          f"{sum(1 for s in symbols.values() if s[1] is not None)} read; {len(instances)} family instances")
    print(f"Revit's export: {len(revit)} doors and windows with OverallWidth and OverallHeight")

    tally = collections.Counter()
    by_field = collections.Counter()
    misses = []
    for tag, (entity, type_tag, width, height) in sorted(revit.items()):
        tally[(entity, "elements")] += 1
        type_cut = symbols.get(type_tag)
        if type_cut is None:
            tally[(entity, "type has no cut loop")] += 1
        elif equal(type_cut[1], width, height):
            tally[(entity, "type's loop equal")] += 1
        else:
            tally[(entity, "type's loop differs")] += 1
        own = []
        _, fields = instances.get(tag, (None, {}))
        for name, ids in fields.items():
            for i in ids:
                if str(i) in symbols and str(i) != type_tag:
                    own.append((name, str(i)))
        own += [("owned", s) for s in owned.get(tag, [])]
        matched = False
        for name, symbol in own:
            if equal(symbols[symbol][1], width, height):
                by_field[(entity, name, "equal")] += 1
                matched = True
            else:
                by_field[(entity, name, "differs")] += 1
        if not own:
            tally[(entity, "instance names no other cut symbol")] += 1
        if not matched and not equal(type_cut[1] if type_cut else None, width, height):
            misses.append((tag, entity, type_tag, width, height, own))

    for entity in sorted({e for e, _ in tally}):
        print(f"\n{entity}: {tally[(entity, 'elements')]}")
        for label in ("type's loop equal", "type's loop differs", "type has no cut loop",
                      "instance names no other cut symbol"):
            print(f"  {label:38} {tally[(entity, label)]}")
        for (e, name, verdict), count in sorted(by_field.items()):
            if e == entity:
                print(f"  instance's {name} -> cut symbol, loop {verdict:7} {count}")
    print(f"\n{len(misses)} with no cut loop equal to Revit's size")
    if listing:
        for tag, entity, type_tag, width, height, own in misses[:60]:
            t = symbols.get(type_tag)
            text = ", ".join(f"{name}={s} {symbols[s][1]}" for name, s in own)
            print(f"  {entity} {tag} type {type_tag}: Revit {width:.4f} x {height:.4f} ft; "
                  f"type loop {t[1] if t else None}; own: {text or '-'}")


main()
