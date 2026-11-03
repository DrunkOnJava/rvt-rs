#!/usr/bin/env python3
"""Does the X extent of a door's or window's symbol graphics, hidden
branches included, give Revit's `OverallWidth` (RE-318, #810)?

Reads `probes/examples/probe_re318_hidden_geometry.rs`'s output and Revit's
IFC export. For each door and window the probe lists, joined to Revit's by
Tag, it compares Revit's `OverallWidth`, within 0.001 ft, with the extent
along the symbol's X of the primitives of the symbol that draws it:

- visible: the primitives no hidden branch leads to (what the saved graphics
  show);
- all: those and the ones the visibility flags hide.

Per class it counts how often each is Revit's width, and lists every door and
window where `all` is not, and every one where it is and `visible` is not.

Usage:

    python3 tools/re/hidden_geometry_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from door_window_sizes_vs_ifc import TOLERANCE_FEET, revit_sizes  # noqa: E402


def read_probe(path):
    instances, prims = {}, collections.defaultdict(list)
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) >= 3 and parts[0] == "inst":
            fields = dict(p.split("=", 1) for p in parts[3:] if "=" in p)
            instances[parts[1]] = (parts[2], fields.get("symbol"))
        elif len(parts) >= 3 and parts[0] == "prim":
            fields = dict(p.split("=", 1) for p in parts[2:] if "=" in p)
            if fields.get("box", "-") == "-":
                continue
            box = tuple(float(v) for v in fields["box"].split(","))
            prims[parts[1]].append((fields["hidden"] == "1", box))
    return instances, prims


def x_extent(boxes):
    if not boxes:
        return None
    return max(b[3] for b in boxes) - min(b[0] for b in boxes)


def close(a, b):
    return a is not None and b is not None and abs(a - b) <= TOLERANCE_FEET


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    instances, prims = read_probe(args[0])
    revit = revit_sizes(args[1])
    for tag, (cls, width, height) in sorted(revit.items(), key=lambda kv: (len(kv[0]), kv[0])):
        if tag in instances:
            w = "-" if width is None else f"{width:.6f}"
            h = "-" if height is None else f"{height:.6f}"
            print(f"revit {tag} {cls} {w} {h}")
    print(f"probe: {len(instances)} doors and windows; Revit's export: {len(revit)}")
    counts = collections.defaultdict(collections.Counter)
    shown = collections.defaultdict(list)
    for tag, (cls, symbol) in sorted(instances.items(), key=lambda kv: (len(kv[0]), kv[0])):
        if tag not in revit or symbol not in prims:
            continue
        _, width, _ = revit[tag]
        if width is None:
            continue
        rows = prims[symbol]
        visible = x_extent([b for hidden, b in rows if not hidden])
        everything = x_extent([b for _, b in rows])
        hidden_n = sum(1 for hidden, _ in rows if hidden)
        c = counts[cls]
        c["n"] += 1
        c["visible"] += close(visible, width)
        c["all"] += close(everything, width)
        c["has hidden primitives"] += hidden_n > 0
        c["all differs from visible"] += not close(everything, visible)
        if not close(everything, width) or (close(everything, width) and not close(visible, width)):
            shown[cls].append((tag, symbol, width, visible, everything, hidden_n, len(rows)))
    for cls, c in sorted(counts.items()):
        print(
            f"{cls}: {c['n']} joined; visible = Revit on {c['visible']}, all = Revit on {c['all']}; "
            f"{c['has hidden primitives']} symbols hold hidden primitives, "
            f"{c['all differs from visible']} change width with them"
        )
        for tag, symbol, width, visible, everything, hidden_n, n in shown[cls]:
            print(
                f"  {tag} symbol {symbol}: Revit {width:.4f}, visible {visible if visible is None else round(visible, 4)}, "
                f"all {everything if everything is None else round(everything, 4)} ({hidden_n} of {n} hidden)"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
