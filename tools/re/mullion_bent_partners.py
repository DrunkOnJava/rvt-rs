#!/usr/bin/env python3
"""Whether a continuation that is bent off a mullion's axis hides its end face
in Revit's NetSurfaceArea (RE-432, #1735, W7).

RE-268 reads which end faces of a curtain mullion Revit's solid keeps from the
boxes of the mullions of its curtain wall: an end is hidden where a mullion
that runs the same way (a cosine above 0.999) has an end within 3 mm and its
body beyond, its partner. A partner can run the same way and still be bent off
the axis, by up to 2.56 degrees at that cosine. This script takes every end
with a partner of rvt-rs's boxes (an export made with `--no-saved-meshes`, as
`mullion_ends_vs_boxes.py` reads), the angle between the two axes, and counts,
by angle, the ends:

- the 5 angle bands from under 0.01 degrees to the 2.56 the cosine allows, and
  how many mullions have a partner in each;
- for each mullion with at least one partner bent by 0.05 degrees or more, the
  number of end faces Revit's own solid leaves out of its NetSurfaceArea,
  (the solid's surface area minus the value) over the end face's area, the end
  face being the NetVolume over the Length, and by the number of bent ends and
  of coinciding partners (under 0.05 degrees) the mullion has.

Usage:

    python3 tools/re/mullion_bent_partners.py <rvt-rs.ifc> <revit-export.ifc>

`<rvt-rs.ifc>` must be an export made with `--no-saved-meshes`. Needs
IfcOpenShell (the version tools/ci/requirements-ifc.txt pins) and numpy.
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mullion_ends_vs_boxes as boxes_module  # noqa: E402

BENT_DEGREES = 0.05
BANDS = ((0.01, "under 0.01"), (0.05, "0.01 to 0.05"), (0.25, "0.05 to 0.25"), (1.0, "0.25 to 1"), (2.0, "1 to 2"), (2.6, "2 to 2.56"))


def band(degrees):
    for limit, name in BANDS:
        if degrees < limit:
            return name
    return "2.56 and over"


def partner_angles(mullion, point, out, group):
    """The angle, degrees, to each partner of `mullion` at the end `point`."""
    angles = []
    for other in group:
        if other is mullion or other["axis"] is None:
            continue
        if boxes_module.box_distance(other, point) > boxes_module.TOL:
            continue
        cosine = abs(other["u"][0] @ mullion["u"][0])
        if cosine <= boxes_module.PARALLEL:
            continue
        for end, _ in boxes_module.end_points(other):
            if np.linalg.norm(end - point) < boxes_module.TOL:
                far = [e for e, _ in boxes_module.end_points(other) if np.linalg.norm(e - point) >= boxes_module.TOL]
                if far and (far[0] - point) @ out > boxes_module.TOL:
                    angles.append(float(np.degrees(np.arccos(min(1.0, cosine)))))
                break
    return angles


def surface_area(settings, member):
    shape = ifcopenshell.geom.create_shape(settings, member)
    vertices = np.array(shape.geometry.verts).reshape(-1, 3)
    faces = np.array(shape.geometry.faces).reshape(-1, 3)
    a, b, c = vertices[faces[:, 0]], vertices[faces[:, 1]], vertices[faces[:, 2]]
    return 0.5 * float(np.linalg.norm(np.cross(b - a, c - a), axis=1).sum())


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours = ifcopenshell.open(argv[1])
    revit = ifcopenshell.open(argv[2])
    boxes, _ = boxes_module.read_boxes(ours)
    by_host = collections.defaultdict(list)
    for box in boxes.values():
        if box["host"] is not None:
            by_host[box["host"]].append(box)
    revit_members = {str(m.Tag): m for m in revit.by_type("IfcMember") if m.Tag}
    settings = ifcopenshell.geom.settings()
    settings.set(settings.USE_WORLD_COORDS, True)

    ends_by_band = collections.Counter()
    mullions_by_band = collections.Counter()
    bent_mullions = []
    for group in by_host.values():
        for m in group:
            if m["axis"] is None:
                continue
            per_end = []
            for point, out in boxes_module.end_points(m):
                angles = partner_angles(m, point, out, group)
                per_end.append(max(angles) if angles else None)
            for angle in per_end:
                if angle is not None:
                    ends_by_band[band(angle)] += 1
            largest = max((a for a in per_end if a is not None), default=None)
            if largest is not None:
                mullions_by_band[band(largest)] += 1
            if any(a is not None and a >= BENT_DEGREES for a in per_end):
                bent_mullions.append((m["tag"], per_end))

    print("ends with a partner, by the angle between the axes (degrees):")
    for _, name in BANDS + ((None, "2.56 and over"),):
        print(f"  {name}: ends {ends_by_band[name]}, mullions {mullions_by_band[name]}")
    print(f"mullions with a partner bent by {BENT_DEGREES} degrees or more: {len(bent_mullions)}")

    hidden = collections.Counter()
    for tag, per_end in bent_mullions:
        member = revit_members.get(tag)
        quantities = ifcopenshell.util.element.get_psets(member, qtos_only=True).get("Qto_MemberBaseQuantities", {}) if member else {}
        value, volume, length = (quantities.get(k) for k in ("NetSurfaceArea", "NetVolume", "Length"))
        bent = sum(1 for a in per_end if a is not None and a >= BENT_DEGREES)
        coinciding = sum(1 for a in per_end if a is not None and a < BENT_DEGREES)
        if member is None or not (value and volume and length):
            hidden[(bent, coinciding, "Revit writes no NetSurfaceArea")] += 1
            continue
        end_area = volume / length
        ends = (surface_area(settings, member) - value) / end_area
        hidden[(bent, coinciding, f"Revit leaves out {round(ends)} end faces" if abs(ends - round(ends)) < 0.05 else "not a whole number of end faces")] += 1
    print("Revit's value, by the partners the mullion has:")
    for (bent, coinciding, text), n in sorted(hidden.items()):
        print(f"  {bent} bent ends, {coinciding} coinciding: {text}: {n}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
