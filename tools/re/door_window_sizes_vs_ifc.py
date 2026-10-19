#!/usr/bin/env python3
"""Score each candidate size of a door or window type against Revit's IFC.

Research tool for RE-187 (#759). `examples/probe_re187_door_window_sizes.rs`
prints, per door and window, the sizes its type holds: the BuiltInParameters
Width and Height (-1001301, -1001300), Rough Width and Rough Height
(-1001305, -1001304), and family parameters named Width and Height. This
scorer joins those rows to Revit's own export by `Tag` (the ElementId) and
counts, per class, how many of Revit's `OverallWidth` and `OverallHeight`
each candidate pair equals, within 0.001 ft. The probe also prints the
element's record box along its own axes (RE-192, #810), which gives two more
candidates: the box along X and Z, and along X and Y. Each candidate's width
and height are also counted apart, and the types whose instances Revit gives
more than one size are counted, since Revit's exporter takes both from the
type's symbol. A last candidate comes from Revit's own export, not from
rvt-rs: the opening each door or window fills (`IfcRelFillsElement`), the
corners of its extruded body taken into the filler's own placement, along X
and Z, since revit-ifc takes both sizes from the symbol
(`GetDoorWindowDimensionFromSymbol`, CLEANROOM.md source 5).

Given an rvt-rs export instead of the probe's output, it scores the
`OverallWidth` and `OverallHeight` rvt-rs writes against Revit's, joined the
same way: per class, how many are equal, differ, or are `$`.

Usage:

    python3 tools/re/door_window_sizes_vs_ifc.py <probe output> <revit-export.ifc> [--list]
    python3 tools/re/door_window_sizes_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--list]

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

FEET = 0.3048
TOLERANCE_FEET = 1e-3
CANDIDATES = [
    ("width/height (-1001301/-1001300)", "width", "height"),
    ("rough width/height (-1001305/-1001304)", "rough_width", "rough_height"),
    ("family Width/Height", "family_width", "family_height"),
    ("record box along its X and Z", "box_x", "box_z"),
    ("record box along its X and Y", "box_x", "box_y"),
    ("Revit's own opening, along its X and Z", "cut_width", "cut_height"),
]


def profile_points(profile):
    """A profile's outline points in its own plane, or None for a kind not read."""
    if profile.is_a("IfcRectangleProfileDef"):
        x, y = profile.XDim / 2, profile.YDim / 2
        points = [(-x, -y), (x, -y), (x, y), (-x, y)]
    elif profile.is_a("IfcArbitraryClosedProfileDef") and profile.OuterCurve.is_a("IfcPolyline"):
        points = [tuple(point.Coordinates[:2]) for point in profile.OuterCurve.Points]
    elif profile.is_a("IfcArbitraryClosedProfileDef") and profile.OuterCurve.is_a("IfcIndexedPolyCurve"):
        points = [tuple(point[:2]) for point in profile.OuterCurve.Points.CoordList]
    else:
        return None
    position = getattr(profile, "Position", None)
    if position is None:
        return points
    ox, oy = position.Location.Coordinates[:2]
    rx, ry = position.RefDirection.DirectionRatios[:2] if position.RefDirection else (1.0, 0.0)
    norm = (rx * rx + ry * ry) ** 0.5
    rx, ry = rx / norm, ry / norm
    return [(ox + rx * px - ry * py, oy + ry * px + rx * py) for px, py in points]


def opening_points(opening):
    """The corners of an opening's extruded body, in model coordinates, or None."""
    if opening.Representation is None:
        return None
    corners = []
    for representation in opening.Representation.Representations:
        if representation.RepresentationIdentifier not in (None, "Body"):
            continue
        for item in representation.Items:
            if not item.is_a("IfcExtrudedAreaSolid"):
                return None
            outline = profile_points(item.SweptArea)
            if outline is None:
                return None
            direction = np.array(item.ExtrudedDirection.DirectionRatios, dtype=float)
            direction = direction / np.linalg.norm(direction) * item.Depth
            base = [np.array([x, y, 0.0]) for x, y in outline]
            solid = ifcopenshell.util.placement.get_axis2placement(item.Position) if item.Position else np.eye(4)
            corners.extend(solid @ np.append(point, 1.0) for point in base + [point + direction for point in base])
    if not corners:
        return None
    placement = ifcopenshell.util.placement.get_local_placement(opening.ObjectPlacement)
    return [(placement @ corner)[:3] for corner in corners]


def cut_sizes(f, metres):
    """Tag -> (width ft, height ft): the opening a door or window fills, along its X and Z."""
    out = {}
    for relation in f.by_type("IfcRelFillsElement"):
        element = relation.RelatedBuildingElement
        if not (element.is_a("IfcDoor") or element.is_a("IfcWindow")) or not element.Tag:
            continue
        corners = opening_points(relation.RelatingOpeningElement)
        if corners is None:
            continue
        to_element = np.linalg.inv(ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement))
        local = np.array([(to_element @ np.append(corner, 1.0))[:3] for corner in corners])
        extent = local.max(axis=0) - local.min(axis=0)
        out[str(element.Tag)] = (extent[0] * metres / FEET, extent[2] * metres / FEET)
    return out


def revit_sizes(path, cuts=None):
    """Tag -> (class, OverallWidth ft, OverallHeight ft) for every door and window.

    With a dict for `cuts`, it also fills in `cut_sizes`."""
    f = ifcopenshell.open(path)
    metres = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for entity in ("IfcDoor", "IfcWindow"):
        for element in f.by_type(entity):
            if not element.Tag:
                continue
            width, height = element.OverallWidth, element.OverallHeight
            out[str(element.Tag)] = (
                entity,
                None if width is None else width * metres / FEET,
                None if height is None else height * metres / FEET,
            )
    if cuts is not None:
        cuts.update(cut_sizes(f, metres))
    return out


def score_export(ours_path, revit_path, listing):
    """Count the OverallWidth / OverallHeight an rvt-rs export writes against Revit's."""
    ours, revit = revit_sizes(ours_path), revit_sizes(revit_path)
    tally = collections.Counter()
    seen = collections.Counter()
    worst = []
    for tag, (entity, width, height) in revit.items():
        if width is None or height is None or tag not in ours:
            continue
        seen[entity] += 1
        _, w, h = ours[tag]
        if w is None or h is None:
            tally[(entity, "$")] += 1
        elif abs(w - width) <= TOLERANCE_FEET and abs(h - height) <= TOLERANCE_FEET:
            tally[(entity, "equal")] += 1
        else:
            tally[(entity, "differs")] += 1
            worst.append((tag, entity, width, height, w, h))
    for entity in sorted(seen):
        print(f"{entity}: {seen[entity]} matched by Tag, OverallWidth and OverallHeight "
              f"equal {tally[(entity, 'equal')]}  differ {tally[(entity, 'differs')]}  $ {tally[(entity, '$')]}")
    if listing:
        for tag, entity, width, height, w, h in worst[:60]:
            print(f"  {entity} {tag}: Revit {width:.4f} x {height:.4f} ft, rvt-rs {w:.4f} x {h:.4f} ft")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    probe_path, ifc_path = args[0], args[1]
    if probe_path.lower().endswith(".ifc"):
        score_export(probe_path, ifc_path, listing)
        return
    rows = []
    for line in open(probe_path):
        line = line.strip()
        if line.startswith("{"):
            row = json.loads(line)
            if row.get("id") is not None:
                oriented = row.get("oriented") or [None, None, None]
                row["box_x"], row["box_y"], row["box_z"] = oriented
                rows.append(row)
    cuts = {}
    revit = revit_sizes(ifc_path, cuts)
    for row in rows:
        row["cut_width"], row["cut_height"] = cuts.get(str(row["id"]), (None, None))
    with_size = {tag: v for tag, v in revit.items() if v[1] is not None and v[2] is not None}
    print(f"Revit's export: {len(revit)} doors and windows, {len(with_size)} with OverallWidth and OverallHeight")
    print(f"probe: {len(rows)} doors and windows from element records")

    tally = collections.Counter()
    seen = collections.Counter()
    sizes_by_type = collections.defaultdict(set)
    worst = []
    for row in rows:
        tag = str(row["id"])
        if tag not in with_size:
            continue
        entity, width, height = with_size[tag]
        seen[entity] += 1
        sizes_by_type[(entity, row.get("type"))].add((round(width, 3), round(height, 3)))
        matched = []
        for label, wkey, hkey in CANDIDATES:
            w, h = row.get(wkey), row.get(hkey)
            if w is not None and abs(w - width) <= TOLERANCE_FEET:
                tally[(entity, label, "width equal")] += 1
            if h is not None and abs(h - height) <= TOLERANCE_FEET:
                tally[(entity, label, "height equal")] += 1
            if w is None or h is None:
                tally[(entity, label, "not read")] += 1
                continue
            if abs(w - width) <= TOLERANCE_FEET and abs(h - height) <= TOLERANCE_FEET:
                tally[(entity, label, "equal")] += 1
                matched.append(label)
            else:
                tally[(entity, label, "differs")] += 1
        if not matched:
            worst.append((tag, entity, width, height, row))

    for entity in sorted(seen):
        print(f"\n{entity}: {seen[entity]} matched by Tag")
        for label, _, _ in CANDIDATES:
            equal = tally[(entity, label, "equal")]
            differs = tally[(entity, label, "differs")]
            missing = tally[(entity, label, "not read")]
            print(f"  {label:42} equal {equal:4}  differs {differs:4}  not read {missing:4}"
                  f"  (width equal {tally[(entity, label, 'width equal')]:4},"
                  f" height equal {tally[(entity, label, 'height equal')]:4})")
        types = [sizes for (e, _), sizes in sizes_by_type.items() if e == entity]
        varying = sum(1 for sizes in types if len(sizes) > 1)
        print(f"  types: {len(types)}, of which Revit gives more than one size: {varying}")
    if listing:
        print(f"\n{len(worst)} with no candidate equal to Revit's size:")
        for tag, entity, width, height, row in worst[:60]:
            cands = ", ".join(
                f"{label.split(' (')[0]}={row.get(w)}x{row.get(h)}" for label, w, h in CANDIDATES
            )
            print(f"  {entity} {tag} type {row.get('type')}: Revit {width:.4f} x {height:.4f} ft; {cands}; box {row.get('box')};"
                  f" upright {row.get('upright')}")


main()
