"""Compare the head and base results of one Measure run, per model.

Usage: python3 -I cmp_measure.py <measure-base directory> <measure-head directory>
(each holds one directory per model, as the Measure artifacts of those names do)

Prints: the models whose export moved (ifc.sha256), the scorer files that differ by name,
every model where a property or quantity census's `equal` fell or `different` or `missing`
rose, and what the census is blind to: an element's volume that moved away from Revit's, a
"N of M" coverage count that fell, and rows on a different storey that rose. A summary line
for each ends the output: "census regressions: N" and "body scorer regressions: N".
"""
import collections
import glob
import os
import re
import sys

if len(sys.argv) != 3:
    sys.exit(__doc__)


def models(root):
    return {os.path.basename(path): path for path in glob.glob(f"{root}/*") if os.path.isdir(path)}


base, head = models(sys.argv[1]), models(sys.argv[2])
print(f"{len(head)} head models, {len(base)} base models")
moved = []
for name in sorted(head):
    if name not in base:
        continue
    a = base[name] + "/ifc.sha256"
    b = head[name] + "/ifc.sha256"
    if os.path.exists(a) and os.path.exists(b):
        if open(a).read().split()[:1] != open(b).read().split()[:1]:
            moved.append(name)
print(f"exports that moved: {len(moved)}")
print("  " + ", ".join(moved[:200]))

skip = {"ifc.sha256", "timings.log", "info.log", "rvt-ifc.log", "runner.txt", "info.exit", "info.json", "mcp-dump.json"}
differing = collections.defaultdict(list)
for name in sorted(head):
    if name not in base:
        continue
    for f in sorted(os.listdir(head[name])):
        if f in skip or os.path.isdir(f"{head[name]}/{f}"):
            continue
        g = f"{base[name]}/{f}"
        if not os.path.exists(g):
            differing[f].append((name, "only on head"))
            continue
        if open(g, errors="replace").read() != open(f"{head[name]}/{f}", errors="replace").read():
            differing[f].append((name, "differs"))
print("\nscorer files that differ, by file (number of models):")
for f, items in sorted(differing.items(), key=lambda kv: -len(kv[1])):
    print(f"  {f}: {len(items)}")

# Census headers: "...; equal: N; different: M; missing: K; ..."
pat = re.compile(r"equal: (\d+); different: (\d+); missing: (\d+)")
print("\nproperty and quantity census, equal fell / different or missing rose:")
bad = 0
for f in ("psets_vs_ifc.txt", "qtos_vs_ifc.txt"):
    for name in sorted(head):
        if name not in base:
            continue
        a, b = base[name] + "/" + f, head[name] + "/" + f
        if not (os.path.exists(a) and os.path.exists(b)):
            continue
        ma, mb = pat.search(open(a, errors="replace").readline()), pat.search(open(b, errors="replace").readline())
        if not (ma and mb):
            continue
        ea, da, xa = map(int, ma.groups())
        eb, db, xb = map(int, mb.groups())
        if eb < ea or db > da:
            bad += 1
            print(f"  {f} {name}: equal {ea} -> {eb}, different {da} -> {db}, missing {xa} -> {xb}")
        elif xb != xa or eb != ea or db != da:
            print(f"  (moved, no loss) {f} {name}: equal {ea} -> {eb}, different {da} -> {db}, missing {xa} -> {xb}")
print(f"census regressions: {bad}")

# Body and placement scorers: the census above is blind to them. Flag, per model, an element whose
# volume moved away from Revit's by more than 5 percent of Revit's, a "N of M" coverage line whose N
# fell, and a count of rows on a different storey that rose.
vol = re.compile(r"^\s+(\d+): .*?volume rvt-rs ([\d.]+) ft3, Revit ([\d.]+) ft3")
cov = re.compile(r"^(.*): (\d+) of (\d+)$")
sto = re.compile(r"^\s+(\d+)\s+(a different storey from Revit's|on no storey; Revit's is a storey)$")
census = {"psets_vs_ifc.txt", "qtos_vs_ifc.txt"}
print("\nbody and placement scorers, worse than base:")
worse = 0
for name in sorted(head):
    if name not in base:
        continue
    for f in sorted(os.listdir(head[name])):
        if f in skip or f in census or not f.endswith(".txt") or not os.path.exists(f"{base[name]}/{f}"):
            continue
        a = open(f"{base[name]}/{f}", errors="replace").read().splitlines()
        b = open(f"{head[name]}/{f}", errors="replace").read().splitlines()
        va = {m.group(1): (float(m.group(2)), float(m.group(3))) for m in map(vol.match, a) if m}
        vb = {m.group(1): (float(m.group(2)), float(m.group(3))) for m in map(vol.match, b) if m}
        moved_away = [
            (k, va[k][0], vb[k][0], vb[k][1])
            for k in vb
            if k in va and vb[k][1] > 0 and abs(vb[k][0] - vb[k][1]) > abs(va[k][0] - va[k][1]) + 0.05 * vb[k][1]
        ]
        if moved_away:
            worse += 1
            worst = max(moved_away, key=lambda t: abs(t[2] - t[3]) / t[3])
            print(f"  {f} {name}: {len(moved_away)} element volumes moved away from Revit's; worst {worst[0]}: {worst[1]:.1f} -> {worst[2]:.1f} ft3, Revit {worst[3]:.1f}")
        # The same label opens several groups of one file: pair the nth line of a label with the nth.
        def numbered(lines):
            seen, out = collections.Counter(), {}
            for m in filter(None, map(cov.match, lines)):
                out[(m.group(1), seen[m.group(1)])] = (int(m.group(2)), m.group(3))
                seen[m.group(1)] += 1
            return out
        ca, cb = numbered(a), numbered(b)
        for key, (n, total) in cb.items():
            if key in ca and n < ca[key][0]:
                worse += 1
                print(f"  {f} {name}: '{key[0].strip()}' fell {ca[key][0]} -> {n} of {total}")
        sa = {m.group(2): int(m.group(1)) for m in map(sto.match, a) if m}
        for m in map(sto.match, b):
            if m and int(m.group(1)) > sa.get(m.group(2), 0):
                worse += 1
                print(f"  {f} {name}: rows '{m.group(2)}' rose {sa.get(m.group(2), 0)} -> {m.group(1)}")
print(f"body scorer regressions: {worse}")
