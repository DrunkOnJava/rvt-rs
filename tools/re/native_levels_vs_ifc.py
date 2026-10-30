#!/usr/bin/env python3
"""Score the Level and the phases an element's native record names against
the storey Revit's own IFC export puts the element in.

Research tool for `reports/element-framing/RE-281-native-admission.md` (#879).

Reads `probe_re281_native_levels`' `element <id> <class> <assoc level>
<created phase> <demolished phase>` lines and Revit's export, pairs the
elements by `Tag` (the ElementId), and prints counts only, in the
`native levels:` lines `tools/ci/scrub-holdout-results.sh` keeps for a model
without a licence: never an ElementId or a name.

An element's storey is the `IfcBuildingStorey` its
`IfcRelContainedInSpatialStructure` (or the whole it is aggregated under)
names, as `storeys_vs_ifc.py` reads it. The Level a record names is
a storey's when every element naming it is in one storey; two Levels naming
one storey would be two Levels at one elevation.

Usage:

    python3 tools/re/native_levels_vs_ifc.py <probe output> <revit-export.ifc> [--list]
"""

import collections
import os
import sys

import ifcopenshell

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import storeys_vs_ifc  # noqa: E402


def paired_tags(paired, class_of, klass):
    return [t for t, _ in paired if class_of.get(t) == klass]


def footing_elevations(model, storey_of, class_of):
    import ifcopenshell.geom
    import ifcopenshell.util.placement
    import ifcopenshell.util.unit

    scale = ifcopenshell.util.unit.calculate_unit_scale(model)
    storeys = {}
    for storey in model.by_type("IfcBuildingStorey"):
        if storey.Elevation is not None and storey.Name:
            storeys[storey.Name] = float(storey.Elevation) * scale
    order = sorted(storeys.items(), key=lambda item: item[1])
    settings = ifcopenshell.geom.settings()
    settings.set(settings.USE_WORLD_COORDS, True)
    rows = []
    for entity in model.by_type("IfcFooting"):
        tag = getattr(entity, "Tag", None)
        where = storey_of.get(tag)
        if where not in storeys:
            continue
        place = float(ifcopenshell.util.placement.get_local_placement(entity.ObjectPlacement)[2][3]) * scale
        shape = ifcopenshell.geom.create_shape(settings, entity)
        zs = list(shape.geometry.verts[2::3])
        rows.append((where, place, min(zs), max(zs)))
    print(f"native levels: footings with an elevation {len(rows)} of {len(model.by_type('IfcFooting'))}, storeys {len(storeys)}")
    if not rows:
        return

    def at_or_below(z):
        below = [name for name, elevation in order if elevation <= z + 0.001]
        return below[-1] if below else order[0][0]

    def nearest(z):
        return min(order, key=lambda item: abs(item[1] - z))[0]

    for label, pick in (("storey at or below", at_or_below), ("nearest storey", nearest)):
        for what, index in (("base", 2), ("top", 3), ("placement", 1)):
            hit = sum(1 for row in rows if pick(row[index]) == row[0])
            print(f"native levels: footing {what} elevation, {label}: {hit} of {len(rows)} in Revit storey")
    lowest = order[0][0]
    print(f"native levels: footings in the lowest storey {sum(1 for row in rows if row[0] == lowest)} of {len(rows)}")
    print(f"native levels: footing storeys {len({row[0] for row in rows})}")
    print(
        "native levels: footing base above its storey elevation "
        f"{sum(1 for row in rows if row[2] >= storeys[row[0]] - 0.001)} of {len(rows)}, "
        f"top below it {sum(1 for row in rows if row[3] <= storeys[row[0]] + 0.001)} of {len(rows)}"
    )


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    elements = {}
    params = collections.defaultdict(dict)
    for line in open(args[0], errors="replace"):
        parts = line.split()
        if len(parts) == 6 and parts[0] == "element":
            elements[parts[1]] = (parts[2], int(parts[3]), int(parts[4]), int(parts[5]))
        elif len(parts) == 4 and parts[0] == "param":
            params[int(parts[2])].setdefault(parts[1], int(parts[3]))
        elif line.startswith("native levels: "):
            print(line.rstrip("\n"))
    model = ifcopenshell.open(args[1])
    storey_of = storeys_vs_ifc.storeys(model)
    class_of = {}
    for entity in model.by_type("IfcElement"):
        tag = getattr(entity, "Tag", None)
        if tag:
            class_of.setdefault(tag, entity.is_a())

    paired = [(tag, el) for tag, el in elements.items() if tag in storey_of]
    print(f"native levels: elements in the Revit export with a Tag {len(class_of)}, with a native record read {sum(1 for t in class_of if t in elements)}")
    named = [(tag, el) for tag, el in paired if el[1] != -1 and storey_of[tag] and not storey_of[tag].startswith("<")]
    print(f"native levels: elements on a storey in the Revit export {len(paired)}, of them naming a Level {len(named)}")

    # Each Level's storeys, over every element naming it.
    by_level = collections.defaultdict(collections.Counter)
    for tag, el in named:
        by_level[el[1]][storey_of[tag]] += 1
    split = [lv for lv, c in by_level.items() if len(c) > 1]
    print(f"native levels: Levels named {len(by_level)}, of them whose elements are in more than one storey {len(split)}")
    storeys_claimed = collections.defaultdict(set)
    for lv, c in by_level.items():
        for name in c:
            storeys_claimed[name].add(lv)
    shared = [name for name, lvs in storeys_claimed.items() if len(lvs) > 1]
    print(f"native levels: storeys holding elements of more than one Level {len(shared)} of {len(storeys_claimed)}")

    # An element agrees when its storey is the majority storey of its Level.
    majority = {lv: c.most_common(1)[0][0] for lv, c in by_level.items()}
    agree = collections.Counter()
    total = collections.Counter()
    for tag, el in named:
        klass = class_of.get(tag, "-")
        total[klass] += 1
        agree[klass] += int(storey_of[tag] == majority[el[1]])
    print(f"native levels: elements in the majority storey of their Level {sum(agree.values())} of {sum(total.values())}")
    for klass in sorted(total):
        print(f"native levels: class {klass} {agree[klass]} of {total[klass]}")

    # The footings (#879): the elements the exporter leaves off a storey.
    for klass in ("IfcFooting",):
        mine = [(t, el) for t, el in paired if class_of.get(t) == klass]
        naming = [(t, el) for t, el in mine if el[1] != -1]
        others = collections.defaultdict(collections.Counter)
        for tag, el in named:
            if class_of.get(tag) != klass:
                others[el[1]][storey_of[tag]] += 1
        print(f"native levels: {klass} on a storey {len(mine)}, naming a Level {len(naming)}")
        levels = collections.Counter(el[1] for _, el in naming)
        print(f"native levels: {klass} Levels named {len(levels)}, of them also named by another class {sum(1 for lv in levels if lv in others)}")
        with_others = [(t, el) for t, el in naming if el[1] in others]
        print(
            f"native levels: {klass} whose Level holds other elements, all in the footing storey "
            f"{sum(1 for t, el in with_others if set(others[el[1]]) == {storey_of[t]})} of {len(with_others)}"
        )
        mine_storeys = collections.Counter(storey_of[t] for t, _ in naming)
        print(f"native levels: {klass} storeys {len(mine_storeys)}")
        by_own = collections.defaultdict(set)
        for t, el in naming:
            by_own[el[1]].add(storey_of[t])
        print(f"native levels: {klass} Levels whose footings are in more than one storey {sum(1 for s in by_own.values() if len(s) > 1)}")
        taken = {n for lv, c in others.items() for n in c}
        print(f"native levels: {klass} storeys no other class Level names {len([n for n in mine_storeys if n not in taken])} of {len(mine_storeys)}")

    # Which ElementId parameter names a storey's Level: the Levels the Element
    # bases name map to storeys (above); a parameter whose value is one of
    # those Levels is a Level parameter when the element is in that storey.
    footings = paired_tags(paired, class_of, "IfcFooting")
    for pid in sorted(params):
        held = [(t, v) for t, v in params[pid].items() if t in storey_of and not storey_of[t].startswith("<")]
        known = [(t, v) for t, v in held if v in majority]
        if len(known) < 10:
            continue
        ok = sum(1 for t, v in known if storey_of[t] == majority[v])
        mine = [(t, v) for t, v in known if class_of.get(t) == "IfcFooting"]
        mine_ok = sum(1 for t, v in mine if storey_of[t] == majority[v])
        print(
            f"native levels: ElementId parameter {pid} holds a Level an Element base names on {len(known)} of {len(held)} elements, "
            f"{ok} of them in that Level storey; IfcFooting {len(mine)} of {len(footings)} hold one, {mine_ok} in its storey"
        )
        by_class = collections.defaultdict(lambda: [0, 0])
        for t, v in known:
            entry = by_class[class_of.get(t, "-")]
            entry[0] += 1
            entry[1] += int(storey_of[t] == majority[v])
        for klass, (n, k) in sorted(by_class.items()):
            print(f"native levels: ElementId parameter {pid} class {klass} {k} of {n}")
        # Every other element on a storey that holds the parameter: unset (-1),
        # or an ElementId that is not a Level an Element base names.
        rest = collections.defaultdict(list)
        for t, v in held:
            if v not in majority:
                rest[class_of.get(t, "-")].append(v)
        for klass, values in sorted(rest.items()):
            unset = sum(1 for v in values if v == -1)
            other = [v for v in values if v != -1]
            print(
                f"native levels: ElementId parameter {pid} class {klass} not a Level an Element base names {len(values)}, "
                f"unset {unset}, another ElementId {len(other)} with {len(set(other))} distinct values"
            )
        used = {v for _, v in mine}
        print(f"native levels: ElementId parameter {pid} IfcFooting Levels {len(used)}, of them a Level an Element base names {len([v for v in used if v in majority])}")

    # Where Revit puts the footings relative to the storeys' elevations, for a
    # footing that names no Level (#879): counts only.
    try:
        footing_elevations(model, storey_of, class_of)
    except Exception as error:  # noqa: BLE001
        print(f"native levels: footing elevations not measured {type(error).__name__}")

    # Phases: the export phase is the last of the project's (RE-184).
    phases = collections.Counter()
    for tag, el in elements.items():
        phases[(el[2] != -1, el[3] != -1)] += 1
    print(f"native levels: elements with a created phase {sum(n for (c, _), n in phases.items() if c)}, demolished in a phase {sum(n for (_, d), n in phases.items() if d)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
