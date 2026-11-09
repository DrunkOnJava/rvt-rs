#!/usr/bin/env python3
"""Each wall's `Qto_WallBaseQuantities` in two rvt-rs exports of the same
project saved by two Revit releases, wall for wall (RE-364 section 9).

No Revit export of Autodesk's sample projects exists, so a quantity rvt-rs
writes on a Revit 2015 to 2023 save is held to the same project's later save,
whose export is measured against Revit's own on the reference models. Walls
are paired by `Tag` (their ElementId). For each quantity but `Width` and
`Length` it counts:

  equal               both saves write it, within 1e-6 relative
  different           both write it, and the values differ
  only candidate      the candidate writes it and the reference does not:
                      split by whether the reference holds the wall at all
  only reference      the reference writes it and the candidate does not

A value only the candidate writes on a wall the reference holds is checked
against nothing. For those walls the reference's `BodySource` is listed, and
the candidate's `GrossVolume` is compared with the volume of the reference's
body, meshed without its openings (0.1% relative), since the reference may draw
a body its own rule writes no quantity for.

What this cannot catch: an error both saves share.

Usage:

    python3 tools/re/save_pair_wall_quantities.py <candidate.ifc> <reference.ifc>

The exit status is 1 when a value differs, 0 otherwise.
"""
import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.unit
import numpy as np

SKIPPED = ("Width", "Length")


def walls(model):
    out = {}
    for wall in model.by_type("IfcWall"):
        if wall.Tag:
            out.setdefault(wall.Tag, []).append(wall)
    return out


def quantities(wall):
    found = ifcopenshell.util.element.get_psets(wall, qtos_only=True).get("Qto_WallBaseQuantities", {})
    return {k: v for k, v in found.items() if k not in SKIPPED and k != "id" and isinstance(v, (int, float))}


def body_source(wall):
    return ifcopenshell.util.element.get_psets(wall).get("RvtElementRecordGeometry", {}).get("BodySource")


def volume(settings, wall):
    try:
        shape = ifcopenshell.geom.create_shape(settings, wall)
    except RuntimeError as error:
        return None, (str(error).strip().splitlines() or [""])[0] or type(error).__name__
    verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
    if len(faces) == 0:
        return None, "no faces"
    origin = verts[0]
    a, b, c = (verts[faces[:, i]] - origin for i in range(3))
    return abs(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0), None


def close(a, b, relative):
    return abs(a - b) <= max(1e-9, relative * max(abs(a), abs(b)))


def main():
    if len(sys.argv) != 3:
        print(__doc__.strip().split("Usage:")[1].strip(), file=sys.stderr)
        return 2
    candidate, reference = (ifcopenshell.open(path) for path in sys.argv[1:])
    scale = ifcopenshell.util.unit.calculate_unit_scale(reference)
    settings = ifcopenshell.geom.settings()
    settings.set("disable-opening-subtractions", True)
    ours, theirs = walls(candidate), walls(reference)
    counts = collections.defaultdict(collections.Counter)
    unchecked = {}
    different = []
    for tag, items in sorted(ours.items()):
        if len(items) != 1 or len(theirs.get(tag, [])) > 1:
            counts["(any)"]["tag on more than one wall, not compared"] += 1
            continue
        mine = quantities(items[0])
        other = quantities(theirs[tag][0]) if tag in theirs else {}
        for name, value in mine.items():
            if name in other:
                if close(value, other[name], 1e-6):
                    counts[name]["equal"] += 1
                else:
                    counts[name]["different"] += 1
                    different.append((tag, name, value, other[name]))
            elif tag in theirs:
                counts[name]["only candidate, reference holds the wall"] += 1
                unchecked.setdefault(tag, set()).add(name)
            else:
                counts[name]["only candidate, wall not in reference"] += 1
        for name in other:
            if name not in mine:
                counts[name]["only reference"] += 1
    for name in sorted(counts):
        print(f"{name}: " + ", ".join(f"{k} {v}" for k, v in sorted(counts[name].items())))
    for tag, name, mine, theirs_value in different:
        print(f"different: wall {tag} {name} {mine:.9g} against {theirs_value:.9g}")
    volumes = collections.Counter()
    for tag in sorted(unchecked):
        wall, other = ours[tag][0], theirs[tag][0]
        gross = quantities(wall).get("GrossVolume")
        line = f"unchecked: wall {tag} {','.join(sorted(unchecked[tag]))}; reference BodySource={body_source(other)}"
        if gross is not None:
            v, why = volume(settings, other)
            if v is None:
                volumes["reference body did not mesh"] += 1
                line += f"; reference body did not mesh: {why}"
            else:
                # The quantity is in the file's units, the mesh in metres.
                gross *= scale ** 3
                same = close(gross, v, 1e-3)
                volumes["GrossVolume equals the reference body's volume" if same else "GrossVolume differs from the reference body's volume"] += 1
                line += f"; GrossVolume {gross:.6f} against the reference body's {v:.6f} m3 ({v - gross:+.6f})"
        print(line)
    for k, v in sorted(volumes.items()):
        print(f"{k}: {v}")
    return 1 if different else 0


if __name__ == "__main__":
    sys.exit(main())
