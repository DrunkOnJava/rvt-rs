#!/usr/bin/env python3
"""Score the bodies a change draws from Revit's saved meshes that its base drew as record boxes
against Revit's own IFC export (#1645, RE-408).

`mesh_bodies_vs_ifc.py` scores every saved-mesh body of a file, so the few a change adds are lost
in its class table. This takes the rvt-rs export of the base and of the change (Measure's
`keep_ifc=true` keeps both) and Revit's export, finds the Tags whose `BodySource` is
`saved_graphics_mesh` in the change and is not in the base, and scores those alone, by IFC class:
how many Revit's export holds, how many have every vertex of Revit's within 0.01 ft of rvt-rs's
surface, how many have it both ways, the volume within 1% and 2% of Revit's, and what
`volume_readings.py` makes of them (equal, explained by an inscribed round section, doubled,
other). Then the 15 farthest and the 15 whose volume is farthest from Revit's.

Usage:

    python3 tools/re/changed_bodies_vs_ifc.py <base.ifc> <change.ifc> <revit.ifc>

Needs IfcOpenShell and numpy. RVT_MESH_WORKERS sets the processes, as for `mesh_bodies_vs_ifc.py`.
"""

import collections
import os
import sys

import ifcopenshell

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mesh_bodies_vs_ifc as meshes  # noqa: E402
import volume_readings  # noqa: E402
from saved_meshes_vs_ifc import body_sources  # noqa: E402


def sources(model):
    """Tag -> (IFC class, BodySource, Name) of each element that has a Tag."""
    out = {}
    for element in model.by_type("IfcElement"):
        if element.Tag and not element.is_a("IfcOpeningElement"):
            out.setdefault(element.Tag, (element.is_a(), body_sources(element), element.Name))
    return out


def main():
    if len(sys.argv) != 4:
        print(__doc__)
        return 2
    base_path, change_path, ref_path = sys.argv[1:4]
    base = sources(ifcopenshell.open(base_path))
    change_model = ifcopenshell.open(change_path)
    change = sources(change_model)
    new = {
        tag
        for tag, (_, source, _) in change.items()
        if source == meshes.SOURCE and base.get(tag, (None, None, None))[1] != meshes.SOURCE
    }
    print(f"bodies drawn from a saved mesh in the change and not in the base: {len(new)}")
    print("by class: " + ", ".join(f"{cls} {n}" for cls, n in sorted(collections.Counter(change[t][0] for t in new).items())))
    ours = meshes.meshes(change_path, tags=new, source=meshes.SOURCE, model=change_model, role="ours")
    del change_model
    ref = meshes.meshes(ref_path, tags=set(ours), role="ref")
    tags = sorted(set(ours) & set(ref), key=lambda t: (len(t), t))
    print(f"of them in Revit's export with a body: {len(tags)}")
    scores = dict(zip(tags, meshes.score_all(ours, ref, tags)))
    rows = collections.defaultdict(collections.Counter)
    scored = []
    for tag in tags:
        ratio, to_ours, to_ref = scores[tag]
        cls = ours[tag][0]
        far = max(to_ours, to_ref)
        kind = volume_readings.verdict(volume_readings.read_body(ours[tag][2], ref[tag][2], far))
        for key in (cls, "total"):
            row = rows[key]
            row["n"] += 1
            row["Revit on ours 0.01 ft"] += to_ours <= 0.01
            row["both 0.01 ft"] += far <= 0.01
            row["vol 1%"] += abs(ratio - 1) <= 0.01
            row["vol 2%"] += abs(ratio - 1) <= 0.02
            row["both bars"] += far <= 0.01 and abs(ratio - 1) <= 0.02
            row["read " + kind] += 1
        scored.append((far, tag, cls, change[tag][2], ratio, to_ours, to_ref, kind))
    for key in sorted(rows, key=lambda k: (k == "total", -rows[k]["n"])):
        print(f"{key:28s} " + " ".join(f"{name}={count}" for name, count in sorted(rows[key].items())))
    print("farthest 15:")
    for far, tag, cls, name, ratio, to_ours, to_ref, kind in sorted(scored, reverse=True)[:15]:
        print(
            f"  {tag} {cls} '{name}': volume ratio {ratio:.4f}, Revit's vertices to rvt-rs's surface {to_ours:.4g} ft,"
            f" rvt-rs's to Revit's {to_ref:.4g} ft ({kind})"
        )
    print("volume farthest from Revit's, 15:")
    for far, tag, cls, name, ratio, to_ours, to_ref, kind in sorted(scored, key=lambda s: -abs(s[4] - 1))[:15]:
        print(f"  {tag} {cls} '{name}': volume ratio {ratio:.4f}, farthest vertex {far:.4g} ft ({kind})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
