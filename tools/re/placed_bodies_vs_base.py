#!/usr/bin/env python3
"""Whether a change to the local axes of doors and windows moved their bodies
or their openings in the world (RE-347, #1447).

Research tool for `reports/element-framing/RE-347-door-window-placement-axes.md`.
Two exports of the same model by two builds, the base's and the head's, are
paired by Tag. For every `IfcDoor` and `IfcWindow` in both:

- whether its placement axes moved (any entry of the composed 3 x 3 differs by
  more than 1e-6);
- its body meshed with its placement applied, in world coordinates: its volume
  (relative difference) and its vertices (how far the farthest vertex of one is
  from the nearest of the other, both ways);
- the same for the `IfcOpeningElement` it fills (paired by GlobalId, which
  rvt-rs derives from the filler's);
- its GlobalId, its type's GlobalId and the host wall of its opening.

A body that meshes in neither export, for the same reason, is counted apart
(`meshed in neither export, not compared`) and is not in `paired`.

A body or an opening that moves in the world is a bug, unless the change
moves it toward Revit's, which this does not measure: a nonzero count below
is for the reader to resolve against `door_bodies_vs_ifc` and
`symbol_cuts_vs_ifc`. It prints count lines and the largest deviations, never
an ElementId of a model that is held out; run it on open models only.

Usage:

    python3 tools/re/placed_bodies_vs_base.py <base.ifc> <head.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins) and NumPy.
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement
import numpy as np

FEET = 0.3048
VERTEX_TOLERANCE = 1e-6
VOLUME_TOLERANCE = 1e-9
AXES_TOLERANCE = 1e-6
SHOW = 10


def tagged(f):
    out = collections.defaultdict(list)
    for cls in ("IfcDoor", "IfcWindow"):
        for element in f.by_type(cls):
            if element.Tag is not None:
                out[str(element.Tag)].append(element)
    for tag in out:
        out[tag].sort(key=lambda e: e.GlobalId or "")
    return out


def axes(element):
    if element.ObjectPlacement is None:
        return None
    return np.array(ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement), dtype=float)[:3, :3]


def mesh(settings, element):
    """(volume, vertices) in metres, or the reason it did not mesh."""
    if element is None or element.Representation is None:
        return "no representation"
    try:
        shape = ifcopenshell.geom.create_shape(settings, element)
    except RuntimeError as error:
        return (str(error).strip().splitlines() or [""])[0] or type(error).__name__
    verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
    faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
    if len(faces) == 0:
        return "no faces"
    a, b, c = (verts[faces[:, i]] - verts[0] for i in range(3))
    volume = abs(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0)
    return volume, np.unique(np.round(verts, 9), axis=0)


def reach(a, b):
    worst = 0.0
    step = max(1, 2_000_000 // max(len(b), 1))
    for i in range(0, len(a), step):
        distance = np.linalg.norm(a[i : i + step, None, :] - b[None, :, :], axis=2).min(axis=1)
        worst = max(worst, float(distance.max()))
    return worst


def opening_of(element):
    for rel in getattr(element, "FillsVoids", None) or ():
        return rel.RelatingOpeningElement
    return None


def host_of(opening):
    if opening is None:
        return None
    for rel in getattr(opening, "VoidsElements", None) or ():
        return rel.RelatingBuildingElement.GlobalId
    return None


class Tally:
    def __init__(self, label):
        self.label = label
        self.paired = 0
        self.unmeshed = 0
        self.failed_alike = 0
        self.volume_moved = 0
        self.vertices_moved = 0
        self.worst_vertex = 0.0
        self.worst_volume = 0.0
        self.shown = []

    def add(self, key, base, head):
        if isinstance(base, str) or isinstance(head, str):
            if base == head:
                # Neither export meshes it, for the same reason: nothing is compared.
                self.failed_alike += 1
                self.shown.append((key, f"meshed in neither export: {base}"))
            else:
                self.paired += 1
                self.unmeshed += 1
                self.shown.append((key, f"meshes differ: {base if isinstance(base, str) else 'meshes'} / {head if isinstance(head, str) else 'meshes'}"))
            return
        self.paired += 1
        relative = abs(base[0] - head[0]) / max(base[0], 1e-12)
        self.worst_volume = max(self.worst_volume, relative)
        away = max(reach(base[1], head[1]), reach(head[1], base[1]))
        self.worst_vertex = max(self.worst_vertex, away)
        if relative > VOLUME_TOLERANCE:
            self.volume_moved += 1
        if away > VERTEX_TOLERANCE:
            self.vertices_moved += 1
            self.shown.append((key, f"vertex {away / FEET:.6f} ft away"))

    def line(self):
        return (
            f"placed bodies vs base: {self.label}: paired {self.paired}; meshed in neither export, not compared {self.failed_alike}; "
            f"meshed in one export only or differently {self.unmeshed}; "
            f"volume moved {self.volume_moved} (largest relative change {self.worst_volume:.3g}); "
            f"vertices moved {self.vertices_moved} (largest {self.worst_vertex / FEET:.6f} ft)"
        )


def main(argv):
    if len(argv) != 3:
        raise SystemExit(__doc__)
    base, head = ifcopenshell.open(argv[1]), ifcopenshell.open(argv[2])
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    by_base, by_head = tagged(base), tagged(head)
    only_base = sorted(set(by_base) - set(by_head))
    only_head = sorted(set(by_head) - set(by_base))
    print(f"placed bodies vs base: doors and windows in the base {sum(map(len, by_base.values()))}, in the head {sum(map(len, by_head.values()))}; tags only in the base {len(only_base)}, only in the head {len(only_head)}")
    bodies, openings = Tally("bodies"), Tally("openings")
    moved = collections.Counter()
    identity = collections.Counter()
    for tag in sorted(set(by_base) & set(by_head), key=lambda t: (len(t), t)):
        if len(by_base[tag]) != len(by_head[tag]):
            identity["products under a Tag differ in number"] += 1
            continue
        for left, right in zip(by_base[tag], by_head[tag]):
            if left.is_a() != right.is_a():
                identity["class"] += 1
            if left.GlobalId != right.GlobalId:
                identity["GlobalId"] += 1
            left_type, right_type = ifcopenshell.util.element.get_type(left), ifcopenshell.util.element.get_type(right)
            if (left_type.GlobalId if left_type else None) != (right_type.GlobalId if right_type else None):
                identity["type GlobalId"] += 1
            left_axes, right_axes = axes(left), axes(right)
            turned = left_axes is not None and right_axes is not None and float(np.abs(left_axes - right_axes).max()) > AXES_TOLERANCE
            moved[(left.is_a(), turned)] += 1
            bodies.add(f"{tag} {left.is_a()}", mesh(settings, left), mesh(settings, right))
            opening_left, opening_right = opening_of(left), opening_of(right)
            if (opening_left is None) != (opening_right is None):
                identity["opening present in one"] += 1
            elif opening_left is not None:
                if opening_left.GlobalId != opening_right.GlobalId:
                    identity["opening GlobalId"] += 1
                if host_of(opening_left) != host_of(opening_right):
                    identity["host wall"] += 1
                openings.add(f"{tag} opening of {left.is_a()}", mesh(settings, opening_left), mesh(settings, opening_right))
    for cls in ("IfcDoor", "IfcWindow"):
        print(f"placed bodies vs base: {cls} placement axes moved {moved[(cls, True)]}, unchanged {moved[(cls, False)]}")
    print(bodies.line())
    print(openings.line())
    print("placed bodies vs base: identity differences: " + (", ".join(f"{k} {v}" for k, v in sorted(identity.items())) or "none"))
    for tally in (bodies, openings):
        for key, what in tally.shown[:SHOW]:
            print(f"  {key}: {what}")


if __name__ == "__main__":
    main(sys.argv)
