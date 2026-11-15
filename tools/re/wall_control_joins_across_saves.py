#!/usr/bin/env python3
"""Whether a Revit 2015 to 2023 wall record holds the control joins the same
project's later save holds (#1584, RE-364 section 10).

No Revit export of a 2015 to 2023 sample save is local, so the witness is the
same Autodesk sample project saved by a later release, wall by wall by
ElementId. `probe_1584_wall_control_joins` prints one line per wall of a save
(its `VWallDriver.m_controlJoinsSet`, `CDJoinInfo` and `JoinEndGStep`
objects); Measure writes it to `<model>/probe.txt` for every sample save.
This script reads those files and prints, for each pair, how many walls hold
the same:

  sets      the control joins (`sets=`: per join, the walls and their ends)
  ids       the number of entries whose ElementId decoded (`control_ids=`)
  cdjoin    the `CDJoinInfo` objects and the entries in their `m_info`
  steps     the `JoinEndGStep` objects and the entries in their
            `m_exposedFaceMap` (`endsteps=`, `exposed=`)

and, for the walls whose sets differ, how many name a wall the later save does
not. None means the later save only gains partners, which is what an edit that
joins more walls does and what an earlier read that misses entries does too:
the two cannot be told apart from these files.

Both saves are read by the build under test, so what this cannot catch is a
read the two saves share. Counts and ElementIds only.

Usage: wall_control_joins_across_saves.py DIR
  DIR holds the `sample-<year>-<project>/probe.txt` directories of a Measure
  run with `-f samples=true -f probe=probe_1584_wall_control_joins`.
"""

import re
import sys
from pathlib import Path

# (earlier save, later save): the sample project's name in each year's file.
PAIRS = [
    ("2015-rac_advanced_sample_project", "2024-racadvancedsampleproject"),
    ("2019-rac_advanced_sample_project", "2024-racadvancedsampleproject"),
    ("2020-racadvancedsampleproject", "2024-racadvancedsampleproject"),
    ("2021-racadvancedsampleproject", "2024-racadvancedsampleproject"),
    ("2022-racadvancedsampleproject", "2024-racadvancedsampleproject"),
    ("2023-racadvancedsampleproject", "2024-racadvancedsampleproject"),
    ("2024-racadvancedsampleproject", "2027-racadvancedsampleproject"),
    ("2015-rac_basic_sample_project", "2023-racbasicsampleproject"),
    ("2019-rac_basic_sample_project", "2023-racbasicsampleproject"),
    ("2020-racbasicsampleproject", "2023-racbasicsampleproject"),
    ("2022-racbasicsampleproject", "2023-racbasicsampleproject"),
    ("2016-rst_basic_sample_project", "2024-rstbasicsampleproject"),
    ("2022-rstbasicsampleproject", "2024-rstbasicsampleproject"),
    ("2023-rstbasicsampleproject", "2024-rstbasicsampleproject"),
    ("2019-rme_basic_sample_project", "2023-rmebasicsampleproject"),
    ("2019-rme_advanced_sample_project", "2024-rmeadvancedsampleproject"),
]


def load(directory: Path, model: str) -> dict[int, dict]:
    """Each wall line of a save's probe output, keyed by ElementId."""
    walls: dict[int, dict] = {}
    for line in (directory / f"sample-{model}" / "probe.txt").read_text().splitlines():
        if not line.startswith("wall ") or "sets=" not in line:
            continue
        head, sets = line.split("sets=", 1)
        fields = head.split()
        values = dict(t.split("=", 1) for t in fields[3:] if "=" in t)
        walls[int(fields[1])] = {
            "sets": sets.strip(),
            "ids": values.get("control_ids"),
            "cdjoin": values.get("cdjoin"),
            "steps": (values.get("endsteps"), values.get("exposed")),
            "named": frozenset(int(n) for n in re.findall(r"\[(\d+),\d+\]", sets)),
        }
    return walls


def main(directory: Path) -> None:
    for earlier, later in PAIRS:
        a, b = load(directory, earlier), load(directory, later)
        common = sorted(set(a) & set(b))
        same = {key: sum(1 for i in common if a[i][key] == b[i][key]) for key in ("sets", "ids", "cdjoin", "steps")}
        differing = [i for i in common if a[i]["sets"] != b[i]["sets"]]
        earlier_only = sum(1 for i in differing if a[i]["named"] - b[i]["named"])
        print(
            f"{earlier} vs {later}: walls {len(a)}/{len(b)}, in both {len(common)}; "
            f"same sets {same['sets']}, ids {same['ids']}, cdjoin {same['cdjoin']}, steps {same['steps']}; "
            f"differing sets {len(differing)}, of which name a wall the later save does not {earlier_only}"
        )


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(Path(sys.argv[1]))
