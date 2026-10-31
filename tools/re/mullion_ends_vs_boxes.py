#!/usr/bin/env python3
"""Which end faces of a curtain mullion Revit's NetSurfaceArea counts, from
rvt-rs's boxes of the mullions of one curtain wall (#1046, RE-268, W7).

RE-248: Revit's `Qto_MemberBaseQuantities.NetSurfaceArea` of a mullion drawn
as one extrusion of its section is the section's perimeter times its length
plus 0, 1 or 2 times the section's area, the end faces Revit's solid keeps.
Revit's solids of the mullions of a wall are the boxes rvt-rs draws (their
centres agree within a micrometre after the IfcSite placement), so which
ends are kept follows from where the boxes meet. For each end point `p` of a
mullion, outward along its axis, the other mullions of the same curtain wall
(the one `IfcRelAggregates` names) are classified by their box:

  PARTNER      a mullion running the same way with an end at `p` and its body
               on the far side: the mullion's continuation
  PERP_CENTER  a mullion across it whose box holds `p` on its axis
  PERP_FACE    a mullion across it whose box holds `p` on its side face
  PERP_IN      a mullion across it whose box holds `p` elsewhere
  PAR_SAME     a mullion running the same way with an end at `p` and its body
               on the same side (it lies over this one)
  PAR_IN       a mullion running the same way whose box holds `p`
  OBLIQUE      a mullion neither along nor across it whose box holds `p`

An end is not counted (hidden) when it has a PARTNER and nothing across
covers `p`, or when a mullion across covers `p` (PERP_CENTER or PERP_IN) and
there is no PARTNER; PERP_FACE (the end butts against a side face) and no
neighbour at all leave it counted. A mullion is left out, its count not
claimed, when an end is PERP_IN, PAR_SAME, PAR_IN or OBLIQUE, when its axis is
not known (no rvt-rs `Length` equal to a side of its box), or when an end
meets, within NEAR, a mullion that is one of those or is shorter than its own
section (a mullion shorter than its section keeps its own count).

It prints the boxes read, the mullions left out by cause, the confusion table
of the count against the one Revit's NetSurfaceArea implies, and, for the
mullions rvt-rs writes a `NetVolume` on (the ones drawn as an extrusion of
their box section, RE-243) and the rule does not leave out, whether
`perimeter x length + ends x area` of rvt-rs's box is Revit's value.

Usage:

    python3 tools/re/mullion_ends_vs_boxes.py <rvt-rs.ifc> <revit-export.ifc> [--list]
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402

FEET_PER_METRE = 1 / 0.3048
TOL = 0.01
NEAR = 0.2
PARALLEL = 0.999
ACROSS = 0.01
AXIS_MATCH = 1e-3 * FEET_PER_METRE
UNSURE = ("PERP_IN", "PAR_SAME", "PAR_IN", "OBLIQUE")


def key_of(member):
    return f"tag:{member.Tag}" if member.Tag else None


def read_boxes(f):
    """Tag -> box of each IfcMember of rvt-rs's export drawn as one extruded
    rectangle: centre, three unit axes (rows), half extents, all in feet; the
    mullion's own axis is the side whose length is the member's `Length`."""
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    host_of = {}
    for rel in f.by_type("IfcRelAggregates"):
        for part in rel.RelatedObjects:
            host_of[part.id()] = rel.RelatingObject
    boxes = {}
    skipped = collections.Counter()
    for member in f.by_type("IfcMember"):
        tag = str(member.Tag) if member.Tag else None
        if tag is None or member.Representation is None:
            skipped["no body"] += 1
            continue
        solids = [
            item
            for rep in member.Representation.Representations
            if rep.RepresentationIdentifier == "Body"
            for item in rep.Items
        ]
        if len(solids) != 1 or not solids[0].is_a("IfcExtrudedAreaSolid") or not solids[0].SweptArea.is_a("IfcRectangleProfileDef"):
            skipped["not one extruded rectangle"] += 1
            continue
        solid = solids[0]
        place = np.array(ifcopenshell.util.placement.get_local_placement(member.ObjectPlacement), dtype=float)
        if solid.Position is not None:
            place = place @ np.array(ifcopenshell.util.placement.get_axis2placement(solid.Position), dtype=float)
        place[:3, 3] *= scale
        profile = solid.SweptArea
        cx = cy = 0.0
        if profile.Position is not None:
            cx, cy = profile.Position.Location.Coordinates[0] * scale, profile.Position.Location.Coordinates[1] * scale
        depth = solid.Depth * scale
        direction = np.array(solid.ExtrudedDirection.DirectionRatios, dtype=float)
        direction = place[:3, :3] @ direction
        direction /= np.linalg.norm(direction)
        axes = [place[:3, 0] / np.linalg.norm(place[:3, 0]), place[:3, 1] / np.linalg.norm(place[:3, 1]), direction]
        extents = [profile.XDim * scale, profile.YDim * scale, depth]
        start = place @ np.array([cx / 1.0, cy / 1.0, 0.0, 1.0])
        centre = start[:3] + direction * depth / 2
        qtos = ifcopenshell.util.element.get_psets(member, qtos_only=True).get("Qto_MemberBaseQuantities", {})
        own_length = qtos.get("Length")
        host = host_of.get(member.id())
        boxes[tag] = {
            "tag": tag,
            "host": str(host.Tag) if host is not None and host.Tag else None,
            "centre": centre * FEET_PER_METRE,
            "axes": axes,
            "extents": [e * FEET_PER_METRE for e in extents],
            "own_length": None if own_length is None else own_length * scale * FEET_PER_METRE,
            "plain": qtos.get("NetVolume") is not None,
            "type": (member.ObjectType or "-").split(":")[-1].replace(" ", "_")[:24],
        }
    for box in boxes.values():
        order = None
        if box["own_length"] is not None:
            diffs = [abs(e - box["own_length"]) for e in box["extents"]]
            best = int(np.argmin(diffs))
            if diffs[best] <= AXIS_MATCH:
                order = [best] + [i for i in range(3) if i != best]
        if order is None:
            box["axis"] = None
            box["u"] = box["axes"]
            box["half"] = [e / 2 for e in box["extents"]]
        else:
            box["axis"] = 0
            box["u"] = [box["axes"][i] for i in order]
            box["half"] = [box["extents"][i] / 2 for i in order]
    return boxes, skipped


def box_distance(box, p):
    v = p - box["centre"]
    d = np.array([max(abs(v @ box["u"][i]) - box["half"][i], 0.0) for i in range(3)])
    return float(np.linalg.norm(d))


def end_points(box):
    axis, half = box["u"][0], box["half"][0]
    return [(box["centre"] - axis * half, -axis), (box["centre"] + axis * half, axis)]


def labels(m, p, out, others):
    found = set()
    for o in others:
        if o is m or o["axis"] is None:
            continue
        if box_distance(o, p) > TOL:
            continue
        cosine = abs(o["u"][0] @ m["u"][0])
        if cosine > PARALLEL:
            for q, _ in end_points(o):
                if np.linalg.norm(q - p) < TOL:
                    far = [e for e, _ in end_points(o) if np.linalg.norm(e - p) >= TOL]
                    found.add("PARTNER" if far and (far[0] - p) @ out > TOL else "PAR_SAME")
                    break
            else:
                found.add("PAR_IN")
        elif cosine < ACROSS:
            off = abs((p - o["centre"]) @ m["u"][0])
            reach = sum(abs(m["u"][0] @ o["u"][i]) * o["half"][i] for i in (1, 2))
            if off < TOL:
                found.add("PERP_CENTER")
            elif abs(off - reach) < TOL:
                found.add("PERP_FACE")
            else:
                found.add("PERP_IN")
        else:
            found.add("OBLIQUE")
    return found


def hidden(found):
    covered = "PERP_CENTER" in found or "PERP_IN" in found
    return ("PARTNER" in found) != covered


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours = ifcopenshell.open(args[0])
    boxes, skipped = read_boxes(ours)
    revit = {}
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == "Qto_MemberBaseQuantities" and value is not None:
            revit.setdefault(key, {})[name] = value
    by_host = collections.defaultdict(list)
    for box in boxes.values():
        if box["host"] is not None:
            by_host[box["host"]].append(box)

    # Per mullion: its ends' labels, and why it is left out.
    cause = {}
    short = set()
    found = {}
    for host, group in by_host.items():
        for m in group:
            if m["axis"] is None:
                cause[m["tag"]] = "axis not known"
                continue
            ends = end_points(m)
            found[m["tag"]] = [labels(m, p, out, group) for p, out in ends]
            if any(set(UNSURE) & f for f in found[m["tag"]]):
                cause[m["tag"]] = "an end is " + "/".join(sorted(set(UNSURE) & set().union(*found[m["tag"]])))
            elif 2 * m["half"][0] < 2 * min(m["half"][1], m["half"][2]) - 1e-3:
                short.add(m["tag"])
    # A mullion shorter than its section keeps its own count; one whose end
    # meets it, or a mullion left out, is left out as well.
    sources = set(cause) | short
    for host, group in by_host.items():
        for m in group:
            if m["tag"] in cause or m["axis"] is None:
                continue
            for p, _ in end_points(m):
                if any(o["tag"] in sources and o is not m and box_distance(o, p) < NEAR for o in group):
                    cause[m["tag"]] = "an end meets one left out or shorter than its section"
                    break
    print(f"IfcMembers: {len(ours.by_type('IfcMember'))}, boxes: {len(boxes)}, not read: {dict(skipped)}")
    print(f"in a curtain wall: {sum(len(g) for g in by_host.values())}, hosts: {len(by_host)}")
    print(f"shorter than their section (kept unless an end meets one left out): {len(short)}")
    print("left out, by cause: mullions")
    for name, n in sorted(collections.Counter(cause.values()).items(), key=lambda kv: -kv[1]):
        print(f"  {name}: {n}")

    confusion = collections.Counter()
    values = collections.Counter()
    by_class = collections.Counter()
    left_out = collections.Counter()
    by_labels = collections.Counter()
    different = []
    for host, group in by_host.items():
        for m in group:
            r = revit.get(f"tag:{m['tag']}", {})
            nsa, length = r.get("NetSurfaceArea"), r.get("Length")
            sides = [2 * m["half"][1] / FEET_PER_METRE, 2 * m["half"][2] / FEET_PER_METRE]
            span = 2 * m["half"][0] / FEET_PER_METRE
            area, perimeter = sides[0] * sides[1], 2 * (sides[0] + sides[1])
            implied = None
            if nsa is not None and m["axis"] is not None and area > 0:
                ends = (nsa - perimeter * span) / area
                if abs(ends - round(ends)) < 1e-3 and round(ends) in (0, 1, 2):
                    implied = round(ends)
            if m["tag"] in found and m["tag"] not in cause:
                predicted = sum(0 if hidden(f) else 1 for f in found[m["tag"]])
                confusion[(predicted, implied if implied is not None else "-")] += 1
                if implied is not None:
                    pair = sorted("+".join(sorted(f)) or "nothing" for f in found[m["tag"]])
                    by_labels[(" | ".join(pair), implied)] += 1
                if m["plain"]:
                    if nsa is None:
                        values["Revit writes none"] += 1
                        continue
                    written = perimeter * span + predicted * area
                    tolerance = max(1e-5 * abs(nsa), 1e-6)
                    if abs(written - nsa) <= tolerance:
                        values["equal"] += 1
                        by_class[(m["type"], "upright" if abs(m["u"][0][2]) > 0.999 else "not", predicted)] += 1
                    else:
                        values["different"] += 1
                        different.append((m["tag"], predicted, implied, round(written, 6), round(nsa, 6)))
            elif m["plain"] and nsa is not None:
                values["left out"] += 1
                if m["tag"] in found:
                    predicted = sum(0 if hidden(f) else 1 for f in found[m["tag"]])
                    right = abs(perimeter * span + predicted * area - nsa) <= max(1e-5 * abs(nsa), 1e-6)
                    left_out[(cause[m["tag"]], "its count would be right" if right else "its count would be wrong")] += 1
    print("predicted ends, ends implied by Revit's NetSurfaceArea (- not a whole 0 to 2): mullions")
    for (a, b), n in sorted(confusion.items(), key=lambda kv: (str(kv[0][0]), str(kv[0][1]))):
        print(f"  {a} {b}: {n}")
    print("what lies at the two ends, ends implied by Revit's NetSurfaceArea: mullions")
    for (pair, implied), n in sorted(by_labels.items(), key=lambda kv: -kv[1]):
        print(f"  {pair}: {implied}: {n}")
    plain = sum(1 for b in boxes.values() if b["plain"])
    print(f"NetSurfaceArea on the {plain} mullions with a NetVolume (drawn as their box's extrusion): " + ", ".join(f"{k} {v}" for k, v in sorted(values.items())))
    print("left out among them, by cause, and the value the rule would have given: mullions")
    for (name, verdict), n in sorted(left_out.items()):
        print(f"  {name}, {verdict}: {n}")
    print("type, upright or not, ends written: equal")
    for (t, u, e), n in sorted(by_class.items()):
        print(f"  {t} {u} {e}: {n}")
    if "--list" in argv or different:
        print("different (up to 30):")
        for row in different[:30]:
            print(f"  tag {row[0]} predicted {row[1]} implied {row[2]} written {row[3]} Revit {row[4]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
