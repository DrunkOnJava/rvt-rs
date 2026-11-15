#!/usr/bin/env python3
"""Whether a Revit 2015 to 2023 family type's geometry-material map names the
materials the same project's 2024 save names (#1603, RE-473).

No Revit export of a 2015 to 2023 sample save is local, so the witness is the
same Autodesk sample project saved by a later release, type by type by
ElementId. `probe_1603_type_maps_pre2024` prints one `type map:` line for each
family type of a save whose record holds `m_geomTag2MaterialId` (its classes,
the number of entries, whether any value is -1 and the materials its entries
name); Measure writes them to `<model>/probe.txt` for every sample save. This
script reads those files, pairs each save before 2024 with the same project's
2024 save (a project with none is paired with its latest save), and prints for
each pair how many types are in both saves and how many of those name the same
materials, other materials, and, by class of instance, the same for the classes
named in CLASSES.

A material is an ElementId, and a project keeps its materials' ids across
saves. Both saves are read by the build under test, so what this cannot catch
is a read the two saves share, and a type edited between the saves looks like a
read that missed. Counts only.

Usage: type_maps_across_saves.py DIR
  DIR holds the `sample-<year>-<project>/probe.txt` directories of a Measure
  run with `-f samples=true -f probe=probe_1603_type_maps_pre2024`.
"""

import re
import sys
from collections import defaultdict
from pathlib import Path

CLASSES = ("Window", "Door", "Furniture", "Casework", "FurnitureSystem", "LightingFixture")

LINE = re.compile(r"^type map: (\d+) classes (\S*) entries (\d+) unset (true|false) named (\S*)$")


def project(model: str) -> tuple[int, str]:
    """The year and the project's name without separators, from `sample-2019-rac_basic_sample_project`."""
    _, year, name = model.split("-", 2)
    return int(year), name.replace("_", "")


def load(path: Path) -> dict[int, tuple[frozenset[int], frozenset[str], bool]]:
    types = {}
    for line in path.read_text(errors="replace").splitlines():
        found = LINE.match(line)
        if found:
            type_id, classes, _, unset, named = found.groups()
            types[int(type_id)] = (
                frozenset(int(n) for n in named.split(",") if n),
                frozenset(c for c in classes.split(",") if c),
                unset == "true",
            )
    return types


def main(directory: Path) -> None:
    saves = defaultdict(dict)
    for sub in sorted(directory.glob("sample-*/probe.txt")):
        year, name = project(sub.parent.name)
        if "snowdon" not in name:
            saves[name][year] = load(sub)
    total = defaultdict(lambda: [0, 0, 0])
    for name, years in sorted(saves.items()):
        later_year = 2024 if 2024 in years else max(years)
        for year in sorted(y for y in years if y < later_year):
            a, b = years[year], years[later_year]
            common = sorted(set(a) & set(b))
            same = sum(1 for t in common if a[t][0] == b[t][0])
            print(
                f"{year} vs {later_year} {name}: types with a map {len(a)}/{len(b)}, in both {len(common)}; "
                f"same materials {same}, other materials {len(common) - same}, "
                f"earlier only {len(set(a) - set(b))}, later only {len(set(b) - set(a))}"
            )
            for t in common:
                for cls in a[t][1] & set(CLASSES):
                    row = total[cls]
                    row[0] += 1
                    row[1] += a[t][0] == b[t][0]
                    row[2] += a[t][0] != b[t][0]
    print("by class over every pair: class, types in both, same materials, other materials")
    for cls in CLASSES:
        print(f"  {cls}: {total[cls][0]}, {total[cls][1]}, {total[cls][2]}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(Path(sys.argv[1]))
