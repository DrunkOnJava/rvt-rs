#!/usr/bin/env python3
"""Score door bodies against Revit's IFC4, and the wall-centre mirror rule (RE-121).

Research tool for `reports/element-framing/RE-121-door-body-boxes.md`. Both
files' doors are moved into Revit's internal feet through each export's own
site placement (`opening_boxes_vs_ifc.site_frame`) and keyed by Tag. The
host wall is the one rvt-rs's file voids with the door's opening, and the
axis across the wall is that wall box's thinner plan axis.

For each door in both files it reports:

- whether rvt-rs's box is already Revit's body box;
- whether the "mirror" rule gives it: across the wall, keep the side of
  rvt-rs's box nearer the wall's centreline and reflect it about the
  centreline onto the far side;
- where Revit's body sits across the wall: centred on the wall's
  centreline, or off it.

Usage:

    python3 tools/re/door_bodies_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--class IfcWindow] [--tol FEET] [--list N]

`--class` scores another filler class (default `IfcDoor`).

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins) and NumPy.
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from opening_boxes_vs_ifc import site_frame  # noqa: E402

SETTINGS = ifcopenshell.geom.settings()
SETTINGS.set("use-world-coords", True)
# A host wall's box is taken before its openings are cut out of it: only the
# wall's extent across and along is read, and cutting a wall's openings is
# most of the time meshing it takes.
WALL_SETTINGS = ifcopenshell.geom.settings()
WALL_SETTINGS.set("use-world-coords", True)
WALL_SETTINGS.set("disable-opening-subtractions", True)


def boxes(f, frame, elements, settings=SETTINGS):
    """Element id -> its box, for those of `elements` that mesh. IfcOpenShell's
    iterator meshes a representation several elements map (a door type's) once,
    where `create_shape` meshes it again for each; the boxes are the same."""
    out = {}
    if not elements:
        return out
    iterator = ifcopenshell.geom.iterator(settings, f, num_threads=1, include=elements)
    if not iterator.initialize():
        return out
    while True:
        shape = iterator.get()
        verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        if len(verts):
            v = frame(verts)
            out[shape.id] = np.concatenate([v.min(axis=0), v.max(axis=0)])
        if not iterator.next():
            return out


def indexed_points(face_set, cache):
    """The points of an IfcPolygonalFaceSet its faces index, kept in `cache`
    (one per file) so a face set several elements map is read once."""
    if face_set.id() not in cache:
        index = []
        for face in face_set.Faces:
            index.extend(face.CoordIndex)
            for inner in getattr(face, "InnerCoordIndices", None) or ():
                index.extend(inner)
        if face_set.PnIndex:
            index = [face_set.PnIndex[i - 1] for i in index]
        points = np.array(face_set.Coordinates.CoordList, dtype=float)
        cache[face_set.id()] = points[np.unique(np.array(index, dtype=int)) - 1] if index else points[:0]
    return cache[face_set.id()]


def face_set_box(element, scale, frame, cache):
    """The box of an element whose one representation is a Body of
    IfcMappedItems of IfcPolygonalFaceSets, read from the points its faces
    index: those are the points IfcOpenShell's mesh of it has, and building
    each face as a B-rep face to mesh it (934,846 faces on the BIM4LCA office's
    doors in Revit's export) is nearly all the time meshing them takes. None
    for any other element, which is meshed."""
    representations = element.Representation.Representations if element.Representation else ()
    if len(representations) != 1:
        return None
    body = representations[0]
    if body.RepresentationIdentifier != "Body" or body.ContextOfItems.ContextType != "Model":
        return None
    placement = ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement)
    points = []
    for item in body.Items:
        if not item.is_a("IfcMappedItem"):
            return None
        mapping = ifcopenshell.util.placement.get_mappeditem_transformation(item)
        if mapping is None:
            return None
        for face_set in item.MappingSource.MappedRepresentation.Items:
            if not face_set.is_a("IfcPolygonalFaceSet"):
                return None
            local = indexed_points(face_set, cache)
            if not len(local):
                continue
            world = (placement @ mapping @ np.c_[local, np.ones(len(local))].T).T[:, :3]
            points.append(world * scale)
    if not points:
        return None
    v = frame(np.vstack(points))
    return np.concatenate([v.min(axis=0), v.max(axis=0)])


def doors(path, with_hosts, cls):
    """Filler Tag -> (door box, host wall box or None), in internal feet."""
    f = ifcopenshell.open(path)
    frame = site_frame(f)
    host = {}
    if with_hosts:
        voids = {r.RelatedOpeningElement.id(): r.RelatingBuildingElement for r in f.by_type("IfcRelVoidsElement")}
        for rel in f.by_type("IfcRelFillsElement"):
            wall = voids.get(rel.RelatingOpeningElement.id())
            if wall is not None:
                host[rel.RelatedBuildingElement.id()] = wall
    tagged = [door for door in f.by_type(cls) if door.Tag]
    # A door an opening voids is meshed, so the opening is cut out of it.
    voided = {rel.RelatingBuildingElement.id() for rel in f.by_type("IfcRelVoidsElement")}
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    door_boxes, cache = {}, {}
    for door in tagged:
        b = None if door.id() in voided else face_set_box(door, scale, frame, cache)
        if b is not None:
            door_boxes[door.id()] = b
    door_boxes.update(boxes(f, frame, [door for door in tagged if door.id() not in door_boxes]))
    # Each host wall is meshed once, however many doors it hosts.
    walls = {host[door.id()].id(): host[door.id()] for door in tagged if door.id() in host}
    wall_boxes = boxes(f, frame, list(walls.values()), WALL_SETTINGS)
    out = {}
    for door in tagged:
        b = door_boxes.get(door.id())
        if b is None:
            continue
        wall = host.get(door.id())
        out[door.Tag] = (b, wall_boxes.get(wall.id()) if wall is not None else None)
    return out


def main(argv):
    args = argv[1:]
    tol, listing, cls = 0.01, 0, "IfcDoor"
    for flag in ("--tol", "--list", "--class"):
        if flag in args:
            i = args.index(flag)
            value = args[i + 1]
            del args[i : i + 2]
            if flag == "--tol":
                tol = float(value)
            elif flag == "--class":
                cls = value
            else:
                listing = int(value)
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours, ref = doors(args[0], True, cls), doors(args[1], False, cls)
    counts, placement, rows = collections.Counter(), collections.Counter(), []
    for tag, (b, wall) in sorted(ours.items()):
        if tag not in ref:
            counts["not in Revit's export"] += 1
            continue
        r = ref[tag][0]
        if wall is None:
            counts["no host wall in rvt-rs's file"] += 1
            continue
        wx, wy = wall[3] - wall[0], wall[4] - wall[1]
        if abs(wx - wy) < 1e-6:
            counts["square host wall box"] += 1
            continue
        ax = 0 if wx < wy else 1
        centre = (wall[ax] + wall[ax + 3]) / 2
        offset = (r[ax] + r[ax + 3]) / 2 - centre
        placement["Revit's body centred on the wall" if abs(offset) < tol else "Revit's body off the wall's centre"] += 1
        if np.all(np.abs(b - r) < tol):
            counts["rvt-rs's box is Revit's"] += 1
            continue
        mirrored = b.copy()
        if centre - b[ax] < b[ax + 3] - centre:
            mirrored[ax + 3] = 2 * centre - b[ax]
        else:
            mirrored[ax] = 2 * centre - b[ax + 3]
        if np.all(np.abs(mirrored - r) < tol):
            counts["mirror rule gives Revit's"] += 1
        else:
            counts["mirror rule wrong"] += 1
            rows.append((tag, round(offset, 3), np.round(b - r, 3), np.round(mirrored - r, 3)))
    print(f"{len(ours)} {cls} in rvt-rs's file; tolerance {tol} ft")
    for key, value in sorted(counts.items()):
        print(f"  {value:5d} {key}")
    for key, value in sorted(placement.items()):
        print(f"  {value:5d} {key}")
    for row in rows[:listing]:
        print("  Tag %s, Revit centre off the wall's by %s ft; box - Revit %s; mirrored - Revit %s" % row)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
