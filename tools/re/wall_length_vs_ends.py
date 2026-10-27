#!/usr/bin/env python3
"""How a wall's Length in Revit's export follows from the walls joined at its
ends (#1046, RE-252).

Reads `examples/probe_re252_wall_ends.rs`'s JSON lines and Revit's IFC
export. The walls joined at a wall's end are those its
`VWallDriver.m_controlJoinsSet` names for that end and those whose
`CDJoinInfo` pairs this wall's end with one of their sides. Each end of the
wall's `GLine` (its body's centreline) may stay where it is (`axis`) or stop
where the line crosses a plane of a wall joined there: its middle (`mid`),
the face nearer this wall's other end (`near`), the other face (`far`) or
another of its planes (`other`). For each wall it looks for a choice at each
end whose distance along the line is Revit's `Length` (feet, within 1e-4
ft), and prints how many walls one gives, the choices that do (the first
found, `axis` first), the walls by the choices at each end and whether
`axis, axis` alone gives it, and up to 40 walls none does. With `--list`,
it also prints a `wall <id> <Length> <choices>` line per wall, the choices
at each end as `[label, joined wall id, GLine parameter]`, and an `axis <id>
<length> <start> <end> <body>` line per wall with a straight `Axis` in the
export: its length, where its ends fall along the GLine (parameters), its
Body items' entities and its ends in the export's plan coordinates (feet),
through
the plan transform from the model's coordinates to the export's fitted to
the walls whose GLine ends give their Length. Then, for walls whose line cut
back at its joins (`trimmed_line`) is as long as the GLine, how many there
are and on how many the GLine's span is Revit's Length; and of the walls whose
`Width` Revit writes but the partition reader gives no centreline, on how
many it is the total of the layers the reader gives them (with `--list`, a
`width <id> <Width> <layers> <cross section>` line each). Last, for each
candidate rule for walls whose drawn and trimmed lines agree (no wall joined at either end;
also no wall among the `pairs` partners; also no partner at all), the walls
it selects, on how many the GLine's span is Revit's Length, and those it is
not.

Usage:

    python3 tools/re/wall_length_vs_ends.py <probe.jsonl> <revit.ifc> [--list]
"""

import collections
import itertools
import json
import math
import os
import sys

import ifcopenshell
import ifcopenshell.util.placement
import ifcopenshell.util.unit

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402

FOOT = 0.3048
TOL = 1e-4


def gline(wall):
    ox, oy, dx, dy, e0, e1 = wall["gline"]
    return (ox, oy), (dx, dy), (e0, e1)


def offsets(wall):
    """Each plane's signed distance from the wall's GLine, and the plane."""
    (ox, oy), (dx, dy), _ = gline(wall)
    out = []
    for px, py, xx, xy in wall.get("planes", []):
        if abs(dx * xy - dy * xx) > 1e-6:
            continue
        out.append(((px - ox) * -dy + (py - oy) * dx, (px, py, xx, xy)))
    return out


def crossing(own, plane):
    """The GLine parameter of `own` where it crosses `plane`'s line, or None."""
    (ox, oy), (dx, dy), _ = own
    px, py, xx, xy = plane
    cross = dx * xy - dy * xx
    if abs(cross) < 1e-9:
        return None
    return ((px - ox) * xy - (py - oy) * xx) / cross


def joined(walls):
    """(wall id, end) -> ids of the walls joined there."""
    out = collections.defaultdict(set)
    for wall in walls.values():
        for index, entries in wall.get("control", []):
            for other, _ in entries:
                if other != wall["id"]:
                    out[(wall["id"], index)].add(other)
        for join in wall.get("joins", []):
            pairs = [p for p in join[:-1] if isinstance(p, list)]
            for other, end in pairs:
                if other != wall["id"] and end in (0, 1):
                    out[(other, end)].add(wall["id"])
    return out


def choices(walls, meets, wall, end):
    own = gline(wall)
    here, there = own[2][end], own[2][1 - end]
    out = [("axis", None, here)]
    for other in sorted(meets.get((wall["id"], end), ())):
        o = walls.get(other)
        if o is None or o.get("gline") is None:
            continue
        planes = offsets(o)
        if not planes:
            continue
        half = max(abs(off) for off, _ in planes)
        found = []
        for off, plane in planes:
            t = crossing(own, plane)
            if t is not None:
                found.append((off, t))
        for off, t in found:
            if abs(off) < 1e-6:
                label = "mid"
            elif abs(abs(off) - half) < 1e-6:
                faces = [u for v, u in found if abs(abs(v) - half) < 1e-6]
                label = "near" if abs(t - there) == min(abs(u - there) for u in faces) else "far"
            else:
                label = "other"
            out.append((label, other, t))
    return out


def outer_faces(walls, own, other):
    """Where `own` crosses `other`'s outermost two planes, GLine parameters."""
    o = walls.get(other)
    if o is None or o.get("gline") is None:
        return []
    planes = offsets(o)
    if not planes:
        return []
    lo, hi = min(off for off, _ in planes), max(off for off, _ in planes)
    out = set()
    for off, plane in planes:
        if abs(off - lo) < 1e-6 or abs(off - hi) < 1e-6:
            t = crossing(own, plane)
            if t is not None:
                out.add(round(t, 9))
    return sorted(out)


def face_end(walls, meets, wall, end):
    """The end the join records give: the GLine's where no wall is joined
    there; the far face of the wall the end step's exposed face map names;
    else the nearest near face of a wall joined there. None where unread."""
    own = gline(wall)
    here, there = own[2][end], own[2][1 - end]
    named = sorted(meets.get((wall["id"], end), ()))
    if not named:
        return here, "free"
    steps = [s for s in wall.get("ends", []) if s[0] == end]
    exposed = [f[0] for f in (steps[0][4] if steps and len(steps[0]) > 4 else []) if f[0] in walls]
    if exposed:
        faces = outer_faces(walls, own, exposed[0])
        return (max(faces, key=lambda t: abs(t - there)), "far") if faces else (None, "far")
    near = [min(f, key=lambda t: abs(t - there)) for f in (outer_faces(walls, own, o) for o in named) if f]
    return (min(near, key=lambda t: abs(t - there)), "near") if near else (None, "near")


def reach_end(wall, end):
    """The end the partition reader's join reach gives (RE-70, RE-71, RE-73,
    RE-222), the layer reach of largest size where its layers end apart."""
    reach = wall.get("reach") or {}
    side = "start" if end == 0 else "end"
    value = reach.get(f"m_wall_join_{side}_reach")
    layers = reach.get(f"m_wall_join_{side}_layer_reaches")
    if value is None and layers:
        value = max(layers, key=abs)
    if value is None:
        return None
    here = gline(wall)[2][end]
    return here - value if end == 0 else here + value


def trimmed_line(walls, meets, wall):
    """Both ends of the wall's line cut back at its joins, where the join
    records and the join reaches agree on each (a free end needs no join
    step partner), and every join step partner is a wall; else None."""
    if any(other not in walls for _, other in wall.get("pairs", [])):
        return None
    out = []
    for end in (0, 1):
        t, kind = face_end(walls, meets, wall, end)
        r = reach_end(wall, end)
        if t is not None and r is not None and abs(t - r) <= TOL:
            out.append(t)
        elif t is not None and kind == "free" and r is None and not wall.get("pairs"):
            out.append(t)
        else:
            return None
    return out


def line_ends(item):
    """A straight axis item's two plan end points, local coordinates."""
    if item.is_a("IfcPolyline") and len(item.Points) >= 2:
        return [tuple(p.Coordinates[:2]) for p in (item.Points[0], item.Points[-1])]
    if item.is_a("IfcIndexedPolyCurve") and len(item.Points.CoordList) >= 2:
        return [tuple(item.Points.CoordList[k][:2]) for k in (0, -1)]
    if item.is_a("IfcTrimmedCurve") and item.BasisCurve.is_a("IfcLine"):
        line = item.BasisCurve
        (px, py), v = line.Pnt.Coordinates[:2], line.Dir
        dx, dy = v.Orientation.DirectionRatios[:2]
        norm = math.hypot(dx, dy)
        dx, dy = dx / norm * v.Magnitude, dy / norm * v.Magnitude
        ends = []
        for trim in (item.Trim1, item.Trim2):
            for t in trim:
                if t.is_a("IfcCartesianPoint"):
                    ends.append(tuple(t.Coordinates[:2]))
                    break
            else:
                t = next(t for t in trim if t.is_a("IfcParameterValue")).wrappedValue
                ends.append((px + t * dx, py + t * dy))
        return ends
    return None


listing_sets = "--list" in sys.argv


def axes(path):
    """Wall tag -> its `Axis` item's plan end points, feet, and its Body
    items' entities; and a census of the walls' representations as
    (identifier, item entity) -> count."""
    f = ifcopenshell.open(path)
    header = f.header.file_name
    print(f"export: {header.originating_system} / {header.preprocessor_version}")
    scale = ifcopenshell.util.unit.calculate_unit_scale(f) / FOOT
    out = {}
    bodies = {}
    census = collections.Counter()
    for wall in f.by_type("IfcWall"):
        if not (wall.Tag or "").isdigit() or wall.Representation is None:
            continue
        for rep in wall.Representation.Representations:
            for item in rep.Items:
                census[(rep.RepresentationIdentifier, item.is_a())] += 1
            if rep.RepresentationIdentifier == "Body":
                bodies[int(wall.Tag)] = "+".join(sorted({item.is_a() for item in rep.Items}))
            if rep.RepresentationIdentifier != "Axis":
                continue
            for item in rep.Items:
                local = line_ends(item)
                if local is None:
                    continue
                m = ifcopenshell.util.placement.get_local_placement(wall.ObjectPlacement)
                out[int(wall.Tag)] = [
                    (
                        (m[0][0] * x + m[0][1] * y + m[0][3]) * scale,
                        (m[1][0] * x + m[1][1] * y + m[1][3]) * scale,
                    )
                    for x, y in local
                ]
    sets = collections.Counter()
    for rel in f.by_type("IfcRelDefinesByProperties"):
        qto = rel.RelatingPropertyDefinition
        if not qto.is_a("IfcElementQuantity") or qto.Name != "Qto_WallBaseQuantities":
            continue
        lengths = [q.LengthValue for q in qto.Quantities if q.is_a("IfcQuantityLength") and q.Name == "Length"]
        for obj in rel.RelatedObjects:
            if obj.is_a("IfcWall") and (obj.Tag or "").isdigit():
                sets[int(obj.Tag)] += 1
                if listing_sets:
                    print(f"qto {obj.Tag} {qto.id()} {qto.MethodOfMeasurement!r} {[float(v) * scale for v in lengths]!r}")
    print(f"walls by their number of Qto_WallBaseQuantities sets: {sorted(collections.Counter(sets.values()).items())}")
    return out, bodies, census


def fit(pairs):
    """The plan rotation and translation taking the first of each pair to the second."""
    n = len(pairs)
    ax = sum(a[0] for a, _ in pairs) / n
    ay = sum(a[1] for a, _ in pairs) / n
    bx = sum(b[0] for _, b in pairs) / n
    by = sum(b[1] for _, b in pairs) / n
    sxx = sum((a[0] - ax) * (b[0] - bx) + (a[1] - ay) * (b[1] - by) for a, b in pairs)
    sxy = sum((a[0] - ax) * (b[1] - by) - (a[1] - ay) * (b[0] - bx) for a, b in pairs)
    angle = math.atan2(sxy, sxx)
    c, s = math.cos(angle), math.sin(angle)
    return c, s, bx - (c * ax - s * ay), by - (s * ax + c * ay)


def main(argv):
    listing = "--list" in argv
    argv = [a for a in argv if a != "--list"]
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    walls = {}
    for raw in open(argv[1], encoding="utf-8"):
        raw = raw.strip()
        if raw.startswith("{"):
            row = json.loads(raw)
            walls[row["id"]] = row
    revit = {}
    widths = {}
    for (key, qset, name), (_, value) in quantities(argv[2])[0].items():
        tag = key.removeprefix("tag:")
        if qset != "Qto_WallBaseQuantities" or value is None or not tag.isdigit():
            continue
        if name == "Length":
            revit[int(tag)] = value / FOOT
        elif name == "Width":
            widths[int(tag)] = value / FOOT
    meets = joined(walls)
    exported, bodies, census = axes(argv[2])
    print("the export's wall representations, identifier and item: count")
    for (identifier, entity), n in census.most_common():
        print(f"  {identifier} {entity}: {n}")
    by_choice = collections.Counter()
    by_kind = collections.Counter()
    unmatched = []
    agreed = {}
    total = 0
    for i, length in sorted(revit.items()):
        wall = walls.get(i)
        if wall is None or wall.get("gline") is None:
            continue
        total += 1
        ends = [choices(walls, meets, wall, end) for end in (0, 1)]
        found = None
        if listing:
            print(f"wall {i} {length!r} {json.dumps(ends)}")
        for a, b in itertools.product(*ends):
            if abs(abs(b[2] - a[2]) - length) <= TOL:
                found = (a[0], b[0])
                break
        kind = tuple(len(meets.get((i, end), ())) for end in (0, 1))
        by_kind[(kind, found is not None, found == ("axis", "axis"))] += 1
        if found is not None:
            agreed[i] = (found, length)
        if found is None:
            span = abs(ends[1][0][2] - ends[0][0][2])
            unmatched.append((i, length, span - length, kind, wall.get("keyref")))
        else:
            by_choice[tuple(sorted(found))] += 1
    pairs = []
    for i, (label, _) in agreed.items():
        if label == ("axis", "axis") and i in exported:
            (ox, oy), (dx, dy), (e0, e1) = gline(walls[i])
            ours = [(ox + e0 * dx, oy + e0 * dy), (ox + e1 * dx, oy + e1 * dy)]
            theirs = exported[i]
            if math.dist(ours[0], ours[1]) > 1:
                if math.dist(theirs[0], theirs[1]) and abs(math.dist(*theirs) - math.dist(*ours)) <= TOL:
                    pairs += list(zip(ours, theirs))
    moved = 0
    if len(pairs) >= 4:
        c, s, tx, ty = fit(pairs)
        worst = max(math.dist((c * a[0] - s * a[1] + tx, s * a[0] + c * a[1] + ty), b) for a, b in pairs)
        flipped = 0
        print(f"plan transform fitted to {len(pairs) // 2} walls, worst end {worst:.6f} ft")
        for i in sorted(exported):
            if i not in walls or walls[i].get("gline") is None:
                continue
            (ox, oy), (dx, dy), (e0, e1) = gline(walls[i])
            params = []
            for x, y in exported[i]:
                u, v = x - tx, y - ty
                px, py = c * u + s * v, -s * u + c * v
                params.append((px - ox) * dx + (py - oy) * dy)
            if abs(params[0] - e0) > TOL or abs(params[1] - e1) > TOL:
                moved += 1
            if abs(params[0] - e1) < abs(params[0] - e0):
                flipped += 1
            if listing:
                length = float(math.dist(*exported[i]))
                (ax, ay), (bx, by) = exported[i]
                print(
                    f"axis {i} {length!r} {float(params[0])!r} {float(params[1])!r} {bodies.get(i, '-')}"
                    f" {float(ax)!r} {float(ay)!r} {float(bx)!r} {float(by)!r}"
                )
        print(f"walls whose exported Axis ends are not their GLine's: {moved}; reversed: {flipped}")
    rules = {
        "free ends": lambda w: True,
        "free ends, no wall partner": lambda w: not any(o in walls for _, o in w.get("pairs", [])),
        "free ends, no partner": lambda w: not w.get("pairs"),
    }
    for name, extra in rules.items():
        chosen = [
            i
            for i in revit
            if i in walls
            and walls[i].get("gline") is not None
            and not meets.get((i, 0))
            and not meets.get((i, 1))
            and extra(walls[i])
        ]
        wrong = [i for i in chosen if abs((walls[i]["gline"][5] - walls[i]["gline"][4]) - revit[i]) > TOL]
        print(f"rule {name}: {len(chosen)} walls, span is Revit's Length on {len(chosen) - len(wrong)}, not on {wrong}")
    shipped, wrong = 0, []
    for i, length in sorted(revit.items()):
        wall = walls.get(i)
        if wall is None or wall.get("gline") is None or wall.get("start") is None:
            continue
        ends = trimmed_line(walls, meets, wall)
        span = wall["gline"][5] - wall["gline"][4]
        if ends is not None and abs((ends[1] - ends[0]) - span) <= TOL:
            shipped += 1
            if abs(span - length) > TOL:
                wrong.append(i)
    print(f"rule trimmed line = drawn line: {shipped} walls, span is Revit's Length on {shipped - len(wrong)}, not on {wrong}")
    unplaced = [(i, w) for i, w in sorted(widths.items()) if i in walls and walls[i].get("thickness") is None]
    layered = [(i, w) for i, w in unplaced if walls[i].get("layers") is not None]
    same = [i for i, w in layered if abs(walls[i]["layers"] - w) <= TOL]
    print(
        f"walls Revit gives a Width the partition reader gives no centreline: {len(unplaced)}; "
        f"with layers {len(layered)}, Width = their total on {len(same)}"
    )
    if listing:
        for i, w in unplaced:
            keyref = walls[i].get("keyref") or []
            print(f"width {i} {w!r} {walls[i].get('layers')!r} {keyref[3] if len(keyref) > 3 else None}")
    print(f"walls with Revit's Length and a GLine: {total}")
    print(f"a choice at each end gives the Length: {sum(by_choice.values())}")
    for c, n in by_choice.most_common():
        print(f"  {c[0]}, {c[1]}: {n}")
    print("walls joined at the two ends, given, by axis alone: walls")
    for (k, ok, axis), n in sorted(by_kind.items(), key=lambda kv: -kv[1]):
        print(f"  {k[0]} {k[1]} {ok} {axis}: {n}")
    print("nothing gives the Length:")
    for i, length, diff, k, keyref in unmatched[:40]:
        print(f"  {i}: Length {length:.6f} ft, GLine less Length {diff:+.6f} ft, joined {k}, keyref {keyref}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
