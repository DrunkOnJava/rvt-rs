#!/usr/bin/env python3
"""Which frame of an element framed in several partitions agrees with Revit's body in height (RE-444).

Reads the output of `probes/examples/probe_re444_frame_copies.rs`, one `element <id> ...` line per element
with more than one frame of an exported instance, and Revit's IFC export of the same model. Pairs each element
by `Tag` with the `IfcElement` of Revit's export and meshes it in the world frame; its vertical range, a length
no rotation about the site's vertical turns, is Revit's height of the body. The height of a frame's box is
set against it, and a frame agrees where the two are within 2 mm.

Only the elements whose frames disagree on the vertical extent are scored (`z_disagree 1`). Of those, the ones
where the frame the exporter keeps (the greatest vertical extent, a tie going to the newest) is not the newest
frame decide between the two rules; they are counted apart: the frame kept agrees with Revit's body, the newest
frame does, both, or neither. Every summary line starts `frame copies:` and holds counts beside fixed words,
which the held-out scrub keeps; `--list` adds a line for each element where the frame kept is not the newest,
and those name an ElementId and are dropped there.

An element with no body in Revit's export, or whose Tag no `IfcElement` of it holds, is counted and not
scored.

Usage:

    python3 tools/re/frame_copies_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Needs IfcOpenShell and numpy.
"""

import collections
import re
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

FEET_METRES = 0.3048
AGREE_METRES = 0.002

ELEMENT = re.compile(
    r"^element (\d+) (\S+) box_disagree ([01]) z_disagree ([01]) selected (\d+) newest (\d+) current (\d+|-) copies (.*)$"
)


def parse(path):
    """The probe's elements: id, category, the partition kept, the newest, the current, and each copy's height."""
    elements = []
    for line in open(path, errors="replace"):
        match = ELEMENT.match(line.strip())
        if not match:
            continue
        ident, category, _, z_disagree, selected, newest, current, copies = match.groups()
        frames = []
        for copy in copies.split():
            partition, offset, _dx, _dy, dz, _z = copy.split("/")
            frames.append((int(partition), int(offset), float(dz) * FEET_METRES))
        elements.append(
            {
                "id": ident,
                "category": category,
                "z_disagree": z_disagree == "1",
                "selected": int(selected),
                "newest": int(newest),
                "current": None if current == "-" else int(current),
                "frames": frames,
            }
        )
    return elements


def revit_height(settings, element):
    """The vertical range of Revit's body of `element`, metres, or None where it has none."""
    try:
        shape = ifcopenshell.geom.create_shape(settings, element)
    except Exception:  # noqa: BLE001 - an element with no body is counted, not scored
        return None
    vertices = np.array(shape.geometry.verts).reshape(-1, 3)
    if len(vertices) == 0:
        return None
    return float(vertices[:, 2].max() - vertices[:, 2].min())


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    listing = "--list" in sys.argv[1:]
    if len(args) != 2:
        sys.exit(__doc__)
    elements = parse(args[0])
    model = ifcopenshell.open(args[1])
    settings = ifcopenshell.geom.settings()
    settings.set(settings.USE_WORLD_COORDS, True)
    tags = {}
    for element in model.by_type("IfcElement"):
        tags.setdefault(str(element.Tag), element)

    counts = collections.Counter()
    by_category = collections.defaultdict(collections.Counter)
    lines = []
    for item in elements:
        counts["multi-framed"] += 1
        if not item["z_disagree"]:
            counts["frames agree in height"] += 1
            continue
        counts["frames disagree in height"] += 1
        element = tags.get(item["id"])
        height = revit_height(settings, element) if element is not None else None
        if height is None:
            counts["no body in Revit's export"] += 1
            continue
        frames = item["frames"]
        # The frame kept: the greatest height quantised to 0.1 mm, a tie going to the newest.
        kept = max(frames, key=lambda f: (round(f[2] / FEET_METRES * 1e4), f[0], f[1]))
        newest = max(frames, key=lambda f: (f[0], f[1]))
        oldest = min(frames, key=lambda f: (f[0], f[1]))
        current = [f for f in frames if item["current"] is not None and f[0] == item["current"]]
        agrees = lambda f: abs(f[2] - height) <= AGREE_METRES  # noqa: E731
        category = item["category"]
        counts["scored"] += 1
        by_category[category]["scored"] += 1
        if kept[:2] == newest[:2]:
            counts["the frame kept is the newest"] += 1
            counts["the frame kept is the newest and agrees" if agrees(kept) else "the frame kept is the newest and does not"] += 1
            continue
        counts["the frame kept is not the newest"] += 1
        by_category[category]["kept is not the newest"] += 1
        kept_ok, newest_ok = agrees(kept), agrees(newest)
        verdict = (
            "both agree" if kept_ok and newest_ok
            else "the frame kept agrees, the newest does not" if kept_ok
            else "the newest agrees, the frame kept does not" if newest_ok
            else "neither agrees"
        )
        counts["not the newest: " + verdict] += 1
        by_category[category]["not the newest: " + verdict] += 1
        if current:
            counts["not the newest: the current frame agrees" if agrees(current[0]) else "not the newest: the current frame does not"] += 1
        counts["not the newest: the oldest frame agrees" if agrees(oldest) else "not the newest: the oldest frame does not"] += 1
        lines.append(
            f"  not the newest {item['id']} {category}: Revit {height:.4f} m, kept (partition {kept[0]}) {kept[2]:.4f} m, "
            f"newest (partition {newest[0]}) {newest[2]:.4f} m, current {item['current']}: {verdict}"
        )

    for name in sorted(counts):
        print(f"frame copies: {name}: {counts[name]}")
    for category in sorted(by_category):
        parts = ", ".join(f"{name} {n}" for name, n in sorted(by_category[category].items()))
        print(f"frame copies: {category}: {parts}")
    if listing:
        for line in lines:
            print(line)


if __name__ == "__main__":
    main()
