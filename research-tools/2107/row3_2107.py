"""Row 3 of #2107: every body whose mesh moves in run M is one run P lists with a cylinder pair.

    python row3_2107.py <run M download dir> <p2-owners.json>

For each model whose ifc.sha256 differs between measure-base and measure-head, reads both
exports with ifc_body_diff.read and lists the bodies whose geometry or BodySource changes, or
that exist on one side only. Each is checked against run P's lists for that model: its bodies
with a cylinder/cylinder pair line, and every body it read. Prints counts by model and class, and
every body outside the cylinder-pair list.
"""
import collections
import importlib.util
import json
import os
import sys

spec = importlib.util.spec_from_file_location(
    "bd", __import__("os").path.join(__import__("os").path.dirname(__import__("os").path.abspath(__file__)), "ifc_body_diff.py"))
bd = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bd)

root, owners = sys.argv[1], json.load(open(sys.argv[2]))


def sha(side, model):
    p = os.path.join(root, side, model, "ifc.sha256")
    return open(p).read().split()[0] if os.path.isfile(p) else None


models = sorted(m for m in os.listdir(os.path.join(root, "measure-head"))
                if os.path.isdir(os.path.join(root, "measure-head", m)) and sha("measure-base", m) != sha("measure-head", m))
outside, total = [], 0
for m in models:
    base = os.path.join(root, "measure-base", m, "model.ifc")
    head = os.path.join(root, "measure-head", m, "model.ifc")
    if not (os.path.isfile(base) and os.path.isfile(head)):
        print(f"{m:52} export not kept")
        continue
    bp, hp = bd.read(base), bd.read(head)
    moved = [g for g in set(bp) | set(hp) if g not in bp or g not in hp or bp[g][4] != hp[g][4] or bp[g][0] != hp[g][0]]
    tags = {g: str((hp.get(g) or bp.get(g))[1]) for g in moved}
    cyl = set(owners.get(m, {}).get("cylinder_pair", []))
    read = set(owners.get(m, {}).get("read", []))
    classes = collections.Counter((hp.get(g) or bp.get(g))[0] for g in moved)
    out = [(g, tags[g], (hp.get(g) or bp.get(g))[0], tags[g] in read, g in bp, g in hp) for g in moved if tags[g] not in cyl]
    total += len(moved)
    print(f"{m:52} moved {len(moved):5} outside the cylinder-pair list {len(out):4} | {dict(classes.most_common(4))}")
    outside += [(m,) + o for o in out]
print(f"\nmoved {total}; outside the cylinder-pair list {len(outside)}")
for o in outside[:40]:
    print(f"  {o[0]} Tag {o[2]} {o[3]} read_by_P={o[4]} in_base={o[5]} in_head={o[6]}")
