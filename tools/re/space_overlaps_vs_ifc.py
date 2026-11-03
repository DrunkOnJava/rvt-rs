#!/usr/bin/env python3
"""RE-316: does a room's outline cover the plan of the walls and rooms around it?

A room's outline lies inside the walls that bound it and beside the outlines of
the rooms next to it. An outline that covers a wall's plan or another room's
is not the room's. For each room rvt-rs writes with an outline read from its
stored solid (`SpaceOutlineSource` `stored_solid`, `stored_solid_faces` or
`stored_solid_approximate`), this scorer measures

- the area of the room's top surface that lies inside the top surfaces of the
  walls on the room's level (walls that span at least a foot of its height),
  each shrunk by 0.02 ft so that a room beside a wall is not inside it, and
- the area that lies inside the outlines of the other such rooms on the level,
  shrunk the same way,

against Revit's area for the same GlobalId, so a resolved outline that
disagrees with Revit's can be told from the rest by what it covers.

The lines starting `space overlaps:` are per room, with a GlobalId, and then
counts by source.

Usage:

    python3 tools/re/space_overlaps_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>
"""

import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import numpy as np
import shapely
from shapely.geometry import Polygon

import plan_profiles_vs_ifc as plan
from ifc_common import site_frame

SET = "RvtElementRecordGeometry"
ROOM_SOURCES = ("stored_solid", "stored_solid_faces", "stored_solid_approximate")
SHRINK_FEET = 0.02
LEVEL_FEET = 1.0


def bodies(path, classes):
    """Every element of `classes` with its top surface (the union of its
    upward faces in plan) and the heights its mesh spans, in Revit's feet."""
    f = ifcopenshell.open(path)
    frame = site_frame(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = []
    for cls in classes:
        try:
            elements = f.by_type(cls)
        except RuntimeError:
            continue
        for element in elements:
            if not element.Representation:
                continue
            try:
                shape = ifcopenshell.geom.create_shape(settings, element)
            except Exception:
                continue
            verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
            faces = np.array(shape.geometry.faces, dtype=int).reshape(-1, 3)
            if not len(verts) or not len(faces):
                continue
            v = frame(verts)
            a, b, c = v[faces[:, 0]], v[faces[:, 1]], v[faces[:, 2]]
            normal = np.cross(b - a, c - a)
            length = np.linalg.norm(normal, axis=1)
            up = (length > 1e-12) & (normal[:, 2] > 0.9 * length)
            triangles = [
                Polygon([tuple(p[:2]) for p in tri]).buffer(1e-7)
                for tri in zip(a[up], b[up], c[up])
            ]
            surface = shapely.union_all(triangles) if triangles else Polygon()
            out.append(
                {
                    "id": element.GlobalId,
                    "surface": surface,
                    "z0": float(v[:, 2].min()),
                    "z1": float(v[:, 2].max()),
                    "element": element,
                }
            )
    return out


def shrunk_union(surfaces):
    shrunk = [s.buffer(-SHRINK_FEET) for s in surfaces]
    shrunk = [s for s in shrunk if not s.is_empty]
    return shapely.union_all(shrunk) if shrunk else Polygon()


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours_path, revit_path = argv[1], argv[2]
    spaces = bodies(ours_path, ["IfcSpace"])
    walls = bodies(ours_path, ["IfcWall", "IfcWallStandardCase"])
    revit = plan.tops(revit_path, ["IfcSpace"])
    rooms = []
    for space in spaces:
        props = ifcopenshell.util.element.get_psets(space["element"]).get(SET, {})
        source = props.get("SpaceOutlineSource")
        if source in ROOM_SOURCES and space["surface"].area > 0:
            space["source"] = source
            rooms.append(space)

    def on_level(a, b):
        return a["z0"] < b["z1"] - LEVEL_FEET and a["z1"] > b["z0"] + LEVEL_FEET

    rows = []
    for room in rooms:
        surface = room["surface"]
        near_walls = [
            w["surface"]
            for w in walls
            if on_level(w, room) and w["surface"].intersects(surface)
        ]
        near_rooms = [
            o["surface"]
            for o in rooms
            if o is not room and on_level(o, room) and o["surface"].intersects(surface)
        ]
        area = float(surface.area)
        wall_over = float(surface.intersection(shrunk_union(near_walls)).area)
        room_over = float(surface.intersection(shrunk_union(near_rooms)).area)
        theirs = revit.get(room["id"])
        rows.append(
            {
                "id": room["id"],
                "source": room["source"],
                "area": area,
                "walls": wall_over,
                "rooms": room_over,
                "revit": float(theirs[0]) if theirs else None,
            }
        )

    for source in ROOM_SOURCES:
        mine = [r for r in rows if r["source"] == source]
        if not mine:
            continue
        wall_share = [r["walls"] / r["area"] for r in mine]
        room_share = [r["rooms"] / r["area"] for r in mine]
        print(
            f"space overlaps: {source.replace('_', ' ')}: {len(mine)} rooms; "
            f"walls: above 0.1% of the area on {sum(s > 0.001 for s in wall_share)}, "
            f"above 1% on {sum(s > 0.01 for s in wall_share)}, "
            f"above 5% on {sum(s > 0.05 for s in wall_share)}, "
            f"largest {100 * max(wall_share):.2f}%; "
            f"other rooms: above 0.1% on {sum(s > 0.001 for s in room_share)}, "
            f"above 1% on {sum(s > 0.01 for s in room_share)}, "
            f"above 5% on {sum(s > 0.05 for s in room_share)}, "
            f"largest {100 * max(room_share):.2f}%"
        )
    listed = [r for r in rows if r["source"] != "stored_solid"]
    listed += sorted(
        (r for r in rows if r["source"] == "stored_solid"),
        key=lambda r: -(r["walls"] + r["rooms"]) / r["area"],
    )[:8]
    for r in listed:
        revit_text = "no Revit space" if r["revit"] is None else f"Revit {r['revit']:.2f} ft2"
        print(
            f"  {r['id']}: {r['source']}, area {r['area']:.2f} ft2 ({revit_text}), "
            f"inside walls {r['walls']:.2f} ft2 ({100 * r['walls'] / r['area']:.2f}%), "
            f"inside other rooms {r['rooms']:.2f} ft2 ({100 * r['rooms'] / r['area']:.2f}%)"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
