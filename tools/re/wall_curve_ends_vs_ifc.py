#!/usr/bin/env python3
"""Where Revit ends each wall's body, relative to the wall's native curve (W6).

Reads `examples/probe_wall_curves.rs` output (each wall's curve from its
native record, its location-line key, its end joins) and Revit's own IFC
export. For every straight wall in both, it meshes Revit's body in internal
feet and measures, along the curve from its start, how far the body runs
past the curve's start and past its end (negative: it stops short). Each
end is described by the walls the native record says join there and
whether that end's join is real, so a rule shows as one row.

Usage:

    python3 tools/re/wall_curve_ends_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import collections
import json
import math
import os
import re
import sys

import ifcopenshell
import ifcopenshell.geom

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wall_layers_vs_ifc import site_to_internal  # noqa: E402

CURVE = re.compile(
    r"^curve (\d+) GLine start (\S+) (\S+) end (\S+) (\S+) z (\S+) thickness (\S+) type (\S+) keyref (\S+) keyoffset (\S+) flipped (\S+)"
)


def parse(path):
    curves, drivers, ends, cdjoins = {}, {}, collections.defaultdict(dict), collections.defaultdict(list)
    for line in open(path, encoding="utf-8", errors="replace"):
        m = CURVE.match(line)
        if m:
            tag = m.group(1)
            thickness = float(m.group(7)) if m.group(7) != "-" else None
            curves[tag] = {
                "start": (float(m.group(2)), float(m.group(3))),
                "end": (float(m.group(4)), float(m.group(5))),
                "thickness": thickness,
                "type": m.group(8),
                "keyref": m.group(9),
                "keyoffset": float(m.group(10)) if m.group(10) not in ("null", "-") else 0.0,
                "flipped": m.group(11),
            }
            continue
        if line.startswith("driver "):
            _, tag, _, nojoin, _, control = line.rstrip("\n").split(" ", 5)
            drivers[tag] = (json.loads(nojoin), json.loads(control))
        elif line.startswith("joinend "):
            parts = line.split()
            ends[parts[1]][parts[2]] = parts[4]
    return curves, drivers, ends


def partners(tag, control, end):
    """The walls the control joins set says meet `tag`'s `end`, with the end of theirs."""
    out = []
    for join in control or []:
        info = (join.get("m_JoinInfo") or {}).get("m_info") or []
        mine = [i for i in info if str(i.get("m_elemId")) == tag]
        if not any(i.get("m_joinedEnd") == end for i in mine):
            continue
        out.extend(
            (str(i.get("m_elemId")), i.get("m_joinedEnd")) for i in info if str(i.get("m_elemId")) != tag
        )
    return out


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    listing = "--list" in args
    curves, drivers, ends = parse(args[0])
    f = ifcopenshell.open(args[1])
    to_internal, _ = site_to_internal(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    rows = []
    for wall in f.by_type("IfcWall"):
        tag = str(wall.Tag) if wall.Tag else None
        if tag not in curves or not wall.Representation:
            continue
        body = next(
            (r for r in wall.Representation.Representations if r.RepresentationIdentifier == "Body"), None
        )
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall, body)
        except Exception:
            continue
        verts = shape.geometry.verts
        points = [to_internal(verts[i], verts[i + 1]) for i in range(0, len(verts), 3)]
        if not points:
            continue
        c = curves[tag]
        (sx, sy), (ex, ey) = c["start"], c["end"]
        length = math.hypot(ex - sx, ey - sy)
        if length < 1e-6:
            continue
        ux, uy = (ex - sx) / length, (ey - sy) / length
        along = [(p[0] - sx) * ux + (p[1] - sy) * uy for p in points]
        past_start, past_end = -min(along), max(along) - length
        nojoin, control = drivers.get(tag, (None, None))
        t = c["thickness"]
        row = {
            "tag": tag,
            "t": t,
            "keyref": c["keyref"],
            "past": (round(past_start, 4), round(past_end, 4)),
            "ends": [],
        }
        for end, past in ((0, past_start), (1, past_end)):
            mates = partners(tag, control, end)
            mate_t = [curves.get(m, {}).get("thickness") for m, _ in mates]
            row["ends"].append(
                {
                    "past": past,
                    "real": ends.get(tag, {}).get(str(end), "?"),
                    "nojoin": (nojoin or [None, None])[end] if isinstance(nojoin, list) and len(nojoin) > end else None,
                    "mates": mates,
                    "mate_t": mate_t,
                }
            )
        rows.append(row)

    def kind(e, t):
        p = e["past"]
        if abs(p) <= 0.001:
            return "at curve end"
        ratios = []
        if t:
            ratios.append(f"{p / t:+.3f} own t")
        for mt in e["mate_t"]:
            if mt:
                ratios.append(f"{p / mt:+.3f} mate t")
        return "past " + ("; ".join(ratios) if ratios else f"{p:+.4f} ft")

    groups = collections.Counter()
    for row in rows:
        for e in row["ends"]:
            mates = "+".join(sorted(f"end{j}" for _, j in e["mates"])) or "none"
            groups[(e["real"], mates, kind(e, row["t"]))] += 1
    print(f"walls with a native line and a body in Revit's export: {len(rows)}")
    print("real join | partners (their end) | where Revit's body ends: ends")
    for (real, mates, k), n in sorted(groups.items(), key=lambda kv: -kv[1])[:80]:
        print(f"  {n:>5}  real={real} | {mates} | {k}")
    if listing:
        for row in rows[:400]:
            ends_text = " | ".join(
                f"end{i} past {e['past']:+.4f} real {e['real']} nojoin {e['nojoin']} mates {e['mates']} mate_t {e['mate_t']}"
                for i, e in enumerate(row["ends"])
            )
            print(f"wall {row['tag']} t {row['t']} keyref {row['keyref']} | {ends_text}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
