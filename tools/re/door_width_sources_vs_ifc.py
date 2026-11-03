#!/usr/bin/env python3
"""Which carrier of a door's or window's symbol gives Revit's `OverallWidth`
where the symbol cuts no loop (RE-318, #810)?

Reads `probes/examples/probe_re318_door_width_sources.rs`'s output and
Revit's IFC export. For each door and window the probe lists whose symbol
has an empty `m_pCutLoops`, joined to Revit's by Tag, it compares Revit's
`OverallWidth`, within 0.001 ft, with:

- the extent along the symbol's X of the cut solid's bounded faces;
- the distance from those faces' ends to each of the cut solid's unbounded
  planes whose normal lies along X, and between two such planes;
- the extent along X of the symbol's saved graphics (the symbol's own
  record, in the symbol's frame), over every primitive, over its own
  forms and over each group of one render style, one face tag or one owner.

It prints Revit's two sizes for every door and window first (`revit <tag>
<class> <width> <height>`), so a later analysis can run on the probe's
output alone.

Usage:

    python3 tools/re/door_width_sources_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from door_window_sizes_vs_ifc import TOLERANCE_FEET, revit_sizes  # noqa: E402


def kv(tokens):
    out = {}
    for token in tokens:
        if "=" in token:
            key, value = token.split("=", 1)
            out.setdefault(key, value)
    return out


def box_of(text):
    if text in ("-", "unbounded") or text.startswith("error:"):
        return None
    return tuple(float(v) for v in text.split(","))


def cross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def read_probe(path):
    instances, symbols, faces, prims = {}, {}, collections.defaultdict(list), collections.defaultdict(list)
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "inst" and len(parts) >= 4:
            instances[parts[1]] = {"class": parts[2], **kv(parts[3:])}
        elif parts[0] == "symroot":
            symbols[parts[1]] = kv(parts[2:])
        elif parts[0] == "symface":
            face = kv(parts[4:])
            for key in ("origin", "x", "y"):
                try:
                    face[key] = json.loads(face[key])
                except (KeyError, ValueError):
                    face[key] = None
            face["solid"], face["face"] = parts[2], parts[3]
            faces[parts[1]].append(face)
        elif parts[0] == "prim" and parts[2] == "symbol":
            fields = kv(parts[3:])
            prims[parts[1]].append(
                {
                    "owner": fields["owner"],
                    "tag": fields["tag"],
                    "style": fields["style"],
                    "material": fields["material"],
                    "box": box_of(fields["box"]),
                }
            )
    return instances, symbols, faces, prims


def close(a, b):
    return a is not None and b is not None and abs(a - b) <= TOLERANCE_FEET


def x_extent(boxes):
    boxes = [b for b in boxes if b]
    if not boxes:
        return None
    return max(b[3] for b in boxes) - min(b[0] for b in boxes)


def solid_candidates(face_rows):
    """Candidate widths from the cut solid: bounded X extent, and plane distances."""
    bounded = [box_of(f.get("mesh", "-")) for f in face_rows]
    bounded = [b for b in bounded if b]
    out = {}
    if bounded:
        lo, hi = min(b[0] for b in bounded), max(b[3] for b in bounded)
        out["bounded X"] = hi - lo
        planes = []
        for f in face_rows:
            if f.get("mesh") == "unbounded" and f["x"] and f["y"] and f["origin"]:
                n = cross(f["x"], f["y"])
                if abs(abs(n[0]) - 1.0) < 1e-6 and abs(n[1]) < 1e-6 and abs(n[2]) < 1e-6:
                    planes.append(f["origin"][0])
        for p in planes:
            out[f"plane {p:.4f} to near end"] = min(abs(p - lo), abs(p - hi))
            out[f"plane {p:.4f} to far end"] = max(abs(p - lo), abs(p - hi))
        for i, p in enumerate(planes):
            for q in planes[i + 1 :]:
                out[f"plane {p:.4f} to plane {q:.4f}"] = abs(p - q)
    return out


def graphics_candidates(prim_rows):
    out = {}
    if not prim_rows:
        return out
    out["all primitives X"] = x_extent([p["box"] for p in prim_rows])
    out["own forms X"] = x_extent([p["box"] for p in prim_rows if p["owner"] == "-"])
    for field in ("style", "tag", "owner", "material"):
        groups = collections.defaultdict(list)
        for p in prim_rows:
            groups[p[field]].append(p["box"])
        for value, boxes in groups.items():
            out[f"{field} {value} X"] = x_extent(boxes)
        for value in groups:
            rest = [b for v, boxes in groups.items() if v != value for b in boxes]
            out[f"without {field} {value} X"] = x_extent(rest)
    return out


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    instances, symbols, faces, prims = read_probe(args[0])
    revit = revit_sizes(args[1])
    for tag, (cls, width, height) in sorted(revit.items(), key=lambda kv: (len(kv[0]), kv[0])):
        if tag in instances:
            w = "-" if width is None else f"{width:.6f}"
            h = "-" if height is None else f"{height:.6f}"
            print(f"revit {tag} {cls} {w} {h}")
    print(f"probe: {len(instances)} doors and windows, {len(symbols)} uncut symbols; Revit's export: {len(revit)}")
    totals = collections.defaultdict(collections.Counter)
    rows = []
    for tag, inst in instances.items():
        symbol = inst.get("symbol")
        if tag not in revit or symbol not in symbols:
            continue
        cls, width, _ = revit[tag]
        if width is None:
            continue
        candidates = {}
        candidates.update({f"solid: {k}": v for k, v in solid_candidates(faces.get(symbol, [])).items()})
        candidates.update({f"graphics: {k}": v for k, v in graphics_candidates(prims.get(symbol, [])).items()})
        rows.append((tag, cls, symbol, width, candidates))
        totals[cls]["n"] += 1
        for name, value in candidates.items():
            # Plane positions are named by their value; fold them into one label.
            label = name.split(" ")[0] + " " + " ".join(w for w in name.split(" ")[1:] if not w.replace(".", "").replace("-", "").isdigit())
            if close(value, width):
                totals[cls][label] += 1
    for cls, counts in sorted(totals.items()):
        print(f"{cls}: {counts['n']} with an uncut symbol")
        for label, n in counts.most_common():
            if label != "n":
                print(f"  {label}: equals Revit's width on {n}")
    odd = {"1756089", "1765267", "1382481", "1722215", "1623922"}
    for tag, cls, symbol, width, candidates in rows:
        if "--list" in sys.argv or tag in odd:
            print(f"row {tag} {cls} symbol {symbol} Revit width {width:.4f}")
            for name, value in candidates.items():
                if value is not None:
                    mark = " <== equal" if close(value, width) else ""
                    print(f"    {name}: {value:.4f}{mark}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
