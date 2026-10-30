#!/usr/bin/env python3
"""Which number of a room's native record or saved parameters is each of
Revit's space base quantities (#1016, RE-255).

`wall_numbers_vs_qtos.py` for `probes/examples/probe_re255_space_numbers.rs`: it
reads the probe's `rec` lines and its `roomname <tag> <name>` lines, which
give the `IfcSpace` `Name` the quantity census keys a space by, and looks
for each of Revit's `Qto_SpaceBaseQuantities` among the room's numbers.

Usage:

    python3 tools/re/space_numbers_vs_qtos.py <probe.txt> <revit.ifc> [--list]
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wall_numbers_vs_qtos  # noqa: E402


def tags_by_name(probe):
    names = {}
    for line in open(probe, encoding="utf-8", errors="replace"):
        if line.startswith("roomname "):
            _, tag, *name = line.rstrip("\n").split(" ")
            names[" ".join(name)] = tag
    return lambda key: names.get(key.removeprefix("IfcSpace:"), "")


if __name__ == "__main__":
    sys.exit(wall_numbers_vs_qtos.main("Qto_SpaceBaseQuantities", tags_by_name, "rooms"))
