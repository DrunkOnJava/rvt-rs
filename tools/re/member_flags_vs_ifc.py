"""Score probe_1046_member_flags against Revit's Qto_MemberBaseQuantities.NetSurfaceArea.

Usage: member_flags_vs_ifc.py <probe output> <revit.ifc>
Counts only: of the members Revit gives a NetSurfaceArea, how many equal the area of the primitives
no flag marks, the area of every primitive, and how many members hold a flagged primitive at all.
"""
import collections
import sys

import ifcopenshell
import ifcopenshell.util.element

FT2 = 0.09290304
prims = collections.defaultdict(list)
flags = {}
for line in open(sys.argv[1]):
    w = line.split()
    if not w:
        continue
    if w[0] == "member":
        flags[w[1]] = (int(w[5]), int(w[7]), int(w[9]))
    elif w[0] == "prim":
        prims[w[1]].append((int(w[3]), int(w[5]), float(w[7]) * FT2))
revit = {}
for m in ifcopenshell.open(sys.argv[2]).by_type("IfcMember"):
    q = ifcopenshell.util.element.get_psets(m, qtos_only=True).get("Qto_MemberBaseQuantities", {})
    if "NetSurfaceArea" in q and m.Tag:
        revit[f"tag:{m.Tag}"] = q["NetSurfaceArea"]
close = lambda a, b: abs(a - b) <= max(1e-6, 1e-5 * max(abs(a), abs(b)))
n = collections.Counter()
for tag, value in revit.items():
    if tag not in prims:
        n["no probe record"] += 1
        continue
    p = prims[tag]
    unflagged = sum(a for h, f, a in p if not h and not f)
    every = sum(a for h, f, a in p)
    any_flag = any(h or f for h, f, a in p)
    n["members with a Revit NetSurfaceArea and a mesh"] += 1
    n["with a hidden or filtered primitive"] += any_flag
    n["Revit == area of unflagged primitives"] += close(unflagged, value)
    n["Revit == area of every primitive"] += close(every, value)
    if any_flag:
        n["flagged members: Revit == unflagged area"] += close(unflagged, value)
        n["flagged members: Revit == every primitive"] += close(every, value)
    else:
        n["unflagged members: Revit == area"] += close(every, value)
        n["unflagged members: Revit less than area"] += value < every * (1 - 1e-5)
for k, v in n.items():
    print(f"{k}: {v}")
print("members in probe:", len(prims), "prims per member (max):", max((len(p) for p in prims.values()), default=0))
