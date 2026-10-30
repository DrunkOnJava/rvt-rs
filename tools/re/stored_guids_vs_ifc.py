#!/usr/bin/env python3
"""Tie Revit's GlobalIds to the IfcGUID strings a Revit 2015 file stores (#878).

Reads `probes/examples/probe_878_stored_guids.rs`'s output. For the project, site,
building and every storey of Revit's export, it finds the stored string
equal to the GlobalId (the storeys in elevation order beside the Levels by
elevation, since the model's Levels were renamed after its export), and for every element with a Tag the one with its
last 21 characters (RE-208 §3). It names each string's owner: the nearest
`frame` before it in the same stream. For a storey it also names the Level
of the storey's name. It counts how often the owner is the element itself
(its Tag, or the storey's Level) and prints the parameter id before each
matched string.

Usage:

    python3 tools/re/stored_guids_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import bisect
import collections
import sys

import ifcopenshell


def main():
    levels, frames, gids, nears, holders = {}, collections.defaultdict(list), [], {}, {}
    for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
        parts = line.rstrip("\n").split(" ")
        if parts[0] == "level" and len(parts) >= 4:
            levels.setdefault(parts[1], (float(parts[2]), " ".join(parts[3:])))
        elif parts[0] == "frame" and len(parts) == 5:
            frames[parts[1]].append((int(parts[2]), parts[3], parts[4]))
        elif parts[0] == "holder" and len(parts) == 4:
            holders[(parts[1], int(parts[2]))] = parts[3]
        elif parts[0] == "near" and len(parts) >= 3:
            nears[(parts[1], int(parts[2]))] = [
                (int(d), i) for d, i in (p.split("=") for p in parts[3:] if "=" in p)
            ]
        elif parts[0] == "gid" and len(parts) == 6:
            gids.append((parts[1], int(parts[2]), parts[3], parts[4], parts[5]))
    for stream in frames:
        frames[stream].sort()
    starts = {stream: [f[0] for f in v] for stream, v in frames.items()}

    def owner(stream, offset):
        i = bisect.bisect_right(starts.get(stream, []), offset) - 1
        return frames[stream][i] if i >= 0 else None

    by_text = collections.defaultdict(list)
    by_tail = collections.defaultdict(list)
    for g in gids:
        by_text[g[4]].append(g)
        by_tail[g[4][1:]].append(g)
    model = ifcopenshell.open(sys.argv[2])
    print(f"levels {len(levels)}, frames {sum(len(v) for v in frames.values())}, stored strings {len(gids)}")
    tally = collections.Counter()
    params = collections.Counter()
    storeys = sorted(model.by_type("IfcBuildingStorey"), key=lambda s: s.Elevation or 0.0)
    level_order = [lid for lid, _ in sorted(levels.items(), key=lambda kv: kv[1][0])]
    for entity in ("IfcProject", "IfcSite", "IfcBuilding", "IfcBuildingStorey"):
        for element in (storeys if entity == "IfcBuildingStorey" else model.by_type(entity)):
            hits = by_text.get(element.GlobalId, [])
            if not hits:
                print(f"{entity} {element.Name!r} {element.GlobalId}: no stored string")
                continue
            for stream, offset, param, length, text in hits:
                o = owner(stream, offset)
                level = (
                    level_order[storeys.index(element)]
                    if entity == "IfcBuildingStorey" and len(level_order) == len(storeys)
                    else None
                )
                print(
                    f"{entity} {element.Name!r} {element.GlobalId}: {stream} +{offset} param {param} len {length}; "
                    f"owner {o[1] + ' cat ' + o[2] + ' at +' + str(o[0]) if o else 'none'}"
                    + (f"; Level at that elevation rank {level} {levels.get(level, ('', ''))[1]!r}, owner is it: {bool(o and o[1] == level)}" if entity == "IfcBuildingStorey" else "")
                )
    for element in model.by_type("IfcElement"):
        tag = str(element.Tag or "")
        if not tag:
            continue
        hits = by_tail.get(element.GlobalId[1:], [])
        if not hits:
            tally["no stored string"] += 1
            continue
        for stream, offset, param, length, text in hits:
            params[param] += 1
            o = owner(stream, offset)
            tally["owner is the element" if o and o[1] == tag else "owner is another element" if o else "no frame before"] += 1
            if "--list" in sys.argv and (not o or o[1] != tag):
                print(f"  {element.is_a()} {tag}: {stream} +{offset} owner {o}")
    print(f"elements: {dict(tally)}; parameter ids before the strings: {dict(params)}")
    # The distance before its string at which an element's own id sits.
    distances = collections.Counter()
    matched = 0
    for element in model.by_type("IfcElement"):
        tag = str(element.Tag or "")
        for stream, offset, *_ in by_tail.get(element.GlobalId[1:], []):
            hits = [d for d, i in nears.get((stream, offset), []) if i == tag]
            if hits:
                matched += 1
                distances[hits[0]] += 1
    print(f"elements whose own id is within 4096 bytes before their string: {matched}; distances {distances.most_common(8)}")
    # The element whose data holds each string (its data header's id).
    held = collections.Counter()
    for element in model.by_type("IfcElement"):
        tag = str(element.Tag or "")
        for stream, offset, *_ in by_tail.get(element.GlobalId[1:], []):
            held["holder is the element" if holders.get((stream, offset)) == tag else "holder is another"] += 1
    print(f"elements, by data header: {dict(held)}")
    for entity in ("IfcProject", "IfcSite", "IfcBuilding", "IfcBuildingStorey"):
        for element in (storeys if entity == "IfcBuildingStorey" else model.by_type(entity)):
            for stream, offset, param, *_ in by_text.get(element.GlobalId, []):
                holder = holders.get((stream, offset))
                level = (
                    level_order[storeys.index(element)]
                    if entity == "IfcBuildingStorey" and len(level_order) == len(storeys)
                    else None
                )
                print(f"  {entity} {element.Name!r}: param {param}, held by {holder}" + (f", the Level at its elevation rank is {level}: {holder == level}" if level else ""))
    best = distances.most_common(1)[0][0] if distances else None
    if best is not None:
        for entity in ("IfcProject", "IfcSite", "IfcBuilding", "IfcBuildingStorey"):
            for element in (storeys if entity == "IfcBuildingStorey" else model.by_type(entity)):
                for stream, offset, *_ in by_text.get(element.GlobalId, []):
                    at = [i for d, i in nears.get((stream, offset), []) if d == best]
                    named = levels.get(at[0], ("", "?"))[1] if at else None
                    print(f"  {entity} {element.Name!r}: id at distance {best}: {at[0] if at else None} {named!r}")


if __name__ == "__main__":
    main()
