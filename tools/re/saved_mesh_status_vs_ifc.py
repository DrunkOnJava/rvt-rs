#!/usr/bin/env python3
"""Does the saved scene's record of what it dropped tell a complete saved
mesh from a partial one (RE-224, W6)?

Reads `probes/examples/probe_re224_saved_mesh_status.rs`'s output and the rvt-rs
export beside it (`model.ifc`, made with `--saved-meshes` and kept by
Measure's `keep_ifc`), and scores every saved-mesh body against Revit's body
with the same `Tag` as `mesh_bodies_vs_ifc.py` does: exact when every tested
vertex of each lies within 0.001 ft of the other's surface. A class with more
than 300 such bodies is scored on an even stride of 300 by Tag.

It scores only the meshes the export draws, so the export must draw every
saved mesh: one made with RE-224's gate in `saved_meshes::attach` keeps the
box for each mesh carrying a decode failure, and those are never scored. Run
it on an export from before the gate (commit 14325427, `re/saved-mesh-status`).

Then, for each signal the probe prints (the scene's status, unbounded faces,
rejected filters, hidden branches, curve-only branches, and each kind of
diagnostic), how many bodies with and without it are exact. A signal that
holds on every partial body and on no exact one is what RE-78 lacked.

Usage:

    python3 tools/re/saved_mesh_status_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mesh_bodies_vs_ifc import (  # noqa: E402
    CLASS_CAP,
    SOURCE,
    meshes,
    sample,
    surface_distance,
    volume,
)

import ifcopenshell  # noqa: E402

from saved_meshes_vs_ifc import body_sources  # noqa: E402

COUNTERS = ["unbounded", "empty_trim", "rejected", "hidden", "curves"]


def read_probe(path):
    rows = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.rstrip("\n").split(" ", 11)
        if len(parts) < 12 or parts[0] != "mesh":
            continue
        _, tag, status, primitives, triangles, *counts, diagnostics, messages = parts
        rows[tag] = {
            "status": status,
            "primitives": int(primitives),
            "triangles": int(triangles),
            **dict(zip(COUNTERS, map(int, counts))),
            "diagnostics": int(diagnostics),
            "messages": [] if messages == "-" else messages.split(" | "),
        }
    return rows


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    probe_path, revit_path = args
    ours_path = os.path.join(os.path.dirname(os.path.abspath(probe_path)), "model.ifc")
    rows = read_probe(probe_path)
    print(f"probe: {len(rows)} saved-mesh candidates")
    statuses = collections.Counter(r["status"] for r in rows.values())
    print("status: " + ", ".join(f"{k} {v}" for k, v in sorted(statuses.items())))
    if not os.path.exists(ours_path):
        print(f"no rvt-rs export at {ours_path}: run Measure with keep_ifc and --saved-meshes")
        return 0
    model = ifcopenshell.open(ours_path)
    by_class = collections.defaultdict(list)
    for element in model.by_type("IfcElement"):
        if element.Tag in rows and not element.is_a("IfcOpeningElement") and body_sources(element) == SOURCE:
            by_class[element.is_a()].append(element.Tag)
    chosen = set()
    for cls, tags in by_class.items():
        tags = sorted(set(tags), key=lambda t: (len(t), t))
        step = max(1, -(-len(tags) // CLASS_CAP))
        chosen.update(tags[::step])
        if step > 1:
            print(f"{cls}: {len(tags)} bodies, every {step}th scored")
    del model
    ours = meshes(ours_path, tags=chosen, source=SOURCE)
    ref = meshes(revit_path, tags=set(ours))
    scored = {}
    for tag in sorted(set(ours) & set(ref), key=lambda t: (len(t), t)):
        _, _, our_triangles = ours[tag]
        cls, object_type, ref_triangles = ref[tag]
        ref_volume = volume(ref_triangles)
        ratio = volume(our_triangles) / ref_volume if ref_volume > 0 else float("inf")
        far = max(
            surface_distance(sample(ref_triangles), our_triangles),
            surface_distance(sample(our_triangles), ref_triangles),
        )
        scored[tag] = (far <= 0.001, far, ratio, cls, object_type)
    exact = sum(1 for v in scored.values() if v[0])
    print(f"scored against Revit's export: {len(scored)}, exact {exact}, not exact {len(scored) - exact}")

    def split(name, has):
        cells = collections.Counter()
        for tag, (ok, *_rest) in scored.items():
            cells[(has(rows[tag]), ok)] += 1
        print(
            f"  {name:<58} with: exact {cells[(True, True)]:>5} not {cells[(True, False)]:>5}"
            f"   without: exact {cells[(False, True)]:>5} not {cells[(False, False)]:>5}"
        )

    print("signal (bodies scored)")
    split("status partial_saved_graphics", lambda r: r["status"] == "partial_saved_graphics")
    for counter in COUNTERS:
        split(f"{counter} > 0", lambda r, c=counter: r[c] > 0)
    split("any diagnostic", lambda r: r["diagnostics"] > 0)
    kinds = collections.Counter(m for tag in scored for m in rows[tag]["messages"])
    for kind, _ in kinds.most_common(20):
        split(kind[:58], lambda r, k=kind: k in r["messages"])

    print("not exact with no signal at all")
    clean_misses = [
        (tag, v)
        for tag, v in scored.items()
        if not v[0]
        and rows[tag]["status"] == "decoded_saved_graphics"
        and all(rows[tag][c] == 0 for c in COUNTERS)
    ]
    per_class = collections.Counter(v[3] for _, v in clean_misses)
    for cls, n in per_class.most_common():
        print(f"  {cls:<28} {n}")
    by_type = collections.Counter((v[3], v[4]) for _, v in clean_misses)
    for (cls, object_type), n in by_type.most_common(12):
        print(f"    {n:>5}  {cls} '{object_type}'")
    if "--list" in sys.argv:
        for tag, (ok, far, ratio, cls, object_type) in sorted(scored.items(), key=lambda kv: (len(kv[0]), kv[0])):
            r = rows[tag]
            print(
                f"body {tag} {cls} exact={ok} far={far:.4g} volume={ratio:.4f} {r['status']}"
                f" unbounded={r['unbounded']} rejected={r['rejected']} hidden={r['hidden']}"
                f" curves={r['curves']} diagnostics={r['diagnostics']} '{object_type}'"
                f" {' | '.join(r['messages']) or '-'}"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
