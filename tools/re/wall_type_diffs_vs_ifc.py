"""Counts only: of the wall quantities rvt-rs writes that differ from Revit's, how many are on a
wall the two exports give different types (RE-253), and the ratios of the others.

Usage: wall_type_diffs_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""
import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qtos_vs_ifc as q

ours_path, theirs_path = sys.argv[1], sys.argv[2]
ours, our_objects, our_entities, _, _, _ = q.quantities(ours_path)
theirs, their_objects, _, _, _, _ = q.quantities(theirs_path, our_entities)
beside = os.path.join(os.path.dirname(os.path.abspath(ours_path)), "diagnostics.json")
confirmed = q.confirmed_walls(beside)
our_types, their_types = q.wall_types(ours_path), q.wall_types(theirs_path)
tally = collections.Counter()
for (key, qset, name), value in sorted(theirs.items()):
    if qset != "Qto_WallBaseQuantities" or key not in our_objects:
        continue
    mine = ours.get((key, qset, name))
    if mine is None or q.same(mine, value):
        continue
    other = our_types.get(key) != their_types.get(key)
    tally[(name, "types differ" if other else "same type", "confirmed" if key in confirmed else "unconfirmed",
           "ratio %.3f" % q.relative(mine, value) if hasattr(q, "relative") else "")] += 1
print("wall quantities different from Revit's, by quantity, type agreement and the record's confirmation:")
for k, n in sorted(tally.items()):
    print("  ", n, *k)
print("walls the two exports give different types:", sum(1 for k in our_types if k in their_types and our_types[k] != their_types[k]))
