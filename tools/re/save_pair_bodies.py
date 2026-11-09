#!/usr/bin/env python3
"""Each element's body in two rvt-rs exports of the same project saved by
two Revit releases, element for element (RE-341).

Elements are paired by `Tag` (their ElementId, which a project keeps
across saves) and meshed in world coordinates: the mesh Revit saved for an
element, which rvt-rs writes as an `IfcTriangulatedFaceSet`, is read from the
file, and every other body is meshed by IfcOpenShell. A pair is the same where
the world boxes agree to 3 mm on every side and the volumes to 0.1%. This
is a box and volume check, not shape equality: a mirrored body, or one whose
volume sits differently inside the same box, agrees. Openings are left out
(their own elements). With `--sources`, only the candidate's elements whose
`BodySource` is one of those named are scored, and each one that differs is
listed with the reference's `BodySource`. With `--no-openings`, a body is meshed
without the voids of its openings, so that a wall scores apart from the doors
and windows that cut it (RE-364).

A body that cannot be paired is counted and printed, and the exit status is 1
when there is one: a shape that did not mesh, or a `Tag` more than one product
carries (neither is scored). Products with no `Tag` cannot be paired either;
they are counted and printed but do not change the exit status.

There is no Revit 2026 or 2027 export that can be published here, so a 2026
or 2027 save is scored against the same Autodesk sample saved by Revit 2024
or 2025, whose export is measured against Revit's own.

Usage:

    python3 tools/re/save_pair_bodies.py <candidate.ifc> <reference.ifc>
    python3 tools/re/save_pair_bodies.py <candidate.ifc> <reference.ifc> \\
        --sources partition_beam_axis,native_landing_tread
"""

import argparse
import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

BOX_TOLERANCE_M = 0.003
VOLUME_TOLERANCE = 1e-3

DID_NOT_MESH = "did not mesh"
REPEATED_TAG = "share a Tag with another product"


def bodies(path, only=None, openings=True):
    """(Tag -> (class, BodySource, volume, low corner, high corner),
    Tag -> (why it was not scored, BodySource), products with no Tag).

    A body that is one saved mesh is read from the file (`face_set_mesh`). The others are
    meshed together by IfcOpenShell's iterator on every core, not by one `create_shape`
    call each, which took 52 of the 55 seconds of a Revit 2026 save of `rac_advanced`
    (5,527 bodies) when the gates mesh six saves of each of five projects (#1616). An
    element the iterator does not return did not mesh."""
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    if not openings:
        settings.set("disable-opening-subtractions", True)
    model = ifcopenshell.open(path)
    lost = {}
    seen = {}
    repeated = set()
    untagged = 0
    for element in model.by_type("IfcProduct"):
        if not element.Representation or element.is_a("IfcOpeningElement"):
            continue
        tag = getattr(element, "Tag", None)
        if not tag:
            untagged += 1
            continue
        if only is not None and tag not in only:
            continue
        source = None
        for pset in ifcopenshell.util.element.get_psets(element).values():
            source = pset.get("BodySource", source)
        if tag in seen or tag in repeated:
            repeated.add(tag)
            seen.pop(tag, None)
            lost[tag] = (REPEATED_TAG, source)
            continue
        seen[tag] = (element, source)
    by_id = {element.id(): (tag, element, source) for tag, (element, source) in seen.items()}
    unit_scale = ifcopenshell.util.unit.calculate_unit_scale(model)
    meshed = {}
    to_mesh = []
    for step_id, (_, element, _) in by_id.items():
        mesh = face_set_mesh(element, unit_scale)
        if mesh is None:
            to_mesh.append(element)
        else:
            meshed[step_id] = mesh
    if to_mesh:
        iterator = ifcopenshell.geom.iterator(settings, model, os.cpu_count() or 1, include=to_mesh)
        if iterator.initialize():
            while True:
                shape = iterator.get()
                meshed[shape.id] = (
                    np.array(shape.geometry.verts).reshape(-1, 3),
                    np.array(shape.geometry.faces).reshape(-1, 3),
                )
                if not iterator.next():
                    break
    out = {}
    for step_id, (tag, element, source) in by_id.items():
        mesh = meshed.get(step_id)
        if mesh is None or len(mesh[1]) == 0:
            lost[tag] = (DID_NOT_MESH, source)
            continue
        verts, faces = mesh
        a, b, c = verts[faces[:, 0]], verts[faces[:, 1]], verts[faces[:, 2]]
        volume = abs(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0)
        # The box is that of the corners the triangles use: a point list can hold points
        # no triangle indexes (the writer appends each saved primitive's whole vertex
        # array, 155 of the office's 11,084 face sets hold some), and meshing by
        # IfcOpenShell never sees them.
        used = verts[np.unique(faces)]
        out[tag] = (element.is_a(), source, volume, used.min(0), used.max(0))
    return out, lost, untagged


def face_set_mesh(element, unit_scale):
    """(vertices in world metres, triangles) of an element whose one body item is an
    `IfcTriangulatedFaceSet` with no opening cut from it, which is how rvt-rs writes the
    mesh Revit saved for an element; None for any other body. The triangles are the ones
    the file holds: IfcOpenShell builds an OpenCASCADE face from each and meshes them
    again, and two 163,040-triangle sanitary terminals took 40% of the time a Revit 2026
    `rac_basic` save needed (#1616)."""
    if getattr(element, "HasOpenings", None):
        return None
    representations = element.Representation.Representations
    if (
        len(representations) != 1
        or representations[0].RepresentationIdentifier != "Body"
        or len(representations[0].Items) != 1
    ):
        return None
    item = representations[0].Items[0]
    if not item.is_a("IfcTriangulatedFaceSet") or item.PnIndex is not None:
        return None
    if not item.Coordinates.is_a("IfcCartesianPointList3D"):
        return None
    points = np.array(item.Coordinates.CoordList, dtype=float).reshape(-1, 3)
    faces = np.array(item.CoordIndex, dtype=np.int64).reshape(-1, 3) - 1
    if len(faces) and (faces.min() < 0 or faces.max() >= len(points)):
        return None
    placement = ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement)
    return (points @ placement[:3, :3].T + placement[:3, 3]) * unit_scale, faces


def same(body, reference):
    box = max(
        np.abs(body[3] - reference[3]).max(),
        np.abs(body[4] - reference[4]).max(),
    )
    volume = abs(body[2] - reference[2]) <= VOLUME_TOLERANCE * max(reference[2], 1e-9)
    return box <= BOX_TOLERANCE_M and volume, box


def tag_order(tag):
    return (0, int(tag), "") if tag.isdigit() else (1, 0, tag)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("candidate")
    parser.add_argument("reference")
    parser.add_argument("--sources", help="comma-separated BodySource values to score")
    parser.add_argument(
        "--no-openings",
        action="store_true",
        help="mesh each body without the voids of its openings",
    )
    args = parser.parse_args()
    openings = not args.no_openings
    candidate, candidate_lost, candidate_untagged = bodies(
        args.candidate, openings=openings
    )
    if args.sources:
        wanted = set(args.sources.split(","))
        candidate = {tag: b for tag, b in candidate.items() if b[1] in wanted}
        candidate_lost = {
            tag: why for tag, why in candidate_lost.items() if why[1] in wanted
        }
    reference, reference_lost, reference_untagged = bodies(
        args.reference, set(candidate) | set(candidate_lost), openings
    )
    tally = collections.defaultdict(lambda: [0, 0, 0])
    listed = []
    for tag, body in sorted(candidate.items(), key=lambda item: tag_order(item[0])):
        row = tally[body[0]]
        if tag not in reference:
            row[2] += 1
            continue
        agrees, box = same(body, reference[tag])
        row[0 if agrees else 1] += 1
        if args.sources and not agrees:
            listed.append(f"  {body[0]} {tag} {body[1]} (reference {reference[tag][1]}): box off {box:.4f} m")
    total = [sum(row[i] for row in tally.values()) for i in range(3)]
    print(f"same {total[0]}, differ {total[1]}, not in the reference {total[2]}")
    for cls, (agree, differ, missing) in sorted(tally.items()):
        print(f"  {cls}: same {agree}, differ {differ}, not in the reference {missing}")
    print("\n".join(listed))
    unpaired = {
        "candidate": candidate_lost,
        "reference": {
            tag: why for tag, why in reference_lost.items() if tag in candidate
        },
    }
    for side, lost in unpaired.items():
        for reason in (DID_NOT_MESH, REPEATED_TAG):
            tags = sorted((t for t, why in lost.items() if why[0] == reason), key=tag_order)
            print(f"{side} bodies that {reason}, not scored: {len(tags)}")
            if args.sources:
                print("".join(f"  {tag} {lost[tag][1]}\n" for tag in tags), end="")
    print(
        f"products with no Tag, not paired: {candidate_untagged} in the candidate, "
        f"{reference_untagged} in the reference"
    )
    if any(unpaired.values()):
        sys.exit(1)


if __name__ == "__main__":
    main()
