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
geometry) and which representation Revit draws it with.

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
UNCONNECTED = "P.-1001101"


def same(a, b):
    return abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b))


def body_spans(f, tags):
    """Tag -> (vertical span of Revit's body in feet, its representation
    types), for the walls `tags` names."""
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
            verts = shape.geometry.verts
        except Exception:
            out[tag] = (None, kinds)
            continue
        zs = verts[2::3]
        out[tag] = ((max(zs) - min(zs)) / FOOT if zs else None, kinds)
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
    spans = body_spans(revit_file, misses)
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
        if tag in spans:
            span, kinds = spans[tag]
            body = "no body" if span is None else ("as tall" if same(span, revit) else "not as tall")
            counts[("miss: Revit's body " + body, "/".join(kinds))] += 1
        if group != "agree" or outcome != "Revit's":
            span = spans.get(tag, (None, []))
            lines.append(
                f"  tag:{tag} {group}: depth {depth}, unconnected {unconnected}, Revit {revit},"
                f" Revit's body {span[0]} ({'/'.join(span[1])})"
            )
    print(f"probe: {len(rows)} walls, Revit's export: {len(heights)} with a Height")
    for key, n in sorted(counts.items(), key=lambda kv: str(kv[0])):
        label = key if isinstance(key, str) else f"{key[0]}: {key[1]}"
        print(f"  {label}: {n}")
    if listing:
        print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
