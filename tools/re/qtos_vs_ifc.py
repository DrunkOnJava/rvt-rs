#!/usr/bin/env python3
"""Score the quantity sets in an rvt-rs IFC against Revit's own IFC export
(#1016, W7).

`psets_vs_ifc.py` scores property sets only. This scores every quantity of
every `Qto_` IfcElementQuantity Revit's export relates to an object, keyed
the same way: the object (an element by its `Tag`, a space or storey by its
`Name`, the building as `building`), the set's name and the quantity's
name. A value is its quantity entity and its value in SI units (metres,
square metres, cubic metres, kilograms, seconds) from each file's own unit
assignment; equal means the same entity and a value within a relative
1e-5, or within 1e-6 (a micrometre, a square or cubic one: the rvt-rs
writer's rounding) where that is larger. Revit's IFC2X3 export writes a
pipe's or duct's length in a property set (`psets_vs_ifc`'s
IFC2X3_LENGTHS), which is scored here as the IFC4 base quantity. A quantity carrying its own unit
is counted apart and not scored. Objects rvt-rs does not write at all are
left out. Where Revit's export writes no base quantities at all (no
`Qto_` IfcElementQuantity, nor IFC2X3's `BaseQuantities`), its export setup
had "Export base quantities" off (RE-242), and the quantities rvt-rs writes
are counted apart, not as values Revit does not write: `rvt-ifc
--no-base-quantities` leaves them out. A quantity Revit's export writes that
is known to be wrong (RE-235 §5, `REVIT_DEFECTS`) is left out of rvt-rs's
export and counted apart here, not as missing. So is a wall's missing
`Length` where the two lengths Revit's export may write differ: the length of
the wall's curve, which rvt-rs's export diagnostics list
(`decoded.wall_curve_lengths_feet`, not in its IFC file), and that of the
cut-back curve Revit's `Axis` draws (RE-252), on a model where Revit's
`Length` is one of those two on every wall, so no curve was read wrong. The
diagnostics are the `diagnostics.json` beside the rvt-rs IFC, as Measure
writes them, or the file `--diagnostics` names; without one, no wall is
counted apart.

It prints the totals, a line per (set, quantity): how many values Revit
writes on objects rvt-rs also writes, how many are equal, how many differ
and how many are missing; the entities each missing quantity is on; the
values rvt-rs writes and Revit does not on objects both write, with up to
10 of their objects each and which of the set's quantities Revit gives it; and up to 40 differences, the largest relative ones
first.

Usage:

    python3 tools/re/qtos_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc> [--diagnostics <diagnostics.json>]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import json
import os
import sys

import ifcopenshell
import ifcopenshell.util.unit

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from psets_vs_ifc import IFC2X3_LENGTHS, key_of, property_key_of  # noqa: E402

# Where an IFC2X3 export writes a segment's length (#822, psets_vs_ifc's
# IFC2X3_LENGTHS): the IFC4 base quantity of the entity rvt-rs writes it as.
IFC4_LENGTH_QUANTITY = {
    "IfcPipeSegment": "Qto_PipeSegmentBaseQuantities",
    "IfcDuctSegment": "Qto_DuctSegmentBaseQuantities",
}

# Quantities Revit's export writes that are known to be wrong (RE-235 §5), by
# set and quantity, with what they are wrong on: a predefined type of Revit's
# object, or a class of an object of the same key. A curtain mullion's
# CrossSectionArea grows with its length and is never its section's area; a
# door's opening's Width is its vertical extent and its Height its horizontal
# one, where IFC4 defines a wall opening's the other way (RE-244). An
# opening is keyed by its Tag and whether something fills it (#1161), and
# carries the classes of what fills it, so the door's opening is still known.
# A wall's Length, which Revit's export writes as either its curve's length or
# that of its curve cut back at its joins, by how it draws the wall (RE-252).
# Where the two differ, nothing in the model says which, so a missing Length
# there is counted apart (the maintainer, 2026-10-26): rvt-rs's export
# diagnostics give the curve's, Revit's export's `Axis` the cut-back curve's.
# 1e-4 ft.
WALL_LENGTH = ("Qto_WallBaseQuantities", "Length")
TWO_LENGTHS_TOLERANCE_M = 1e-4 * 0.3048
# A wall's Width where the two exports give the wall different types
# (`ObjectType`), the Widths differ, and the wall's own native record names
# the type rvt-rs gives it (rvt-rs's diagnostics, `walls_type_confirmed`):
# the saved file and Revit's export disagree on its type, which the names
# census scores, and the Width follows from it. A held-out model's export
# gives one wall a type its saved file does not (RE-253), which the
# maintainer chose (2026-10-27) to count apart as an inconsistency of that
# pair; a wall whose type rvt-rs picks wrong is not confirmed and stays
# scored.
WALL_WIDTH = ("Qto_WallBaseQuantities", "Width")

REVIT_DEFECTS = {
    ("Qto_MemberBaseQuantities", "CrossSectionArea"): "MULLION",
    ("Qto_OpeningElementBaseQuantities", "Width"): "IfcDoor",
    ("Qto_OpeningElementBaseQuantities", "Height"): "IfcDoor",
}

KINDS = {
    "IfcQuantityLength": ("LengthValue", "LENGTHUNIT"),
    "IfcQuantityArea": ("AreaValue", "AREAUNIT"),
    "IfcQuantityVolume": ("VolumeValue", "VOLUMEUNIT"),
    "IfcQuantityWeight": ("WeightValue", "MASSUNIT"),
    "IfcQuantityTime": ("TimeValue", "TIMEUNIT"),
    "IfcQuantityCount": ("CountValue", None),
    "IfcQuantityNumber": ("NumberValue", None),
}


def scales(f):
    out = {}
    for unit in {u for _, u in KINDS.values() if u}:
        try:
            out[unit] = ifcopenshell.util.unit.calculate_unit_scale(f, unit)
        except Exception:
            out[unit] = None
    return out


def predefined_type(product):
    """An object's predefined type, or its type object's where its own is
    unset, NOTDEFINED or USERDEFINED."""
    own = getattr(product, "PredefinedType", None)
    if own not in (None, "NOTDEFINED", "USERDEFINED"):
        return own
    for rel in getattr(product, "IsTypedBy", None) or ():
        typed = getattr(rel.RelatingType, "PredefinedType", None)
        if typed not in (None, "NOTDEFINED", "USERDEFINED"):
            return typed
    return own


_OPENED = {}


def open_ifc(path):
    if path not in _OPENED:
        _OPENED[path] = ifcopenshell.open(path)
    return _OPENED[path]


def curve_lengths(path):
    """Object key -> each wall's curve length, metres, from rvt-rs's export
    diagnostics (`decoded.wall_curve_lengths_feet`, RE-252); empty when
    there is no such file."""
    if not os.path.exists(path):
        return {}
    with open(path, encoding="utf-8") as f:
        lengths = json.load(f).get("decoded", {}).get("wall_curve_lengths_feet", {})
    return {f"tag:{tag}": feet * 0.3048 for tag, feet in lengths.items()}


def confirmed_walls(path):
    """Object keys of the walls rvt-rs's export diagnostics confirm the type
    of (`decoded.walls_type_confirmed`, RE-253); empty without the file."""
    if not os.path.exists(path):
        return set()
    with open(path, encoding="utf-8") as f:
        ids = json.load(f).get("decoded", {}).get("walls_type_confirmed", [])
    return {f"tag:{tag}" for tag in ids}


def wall_types(path):
    """Wall key -> its `ObjectType`, its type as the export names it."""
    return {key_of(wall): wall.ObjectType for wall in open_ifc(path).by_type("IfcWall") if key_of(wall) and wall.ObjectType}


def axis_lengths(path):
    """Object key -> the length of a wall's straight `Axis`, metres: in
    Revit's export, the wall's curve cut back at its joins (RE-252)."""
    f = open_ifc(path)
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for wall in f.by_type("IfcWall"):
        key = key_of(wall)
        if not key or wall.Representation is None:
            continue
        for rep in wall.Representation.Representations:
            if rep.RepresentationIdentifier != "Axis":
                continue
            for item in rep.Items:
                if item.is_a("IfcIndexedPolyCurve") and not item.Segments:
                    points = [tuple(p[:2]) for p in item.Points.CoordList]
                elif item.is_a("IfcPolyline"):
                    points = [tuple(p.Coordinates[:2]) for p in item.Points]
                else:
                    continue
                if len(points) == 2:
                    (x0, y0), (x1, y1) = points
                    out[key] = ((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5 * scale
    return out


# A slab's Perimeter where Revit's export writes it in feet, unscaled: its raw
# value, read as feet, is the perimeter of the profile Revit's own body for it
# extrudes (RE-235 §5). Counted apart only on a slab where that holds, within
# a relative 1e-4 (the maintainer, 2026-10-26), and only in a file whose length
# unit is not the foot, where such a value is right; any other is scored.
SLAB_PERIMETER = ("Qto_SlabBaseQuantities", "Perimeter")
FOOT_METRES = 0.3048


def _outline_length(curve):
    """A closed outline's length in file units, for a polyline or an indexed
    poly curve of straight segments; None for another curve."""
    if curve.is_a("IfcPolyline"):
        points = [tuple(p.Coordinates[:2]) for p in curve.Points]
    elif curve.is_a("IfcIndexedPolyCurve") and not curve.Segments:
        points = [tuple(p[:2]) for p in curve.Points.CoordList]
    else:
        return None
    if len(points) < 3:
        return None
    if points[0] != points[-1]:
        points = points + [points[0]]
    return sum(((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2) ** 0.5 for a, b in zip(points, points[1:]))


def _extrusions(item):
    if item.is_a("IfcExtrudedAreaSolid"):
        return [item]
    if item.is_a("IfcBooleanResult"):
        return _extrusions(item.FirstOperand)
    if item.is_a("IfcMappedItem"):
        return [e for i in item.MappingSource.MappedRepresentation.Items for e in _extrusions(i)]
    return []


def slab_profile_perimeters(path):
    """Slab key -> the perimeter of the one profile its body extrudes, metres."""
    f = open_ifc(path)
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for slab in f.by_type("IfcSlab"):
        key = key_of(slab)
        if not key or slab.Representation is None:
            continue
        solids = [
            e
            for rep in slab.Representation.Representations
            if rep.RepresentationIdentifier == "Body"
            for item in rep.Items
            for e in _extrusions(item)
        ]
        if len(solids) != 1:
            continue
        area = solids[0].SweptArea
        if area.is_a("IfcRectangleProfileDef"):
            length = 2 * (area.XDim + area.YDim)
        elif area.is_a("IfcArbitraryClosedProfileDef"):
            length = _outline_length(area.OuterCurve)
        else:
            length = None
        if length:
            out[key] = length * scale
    return out


def quantities(path, entities_of=None):
    """(object key, set, quantity) -> (entity, SI value), the keys of every
    object, each object's entity by key, and how many quantities carry
    their own unit. In an IFC2X3 file, a segment length in a property set
    is keyed as the IFC4 base quantity of the entity `entities_of` (the
    other file's) gives the object."""
    f = open_ifc(path)
    unit_scale = scales(f)
    out = {}
    objects = set()
    entities = {}
    predefined = {}
    own_unit = 0
    if f.schema == "IFC2X3" and entities_of is not None:
        for rel in f.by_type("IfcRelDefinesByProperties"):
            definition = rel.RelatingPropertyDefinition
            if not definition.is_a("IfcPropertySet"):
                continue
            for obj in rel.RelatedObjects:
                key = property_key_of(obj)
                qset = IFC4_LENGTH_QUANTITY.get(entities_of.get(key))
                if qset is None:
                    continue
                for prop in definition.HasProperties or ():
                    if (definition.Name, prop.Name) in IFC2X3_LENGTHS and prop.is_a("IfcPropertySingleValue"):
                        raw = prop.NominalValue.wrappedValue if prop.NominalValue else None
                        scale = unit_scale.get("LENGTHUNIT")
                        value = None if raw is None or scale is None else float(raw) * scale
                        out.setdefault((key, qset, "Length"), ("IfcQuantityLength", value))
    for product in f.by_type("IfcObject"):
        key = property_key_of(product)
        if key:
            objects.add(key)
            entities[key] = product.is_a()
            classes = predefined.setdefault(key, set())
            classes.update({predefined_type(product), product.is_a()})
            if product.is_a("IfcOpeningElement"):
                classes.update(rel.RelatedBuildingElement.is_a() for rel in product.HasFillings or ())
    for rel in f.by_type("IfcRelDefinesByProperties"):
        definition = rel.RelatingPropertyDefinition
        if not definition.is_a("IfcElementQuantity") or not (definition.Name or "").startswith("Qto_"):
            continue
        for obj in rel.RelatedObjects:
            key = property_key_of(obj)
            if not key:
                continue
            for q in definition.Quantities or ():
                kind = KINDS.get(q.is_a())
                if kind is None:
                    continue
                if getattr(q, "Unit", None) is not None:
                    own_unit += 1
                    continue
                attribute, unit = kind
                raw = getattr(q, attribute)
                scale = unit_scale.get(unit) if unit else 1.0
                value = None if raw is None or scale is None else float(raw) * scale
                out[(key, definition.Name, q.Name)] = (q.is_a(), value)
    base = any(
        (q.Name or "").startswith("Qto_") or q.Name == "BaseQuantities" for q in f.by_type("IfcElementQuantity")
    )
    return out, objects, entities, own_unit, base, predefined


def relative(a, b):
    (_, x), (_, y) = a, b
    if x is None or y is None:
        return float("inf")
    return abs(x - y) / max(1e-9, abs(x), abs(y))


def same(a, b):
    (kind_a, x), (kind_b, y) = a, b
    if kind_a != kind_b or x is None or y is None:
        return kind_a == kind_b and x == y
    return abs(x - y) <= max(1e-6, 1e-5 * max(abs(x), abs(y)))


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    diagnostics = None
    if "--diagnostics" in args:
        at = args.index("--diagnostics")
        diagnostics = args[at + 1] if at + 1 < len(args) else None
        del args[at : at + 2]
        if diagnostics is None:
            print(__doc__, file=sys.stderr)
            return 2
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours, our_objects, our_entities, our_own, _, _ = quantities(args[0])
    theirs, their_objects, their_entities, their_own, their_base, their_predefined = quantities(args[1], our_entities)
    rows = collections.defaultdict(lambda: [0, 0, 0, 0])
    missing_classes = collections.defaultdict(collections.Counter)
    differences = []
    defects = collections.Counter()
    unwritten_openings = collections.Counter()
    beside = os.path.join(os.path.dirname(os.path.abspath(args[0])), "diagnostics.json")
    our_curves = curve_lengths(diagnostics or beside)
    confirmed = confirmed_walls(diagnostics or beside)
    their_axes = axis_lengths(args[1])
    # rvt-rs's curve lengths are trusted to count walls apart only where
    # Revit's export never contradicts them: on every wall, Revit's Length
    # is either that curve's length or its own Axis's (RE-252). A wall
    # whose Length is neither shows a curve read wrong, and then no wall is
    # counted apart.
    curve_checks = collections.Counter()
    for (key, qset, name), (_, value) in theirs.items():
        if (qset, name) != WALL_LENGTH or value is None or key not in our_curves:
            continue
        near = lambda other: other is not None and abs(other - value) <= TWO_LENGTHS_TOLERANCE_M
        if near(our_curves[key]):
            curve_checks["curve"] += 1
        elif near(their_axes.get(key)):
            curve_checks["axis"] += 1
        else:
            curve_checks["neither"] += 1
    trust_curves = curve_checks["neither"] == 0
    two_lengths = 0
    their_slab_perimeters = slab_profile_perimeters(args[1])
    their_length_scale = ifcopenshell.util.unit.calculate_unit_scale(open_ifc(args[1]))
    # In a file whose length unit is the foot, a perimeter in feet is right.
    feet_file = abs(their_length_scale - FOOT_METRES) <= 1e-9
    our_types, their_types = wall_types(args[0]), wall_types(args[1])
    other_types = 0
    unconfirmed_types = 0
    for (key, qset, name), value in sorted(theirs.items()):
        if key not in our_objects:
            if key.startswith("opening:"):
                unwritten_openings[(qset, name)] += 1
            continue
        scope = REVIT_DEFECTS.get((qset, name))
        if scope is not None and scope in their_predefined.get(key, ()):
            defects[(qset, name)] += 1
            continue
        if trust_curves and (qset, name) == WALL_LENGTH and (key, qset, name) not in ours:
            curve, axis = our_curves.get(key), their_axes.get(key)
            if curve is not None and axis is not None and abs(curve - axis) > TWO_LENGTHS_TOLERANCE_M:
                two_lengths += 1
                continue
        if (
            (qset, name) == SLAB_PERIMETER
            and not feet_file
            and value[1] is not None
            and key in their_slab_perimeters
        ):
            as_feet = value[1] / their_length_scale * FOOT_METRES
            outline = their_slab_perimeters[key]
            if abs(as_feet - outline) <= 1e-4 * max(abs(as_feet), abs(outline)):
                defects[(qset, name)] += 1
                continue
        if (qset, name) == WALL_WIDTH:
            mine = ours.get((key, qset, name))
            ours_type, theirs_type = our_types.get(key), their_types.get(key)
            if mine is not None and not same(mine, value) and ours_type and theirs_type and ours_type != theirs_type:
                if key in confirmed:
                    other_types += 1
                    continue
                unconfirmed_types += 1
        row = rows[(qset, name)]
        row[0] += 1
        mine = ours.get((key, qset, name))
        if mine is None:
            row[3] += 1
            missing_classes[(qset, name)][their_entities.get(key, "-")] += 1
        elif same(mine, value):
            row[1] += 1
        else:
            row[2] += 1
            differences.append((relative(mine, value), key, qset, name, mine, value))
    totals = [sum(r[i] for r in rows.values()) for i in range(4)]
    extra = collections.Counter(
        (qset, name) for key, qset, name in ours if key in their_objects and (key, qset, name) not in theirs
    )
    setting_off = collections.Counter() if their_base else extra
    if not their_base:
        extra = collections.Counter()
    print(
        f"Revit quantities on objects rvt-rs writes: {totals[0]}; equal: {totals[1]}; "
        f"different: {totals[2]}; missing: {totals[3]}; rvt-rs quantities Revit does not write: {sum(extra.values())}"
    )
    if not their_base:
        print(
            "Revit's export writes no base quantities (its setup's Export base quantities off, RE-242): "
            f"rvt-rs writes {sum(setting_off.values())}, which rvt-ifc --no-base-quantities leaves out"
        )
        for (qset, name), n in sorted(setting_off.items()):
            print(f"  {qset}.{name}: {n}")
    for (qset, name), n in sorted(defects.items()):
        print(f"Revit values known to be wrong, left out (RE-235 §5): {qset}.{name} {n}")
    if unwritten_openings:
        print(
            "Revit quantities on openings rvt-rs does not write (a door or wall cut in a further host,"
            f" a curtain wall's or cabinet's cut, #1165), not scored: {sum(unwritten_openings.values())}"
        )
        for (qset, name), n in sorted(unwritten_openings.items()):
            print(f"  {qset}.{name}: {n}")
    print(
        "walls whose curve and its cut-back curve differ, either of which Revit's export may write "
        f"as their Length, counted apart (RE-252): {two_lengths}"
    )
    print(
        f"walls whose Length is rvt-rs's curve's: {curve_checks['curve']}; Revit's Axis's: {curve_checks['axis']}; "
        f"neither: {curve_checks['neither']}{'' if trust_curves else ' (none counted apart)'}"
    )
    print(f"walls the two exports give different types, their differing Width counted apart (RE-253): {other_types}")
    print(f"walls the two exports give different types whose record does not confirm rvt-rs's, their Width scored (RE-253): {unconfirmed_types}")
    if our_own or their_own:
        print(f"quantities with their own unit, not scored: rvt-rs {our_own}, Revit {their_own}")
    for (qset, name), (n, equal, different, missing) in sorted(rows.items()):
        print(f"  {qset}.{name}: {n}, equal {equal}, different {different}, missing {missing}")
    print("entities Revit writes each missing quantity on:")
    for (qset, name), classes in sorted(missing_classes.items(), key=lambda kv: -sum(kv[1].values())):
        on = ", ".join(f"{c} x{k}" for c, k in classes.most_common(4))
        print(f"  {qset}.{name}: {sum(classes.values())}; on {on}")
    print("rvt-rs quantities Revit does not write:")
    their_sets = collections.defaultdict(set)
    for key, qset, name in theirs:
        their_sets[(key, qset)].add(name)
    for (qset, name), n in sorted(extra.items()):
        print(f"  {qset}.{name}: {n}")
        keys = sorted(
            key for key, s, q in ours if (s, q) == (qset, name) and key in their_objects and (key, s, q) not in theirs
        )
        for key in keys[:10]:
            given = ", ".join(sorted(their_sets.get((key, qset), ()))) or "none"
            print(f"    {key} {their_entities[key]}: Revit's {qset}: {given}")
    print("largest differences (rvt-rs, Revit; SI units):")
    for _, key, qset, name, mine, value in sorted(differences, key=lambda d: -d[0])[:40]:
        print(f"  {key} {qset}.{name}: {mine[0]} {mine[1]}, {value[0]} {value[1]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
