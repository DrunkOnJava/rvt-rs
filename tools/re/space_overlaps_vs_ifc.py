#!/usr/bin/env python3
"""RE-316: does a room's outline cover the plan of the walls and rooms around it?

A room's outline lies inside the walls that bound it and beside the outlines of
the rooms next to it. An outline that covers a wall's plan or another room's
is not the room's. For each room rvt-rs writes with an outline read from its
stored solid (`SpaceOutlineSource` `stored_solid`, `stored_solid_faces`,
`stored_solid_approximate` or `stored_solid_edges`), this scorer measures

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
ROOM_SOURCES = (
    "stored_solid",
    "stored_solid_faces",
    "stored_solid_approximate",
    "stored_solid_edges",
)
SHRINK_FEET = 0.02
LEVEL_FEET = 1.0


def bodies(path, classes, full=False):
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
            floor = every = Polygon()
            if full:
                down = (length > 1e-12) & (normal[:, 2] < -0.9 * length)
                floor = shapely.union_all(
                    [
                        Polygon([tuple(p[:2]) for p in tri]).buffer(1e-7)
                        for tri in zip(a[down], b[down], c[down])
                    ]
                    or [Polygon()]
                )
                solid = length > 1e-12
                every = shapely.union_all(
                    [
                        Polygon([tuple(p[:2]) for p in tri]).buffer(1e-7)
                        for tri in zip(a[solid], b[solid], c[solid])
                        if abs(Polygon([tuple(p[:2]) for p in tri]).area) > 1e-9
                    ]
                    or [Polygon()]
                )
            out.append(
                {
                    "id": element.GlobalId,
                    "surface": surface,
                    "floor": floor,
                    "every": every,
                    "z0": float(v[:, 2].min()),
                    "z1": float(v[:, 2].max()),
                    "props": ifcopenshell.util.element.get_psets(element).get(SET, {})
                    if cls == "IfcSpace"
                    else {},
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
    spaces = bodies(ours_path, ["IfcSpace"], full=True)
    walls = bodies(ours_path, ["IfcWall", "IfcWallStandardCase"])
    revit = {r["id"]: r for r in bodies(revit_path, ["IfcSpace"], full=True)}
    rooms = []
    for space in spaces:
        source = space["props"].get("SpaceOutlineSource")
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
                "up": area,
                "floor": float(room["floor"].area),
                "every": float(room["every"].area),
                "revit": float(theirs["surface"].area) if theirs else None,
                "revit_floor": float(theirs["floor"].area) if theirs else None,
                "revit_every": float(theirs["every"].area) if theirs else None,
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
    paired = [r for r in rows if r["revit"] is not None and r["up"] > 0]
    for name, ours_key, revit_key in (
        ("upward faces", "up", "revit"),
        ("downward faces", "floor", "revit_floor"),
        ("every face", "every", "revit_every"),
    ):
        for source in ROOM_SOURCES:
            mine = [r for r in paired if r["source"] == source]
            if not mine:
                continue
            equal = sum(
                abs(r[ours_key] - r[revit_key]) <= 0.001 * max(r[revit_key], 1e-9) for r in mine
            )
            print(
                f"space overlaps: {name}, {source.replace('_', ' ')}: "
                f"{len(mine)} paired rooms, area within 0.1% of Revit's on {equal}"
            )
    listed = [r for r in rows if r["source"] != "stored_solid"]
    listed += sorted(
        (r for r in rows if r["source"] == "stored_solid"),
        key=lambda r: -(r["walls"] + r["rooms"]) / r["area"],
    )[:8]
    for r in listed:
        revit_text = (
            "no Revit space"
            if r["revit"] is None
            else f"Revit up {r['revit']:.2f}, floor {r['revit_floor']:.2f}, every face {r['revit_every']:.2f} ft2"
        )
        print(
            f"  {r['id']}: {r['source']}, up {r['up']:.2f}, floor {r['floor']:.2f}, "
            f"every face {r['every']:.2f} ft2 ({revit_text}), "
            f"inside walls {r['walls']:.2f} ft2 ({100 * r['walls'] / r['area']:.2f}%), "
            f"inside other rooms {r['rooms']:.2f} ft2 ({100 * r['rooms'] / r['area']:.2f}%)"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
