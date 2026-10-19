#!/usr/bin/env python3
"""Research (office pieces): the Tags more than one element carries, in
rvt-rs's export and in Revit's, by class, with each such Tag's element
counts and names in both files.

Usage: python3 tools/re/repeated_tags_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>
"""

import collections
import sys

import ifcopenshell


def by_tag(path):
    f = ifcopenshell.open(path)
    tags = collections.defaultdict(list)
    for element in f.by_type("IfcElement"):
        tag = getattr(element, "Tag", None)
        if tag:
            tags[tag].append((element.is_a(), element.Name or ""))
    return tags


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = by_tag(argv[1]), by_tag(argv[2])
    for label, tags in (("rvt-rs", ours), ("Revit", revit)):
        repeated = {tag: elements for tag, elements in tags.items() if len(elements) > 1}
        classes = collections.Counter(elements[0][0] for elements in repeated.values())
        print(f"{label}: {len(repeated)} repeated Tags, {sum(map(len, repeated.values()))} elements")
        for cls, count in classes.most_common():
            print(f"  {cls}: {count}")
    print("Tags repeated in either file (Tag: rvt-rs elements | Revit elements):")
    for tag in sorted(set(t for t, e in ours.items() if len(e) > 1) | set(t for t, e in revit.items() if len(e) > 1)):
        o, r = ours.get(tag, []), revit.get(tag, [])
        print(f"  {tag}: {len(o)} {sorted(set(o))} | {len(r)} {sorted(set(r))}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
