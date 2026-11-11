#!/usr/bin/env python3
"""Compare two IFC exports of one model element by element (RE-416).

Pairs every `IfcElement` of the two files by GlobalId and compares its class, `Tag`, `Name` and every
property set and quantity set, with the step ids left out. Prints how many elements are only in one
file, how many differ, for each kind of difference the property or quantity names that were removed,
added or changed, the `Tag`s of the elements that differ, and the change in the number of entities of
each class (head minus base). A change that moves only some elements' quantities, and nothing else, reads
as exactly those elements here, which a scorer against Revit's export cannot show for a model Revit has
no export of.

Usage:

    python3 tools/re/ifc_element_diff.py <base.ifc> <head.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins). Measure keeps both sides' exports
with `-f keep_ifc=true` and an explicit `-f base=<sha>`.
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element as util


def scrub(value):
    """A property-set dictionary without the step ids IfcOpenShell puts in it."""
    if isinstance(value, dict):
        return {key: scrub(item) for key, item in value.items() if key != "id"}
    return value


def load(path):
    """GlobalId -> (class, Tag, Name, property and quantity sets), and the entity count by class."""
    model = ifcopenshell.open(path)
    elements = {}
    for element in model.by_type("IfcElement"):
        elements[element.GlobalId] = (element.is_a(), element.Tag, element.Name, scrub(util.get_psets(element)))
    return elements, collections.Counter(entity.is_a() for entity in model)


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    base, base_counts = load(sys.argv[1])
    head, head_counts = load(sys.argv[2])
    print(
        f"elements: base {len(base)}, head {len(head)}; only in base {len(set(base) - set(head))}, "
        f"only in head {len(set(head) - set(base))}"
    )
    changed = [gid for gid in sorted(base.keys() & head.keys()) if base[gid] != head[gid]]
    print(f"elements that differ: {len(changed)}")
    kinds = collections.Counter()
    tags = set()
    for gid in changed:
        (cls, tag, _, before), (_, _, _, after) = base[gid], head[gid]
        tags.add(str(tag))
        for pset in sorted(set(before) | set(after)):
            old, new = before.get(pset, {}), after.get(pset, {})
            if old == new:
                continue
            removed = tuple(sorted(set(old) - set(new)))
            added = tuple(sorted(set(new) - set(old)))
            moved = tuple(sorted(name for name in set(old) & set(new) if old[name] != new[name]))
            kinds[(cls, pset, removed, added, moved)] += 1
    for (cls, pset, removed, added, moved), count in kinds.most_common():
        print(f"{count:5}  {cls} {pset}: removed {list(removed)} added {list(added)} changed {list(moved)}")
    print("tags:", sorted(tags)[:40])
    delta = {
        name: head_counts[name] - base_counts[name]
        for name in set(head_counts) | set(base_counts)
        if head_counts[name] != base_counts[name]
    }
    print("entity count change (head minus base):", dict(sorted(delta.items())))
    return 0


if __name__ == "__main__":
    sys.exit(main())
