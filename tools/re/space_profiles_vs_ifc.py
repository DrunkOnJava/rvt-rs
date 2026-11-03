#!/usr/bin/env python3
"""Score every IfcSpace's plan area and outline against Revit's export (RE-313).

`plan_profiles_vs_ifc.py` already meshes both files, moves them into Revit's
internal feet through each export's site placement and compares the plan
union of their upward faces, an area and an outline. It keys an IfcSpace,
which has no Tag, by its GlobalId. This runs it with `--class IfcSpace` and
adds what an importer needs to know about rooms:

- which spaces pair: by Revit's GlobalId, or, for a file whose rooms carry
  none of Revit's (Revit 2023), by the nearest top-surface centroid, one
  space to one;
- the spaces in only one file, those of Revit's that rvt-rs writes without a
  body (its Areas), and those whose top surface has no area;
- area within 0.1% and within 1%, and two outline distances, each within
  0.01 ft. `outline` is `plan_profiles_vs_ifc.py`'s: the largest distance from
  a vertex of either mesh's upward faces to the other's top-surface outline,
  so an upward face inside Revit's room (a ceiling step) counts against it.
  `boundary` is the largest distance from a point of either top-surface
  outline to the other's, which such a face does not move. `vertices` is the
  vertex-to-vertex distance. The lines print `outline`, `vertices` and
  `boundary` under those names, and `both` is area and `outline`;
- the lowest and the highest vertex of the two bodies, within 0.01 ft: the
  base and the top of the space, so its height (`lowest and highest vertex`);
- the same counts split by what the space is (a room, an Area, a family
  instance written as a space) and where rvt-rs's outline came from (its
  stored solid, the record box, or an Area's boundary lines);
- every pair that is off in area or outline, with the reason rvt-rs's own
  property set gives (`SpaceOutlineSource`, or the older `ProfileSource`
  and `ProfileResolved` of `RvtElementRecordGeometry`).

The lines starting `space profiles:` are counts only, so the Measure
workflow keeps them for the held-out models.

Usage:

    python3 tools/re/space_profiles_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>
        [--match globalid|centroid] [--list N]
        [--min-in-both N] [--min-area N] [--min-outline N] [--min-boundary N]

`--match` is chosen from the files unless given: centroid when under half of
rvt-rs's spaces carry a GlobalId of Revit's. The `--min-*` flags make the run
a gate: it exits 1 when a count falls below its minimum.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins), NumPy and Shapely.
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element
import shapely

import plan_profiles_vs_ifc as plan

SET = "RvtElementRecordGeometry"
STORED_SOLID = "stored_solid"
RECORD_BOX = "record_box"
AREA_BOUNDARY = "area_boundary"
NO_BODY = "none"
ROOM_SOLID_SOURCE = "partition_room_solid_faces"


def sources(path):
    """GlobalId -> (where rvt-rs's outline of the space came from, what the
    space is: a room, an area or a family instance)."""
    out = {}
    f = ifcopenshell.open(path)
    for space in f.by_type("IfcSpace"):
        props = ifcopenshell.util.element.get_psets(space).get(SET, {})
        marker = props.get("SpaceOutlineSource")
        if marker:
            source = marker
        elif props.get("ProfileSource") == ROOM_SOLID_SOURCE:
            source = STORED_SOLID
        elif props.get("ProfileResolved") is False:
            source = RECORD_BOX
        elif props.get("ProfileSource"):
            source = f"other: {props['ProfileSource']}"
        else:
            source = "not stated"
        # A room has its number or its name; a family instance written as a
        # space has neither.
        named = "RoomNumber" in props or "RoomName" in props
        kind = "room" if named else "family instance" if props else "space"
        # An Area is named as a room is; its marker tells it apart (RE-317).
        if marker in (AREA_BOUNDARY, NO_BODY):
            kind = "area"
        out[space.GlobalId] = (source, kind)
    return out


def shape(surface):
    """Pieces and holes of a top surface."""
    parts = list(surface.geoms) if hasattr(surface, "geoms") else [surface]
    return len(parts), sum(len(part.interiors) for part in parts)


def boundary_spread(p, q):
    """The largest distance from a point of either top surface's outline to
    the other's outline. `plan.outline_spread` measures from every vertex of
    the mesh's upward faces instead, so a ceiling step inside a room, which
    Revit meshes as an upward face of its own, counts as far from the outline."""
    if p.is_empty or q.is_empty:
        return float("inf")

    def points(surface):
        boundary = surface.boundary
        lines = list(boundary.geoms) if hasattr(boundary, "geoms") else [boundary]
        return shapely.points([c for line in lines for c in line.coords])

    return float(
        max(
            shapely.distance(q.boundary, points(p)).max(),
            shapely.distance(p.boundary, points(q)).max(),
        )
    )


def height_gap(ours, revit):
    """The larger of the distances between the lowest and the highest vertex of
    the two bodies, feet; infinite where either is unknown."""
    if ours is None or revit is None:
        return float("inf")
    return max(abs(ours[0] - revit[0]), abs(ours[1] - revit[1]))


def count_spaces(path):
    return len(ifcopenshell.open(path).by_type("IfcSpace"))


def pair_by_centroid(ours, revit):
    """One rvt-rs space to one of Revit's, nearest centroids first."""
    distances = sorted(
        (ours[a][3].centroid.distance(revit[b][3].centroid), a, b) for a in ours for b in revit
    )
    taken_ours, taken_revit, pairs = set(), set(), {}
    for _, a, b in distances:
        if a in taken_ours or b in taken_revit:
            continue
        taken_ours.add(a)
        taken_revit.add(b)
        pairs[a] = b
    return pairs


def main(argv):
    args = argv[1:]
    listing = 0
    match = None
    minimums = {}
    for flag, key in (
        ("--list", None),
        ("--match", None),
        ("--min-in-both", "both"),
        ("--min-area", "area"),
        ("--min-outline", "outline"),
        ("--min-boundary", "boundary"),
    ):
        if flag in args:
            i = args.index(flag)
            value = args[i + 1]
            del args[i : i + 2]
            if flag == "--list":
                listing = int(value)
            elif flag == "--match":
                match = value
            else:
                minimums[key] = int(value)
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours_path, revit_path = args
    ours_z, revit_z = {}, {}
    ours = plan.tops(ours_path, ["IfcSpace"], ours_z)
    revit = plan.tops(revit_path, ["IfcSpace"], revit_z)
    ours_total, revit_total = count_spaces(ours_path), count_spaces(revit_path)
    held = sources(ours_path)

    by_id = [key for key in ours if key in revit]
    if match is None:
        match = "globalid" if ours and 2 * len(by_id) >= len(ours) else "centroid"
    if match == "centroid" and ours and revit:
        pairs = pair_by_centroid(ours, revit)
    else:
        pairs = {key: key for key in by_id}

    rows = []
    for key, other in pairs.items():
        o_area, o_points, _, o_surface = ours[key]
        area, points, _, surface = revit[other]
        if area <= 0:
            continue
        rows.append(
            {
                "key": key,
                "rel": abs(o_area - area) / area,
                "area": area,
                "o_area": o_area,
                "z": height_gap(ours_z.get(key), revit_z.get(other)),
                "vertices": plan.spread(o_points, points),
                "outline": plan.outline_spread(o_points, surface, points, o_surface),
                "boundary": boundary_spread(o_surface, surface),
                "source": held.get(key, ("not stated", "space"))[0],
                "kind": held.get(key, ("not stated", "space"))[1],
                "shapes": (shape(o_surface), shape(surface)),
                "apart": float(o_surface.centroid.distance(surface.centroid)),
            }
        )

    def area_ok(r, tolerance):
        return r["rel"] <= tolerance

    def outline_ok(r):
        return r["outline"] <= 0.01

    in_both = len(rows)
    n_area = sum(area_ok(r, 1e-3) for r in rows)
    n_area_1 = sum(area_ok(r, 1e-2) for r in rows)
    n_outline = sum(outline_ok(r) for r in rows)
    n_vertices = sum(r["vertices"] <= 0.01 for r in rows)
    n_boundary = sum(r["boundary"] <= 0.01 for r in rows)
    n_both = sum(area_ok(r, 1e-3) and outline_ok(r) for r in rows)
    print(f"space profiles: matched by {match}")
    print(
        f"space profiles: IfcSpace in rvt-rs {ours_total}, in Revit {revit_total},"
        f" with a body {len(ours)} and {len(revit)}"
    )
    print(
        f"space profiles: in both {in_both}, only in rvt-rs {len(ours) - len(pairs)},"
        f" only in Revit {len(revit) - len(pairs)}"
    )
    paired_revit = set(pairs.values())
    unpaired = [key for key in revit if key not in paired_revit]
    bodiless = sum(key in held for key in unpaired) if match == "globalid" else 0
    print(
        f"space profiles: only in Revit and in rvt-rs without a body {bodiless},"
        f" paired where the Revit top surface has no area {len(pairs) - in_both}"
    )
    print(
        f"space profiles: area within 0.1% {n_area}, within 1% {n_area_1},"
        f" off {in_both - n_area_1}"
    )
    print(f"space profiles: outline within 0.01 ft {n_outline}, off {in_both - n_outline}")
    print(f"space profiles: vertices within 0.01 ft {n_vertices}, off {in_both - n_vertices}")
    print(f"space profiles: boundary within 0.01 ft {n_boundary}, off {in_both - n_boundary}")
    print(f"space profiles: area within 0.1% and outline within 0.01 ft {n_both}")
    n_height = sum(r["z"] <= 0.01 for r in rows)
    print(
        f"space profiles: lowest and highest vertex within 0.01 ft {n_height},"
        f" off {in_both - n_height}"
    )
    split = collections.defaultdict(lambda: [0, 0, 0, 0, 0])
    for r in rows:
        counts = split[f"{r['kind']} {r['source'].split(':')[0]}"]
        counts[0] += 1
        counts[1] += area_ok(r, 1e-3)
        counts[2] += outline_ok(r)
        counts[3] += area_ok(r, 1e-3) and outline_ok(r)
        counts[4] += r["boundary"] <= 0.01
    for source, (n, a, o, b, e) in sorted(split.items()):
        label = source.replace("_", " ")
        print(
            f"space profiles: rvt-rs {label}: {n},"
            f" area within 0.1% {a}, outline within 0.01 ft {o}, both {b},"
            f" boundary within 0.01 ft {e}"
        )

    height_split = collections.defaultdict(lambda: [0, 0])
    for r in rows:
        counts = height_split[f"{r['kind']} {r['source'].split(':')[0]}"]
        counts[0] += 1
        counts[1] += r["z"] <= 0.01
    for source, (n, h) in sorted(height_split.items()):
        label = source.replace("_", " ")
        print(f"space profiles: rvt-rs {label} height: {n}, lowest and highest vertex within 0.01 ft {h}")

    off = [r for r in rows if not (area_ok(r, 1e-3) and outline_ok(r))]
    shown = sorted(off, key=lambda r: (-r["rel"], -r["outline"]))
    extra = sorted(rows, key=lambda r: (-r["rel"], -r["outline"]))[:listing]
    for r in shown + [r for r in extra if r not in shown]:
        print(
            f"  {r['key']}: area {r['o_area']:.2f} vs {r['area']:.2f} ft2 ({r['rel']:.2%}),"
            f" vertices {r['vertices']:.3f} ft, outline {r['outline']:.3f} ft,"
            f" boundary {r['boundary']:.3f} ft,"
            f" {r['kind']}, rvt-rs outline from {r['source']},"
            f" pieces and holes {r['shapes'][0][0]} and {r['shapes'][0][1]} in rvt-rs,"
            f" {r['shapes'][1][0]} and {r['shapes'][1][1]} in Revit,"
            f" centroids {r['apart']:.3f} ft apart"
        )
    shortfalls = [
        f"{what} {got} below {minimums[key]}"
        for key, what, got in (
            ("both", "in both", in_both),
            ("area", "area", n_area),
            ("outline", "outline", n_outline),
            ("boundary", "boundary", n_boundary),
        )
        if key in minimums and got < minimums[key]
    ]
    for line in shortfalls:
        print(f"space profiles gate: {line}")
    return 1 if shortfalls else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
