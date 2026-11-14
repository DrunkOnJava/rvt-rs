#!/usr/bin/env python3
"""Multistory stairs: the copies Revit writes on each storey, against what rvt-rs writes (#1977).

Revit's export writes a multistory stair once per storey under one `Tag`: the stair, its flights,
stringers, landings and railings again, the copies of the stair, flights, stringers and landings named
with a `:N` suffix. This reads both exports, finds the stairs Revit writes more than once, and counts
for each class of their parts how many entities Revit writes beyond rvt-rs's of the same Tag and class,
and, where rvt-rs writes the copy too, how it sits against Revit's: the name, the storey, the whole
it is aggregated under and the shift from the original, which does not depend on the frame the two files
are placed in (a copy is a vertical translation of one storey). Counts only.

    python3 tools/re/multistory_copies_vs_ifc.py <rvt-rs's export>.ifc <Revit's export>.ifc

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import numpy as np

PART_CLASSES = ("IfcStairFlight", "IfcMember", "IfcSlab", "IfcRailing")
SHIFT_TOLERANCE = 0.001  # metres


def whole_of(entity):
    for relation in getattr(entity, "Decomposes", None) or []:
        return relation.RelatingObject
    return None


def storey_of(entity):
    for relation in getattr(entity, "ContainedInStructure", None) or []:
        return relation.RelatingStructure.Name
    return None


def by_tag(model, classes):
    out = collections.defaultdict(list)
    for entity in model.by_type("IfcElement"):
        if entity.is_a() in classes:
            out[(entity.is_a(), entity.Tag)].append(entity)
    return out


def centre(entity, settings):
    shape = ifcopenshell.geom.create_shape(settings, entity)  # held: a chained .geometry reads back empty
    verts = np.array(shape.geometry.verts).reshape(-1, 3)
    if not len(verts):
        return None
    return (verts.min(0) + verts.max(0)) / 2


def multistory_family(model, multi=None):
    """The stairs the file writes more than once (or the Tags `multi` names), and every part
    aggregated under one of them."""
    stairs = by_tag(model, ("IfcStair",))
    if multi is None:
        multi = {tag for (_, tag), entities in stairs.items() if len(entities) > 1}
    stair_ids = {entity.id() for (_, tag), entities in stairs.items() if tag in multi for entity in entities}
    parts = collections.defaultdict(list)
    for entity in model.by_type("IfcElement"):
        if entity.is_a() not in PART_CLASSES:
            continue
        whole = whole_of(entity)
        if whole is not None and whole.id() in stair_ids:
            parts[(entity.is_a(), entity.Tag)].append(entity)
    return multi, stairs, parts


def main(rvt_path, revit_path):
    revit_model = ifcopenshell.open(revit_path)
    rvt_model = ifcopenshell.open(rvt_path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)

    revit_multi, revit_stairs, revit_parts = multistory_family(revit_model)
    rvt_multi, rvt_stairs, rvt_parts = multistory_family(rvt_model, revit_multi)
    rvt_twice = sum(1 for (_, tag), entities in rvt_stairs.items() if tag in revit_multi and len(entities) > 1)
    print(f"multistory stairs: Revit writes {len(revit_multi)} stairs more than once, rvt-rs {rvt_twice} of them")

    rows = collections.defaultdict(lambda: collections.Counter())
    groups = [("IfcStair", {k: v for k, v in revit_stairs.items() if k[1] in revit_multi}, {k: v for k, v in rvt_stairs.items() if k[1] in revit_multi})]
    for cls in PART_CLASSES:
        groups.append((cls, {k: v for k, v in revit_parts.items() if k[0] == cls}, {k: v for k, v in rvt_parts.items() if k[0] == cls}))

    for cls, revit_groups, rvt_groups in groups:
        row = rows[cls]
        for key, revit_entities in revit_groups.items():
            rvt_entities = rvt_groups.get(key, [])
            row["entities in Revit"] += len(revit_entities)
            row["entities in rvt-rs"] += len(rvt_entities)
            row["Revit beyond rvt-rs"] += max(0, len(revit_entities) - len(rvt_entities))
            row["rvt-rs beyond Revit"] += max(0, len(rvt_entities) - len(revit_entities))
            if len(revit_entities) >= 2 and len(rvt_entities) >= 2:
                row["Tags with the copy in both"] += 1
                ordered_revit = sorted(revit_entities, key=lambda e: e.Name or "")
                ordered_rvt = sorted(rvt_entities, key=lambda e: e.Name or "")
                copy_revit, copy_rvt = ordered_revit[-1], ordered_rvt[-1]
                row["copy name equal"] += int((copy_revit.Name or "") == (copy_rvt.Name or ""))
                row["copy storey equal"] += int(storey_of(copy_revit) == storey_of(copy_rvt))
                row["copy predefined type equal"] += int(getattr(copy_revit, "PredefinedType", None) == getattr(copy_rvt, "PredefinedType", None))
                row["copy has a type object like Revit's"] += int(
                    (ifcopenshell.util.element.get_type(copy_revit) is None) == (ifcopenshell.util.element.get_type(copy_rvt) is None)
                )
                whole_revit, whole_rvt = whole_of(copy_revit), whole_of(copy_rvt)
                row["copy whole equal"] += int(bool(whole_revit is not None and whole_rvt is not None and (whole_revit.Name or "") == (whole_rvt.Name or "")))
                shifts = []
                for ordered in (ordered_revit, ordered_rvt):
                    first, copy = centre(ordered[0], settings), centre(ordered[-1], settings)
                    shifts.append(None if first is None or copy is None else copy - first)
                if shifts[0] is not None and shifts[1] is not None:
                    row["shift measured"] += 1
                    row["shift within 1 mm"] += int(float(np.linalg.norm(shifts[0] - shifts[1])) <= SHIFT_TOLERANCE)

    total_beyond = 0
    for cls, row in rows.items():
        total_beyond += row["Revit beyond rvt-rs"]
        parts = ", ".join(f"{name} {count}" for name, count in row.items())
        print(f"  {cls}: {parts}")
    print(f"entities Revit writes beyond rvt-rs's on these stairs: {total_beyond}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
