#!/usr/bin/env python3
"""A wall's base attachment (a JoinToRoofGStep whose m_baseOrTop is 0) against the class of Revit's body and the kind of its quantity set.
Usage: base_attach.py <probe.txt> <rows.jsonl>"""
import collections, json, re, sys
recs = {}
for line in open(sys.argv[1], errors="replace"):
    if line.startswith("rec "):
        parts = line.split()
        recs[parts[1]] = dict(kv.split("=", 1) for kv in parts[3:] if "=" in kv)
rows = [json.loads(l) for l in open(sys.argv[2])]
c = collections.Counter()
for r in rows:
    rec = recs.get(r["tag"])
    if rec is None: c[(r["cls"], "no record", r["kind"])] += 1; continue
    sides = [v for k, v in rec.items() if re.fullmatch(r"JoinToRoofGStep(#\d+)?\.m_baseOrTop", k)]
    att = "base attached" if "0" in sides else ("top attached only" if sides else "not attached")
    c[(r["cls"], att, r["kind"])] += 1
for cls in ("prism", "less-openings", "unexplained"):
    for att in ("not attached", "top attached only", "base attached", "no record"):
        f, v = c[(cls, att, "full")], c[(cls, att, "volume-only")]
        if f or v: print(f"  {cls:14s} {att:18s} full {f:4d}   volume-only {v:4d}")
