#!/usr/bin/env python3
"""Scratch scorer (never committed to main): for every mullion the rule of RE-400 writes a NetSurfaceArea on, what
lies at its ends, and how Revit's value differs when it does. Counts and geometry only.

Usage: python3 tools/re/mullion_wrong_detail.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import os
import sys

import ifcopenshell
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mullion_ends_vs_boxes as M  # noqa: E402
from qtos_vs_ifc import quantities  # noqa: E402

MM = 304.8


def neighbours(m, p, out, group):
    rows = []
    for o in group:
        if o is m or o["axis"] is None:
            continue
        if M.box_distance(o, p) > M.TOL:
            continue
        cosine = abs(o["u"][0] @ m["u"][0])
        label = "+".join(sorted(M.labels(m, p, out, [o])))
        d = o["centre"] - m["centre"]
        lat = [round(float(d @ m["u"][1]) * MM, 1), round(float(d @ m["u"][2]) * MM, 1)]
        a = b = None
        corners_in = None
        if cosine > M.PARALLEL:
            mid = float((o["centre"] - p) @ out)
            a, b = round((mid - o["half"][0]) * MM, 1), round((mid + o["half"][0]) * MM, 1)
            corners_in = 0
            for s1 in (-1, 1):
                for s2 in (-1, 1):
                    c = p + m["u"][1] * s1 * m["half"][1] + m["u"][2] * s2 * m["half"][2]
                    corners_in += M.box_distance(o, c) <= M.TOL
        rows.append(
            {
                "tag": o["tag"],
                "type": o["type"],
                "label": label,
                "sec_o": sorted(round(2 * h * MM, 1) for h in o["half"][1:]),
                "lat": lat,
                "from": a,
                "to": b,
                "len": round(2 * o["half"][0] * MM, 1),
                "corners_in": corners_in,
            }
        )
    return rows


def main(argv):
    ours = ifcopenshell.open(argv[1])
    boxes, _ = M.read_boxes(ours)
    revit = {}
    for (key, qset, name), (_, value) in quantities(argv[2])[0].items():
        if qset == "Qto_MemberBaseQuantities" and value is not None:
            revit.setdefault(key, {})[name] = value
    by_host = collections.defaultdict(list)
    for box in boxes.values():
        if box["host"] is not None:
            by_host[box["host"]].append(box)

    cause, overlapped, short, found = {}, set(), set(), {}
    for host, group in by_host.items():
        for m in group:
            if m["axis"] is None:
                cause[m["tag"]] = "axis not known"
                continue
            found[m["tag"]] = [M.labels(m, p, out, group) for p, out in M.end_points(m)]
            seen = set().union(*found[m["tag"]])
            if set(M.UNREAD) & seen:
                cause[m["tag"]] = "unread"
            elif set(M.OVERLAPPED) & seen:
                overlapped.add(m["tag"])
            if 2 * m["half"][0] < 2 * min(m["half"][1], m["half"][2]) - 1e-3:
                short.add(m["tag"])
    sources = set(cause) | overlapped
    first = dict(cause)
    near_short = set()
    for host, group in by_host.items():
        for m in group:
            if m["axis"] is None or m["tag"] in first:
                continue
            kinds = set()
            for p, _ in M.end_points(m):
                for o in group:
                    if o is m or M.box_distance(o, p) >= M.NEAR:
                        continue
                    if o["tag"] in sources:
                        kinds.add(first.get(o["tag"], "overlapped"))
                    elif o["tag"] in short:
                        near_short.add(m["tag"])
            if kinds and m["tag"] not in overlapped:
                cause[m["tag"]] = "meets one left out or overlapped"

    verdict = collections.Counter()
    near_short_table = collections.Counter()
    features = collections.Counter()
    wrong = []
    for host, group in by_host.items():
        for m in group:
            tag = m["tag"]
            if tag not in found or tag in cause or not m["plain"]:
                continue
            nsa = revit.get(f"tag:{tag}", {}).get("NetSurfaceArea")
            if nsa is None:
                continue
            sides = [2 * m["half"][1] / M.FEET_PER_METRE, 2 * m["half"][2] / M.FEET_PER_METRE]
            span = 2 * m["half"][0] / M.FEET_PER_METRE
            area, perimeter = sides[0] * sides[1], 2 * (sides[0] + sides[1])
            predicted = sum(0 if M.hidden(f) else 1 for f in found[tag])
            written = perimeter * span + predicted * area
            ok = abs(written - nsa) <= max(1e-5 * abs(nsa), 1e-6)
            kind = "overlapped" if tag in overlapped else ("near_short" if tag in near_short else "plain")
            verdict[(kind, "equal" if ok else "different")] += 1
            ends = (nsa - perimeter * span) / area if area > 0 else None
            if kind == "overlapped":
                for i, (p, out) in enumerate(M.end_points(m)):
                    for row in neighbours(m, p, out, group):
                        if row["label"] in ("PAR_IN", "PAR_SAME"):
                            same_section = row["sec_o"] == sorted(round(2 * h * MM, 1) for h in m["half"][1:])
                            features[
                                (
                                    "right" if ok else "WRONG",
                                    row["label"],
                                    "same section" if same_section else "other section",
                                    "o holds the whole end face" if row["corners_in"] == 4 else f"{row['corners_in']} corners inside",
                                    "same type" if row["type"] == m["type"] else "other type",
                                )
                            ] += 1
            if kind == "near_short":
                touching = []
                for p, _ in M.end_points(m):
                    near = [o for o in group if o is not m and o["tag"] in short and M.box_distance(o, p) < M.NEAR]
                    touching.append(
                        "touch" if any(M.box_distance(o, p) <= M.TOL for o in near) else ("gap" if near else "no short")
                    )
                labels_per_end = tuple("+".join(sorted(f)) or "nothing" for f in found[tag])
                key = ("right" if ok else "WRONG", labels_per_end, tuple(touching), predicted)
                near_short_table[key] += 1
            if not ok:
                wrong.append((tag, host, kind, m, found[tag], predicted, written, nsa, ends, group))
    print("written by the rule, by what it read:", dict(sorted(verdict.items())))
    print("overlapped mullions' PAR neighbours, by feature (end, verdict, label, section, face cover, type): count")
    for k, n in sorted(features.items()):
        print(f"  {k}: {n}")
    print("mullions that meet a mullion shorter than their section, by verdict, labels at each end, whether the short one touches, ends counted: mullions")
    for key, n in sorted(near_short_table.items(), key=lambda kv: (kv[0][0], -kv[1])):
        print(f"  {key}: {n}")
    print(f"different: {len(wrong)}")
    for tag, host, kind, m, fnd, predicted, written, nsa, ends, group in wrong:
        sec = sorted(round(2 * h * MM, 1) for h in m["half"][1:])
        print(
            f"tag {tag} host {host} read as {kind} type {m['type']} section {sec} span {round(2 * m['half'][0] * MM)} "
            f"upright {abs(m['u'][0][2]) > 0.999} predicted {predicted} written {written:.6f} Revit {nsa:.6f} "
            f"Revit's ends equivalent {None if ends is None else round(ends, 4)}"
        )
        for i, (p, out) in enumerate(M.end_points(m)):
            labs = "+".join(sorted(fnd[i])) or "nothing"
            print(f"    end {i}: {labs}")
            for row in neighbours(m, p, out, group):
                print(f"        {row}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
