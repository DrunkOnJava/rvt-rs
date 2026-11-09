#!/usr/bin/env python3
"""A curtain wall's class in a save, against the same project's 2024 save (RE-366).

    python3 tools/re/curtain_classes_across_saves.py SAMPLES_DIR

SAMPLES_DIR holds one directory per Autodesk sample, `sample-<year>-<name>`, each with the
`model.ifc` rvt-rs exported (what a Measure run with `keep_ifc` uploads). For each project with a
2024 save, each earlier save's walls are paired with the 2024 save's by `Tag`, which an ElementId
keeps across saves, and it prints how many are

  IfcCurtainWall in both    the walls the change turns into curtain walls that the 2024 save has as one,
  IfcCurtainWall here, IfcWall in 2024
                            a wrong promotion,
  IfcWall here, IfcCurtainWall in 2024
                            a curtain wall the reader misses,
  unpaired                  a wall with no single counterpart,

and how many mullions and plates sit under an `IfcCurtainWall` and under another wall, here and, for the
second, in the 2024 save. It writes nothing and reads no Revit export: a 2024 save is rvt-rs's own export,
so an error both saves share is not caught.
"""

import collections
import os
import re
import sys

import ifcopenshell
import ifcopenshell.util.element


def project(name):
    """(year, the project's name without separators): `sample-2023-racadvancedsampleproject` and
    `sample-2019-rac_advanced_sample_project` are one project."""
    m = re.match(r"sample-(\d{4})-(.*)$", name)
    return int(m.group(1)), re.sub(r"[_\s]", "", m.group(2)).replace("sampleproject", "")


def read(path):
    """Each wall's class by Tag, and the class of the whole each mullion and plate is aggregated under."""
    model = ifcopenshell.open(path)
    walls = collections.defaultdict(list)
    for wall in model.by_type("IfcWall") + model.by_type("IfcCurtainWall"):
        tag = getattr(wall, "Tag", None)
        if tag:
            walls[tag].append(wall.is_a())
    wholes = collections.Counter()
    for cls in ("IfcMember", "IfcPlate"):
        for part in model.by_type(cls):
            whole = ifcopenshell.util.element.get_aggregate(part)
            wholes[whole.is_a() if whole is not None else None] += 1
    return walls, wholes


def main(argv):
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    root = argv[1]
    projects = collections.defaultdict(dict)
    for name in sorted(os.listdir(root)):
        if name.startswith("sample-") and os.path.exists(f"{root}/{name}/model.ifc"):
            year, key = project(name)
            projects[key][year] = name
    total = collections.Counter()
    for key, years in sorted(projects.items()):
        if 2024 not in years:
            continue
        reference, reference_wholes = read(f"{root}/{years[2024]}/model.ifc")
        for year in sorted(y for y in years if y < 2024):
            walls, wholes = read(f"{root}/{years[year]}/model.ifc")
            counts = collections.Counter()
            for tag, classes in walls.items():
                other = reference.get(tag)
                if other is None or len(classes) != 1 or len(other) != 1:
                    counts["unpaired"] += 1
                else:
                    counts[(classes[0], other[0])] += 1
            curtain = counts[("IfcCurtainWall", "IfcCurtainWall")]
            print(
                f"{year} {key}: IfcCurtainWall in both {curtain}, here and IfcWall in 2024 "
                f"{counts[('IfcCurtainWall', 'IfcWall')]}, IfcWall here and IfcCurtainWall in 2024 "
                f"{counts[('IfcWall', 'IfcCurtainWall')]}, unpaired {counts['unpaired']}; mullions and plates under "
                f"an IfcCurtainWall {wholes['IfcCurtainWall']}, under an IfcWall {wholes['IfcWall']} "
                f"(2024: {reference_wholes['IfcWall']})"
            )
            total.update(counts)
            total["under_curtain"] += wholes["IfcCurtainWall"]
            total["under_wall"] += wholes["IfcWall"]
            total["saves"] += 1
    print(
        f"{total['saves']} saves: IfcCurtainWall in both {total[('IfcCurtainWall', 'IfcCurtainWall')]}, "
        f"here and IfcWall in 2024 {total[('IfcCurtainWall', 'IfcWall')]}, IfcWall here and IfcCurtainWall in 2024 "
        f"{total[('IfcWall', 'IfcCurtainWall')]}, unpaired {total['unpaired']}; mullions and plates under an "
        f"IfcCurtainWall {total['under_curtain']}, under an IfcWall {total['under_wall']}"
    )
    wrong = total[("IfcCurtainWall", "IfcWall")] + total[("IfcWall", "IfcCurtainWall")]
    return 1 if wrong or total["under_wall"] else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
