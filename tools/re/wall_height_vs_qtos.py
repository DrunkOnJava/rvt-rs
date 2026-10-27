#!/usr/bin/env python3
"""Whether a wall's `Height` is Revit's where its two readings agree
(#1016, RE-257).

revit-ifc's `WallExporter` (CLEANROOM.md source 5) writes a wall's
`Qto_WallBaseQuantities.Height` as the vertical span of the wall's solids.
rvt-rs has two readings of it: the depth of the extrusion it draws the wall
as (`B.depth`, from its record box) and the wall's saved Unconnected Height
(`P.-1001101`, RE-251). Each is Revit's on most walls but not all. This
reads `examples/probe_re251_wall_numbers.rs`'s `rec` lines and Revit's IFC
export, and counts, for the walls where the two agree (a relative 1e-6),
how many carry Revit's `Height`, a different one, or none; then, for the
walls where they do not, which of the two Revit's is. It also counts the
walls Revit's export writes more than once (a wall split by storey), and,
for each wall whose `Height` rvt-rs's depth does not give, whether Revit's
own body is as tall as that `Height` (its vertical span, from IfcOpenShell's
geometry) and which representation Revit draws it with, and whether Revit
moves its base, its top or both against rvt-rs's body (`B.z0`, #1067).

Usage:

    python3 tools/re/wall_height_vs_qtos.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import os
import sys

import ifcopenshell

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402
from wall_numbers_vs_qtos import FOOT, read_probe  # noqa: E402

DEPTH = "B.depth"
BASE = "B.z0"
UNCONNECTED = "P.-1001101"


def same(a, b):
    return abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b))


def body_ranges(f, tags):
    """Tag -> (lowest z, highest z of Revit's body in feet, its
    representation types), for the walls `tags` names."""
    import ifcopenshell.geom

    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for wall in f.by_type("IfcWall"):
        tag = str(wall.Tag)
        if tag not in tags or wall.Representation is None:
            continue
        kinds = sorted(
            {r.RepresentationType or "-" for r in wall.Representation.Representations if r.RepresentationIdentifier == "Body"}
        )
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall)
            zs = shape.geometry.verts[2::3]
        except Exception:
            zs = ()
        out[tag] = (min(zs) / FOOT, max(zs) / FOOT, kinds) if zs else (None, None, kinds)
    return out


def main():
    listing = "--list" in sys.argv
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = read_probe(args[0])
    heights = {}
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == "Qto_WallBaseQuantities" and name == "Height" and value is not None:
            heights[key.removeprefix("tag:")] = value / FOOT
    revit_file = ifcopenshell.open(args[1])
    written = collections.Counter(str(wall.Tag) for wall in revit_file.by_type("IfcWall") if wall.Tag)
    misses = {
        tag
        for tag, fields in rows.items()
        if tag in heights and not same(fields.get(DEPTH, [float("nan")])[0], heights[tag])
    }
    ranges = body_ranges(revit_file, set(heights))
    # Revit's export is in shared coordinates: its heights differ from
    # rvt-rs's model-space ones by one offset, taken from the walls whose
    # height already matches.
    offsets = sorted(
        ranges[tag][0] - rows[tag][BASE][0]
        for tag in heights
        if tag in rows and tag not in misses and BASE in rows[tag] and ranges.get(tag, (None,))[0] is not None
    )
    offset = offsets[len(offsets) // 2] if offsets else None
    spans = {tag: (None if ranges[tag][0] is None else ranges[tag][1] - ranges[tag][0], ranges[tag][2]) for tag in misses if tag in ranges}
    counts = collections.Counter()
    lines = []
    for tag, fields in sorted(rows.items()):
        depth = fields.get(DEPTH, [None])[0]
        unconnected = fields.get(UNCONNECTED, [None])[0]
        revit = heights.get(tag)
        if written.get(tag, 0) > 1:
            counts["written more than once"] += 1
        if depth is None or unconnected is None:
            group = "no depth" if depth is None else "no unconnected height"
        elif same(depth, unconnected):
            group = "agree"
        else:
            group = "disagree"
        if revit is None:
            outcome = "Revit writes none"
        elif group == "agree":
            outcome = "Revit's" if same(depth, revit) else "different"
        else:
            hits = [n for n, v in ((DEPTH, depth), (UNCONNECTED, unconnected)) if v is not None and same(v, revit)]
            outcome = " and ".join(hits) + " is Revit's" if hits else "neither is Revit's"
        counts[(group, outcome)] += 1
        moved = ""
        if tag in spans:
            span, kinds = spans[tag]
            body = "no body" if span is None else ("as tall" if same(span, revit) else "not as tall")
            counts[("miss: Revit's body " + body, "/".join(kinds))] += 1
            base = fields.get(BASE, [None])[0]
            low, high, _ = ranges[tag]
            if None not in (offset, base, depth, low):
                raised = low - offset - base
                lowered = base + depth - (high - offset)
                where = "top lowered" if abs(raised) < 1e-4 else ("base raised" if abs(lowered) < 1e-4 else "both moved")
                counts[("miss: Revit's body " + where, "")] += 1
                moved = f", base raised {raised:.4f}, top lowered {lowered:.4f}"
        if group != "agree" or outcome != "Revit's":
            span = spans.get(tag, (None, []))
            lines.append(
                f"  tag:{tag} {group}: depth {depth}, unconnected {unconnected}, Revit {revit},"
                f" Revit's body {span[0]} ({'/'.join(span[1])}){moved}"
            )
    print(f"probe: {len(rows)} walls, Revit's export: {len(heights)} with a Height")
    print(f"  shared-coordinates offset: {offset} ft, from {len(offsets)} walls")
    for key, n in sorted(counts.items(), key=lambda kv: str(kv[0])):
        label = key if isinstance(key, str) else f"{key[0]}: {key[1]}"
        print(f"  {label}: {n}")
    if listing:
        print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
