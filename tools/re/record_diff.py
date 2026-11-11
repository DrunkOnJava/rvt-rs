#!/usr/bin/env python3
"""Which fields of a wall's native record separate the exceptional walls from the walls of the same class that get the full set.
Usage: record_diff.py <probe.txt> <rows.jsonl> <tag>..."""
import collections, json, sys
recs = {}
for line in open(sys.argv[1], errors="replace"):
    if line.startswith("rec "):
        parts = line.split()
        recs[parts[1]] = dict(kv.split("=", 1) for kv in parts[3:] if "=" in kv)
rows = {json.loads(l)["tag"]: json.loads(l) for l in open(sys.argv[2])}
E = [t for t in sys.argv[3:] if t in recs]
C = [t for t, r in rows.items() if r["cls"] == "less-openings" and r["kind"] == "full" and t in recs]
V = [t for t, r in rows.items() if r["kind"] == "volume-only" and t in recs and t not in E]
print(f"exceptions with a record: {len(E)}; full-set walls of the class with a record: {len(C)}; other volume-only walls: {len(V)}")
def skip(k): return any(x in k for x in (".offset", "pointer_token", "m_id.m_id.m_id64", "class_tag")) or k.startswith(("MP.", "MR.", "MF."))
keys = sorted({k for t in E + C for k in recs[t] if not skip(k)})
out = []
for k in keys:
    ev = [recs[t].get(k) for t in E]; cv = [recs[t].get(k) for t in C]; vv = [recs[t].get(k) for t in V]
    ep, cp = sum(v is not None for v in ev), sum(v is not None for v in cv)
    if ep == len(E) and cp == 0: out.append((0, k, f"present on every exception {set(ev)}, on no full-set wall; other volume-only walls with it: {sum(v is not None for v in vv)} of {len(V)}"))
    elif ep == 0 and cp == len(C): out.append((0, k, "absent on every exception, present on every full-set wall"))
    elif ep == len(E) and len(set(ev)) == 1:
        same = sum(v == ev[0] for v in cv)
        if same <= max(1, len(C) // 20): out.append((1 + same, k, f"every exception = {ev[0]}; full-set walls with that value: {same} of {cp} holding the field (their values: {collections.Counter(cv).most_common(3)}); other volume-only walls with it: {sum(v == ev[0] for v in vv)} of {len(V)}"))
for _, k, why in sorted(out)[:40]: print(" ", k, "|", why[:330])
print("fields compared:", len(keys), "| separating candidates:", len(out))
