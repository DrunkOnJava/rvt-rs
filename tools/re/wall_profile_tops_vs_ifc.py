#!/usr/bin/env python3
"""For each wall of a `probe_1607_wall_profile_lines` output, how high Revit's body stands along the wall, at the
ends of the wall's box and of its sketch-line loop, against where the loop's top edge would put it if it ran on to
the wall's end (#1473, RE-382).

A profile-edited wall can stand longer than its sketch lines (a stub of 0.41 ft on the BIM4LCA office's walls 2057668
and 2056705). rvt-rs draws the stub at the box's top; the question is whether Revit's body keeps sloping along the
line of the loop's top edge there. For each wall whose loop lies in a plane along the model's x or y axis, it prints

    revit tag:<Tag> volume <ft3> box <z low> <z high> <x min> <x max> <y min> <y max>
    revit tag:<Tag> axis <x|y> box <low> <high> loop <low> <high> stub <low side> <high side>
    revit tag:<Tag> top at <axis> <s> <z>              (Revit's highest point of the body at s)
    revit tag:<Tag> loop top at <axis> <s> <z>         (the loop's highest point at s, where it reaches)
    revit tag:<Tag> extension at <axis> <s> <z>        (the loop's top edge at that end, run on to s)

Coordinates are in feet in the model's frame, as the probe prints them.

Usage:

    python3 tools/re/wall_profile_tops_vs_ifc.py <probe output> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import re
import sys

import ifcopenshell
import ifcopenshell.geom

FOOT = 0.3048
AT_END = 1e-4
INSIDE = 0.002


def parse_probe(path):
    """The walls of the probe output: Tag, box (low, high) and the loop's lines as pairs of 3-d points."""
    walls = []
    current = None
    with open(path, errors="replace") as handle:
        for line in handle:
            head = re.match(r"wall (\d+) box \(([^)]*)\) \(([^)]*)\) lines (\d+)", line)
            if head:
                current = {
                    "tag": head.group(1),
                    "low": [float(v) for v in head.group(2).split(",")],
                    "high": [float(v) for v in head.group(3).split(",")],
                    "lines": [],
                }
                walls.append(current)
                continue
            seg = re.match(r"\s+line (\d+) \(([^)]*)\) \(([^)]*)\)", line)
            if seg and current is not None:
                a = [float(v) for v in seg.group(2).split(",")]
                b = [float(v) for v in seg.group(3).split(",")]
                current["lines"].append((a, b))
    return walls


def wall_mesh(settings, product):
    """(vertices in feet as 3-tuples, triangles as index triples) of the Body of `product`, or None."""
    if product.Representation is None:
        return None
    body = next((r for r in product.Representation.Representations if r.RepresentationIdentifier == "Body"), None)
    if body is None:
        return None
    try:
        geometry = ifcopenshell.geom.create_shape(settings, product, body).geometry
    except Exception:
        return None
    raw = [v / FOOT for v in geometry.verts]
    faces = list(geometry.faces)
    if not raw or not faces:
        return None
    verts = [(raw[i], raw[i + 1], raw[i + 2]) for i in range(0, len(raw), 3)]
    return verts, [(faces[k], faces[k + 1], faces[k + 2]) for k in range(0, len(faces), 3)]


def volume_of(verts, tris):
    total = 0.0
    for i, j, k in tris:
        a, b, c = verts[i], verts[j], verts[k]
        total += (
            a[0] * (b[1] * c[2] - b[2] * c[1])
            - a[1] * (b[0] * c[2] - b[2] * c[0])
            + a[2] * (b[0] * c[1] - b[1] * c[0])
        ) / 6.0
    return abs(total)


def highest_at(segments, axis, s):
    """The highest z where 3-d segments cross the plane `axis` = s, or None where none does."""
    best = None
    for a, b in segments:
        da, db = a[axis] - s, b[axis] - s
        if da == 0 and db == 0:
            z = max(a[2], b[2])
        elif da * db <= 0 and da != db:
            z = a[2] + da / (da - db) * (b[2] - a[2])
        else:
            continue
        best = z if best is None else max(best, z)
    return best


def mesh_segments(verts, tris):
    for i, j, k in tris:
        for p, q in ((i, j), (j, k), (k, i)):
            yield verts[p], verts[q]


def extension_at(lines, axis, end, s):
    """The loop's top edge at its end `end` along `axis`, run on to s: the line through the highest point there
    and the other end of the edge that is not along the end's own plane."""
    best = None
    for a, b in lines:
        for near, far in ((a, b), (b, a)):
            if abs(near[axis] - end) <= AT_END and abs(far[axis] - end) > AT_END:
                if best is None or near[2] > best[0][2]:
                    best = (near, far)
    if best is None:
        return None
    near, far = best
    slope = (far[2] - near[2]) / (far[axis] - near[axis])
    return near[2] + slope * (s - near[axis])


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    walls = parse_probe(argv[1])
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    model = ifcopenshell.open(argv[2])
    by_tag = {}
    for wall in model.by_type("IfcWall"):
        tag = getattr(wall, "Tag", None)
        if tag:
            by_tag.setdefault(str(tag), wall)
    print(f"{len(walls)} walls with sketch lines in the probe output")
    for w in walls:
        points = [p for line in w["lines"] for p in line]
        if not points:
            continue
        spans = [max(p[i] for p in points) - min(p[i] for p in points) for i in (0, 1)]
        axis = 0 if spans[0] > spans[1] else 1
        if spans[1 - axis] > 0.01:
            print(f"revit tag:{w['tag']} angled, not read")
            continue
        loop_low = min(p[axis] for p in points)
        loop_high = max(p[axis] for p in points)
        box_low, box_high = w["low"][axis], w["high"][axis]
        name = "xy"[axis]
        wall = by_tag.get(w["tag"])
        if wall is None:
            print(f"revit tag:{w['tag']} not in Revit's export")
            continue
        mesh = wall_mesh(settings, wall)
        if mesh is None:
            print(f"revit tag:{w['tag']} no body")
            continue
        verts, tris = mesh
        xs, ys, zs = [v[0] for v in verts], [v[1] for v in verts], [v[2] for v in verts]
        print(
            f"revit tag:{w['tag']} volume {volume_of(verts, tris):.4f} box "
            f"{min(zs):.4f} {max(zs):.4f} {min(xs):.4f} {max(xs):.4f} {min(ys):.4f} {max(ys):.4f}"
        )
        print(
            f"revit tag:{w['tag']} axis {name} box {box_low:.4f} {box_high:.4f} loop {loop_low:.4f} {loop_high:.4f} "
            f"stub {loop_low - box_low:.4f} {box_high - loop_high:.4f}"
        )
        segments = list(mesh_segments(verts, tris))
        positions = [box_low + INSIDE, loop_low, loop_high, box_high - INSIDE]
        positions += [loop_low + (loop_high - loop_low) * k / 6.0 for k in range(1, 6)]
        for s in sorted(set(round(p, 4) for p in positions)):
            top = highest_at(segments, axis, s)
            loop_top = highest_at(w["lines"], axis, s)
            print(f"revit tag:{w['tag']} top at {name} {s:.4f} {top if top is None else format(top, '.4f')}")
            if loop_top is not None:
                print(f"revit tag:{w['tag']} loop top at {name} {s:.4f} {loop_top:.4f}")
        for end, stub_to in ((loop_low, box_low + INSIDE), (loop_high, box_high - INSIDE)):
            if abs(stub_to - end) > 2 * INSIDE:
                ext = extension_at(w["lines"], axis, end, stub_to)
                if ext is not None:
                    print(f"revit tag:{w['tag']} extension at {name} {stub_to:.4f} {ext:.4f}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
