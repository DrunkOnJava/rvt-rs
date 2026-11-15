#!/usr/bin/env python3
"""Does a ceiling's saved mesh store its pieces in the order Revit's export
creates their extrusions (#1209)?

Reads `probes/examples/probe_1209_ceiling_mesh_order.rs`'s `part` lines (each
connected part of each primitive of a ceiling's saved mesh, in the order the
file stores them) and Revit's IFC export. For each ceiling whose Revit Body
holds pieces of more than one area and whose `GrossArea` is one piece's
(`ceiling_piece_order_vs_ifc`: the piece whose extrusion is created last), it
sets the mesh parts' areas (their downward horizontal faces) beside Revit's
items, both in order, and counts:

- whether Revit's piece is the last mesh part, the first, or neither;
- whether the mesh parts, by area to 1e-3 relative, run in the order of
  Revit's items, in the reverse order, or neither.

Prints one `order` line per ceiling, naming its Tag (so a held-out model's
must never be kept), and the counts. Areas in square feet.

Usage:

    python3 tools/re/ceiling_mesh_order_vs_ifc.py <probe.txt> <revit-export.ifc>
"""

import collections
import os
import re
import sys

import ifcopenshell.geom
import ifcopenshell.util.unit

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ceiling_piece_order_vs_ifc as order  # noqa: E402
import qtos_vs_ifc  # noqa: E402

PART = re.compile(
    r"^part (\d+) primitive (\d+) first (\d+) z (\S+) (\S+) down ([\d.]+) up ([\d.]+)$"
)


def near(a, b):
    return abs(a - b) <= order.TOL * max(abs(b), 1e-9)


def same_sequence(xs, ys):
    return len(xs) == len(ys) and all(near(x, y) for x, y in zip(xs, ys))


def revit_ceilings(path):
    """Revit's ceilings in pieces by Tag: (items' areas in list order,
    GrossArea), where GrossArea is one piece's."""
    f = qtos_vs_ifc.open_ifc(path)
    quantities = qtos_vs_ifc.quantities(path)[0]
    length_scale = ifcopenshell.util.unit.calculate_unit_scale(f, "LENGTHUNIT")
    settings = ifcopenshell.geom.settings()
    out = {}
    for product in f.by_type("IfcCovering"):
        rep = order.body(product)
        if not product.Tag or rep is None or len(rep.Items) < 2:
            continue
        tag = str(product.Tag)
        gross = quantities.get((f"tag:{tag}", order.SET, "GrossArea"))
        if gross is None or gross[1] is None:
            continue
        gross = gross[1] / order.FOOT**2
        areas = []
        for item in rep.Items:
            solid = order.extrusion(item)
            area = order.profile_area(settings, solid, length_scale) if solid is not None else None
            areas.append(None if area is None else area / order.FOOT**2)
        if any(a is None for a in areas):
            continue
        distinct = []
        for a in areas:
            if not any(near(a, d) for d in distinct):
                distinct.append(a)
        if len(distinct) < 2 or sum(near(d, gross) for d in distinct) != 1:
            continue
        out[tag] = (areas, gross)
    return out


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    parts = collections.defaultdict(list)
    for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
        m = PART.match(line.strip())
        if m:
            parts[m.group(1)].append((int(m.group(2)), float(m.group(6)), float(m.group(7))))
    revit = revit_ceilings(sys.argv[2])
    counts = collections.Counter()
    for tag, (items, gross) in sorted(revit.items()):
        mesh = parts.get(tag)
        if not mesh:
            counts["no mesh parts"] += 1
            print(f"order {tag} no mesh parts; revit {','.join(f'{a:.4f}' for a in items)} gross {gross:.4f}")
            continue
        counts["scored"] += 1
        down = [d for _, d, _ in mesh]
        last = near(down[-1], gross)
        first = near(down[0], gross)
        counts["last part"] += last
        counts["first part"] += first
        forward = same_sequence(down, items)
        backward = same_sequence(down, items[::-1])
        counts["same order"] += forward
        counts["reverse order"] += backward
        prims = ",".join(str(p) for p, _, _ in mesh)
        print(
            f"order {tag} gross {gross:.4f} last={int(last)} first={int(first)} same={int(forward)} reverse={int(backward)} "
            f"mesh {','.join(f'{d:.4f}' for d in down)} primitives {prims} revit {','.join(f'{a:.4f}' for a in items)}"
        )
    print(
        f"ceiling mesh order: scored {counts['scored']} (no mesh parts {counts['no mesh parts']}); "
        f"Revit's piece the last part {counts['last part']}, the first {counts['first part']}; "
        f"parts in Revit's item order {counts['same order']}, reversed {counts['reverse order']}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
