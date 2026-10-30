#!/usr/bin/env python3
"""Score a family symbol's cut loop against Revit's door and window sizes.

Research tool for RE-215 (#810, #844). `probes/examples/probe_re215_symbol_cuts.rs`
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

It then takes the corners of the `IfcOpeningElement` each element fills, in
that element's own placement, and compares their X and Z ranges with the
matched loop's, and their Y range (across the host) with the loop's plane,
for #227 (an opening cut from the loop rather than the element's box).

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
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

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
    """Symbol -> (owner, loop (min, max) corners or None, curve classes),
    instance -> (owner, {field: [ElementIds]}), and symbol -> its root's
    ElementId-valued fields."""
    cuts, objects, instances, refs = {}, collections.defaultdict(list), {}, {}
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
        elif line.startswith("deep "):
            _, instance, found = line.split(" ", 2)
            ids = collections.defaultdict(list)
            for cls, name, value in json.loads(found):
                ids[f"{cls}.{name}"].append(int(value))
            owner = instances.get(instance, (None, {}))[0]
            instances[instance] = (owner, dict(ids))
        elif line.startswith("symbolrefs "):
            _, symbol, fields = line.split(" ", 2)
            refs[symbol] = json.loads(fields)
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
                extents = (
                    tuple(min(p[i] for p in points) for i in range(3)),
                    tuple(max(p[i] for p in points) for i in range(3)),
                )
        symbols[symbol] = (owner, extents, classes)
    return symbols, instances, refs


def revit_sizes(path):
    """Tag -> (class, type Tag, OverallWidth ft, OverallHeight ft, the
    filled opening's (min, max) corners in the element's placement, ft)."""
    f = ifcopenshell.open(path)
    metres = ifcopenshell.util.unit.calculate_unit_scale(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    fills = {rel.RelatedBuildingElement.id(): rel.RelatingOpeningElement for rel in f.by_type("IfcRelFillsElement")}

    def opening_box(element):
        opening = fills.get(element.id())
        if opening is None or element.ObjectPlacement is None:
            return None
        try:
            shape = ifcopenshell.geom.create_shape(settings, opening)
        except Exception:
            return None
        verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        if not len(verts):
            return None
        frame = np.array(ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement), dtype=float)
        frame[:3, 3] *= metres
        local = (np.linalg.inv(frame) @ np.c_[verts, np.ones(len(verts))].T).T[:, :3] / FEET
        return local.min(axis=0), local.max(axis=0)

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
                opening_box(element),
            )
    return out


def equal(loop, width, height):
    return (
        loop is not None
        and abs(loop[1][0] - loop[0][0] - width) <= TOLERANCE_FEET
        and abs(loop[1][2] - loop[0][2] - height) <= TOLERANCE_FEET
    )


def band(diff):
    return next((f"within {limit} ft" for limit in (0.001, 0.01, 0.1, 0.26, 1.0) if diff <= limit), "over 1 ft")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    symbols, instances, refs = read_probe(args[0])
    revit = revit_sizes(args[1])
    owned = collections.defaultdict(list)
    for symbol, (owner, _, _) in symbols.items():
        owned[str(owner)].append(symbol)
    print(f"probe: {len(symbols)} symbols with a cut loop, "
          f"{sum(1 for s in symbols.values() if s[1] is not None)} read; {len(instances)} family instances")
    print(f"Revit's export: {len(revit)} doors and windows with OverallWidth and OverallHeight")

    tally = collections.Counter()
    by_field = collections.Counter()
    placed = collections.Counter()
    across = collections.Counter()
    misses, worst = [], []
    for tag, (entity, type_tag, width, height, box) in sorted(revit.items()):
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
        matched = None
        for name, symbol in own:
            if equal(symbols[symbol][1], width, height):
                by_field[(entity, name, "equal")] += 1
                matched = matched or symbol
            else:
                by_field[(entity, name, "differs")] += 1
        if matched is None and type_cut is not None and equal(type_cut[1], width, height):
            matched = type_tag
        if matched is not None and box is not None:
            lo, hi = symbols[matched][1]
            diff = max(abs(box[0][0] - lo[0]), abs(box[1][0] - hi[0]), abs(box[0][2] - lo[2]), abs(box[1][2] - hi[2]))
            placed[(entity, band(diff))] += 1
            plane = lo[1]
            across[(entity, "loop plane inside the opening" if box[0][1] - 1e-3 <= plane <= box[1][1] + 1e-3
                    else "loop plane outside the opening")] += 1
            worst.append((diff, tag, entity, matched, [round(float(v), 4) for v in (*box[0], *box[1])],
                          [round(v, 4) for v in (*lo, *hi)]))
        if not own:
            tally[(entity, "instance names no other cut symbol")] += 1
        if matched is None:
            misses.append((tag, entity, type_tag, width, height, own))

    for entity in sorted({e for e, _ in tally}):
        print(f"\n{entity}: {tally[(entity, 'elements')]}")
        for label in ("type's loop equal", "type's loop differs", "type has no cut loop",
                      "instance names no other cut symbol"):
            print(f"  {label:38} {tally[(entity, label)]}")
        for (e, name, verdict), count in sorted(by_field.items()):
            if e == entity:
                print(f"  instance's {name} -> cut symbol, loop {verdict:7} {count}")
        for (e, label), count in sorted(placed.items()):
            if e == entity:
                print(f"  opening's X and Z ranges against the matched loop's, {label}: {count}")
        for (e, label), count in sorted(across.items()):
            if e == entity:
                print(f"  {label}: {count}")
    print(f"\n{len(misses)} with no cut loop equal to Revit's size")
    if listing:
        for tag, entity, type_tag, width, height, own in misses[:60]:
            t = symbols.get(type_tag)
            text = ", ".join(f"{name}={s} {symbols[s][1]}" for name, s in own)
            print(f"  {entity} {tag} type {type_tag}: Revit {width:.4f} x {height:.4f} ft; "
                  f"type loop {t[1] if t else None}; own: {text or '-'}")
        types = {type_tag for _, type_tag, _, _, _ in revit.values()}
        print("\ncut symbols that are no exported element's type, with their ElementId fields:")
        for symbol in sorted(set(symbols) - types, key=int):
            print(f"  {symbol} loop {symbols[symbol][1]}: {json.dumps(refs.get(symbol, {}))[:600]}")
        print("\nopenings against their matched loop, worst first (opening min, max; loop min, max):")
        for diff, tag, entity, symbol, opening, loop in sorted(worst, reverse=True)[:40]:
            print(f"  {entity} {tag} symbol {symbol}: {diff:.4f} ft; opening {opening}; loop {loop}")


main()
