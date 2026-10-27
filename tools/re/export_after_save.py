#!/usr/bin/env python3
"""How long after the model was saved Revit's IFC export was made, and
whether the export names wall types the saved file does not have (#1048,
RE-253).

Reads the `saved <time>` line `examples/probe_re253_wall_type_refs.rs`
prints (the file's `ProjectInformation` Atom `updated`, UTC) and the time in
the export's `FILE_NAME` header (the exporting machine's local time, with no
zone), and prints the export's time less the save's, in hours. An export
made before the model was last saved may describe another state of it; a
difference within 14 hours either way is within the time zones. Revit
exports the model as it is open, saved or not, so an export can also show
changes made after the last save: it then prints how many of the export's
wall types (each `IfcWallType`'s name after its family's, the text after
the first `:`) are not among the names the file's Walls type records give
themselves (the probe's `walltype <name>` lines), and how many of its walls
are of such a type. Only counts are printed.

Usage:

    python3 tools/re/export_after_save.py <probe.txt> <revit.ifc> [--list]
"""

import datetime
import re
import sys

import ifcopenshell
import ifcopenshell.util.element


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    saved = None
    file_types = set()
    for line in open(args[0], encoding="utf-8", errors="replace"):
        if line.startswith("saved ") and line.split()[1] != "-":
            saved = datetime.datetime.fromisoformat(line.split()[1].replace("Z", "+00:00"))
        elif line.startswith("walltype "):
            file_types.add(line[len("walltype ") :].rstrip("\n"))
    f = ifcopenshell.open(args[1])
    absent = set()
    types = f.by_type("IfcWallType")
    for wall_type in types:
        name = (wall_type.Name or "").split(":", 1)[-1]
        if name not in file_types:
            absent.add(wall_type.id())
    walls = 0
    for wall in f.by_type("IfcWall"):
        wall_type = ifcopenshell.util.element.get_type(wall)
        if wall_type is not None and wall_type.id() in absent:
            walls += 1
    print(f"export wall types not in the file: {len(absent)} of {len(types)}; walls of them: {walls}")
    exported = None
    with open(args[1], encoding="utf-8", errors="replace") as f:
        head = f.read(4096)
    match = re.search(r"FILE_NAME\s*\(\s*'(?:[^']|'')*'\s*,\s*'([0-9T:.-]+)'", head)
    if match:
        exported = datetime.datetime.fromisoformat(match.group(1))
    if saved is None or exported is None:
        print(f"export time vs save: not read ({'no save time' if saved is None else 'no export time'})")
        return 0
    hours = (exported.replace(tzinfo=None) - saved.replace(tzinfo=None)).total_seconds() / 3600
    print(f"export time vs save: export less save in hours: {hours:.1f}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
