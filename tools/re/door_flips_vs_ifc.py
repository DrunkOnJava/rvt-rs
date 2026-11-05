#!/usr/bin/env python3
"""Pair the doors and windows `probe_re343_door_flips` prints with Revit's own
IFC export, and tabulate their hand and facing flips (RE-343, #1357).

Research tool for `reports/element-framing/RE-343-door-window-flips.md`. For
every door and window of the probe whose ElementId is the Tag of an IfcDoor or
IfcWindow of the export, it takes:

- the flags the instance's native record names (every scalar root field of
  `FamilyInstance` whose name speaks of a flip, a mirror, a hand, a facing or
  an orientation and that takes at most four values), as the instance's state;
- the stored transform's axes where the reader takes one (RE-87), and the
  determinant of the first orthonormal 3 x 3 of either sign in its element
  data (a reflection has -1);
- Revit's placement axes, moved into Revit's internal frame through the
  export's site placement, and how each compares with the stored axis: the
  same, the reverse, a right angle from it, or none;
- the host wall's side: the sign of each placement axis against the wall's
  direction and against its exterior normal, the wall being the one rvt-rs's
  native record (RE-222) gives for the wall Revit's export voids;
- the door's `OperationType` (the instance's, or its type's IfcDoorType or
  IfcDoorStyle) and the type object the export gives it. revit-ifc's
  `DoorWindowInfo` reverses a type's operation when `HandFlipped` differs from
  `FacingFlipped`, and splits a door type by the symbol's flip against its
  wall.

It prints count lines beginning `door flips:` (counts only, never an id or a
name, in a restricted alphabet, so a held-out model's lines can leave the
runner), then, unless the model is held out, `row` lines (one JSON object per
door or window) and `sample` lines for offline reading.

Usage:

    python3 tools/re/door_flips_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins) and NumPy.
"""

import collections
import itertools
import json
import os
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.placement
import numpy as np

FLAG_WORDS = ("flip", "mirror", "hand", "facing", "orient")
FLAG_VALUES = 4
ANGLE = 1e-4
SIDE = 1e-3
FEATURES = (
    "x_rel",
    "y_rel",
    "z_rel",
    "ifc_x_wall",
    "ifc_y_ext",
    "rvt_x_wall",
    "rvt_y_ext",
    "ifc_wall_x_rel",
    "n_det",
    "n_x_col",
    "n_y_col",
    "n_x_row",
    "n_y_row",
    "op",
    "wall_flipped",
)
HELD_OUT = ("holdout", "bimcollab", "racadv")


def read_probe(path):
    model = {}
    fields = {}
    items = {}
    walls = {}
    with open(path) as handle:
        for line in handle:
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                row = json.loads(line)
            except ValueError:
                continue
            kind = row.get("kind")
            if kind == "model":
                model = row
            elif kind == "fields":
                fields[row["class"]] = row["names"]
            elif kind in ("door", "window"):
                items[str(row["id"])] = row
            elif kind == "wall":
                walls[str(row["id"])] = row
    return model, fields, items, walls


def shown(counter):
    return ", ".join(f"{k}={v}" for k, v in sorted(counter.items(), key=lambda kv: str(kv[0]))) or "none"


def unit(v):
    v = np.asarray(v, dtype=float)
    length = np.linalg.norm(v)
    return v / length if length > 0 else v


def relation(a, b):
    if a is None or b is None:
        return "none"
    dot = float(np.dot(unit(a), unit(b)))
    if dot > 1 - ANGLE:
        return "same"
    if dot < -1 + ANGLE:
        return "reverse"
    if abs(dot) < ANGLE:
        return "right angle"
    return "other"


def side(v, axis):
    """+, - or 0: which way `v` points along `axis` (plan, unit)."""
    if v is None or axis is None:
        return "none"
    dot = float(np.dot(np.asarray(v, dtype=float)[:2], axis))
    return "+" if dot > SIDE else "-" if dot < -SIDE else "0"


def site_rotation(f):
    sites = f.by_type("IfcSite")
    if not sites or not sites[0].ObjectPlacement:
        return np.eye(3)
    return np.array(ifcopenshell.util.placement.get_local_placement(sites[0].ObjectPlacement))[:3, :3]


def placement_axes(element, to_internal):
    matrix = np.array(ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement))
    rotation = to_internal @ matrix[:3, :3]
    return [rotation[:, 0], rotation[:, 1], rotation[:, 2]], float(np.linalg.det(matrix[:3, :3]))


def host_wall(element):
    for rel in getattr(element, "FillsVoids", None) or ():
        opening = rel.RelatingOpeningElement
        for void in getattr(opening, "VoidsElements", None) or ():
            return void.RelatingBuildingElement
    return None


def operation(element, kind):
    """The OperationType the element or its type carries, and the type."""
    element_type = ifcopenshell.util.element.get_type(element)
    op = None
    if kind == "door":
        for source in (element, element_type):
            value = getattr(source, "OperationType", None) if source is not None else None
            if value and value != "NOTDEFINED":
                op = str(value)
                break
        if op is None and element_type is not None:
            op = str(getattr(element_type, "OperationType", None) or "NOTDEFINED")
    return op, element_type


def op_side(op):
    if not op:
        return "-"
    return "L" if op.endswith("LEFT") else "R" if op.endswith("RIGHT") else "-"


def flag_fields(items):
    """Scalar root fields of the native records whose names speak of a flip,
    a mirror, a hand, a facing or an orientation and that take few values."""
    seen = collections.defaultdict(set)
    for row in items.values():
        for record in row.get("records", ())[:1]:
            for name, value in record.get("root", {}).items():
                lower = name.lower()
                if any(word in lower for word in FLAG_WORDS) and isinstance(value, (bool, int, float)):
                    seen[name].add(value)
    return sorted(name for name, values in seen.items() if 0 < len(values) <= FLAG_VALUES)


def native_transform(item):
    """The first InstanceInfo's m_Trf of the record: its 3 x 3 as rows and its
    origin, or None."""
    for info in item.get("info", ()):
        trf = info.get("m_Trf")
        if not isinstance(trf, dict):
            continue
        rows = trf.get("m_3x3")
        if isinstance(rows, list) and len(rows) == 3 and all(isinstance(r, list) and len(r) == 3 for r in rows):
            return np.array(rows, dtype=float), trf.get("m_or")
    return None, None


def paired_rows(items, by_tag, walls, names, to_internal):
    rows = []
    unmatched = 0
    for tag, item in sorted(items.items(), key=lambda kv: int(kv[0])):
        element = by_tag.get(tag)
        if element is None:
            unmatched += 1
            continue
        kind = item["kind"]
        (ix, iy, iz), ifc_det = placement_axes(element, to_internal)
        read = item.get("read")
        rx, ry, rz = (np.array(a) for a in read["axes"]) if read else (None, None, None)
        matrix, _ = native_transform(item)
        n_det = round(float(np.linalg.det(matrix)), 3) if matrix is not None else None
        copies = item.get("copies", [])
        dets = sorted({"+" if (c.get("first") or {}).get("det", 0) > 0 else "-" for c in copies if c.get("first")})
        records = item.get("records", [])
        root = records[0].get("root", {}) if records else {}

        wall_item = walls.get(str(item.get("host")))
        direction = exterior = None
        wall_flipped = None
        if wall_item and wall_item.get("flipped") is not None:
            start, end = np.array(wall_item["start"]), np.array(wall_item["end"])
            direction = unit(end - start)
            left = np.array([-direction[1], direction[0]])
            wall_flipped = bool(wall_item["flipped"])
            exterior = -left if wall_flipped else left
        ifc_wall = host_wall(element)
        wall_x = None
        if ifc_wall is not None and ifc_wall.ObjectPlacement:
            (wall_x, _, _), _ = placement_axes(ifc_wall, to_internal)

        op, element_type = operation(element, kind)
        rows.append(
            {
                "id": int(tag),
                "kind": kind,
                "class": element.is_a(),
                "host": item.get("host"),
                "symbol": str(getattr(element_type, "Tag", None)) if element_type is not None else None,
                "ftype": element_type.id() if element_type is not None else None,
                "op": op,
                "op_side": op_side(op),
                "state": {name: root.get(name) for name in names},
                "dets": "".join(dets) or "none",
                "copies": len(copies),
                "read": bool(read),
                "ifc_det": round(ifc_det, 3),
                "x_rel": relation(ix, rx),
                "y_rel": relation(iy, ry),
                "z_rel": relation(iz, rz),
                "ifc_x_wall": side(ix, direction),
                "ifc_y_ext": side(iy, exterior),
                "rvt_x_wall": side(rx, direction),
                "rvt_y_ext": side(ry, exterior),
                "n_det": n_det,
                "n_x_col": relation(ix, matrix[:, 0]) if matrix is not None else "none",
                "n_y_col": relation(iy, matrix[:, 1]) if matrix is not None else "none",
                "n_x_row": relation(ix, matrix[0, :]) if matrix is not None else "none",
                "n_y_row": relation(iy, matrix[1, :]) if matrix is not None else "none",
                "ifc_wall_x_rel": relation(wall_x, np.append(direction, 0.0)) if direction is not None and wall_x is not None else "none",
                "wall_flipped": wall_flipped,
                "ifc_x": [round(float(v), 6) for v in ix],
                "ifc_y": [round(float(v), 6) for v in iy],
                "rvt_x": [round(float(v), 6) for v in rx] if rx is not None else None,
                "rvt_y": [round(float(v), 6) for v in ry] if ry is not None else None,
                "wall_dir": [round(float(v), 6) for v in direction] if direction is not None else None,
            }
        )
    return rows, unmatched


def parity_lines(rows, names):
    """For the doors of one symbol, how a pair's relative state compares:
    whether a predicate differs between two doors exactly when another does."""
    binary = [n for n in names if all(r["state"].get(n) in (0, 1, None) for r in rows)]

    def predicates(r):
        p = {name: (None if r["state"].get(name) is None else str(r["state"][name])) for name in names}
        for name in ("rvt_y_ext", "rvt_x_wall", "ifc_y_ext", "ifc_x_wall"):
            p[name] = None if r[name] in ("0", "none") else r[name]
        p["op_side"] = None if r["op_side"] == "-" else r["op_side"]
        p["ftype"] = None if r["ftype"] is None else str(r["ftype"])
        return p

    groups = collections.defaultdict(list)
    for r in rows:
        if r["kind"] == "door" and r["symbol"] is not None:
            groups[r["symbol"]].append(r)
    agree = collections.Counter()
    total = collections.Counter()
    for mine in groups.values():
        if len(mine) < 2:
            continue
        mine.sort(key=lambda r: r["id"])
        ref = mine[0]
        ref_p = predicates(ref)
        for r in mine[1:]:
            p = predicates(r)
            changes = {name: p[name] != ref_p[name] for name in p if p[name] is not None and ref_p[name] is not None}
            for a, b in itertools.combinations(binary, 2):
                values = [r["state"].get(a), r["state"].get(b), ref["state"].get(a), ref["state"].get(b)]
                if None not in values:
                    changes[f"{a} xor {b}"] = (values[0] ^ values[1]) != (values[2] ^ values[3])
            for (a, ca), (b, cb) in itertools.combinations(sorted(changes.items()), 2):
                total[(a, b)] += 1
                agree[(a, b)] += ca == cb
    return [f"door flips: pair parity {a} against {b}: pairs={n} agree={agree[(a, b)]}" for (a, b), n in sorted(total.items())]


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        raise SystemExit(__doc__)
    path = os.path.abspath(args[0])
    held_out = any(word in path for word in HELD_OUT)
    model, fields, items, walls = read_probe(args[0])
    f = ifcopenshell.open(args[1])
    to_internal = site_rotation(f).T

    by_tag = {}
    for cls in ("IfcDoor", "IfcWindow"):
        for element in f.by_type(cls):
            if element.Tag is not None:
                by_tag.setdefault(str(element.Tag), element)

    names = flag_fields(items)
    print(
        f"door flips: release {model.get('release', 'unknown')} probe doors {model.get('doors', 0)} "
        f"windows {model.get('windows', 0)} walls {model.get('walls', 0)}; the export has {len(by_tag)} doors and windows"
    )
    print(f"door flips: native record classes with root fields printed: {len(fields)}; flag fields read: {len(names)}")
    for name in names:
        counts = collections.Counter(
            json.dumps(row["records"][0]["root"][name])
            for row in items.values()
            if row.get("records") and name in row["records"][0].get("root", {})
        )
        print(f"door flips: field {name} values {shown(counts)}")

    rows, unmatched = paired_rows(items, by_tag, walls, names, to_internal)
    print(f"door flips: paired {len(rows)} of {len(items)}; not in the export {unmatched}")
    for kind in ("door", "window"):
        mine = [r for r in rows if r["kind"] == kind]
        print(
            f"door flips: {kind}s paired {len(mine)}; transform read {sum(r['read'] for r in mine)}; "
            f"determinants of the first orthonormal block in their element data: {shown(collections.Counter(r['dets'] for r in mine))}; "
            f"export placement determinants: {shown(collections.Counter(r['ifc_det'] for r in mine))}"
        )

    states = collections.defaultdict(list)
    for r in rows:
        key = " ".join(f"{k}={json.dumps(v)}" for k, v in r["state"].items()) or "no flags"
        states[(r["kind"], key)].append(r)
    for (kind, key), mine in sorted(states.items()):
        print(f"door flips: state {kind} {key}: n={len(mine)}")
        for feature in FEATURES:
            counts = collections.Counter(str(r[feature]) for r in mine)
            print(f"door flips:   state {kind} {key} feature {feature}: {shown(counts)}")

    for line in parity_lines(rows, names):
        print(line)

    if not held_out:
        for r in rows:
            print("row " + json.dumps(r, sort_keys=True))
        count = 0
        for tag, item in sorted(items.items(), key=lambda kv: int(kv[0])):
            if item.get("records") and count < 3:
                print("sample " + json.dumps({"id": tag, "records": item["records"][:1], "info": item.get("info", [])[:2], "objects": item.get("objects", [])[:8]}))
                count += 1


if __name__ == "__main__":
    main(sys.argv)
