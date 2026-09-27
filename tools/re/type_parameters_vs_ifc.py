#!/usr/bin/env python3
"""Score rvt-rs's `Fire Rating` (RE-77) per Tag against the `FireRating` Revit's
own IFC export writes in its Pset_*Common property sets.

Usage: python3 tools/re/type_parameters_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (tested with 0.8.5)."""
import sys, re, collections
import ifcopenshell, ifcopenshell.util.element as ue
ours_path, theirs_path = sys.argv[1], sys.argv[2]
def by_tag(path, getter):
    f = ifcopenshell.open(path); out = {}
    for e in f.by_type("IfcElement"):
        if e.is_a("IfcOpeningElement") or not e.Tag: continue
        v = getter(e)
        if v is not None: out[str(e.Tag)] = v
    return out
def ours_fire(e):
    for pset, props in ue.get_psets(e).items():
        if "Fire Rating" in props: return props["Fire Rating"]
def theirs_fire(e):
    for pset, props in ue.get_psets(e).items():
        if pset.endswith("Common") and "FireRating" in props: return props["FireRating"]
o = by_tag(ours_path, ours_fire); t = by_tag(theirs_path, theirs_fire)
both = set(o) & set(t)
eq = sum(1 for k in both if o[k] == t[k])
print(f"ours with Fire Rating {len(o)}, Revit's with FireRating {len(t)}, both {len(both)}, equal {eq}")
bad = [(k, o[k], t[k]) for k in both if o[k] != t[k]][:8]
print("differ sample:", bad)
only_t = collections.Counter(t[k] for k in set(t) - set(o)); print("Revit's only (values):", only_t.most_common(5))
only_o = collections.Counter(o[k] for k in set(o) - set(t)); print("ours only (values):", only_o.most_common(5))
